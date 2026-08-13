# CognyxOS Module Contracts

> **Document ID:** ARCH-005
> **Version:** 1.0.0
> **Status:** Phase 0 - Approved
> **Last Updated:** 2026-08-01
> **Owner:** Architecture Council

---

## Table of Contents

1. [Contract Template](#contract-template)
2. [Kernel Layer Contracts](#kernel-layer-contracts)
   2.1 Kernel Abstraction (HAL)
   2.2 IPC Framework
   2.3 Syscall Interface
3. [System Services Contracts](#system-services-contracts)
   3.1 Process Manager
   3.2 Workspace Manager
   3.3 Task Scheduler
   3.4 Context Engine (within AI Runtime, referenced here)
   3.5 Memory Manager (AI Memory)
   3.6 Permission System (Capability Layer)
   3.7 Filesystem Service
   3.8 Identity Manager
   3.9 Update Manager
   3.10 Security Service
   3.11 Networking Service
   3.12 Telemetry Service
   3.13 Logging Service
   3.14 Configuration Manager
   3.15 Device Manager
   3.16 State Manager
   3.17 Graphics Service
   3.18 Notification Service
   3.19 Search Service
   3.20 Indexing Service
4. [AI Runtime Contracts](#ai-runtime-contracts)
   4.1 LLM Engine
   4.2 Planning Engine
   4.3 Memory Manager (Semantic)
   4.4 Context Engine
   4.5 Embedding Service
   4.6 Agent Orchestrator
5. [Capability Layer Contracts](#capability-layer-contracts)
6. [Application Runtime Contracts](#application-runtime-contracts)
   6.1 App Runtime
   6.2 Container Runtime
   6.3 VM Manager
   6.4 Plugin Framework
7. [Workspace Layer Contracts](#workspace-layer-contracts)
8. [UI Layer Contracts](#ui-layer-contracts)
   8.1 Window System
   8.2 Graphics Layer
9. [Remote Interface Contracts](#remote-interface-contracts)
   9.1 Remote Control API
   9.2 Developer API
10. [Command Bus & IPC Framework Contracts](#command-bus--ipc-framework-contracts)
11. [Cloud Runtime Contracts (Future)](#cloud-runtime-contracts-future)

---

## Contract Template

Every module contract adheres to this structure:

```
## Module: <Name>

### Purpose
<1-2 sentences on core existence justification.

### Responsibilities
<Numbered list of exclusive responsibilities. This module owns these and ONLY these.

### Dependencies (Explicit
<Upstream Modules I depend on
<Downstream Modules depending on me

### Criticality
CRITICAL | HIGH | MEDIUM | LOW

### Public APIs (gRPC Service Definitions)
<protobuf service + messages>

### Internal APIs (Rust traits)
<Internal-only interfaces

### Security Concerns
<What security issues are non-obvious?>

### Performance Considerations
<Hot paths, SLAs, memory footprint, concurrency limits>

### Future Scalability
<How this scales to 10x, 100x load. Known bottlenecks.>

### Failure Modes & Recovery
<Top failure modes. How detected. How recovered.

### Module Lifecycle Interface
Trait Implementation Details

### Observability Requirements
Metrics, logs, traces that MUST emit.
```

---

## Kernel Layer Contracts

### 2.1 Module: Kernel Abstraction (HAL)

#### Purpose
Abstract Linux kernel functionality to provide a stable hardware abstraction for upper layers. Wraps Linux kernel subsystems into discoverable, typed interfaces. No business logic; purely a translation + thin wrapper layer.

#### Responsibilities
1. Device enumeration and discovery (udev event stream abstraction)
2. Device state machine: PROBED → BOUND → ACTIVE → SUSPENDED → REMOVED
3. Power state control per device (suspend, resume, reset)
4. Hardware feature detection and capability reporting (CPU features, GPU, etc.)
5. Interrupt routing info to user-space handlers (UIO, VFIO)
6. Firmware loading interface with signature verification

#### Dependencies
- Upstream: Linux Kernel 6.8+ kernel, udev, libudev
- Downstream: Device Manager, Graphics Service, Security Service

#### Criticality: HIGH

#### Public APIs
```protobuf
service HalService {
  rpc EnumerateDevices(EnumerateRequest) returns (stream DeviceInfo);
  rpc GetDeviceCapabilities(DeviceId) returns (DeviceCapabilities);
  rpc BindDriver(BindRequest) returns (google.protobuf.Empty);
  rpc UnbindDriver(DeviceId) returns (google.protobuf.Empty);
  rpc SetDevicePowerState(PowerStateRequest) returns (google.protobuf.Empty);
  rpc WatchDeviceEvents(WatchRequest) returns (stream HalDeviceEvent);
  rpc GetCpuInfo(CpuInfoRequest) returns (CpuInfo);
  rpc GetMemoryInfo(MemoryInfoRequest) returns (MemoryInfo);
  rpc LoadFirmware(FirmwareRequest) returns (FirmwareResult);
}
```

#### Internal APIs
```rust
#[async_trait]
trait HalBackend {
    async fn enumerate(&self, filter: DeviceFilter) -> Result<Vec<Device>>;
    async fn bind(&self, id: DeviceId, driver: &str) -> Result<()>;
    async fn power_state(&self, id: DeviceId, state: PowerState) -> Result<()>;
    async fn watch(&self) -> Pin<Box<dyn Stream<Item = HalEvent>>>;
}
```

#### Security Concerns
- Firmware loading: must validate Ed25519 signature against kernel firmware signing keys before writing to /sys. Never allow arbitrary firmware paths outside /lib/firmware.
- udev events: carefully filtered; udevd output is kernel→user escape, never user→kernel writes.

#### Performance Considerations
- Enumeration: <50ms for 100 devices.
- Event delivery (hotplug): <1ms to user-space on event.
- Maintain in-memory device cache. No sysfs hot path traversals.

#### Future Scalability
- Heterogeneous multi-socket NUMA topology awareness.
- GPU hotplug (future: CXL devices).
- Thunderbolt / USB4 dynamic tunnel abstraction.
- Remote-attached devices (Fabric).

#### Failure Modes
- udev socket failure: fallback to cached enumeration, flag degraded.

---

### 2.2 Module: IPC Framework

#### Purpose
Low-level inter-process communication primitives. Provides kernel-mediated channels with capability-tagged file descriptors. Higher than raw sockets, lower than message bus.

#### Responsibilities
1. Unix domain socket creation with `SO_PEERCRED` authentication
2. Shared memory (memfd) allocation, sealing, FD passing via `SCM_RIGHTS`
3. Credential validation: PID → Identity mapping broker
4. Rate limiting and priority queuing per channel
5. Channel encryption using per-session keys for cross-namespace IPC

#### Dependencies
- Upstream: Linux Kernel (Unix sockets, memfd, userfaultfd)
- Downstream: Message Bus, All Services

#### Criticality: CRITICAL

#### Public APIs
```protobuf
service IpcBroker {
  rpc CreateChannel(ChannelSpec) returns (ChannelHandle);
  rpc AuthenticateConnection(AuthChallenge) returns (AuthResponse);
  rpc PassFd(FdPassRequest) returns (FdHandle);
  rpc CreateMemfd(MemfdRequest) returns (MemfdHandle);
  rpc RegisterSharedMemory(ShmSpec) returns (ShmHandle);
}
```

#### Security Concerns
- Only message broker never passes FDs to PIDs it can't verify via `SO_PEERCRED`.
- IPC namespace crossing: mandatory session key wrapping.

#### Performance Considerations
- Channel creation: <1ms p99
- FD passing: <2µs for single FD
- Bulk transfer via memfd: zero copy

---

### 2.3 Module: Syscall Interface

#### Purpose
Per-sandbox seccomp-bpf policy compiler and loader.

#### Responsibilities
1. Compile high-level "syscall allow rules into BPF programs.
2. Load BPF programs via `seccomp() into processes.
3. Handle SECCOMP_RET_TRAP → forwarded to user-space handler.
4. Syscall audit trail generation.

#### Criticality: CRITICAL

#### Security Concerns
- BPF JIT spraying attacks: Always enforce constant blinding.
- Denylist vs allowlist: ALWAYS allowlist. Deny default.

---

## System Services Contracts

### 3.1 Module: Process Manager

See SystemDesign.md §1 for full contract.

Additional Lifecycle Interface:
```rust
#[async_trait]
impl ModuleLifecycle for ProcessManagerService {
    async fn start(&mut self) -> Result<()> { /* fork server, register pidfd cache }
    async fn health_check(&self) -> HealthStatus; /* check cgroup write + child reaper status */ }
    async fn shutdown(&mut self, deadline: Duration) -> Result<()>; /* gracefully SIGTERM tree, then SIGKILL after deadline */
}
```

Observability Required Metrics:
- `process_spawn_total{result}` per workspace_id, result=success|failure}` counter
- `process_live{workspace_id}` gauge
- `process_spawn_duration_seconds` histogram

---

### 3.2 Module: Workspace Manager

#### Purpose
Lifecycle of workspace isolation domains. Primary context boundary for all user-visible work.

#### Responsibilities (exclusive)
1. CRUD on workspaces, persistence on disk.
2. Namespace (6 namespace types: USER, PID, MNT, NET, UTS, IPC) creation and attachment
3. Cgroup subtree creation per workspace, initial limits
4. Workspace state machine transitions (INACTIVE → ACTIVE → HIBERNATED → ARCHIVED → DELETED)
5. Cross-workspace capability broker mediation request generation (user prompts)
6. Workspace cloning with deduplication using reflink Btrfs/ZFS

#### Dependencies
- Upstream: Process Manager, Capability Token Service, State Manager
- Downstream: AI Runtime, UI Shell, Application Runtimes

#### Criticality: CRITICAL

#### Public APIs
See SystemDesign.md §3.

#### Internal APIs
```rust
trait WorkspaceBackend {
    fn mount_namespaces(w: &Workspace) -> Result<NamespaceSet>;
    fn create_cgroup_subtree(id: WorkspaceId, limits: &ResourceLimits) -> Result<()>;
    fn hibernate_memory(w: &Workspace) -> Result<SnapshotPath>;
    fn restore_from_hibernate(snap: SnapshotPath) -> Result<Workspace>;
}
```

#### Security Concerns
- Namespace creation fails closed. Root in USER_NS never leaks host IDs.
- Cross-workspace messages: ALWAYS broker user confirmation; no side channels.
- Cgroup subtree control files never bind-read-only to workspace.

#### Performance
- Workspace activate (cold): <500ms (minimal empty workspace, 128M)
- Workspace hibernate: <1s for 2GB (ZRAM used
- Workspace clone: 100GB tree (reflink, O(1) metadata)

#### Scalability
- 100 simultaneously active workspaces (memory mapping 4GB RAM minimum)
- 10,000 total workspaces on disk, cold archived.

#### Failure Modes
- Namespace clone fails (ENOMEM): user alert, fall back to shared workspace.

---

### 3.3 Module: Task Scheduler

#### Purpose
Multi-queue, deadline-aware, dependency scheduling of AI plans, OS internal tasks, user-initiated background tasks.

#### Responsibilities
1. Submit tasks to workers (task graphs) and enforce QoS.
2. Deadline enforcement with EDF + WFQ composite.
3. Task cancellation, pause, resume.
4. Deadline misses reporting.

#### Criticality: HIGH

#### Public APIs
See SystemDesign.md §2.

#### Performance
- 10,000 concurrent tasks, 1ms queue depth per-queue.
- Task dispatch latency: <50µs worker-to-worker.

---

### 3.4 Module: Context Engine

Located in AI Runtime Layer. See AIArchitecture.md §5.

---

### 3.5 Module: Memory Manager (AI Semantic)

Located in AI Runtime Layer. See AIArchitecture.md §4.

---

### 3.6 Module: Permission System

The Capability Token Service + Policy Engine. See Security.md §3 + Permissions.md.

---

### 3.7 Module: Filesystem Service

#### Purpose
All file access mediation, VFS abstraction with capability-checked per-open virtual file operations, snapshots.

#### Responsibilities
1. All open() equivalent on any path → callers go through it.
2. Capability verification at file-descriptor-level per-open.
3. Snapshot create / restore
4. Watch events (inotify abstraction)
5. Indexing hooks into Indexing Service

#### Criticality: CRITICAL

#### Public APIs
See SystemDesign.md §4.

#### Security Concerns
- Symlink resolution: always resolve within workspace root; never ".." escape detection.
- Extended attributes: security.* namespace only SecurityContext may read, never write directly.

#### Performance
- stat() hot path: <10µs (cached metadata).
- Snapshot creation (1TB subvolume): <100ms.

---

### 3.8 Module: Identity Manager

#### Purpose
Cryptographic identities (users, services, devices, agents).

#### Responsibilities
1. Identity lifecycle (create, delete, lock, unlock)
2. Authentication factors (webauthn, password, totp, biometric, tpm)
3. Session lifecycle, short-lived tokens, revocation
4. Key backup via recovery seeds
5. Identity federation hooks (OIDC/OAuth2/SAML clients)

#### Criticality: CRITICAL

#### Public APIs
See SystemDesign.md §6.

#### Security Concerns
- Password hashing: Argon2id mem=64MB t=3.
- Hardware keys preferred.
- Biometric: never store raw biometrics; only matching on-device only (sensor→matcher signed result→Identity signs result.

---

### 3.9 Module: Update Manager

#### Purpose
Atomic A/B OS updates, rollback, delta download, firmware updates.

#### Responsibilities
1. OSTree manage sysroot deployments.
2. Signed delta downloads, Ed25519 manifest signatures.
3. A/B brownout detection.
4. Firmware via fwupd integration.
5. Workspace migration scripts between versions.

#### Criticality: HIGH

#### Public APIs
See SystemDesign.md §17.

---

### 3.10 Module: Security Service

#### Purpose
Integrity monitoring, attestation, intrusion detection.

#### Responsibilities
1. File integrity hash database + change detection (FIM)
2. TPM quote generation, PCR banks
3. IMA appraisal extension
4. eBPF-based intrusion detector hooks execve, connect security_socket_create
5. Secure Boot verification runtime checks

#### Criticality: HIGH

---

### 3.11 Module: Networking Service

#### Purpose
All network configuration, firewall, VPN, DNS.

#### Responsibilities
1. Per-workspace virtual ethernet bridge routing.
2. nftables backend ruleset firewall rules.
3. WireGuard / OpenVPN tunnel management.
4. DNS resolver DoT/DoH per workspace policies.
5. Network QoS, traffic shaping.
6. Transparent HTTP(S) inspection proxy with user consent.

#### Criticality: HIGH

---

### 3.12 Module: Telemetry Service

#### Purpose
Opt-in metrics, tracing.

#### Responsibilities
1. Prometheus exposition.
2. OpenTelemetry trace pipeline.
3. Error report aggregation.
4. Differential privacy noise on any off-device export.
5. Strictly opt-in consent everywhere.

#### Criticality: LOW

#### Security Tenet (Non-Negotiable)
Telemetry data NEVER leaves device without PER-CATEGORY explicit opt-in plus per-transmission consent.

---

### 3.13 Module: Logging Service

#### Purpose
Aggregation, query, rotation of logs security integrity chain.

#### Responsibilities
1. Structured ingestion with causation_id/correlation_id flow.
2. Per-service buffering batching for throughput.
3. Hash chaining for security-critical logs integrity verification queries.
4. Retention policy enforcement.

#### Criticality: HIGH

---

### 3.14 Module: Configuration Manager

#### Purpose
Hierarchical schema-validated configuration defaults → system → workspace → user.

#### Responsibilities
1. 4-layer config: DEFAULTS, SYSTEM, WORKSPACE, USER
2. Schema JSON validation per-key schema validation
3. Change notifications
4. Rollback history versions rollback to previous config version
5. Import / Export config sets

#### Criticality: HIGH

---

### 3.15 Module: Device Manager

#### Purpose
Hotplug, permissions, driver binding.

#### Responsibilities
1. udev events via HAL. Present device graph of devices.
2. Per workspace device access control device capabilities minting
3. GPU passthrough DRM lease + SR-IOV
4. USB authorization filtering per-device

#### Criticality: HIGH

---

### 3.16 Module: State Manager

#### Purpose
Global state store, transactions, watch subscriptions,CRDT sync future.

#### Responsibilities
1. Hierarchical key space
2. RocksDB backend, compactions, snapshotting with WAL.
3. Watch prefix watches / range queries
4. Transactions with CAS multi-key ACID transactions
5. Future CRDT for multi-node cloud state

#### Criticality: CRITICAL

---

### 3.17 Module: Graphics Service

#### Purpose
GPU scheduling, buffer allocation, display config, Wayland security filter.

#### Responsibilities
1. DRM-KMS display output model
2. GPU scheduling (time-sliced processes
3. Buffer (DMA-BUF) sharing with cap
4. Wayland socket provision
5. Hardware video encode/decode caps

#### Criticality: HIGH

---

### 3.18 Module: Notification Service

#### Purpose
System notifications across channels.

#### Responsibilities
1. Notify priority routed notifs shell email (opt-in) channels
2. notification lifecycle dismissals, group summarization AI assistance triage.
3. Rules engine user rules for routing.

#### Criticality: MEDIUM

---

### 3.19 Module: Search Service

#### Purpose
Full-text + semantic + hybrid search.

#### Responsibilities
1. SQLite FTS5 + Qdrant backends combined.
2. Hybrid retrieval BM25 reranking.
3. Federated queries plugins registered query parsing, plugins results
4. Personalization ranking re-ranking per user.

#### Criticality: MEDIUM

---

### 3.20 Module: Indexing Service

#### Purpose
Watch file watcher inotify + metadata extraction pipeline.

#### Responsibilities
1. inotify recursive hooks per per per-workspace crawler.
2. Document content extraction (PDF, DOCX, OCR via pluggable extractors).
3. Dispatch to embedding + vector search index.
4. Backpressure throttling under IO load.

#### Criticality: MEDIUM

---

## AI Runtime Contracts

### 4.1 Module: LLM Engine

See AIArchitecture.md §2.

Additional Contract Additions:
#### Observability
- `llm_request_duration_seconds{model, backend, status}` histogram
- `llm_tokens_total{model, direction=in|out}` counter
- `llm_cache_hits_total`, cache miss
- Per-request trace spans: llm.generate, llm.embed

---

### 4.2 Module: Planning Engine (HTN Planner)

See AIArchitecture.md §3.

#### Failure Modes
- Plan exhaustion max depth = 20 → PAUSED HITL.
- Verification step fails 3 times → HITL.

---

### 4.3 Module: Semantic Memory

See AIArchitecture.md §4.

---

### 4.4 Module: Context Engine

See AIArchitecture.md §5.

---

### 4.5 Module: Embedding Service

See AIArchitecture.md §6.

---

### 4.6 Module: Agent Orchestrator

See AIArchitecture.md §8.

---

## Capability Layer Contracts

### Module: Capability Token Service

#### Purpose
Centralized unforgable token mint validate, delegate revoke unforgable.

#### Responsibilities
1. Mint tokens Ed25519 issuance signed.
2. Validate tokens verify signature not expiry.
3. Revoke revocation check.
4. Delegation chain validatation: monotonically reduce constraints.
5. Token TTL, rate limit, one-shot counters.
6. Token storage: minting identity provenance log.

#### Criticality: CRITICAL

#### Security Concerns
- Signing key HSM. Never leaves security domain (TPM-backed in phase-1, eventually PKCS#11 HSM Phase3).

---

### Module: Policy Engine (OPA/Rego)

#### Purpose
Negative guardrail policy evaluation on every bus message.

#### Responsibilities
1. Load OPA Rego policies bundles, hot reload without downtime.
2. Evaluate per-message: input (sender, target, operation, payload, workspace, time).
3. Decision: ALLOW | DENY | REQUIRE_HITL | ESCALATE.
4. Custom policies per workspace, per enterprise.
5. Policy change audit log.

#### Criticality: CRITICAL

#### Performance
- <50µs per decision p99 (JIT compiled Rego).

---

### Module: Sandbox Manager

Builds sandboxes all layers.

#### Responsibilities
1. Compose namespaces, cgroup config seccomp filters LSM policies.
2. Sandbox stacking per workload type.

---

## Application Runtime Contracts

### 6.1 Module: Native App Runtime

See Runtime.md §2.

---

### 6.2 Module: Container Runtime (Podman Wrapper)

See Runtime.md §3.

---

### 6.3 Module: VM Manager (libvirt + QEMU/KVM)

See Runtime.md §4.

---

### 6.4 Module: Plugin Host (Wasmtime Wasmtime)

See Runtime.md §5.

---

## Workspace Layer Contracts

### Module: Isolation Domain Controller

Per workspace lifecycle bridge between Workspace Manager and Capability Layer.

#### Responsibilities
1. Enforce namespace attachment of all workloads to workspaces.
2. Broker cross-workspace comms explicit HITL.
3. Workspace state serialization/deserialization format versioned.

#### Criticality: CRITICAL

---

## UI Layer Contracts

### 8.1 Module: Shell (AI-First User Interface)

#### Responsibilities
1. Primary user interaction surface.
2. Workspace switcher status area.
3. AI conversation thread.
4. Permission prompts.
5. Notifications center.
6. User settings pages.

#### Dependencies
- Upstream: AI Runtime, Workspace Manager, Notification Service, Capability Layer.

#### Criticality: HIGH (User-Facing)

---

### 8.2 Module: Wayland Compositor

#### Responsibilities
1. Render composition.
2. Wayland protocol interface.
3. Window management.
4. GPU-accelerated rendering.
5. Security filter Wayland protocol security filter.

#### Criticality: HIGH

---

### 8.3 Module: Window Manager

#### Responsibilities
1. Tiling / floating windows.
2. Per-workspace window lists.
3. Keyboard shortcuts.
4. Window rules per app.

#### Criticality: MEDIUM

---

## Remote Interface Contracts

### 9.1 Module: Remote Control API

#### Purpose
Secure remote control of CognyxOS instance.

#### Responsibilities
1. gRPC server mTLS authentication.
2. WebRTC data channel video remote desktop.
3. Remote workspace.
4. Audit every remote session full session recording opt-in.

#### Criticality: HIGH

#### Public APIs
```protobuf
service RemoteControl {
  rpc EstablishSession(SessionRequest) returns (SessionToken);
  rpc SendInputEvent(InputEvent) returns (google.protobuf.Empty);
  rpc ReceiveFrame(FrameRequest) returns (stream VideoFrame);
  rpc ExecuteRemoteCommand(RemoteCommand) returns (CommandResult);
  rpc TerminateSession(SessionToken) returns (google.protobuf.Empty);
}
```

---

### 9.2 Module: Developer API

REST + GraphQL + gRPC full surface for application plugins.

#### Responsibilities
1. API gateway with OAuth2 capability scoped JWT.
2. Rate limiting per consumer.
3. Schema auto-generation from proto.

---

## Command Bus & IPC Framework Contracts

### Module: Secure Message Bus

#### Purpose
Central asynchronous communication all modules all patterns.

#### Patterns Supported
- Command (exactly-once ordered).
- Query (request/response).
- Event (pub/sub at-least-once per topic).
- Stream (flow controlled).

#### Responsibilities
1. Module identity auth via socket pairs per peer authentication
2. Capability validate messages messages validation
3. Policy Engine evaluation on all before delivery
4. Audit every logging all
5. Cancellation, deadlines, retry policy handling
6. Back-pressure handling, traffic shapping

#### Public APIs (Rust traits, used by every service)
```rust
#[async_trait]
pub trait MessageBus: Send + Sync {
    // Command pattern
    async fn send_command(&self, target: &str, cmd: impl Message + 'static) -> Result<impl Message>;

    // Event pattern
    async fn publish_event(&self, topic: &str, event: impl Message + 'static) -> Result<()>;
    async fn subscribe(&self, topic_filter: &str) -> Result<Pin<Box<dyn Stream<Item = BusMessage> + Send>>>;

    // Query pattern
    async fn query(&self, target: &str, q: impl Message) -> Result<impl Message>;

    // Streaming
    async fn open_stream(&self, target: &str, spec: StreamSpec) -> Result<(Sender<Bytes>, Receiver<Bytes>)>;

    // Lifecycle
    async fn register_module(&self, id: ModuleIdentity) -> Result<ModuleRegistration>;
}
```

#### Criticality: CRITICAL

#### Performance
- Throughput: 1M messages per second per node
- Latency p99 <10 microseconds one-way

---

## 11. Cloud Runtime Contracts (Future, Phase 6+)

### Module: Cloud Federation Controller

#### Purpose
Distributed workspace across devices cloud nodes.

#### Responsibilities (Future)
1. Workspace replication distributed cluster (CRDT) sync
2. Distributed AI inference
3. Workload scheduling cloud bursting
4. Shared agent swarm agents
5. Identity provider identity across instances cloud providers

#### Criticality: HIGH (Future)

---

### Future Known Unknowns Known Gaps
This contract explicitly deferred:
- Raft cluster state manager;
- Cross-region sync.
