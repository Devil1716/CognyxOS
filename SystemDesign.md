# CognyxOS System Design

> **Document ID:** ARCH-002
> **Version:** 1.0.0
> **Status:** Phase 0 - Approved
> **Last Updated:** 2026-08-01
> **Owner:** Architecture Council

---

## Table of Contents

1. [Introduction](#introduction)
2. [System Services Deep Dive](#system-services-deep-dive)
3. [State Management Architecture](#state-management-architecture)
4. [Process Management](#process-management)
5. [Filesystem Architecture](#filesystem-architecture)
6. [Networking Architecture](#networking-architecture)
7. [Identity & Authentication](#identity--authentication)
8. [Device & Graphics Management](#device--graphics-management)
9. [Update & Lifecycle Management](#update--lifecycle-management)
10. [Telemetry, Logging, & Observability](#telemetry-logging--observability)

---

## Introduction

This document provides the detailed system design for each of the 21 system services in CognyxOS. It complements the Architecture overview with concrete design specifications, internal data models, and inter-service interaction protocols.

Each service is described according to the **CognyxOS Service Contract** which mandates:
- Explicit purpose and scope boundaries
- Formal dependency declaration
- Public API surface (gRPC + capability tokens)
- Internal state model
- Failure modes and recovery procedures
- Performance SLAs
- Security posture

---

## System Services Deep Dive

### 1. Process Manager (`services/process`)

**Purpose:** Lifecycle management of all user-space processes and supervision trees. This is the only service that may create processes (via `fork()`/`clone()`).

**Responsibilities:**
- Process creation, termination, restart policies
- Cgroup resource allocation and enforcement
- Linux namespace construction
- Seccomp-bpf policy application
- Process supervision (monitoring, health checks, failure reporting)
- Zombie reaping
- Process hierarchy and ownership tracking

**Dependencies:**
- Capability Token Service (for process capabilities)
- Logging Service (for process stdout/stderr)
- Security Service (for LSM policy attachment)

**State Model:**
```
ProcessRecord {
  pid: u32
  parent_pid: u32
  workspace_id: Option<Uuid>
  owner_identity: IdentityId
  binary_path: PathBuf
  binary_hash: Sha256
  created_at: Timestamp
  resource_limits: ResourceLimits
  cgroup_path: PathBuf
  namespace_set: NamespaceConfig
  seccomp_policy: PolicyId
  supervision_policy: SupervisionPolicy
  status: RUNNING | SLEEPING | STOPPED | ZOMBIE | DEAD
  exit_code: Option<i32>
  memory_usage_bytes: u64
  cpu_usage_ns: u64
}
```

**Public APIs:**
```protobuf
service ProcessManager {
  rpc SpawnProcess(SpawnRequest) returns (SpawnResponse);
  rpc TerminateProcess(TerminateRequest) returns (google.protobuf.Empty);
  rpc SendSignal(SignalRequest) returns (google.protobuf.Empty);
  rpc GetProcessInfo(ProcessInfoRequest) returns (ProcessInfo);
  rpc ListProcesses(ListProcessesRequest) returns (stream ProcessInfo);
  rpc WatchProcessEvents(WatchRequest) returns (stream ProcessEvent);
  rpc SetResourceLimits(LimitsRequest) returns (google.protobuf.Empty);
}
```

**Security Concerns:**
- Spawn capability required; includes binary whitelist, namespace config, resource limits
- No process may escape its cgroup or namespace once created
- Setuid binaries are blocked by default; must be explicitly whitelisted per workspace

**Performance:**
- Spawn latency: < 5ms (cached fork server)
- Event delivery: < 1ms p99
- Process capacity: 65536 concurrent processes per instance

---

### 2. Task Scheduler (`services/scheduler`)

**Purpose:** AI-generated task and OS-internal task scheduling. Manages priorities, deadlines, dependencies, and resource allocation.

**Responsibilities:**
- Task queue management (multi-queue priority)
- Deadline enforcement and preemption
- Dependency graph resolution (DAG scheduling)
- Resource-aware task placement (CPU/GPU/memory affinity)
- Task suspension, resumption, cancellation
- Periodic and cron-like scheduled tasks
- Work stealing across CPU cores / NUMA nodes

**Dependencies:**
- Process Manager (to spawn task workers)
- State Manager (persistent task state)
- Capability Token Service (task execution capabilities)

**Task Model:**
```
ScheduledTask {
  task_id: Uuid
  workspace_id: Option<Uuid>
  owner: IdentityId
  task_type: AI_PLAN | OS_INTERNAL | USER_REQUEST | PERIODIC
  priority: u16 (0-65535, lower = higher priority)
  deadline: Option<Timestamp>
  depends_on: Vec<Uuid>
  command: TaskCommand
  resource_requirements: ResourceRequirements
  state: PENDING | READY | RUNNING | SUSPENDED | COMPLETED | FAILED | CANCELLED
  retry_policy: RetryPolicy
  result: Option<TaskResult>
  worker_pid: Option<u32>
  created_at: Timestamp
  started_at: Option<Timestamp>
  completed_at: Option<Timestamp>
}
```

**Scheduling Algorithm:**
- Earliest Deadline First (EDF) for deadline-bound tasks
- Weighted Fair Queueing (WFQ) for non-deadline tasks
- Priority ceiling protocol to prevent inversion
- Deadline miss rate < 0.1% for CRITICAL priority tasks

**Public APIs:**
```protobuf
service TaskScheduler {
  rpc SubmitTask(SubmitTaskRequest) returns (TaskHandle);
  rpc CancelTask(CancelRequest) returns (google.protobuf.Empty);
  rpc SuspendTask(SuspendRequest) returns (google.protobuf.Empty);
  rpc ResumeTask(ResumeRequest) returns (google.protobuf.Empty);
  rpc GetTaskStatus(StatusRequest) returns (TaskStatus);
  rpc WatchTaskEvents(WatchRequest) returns (stream TaskEvent);
  rpc SchedulePeriodic(PeriodicRequest) returns (ScheduledHandle);
  rpc GetQueueDepth(QueueDepthRequest) returns (QueueDepthInfo);
}
```

---

### 3. Workspace Manager (`services/workspace`)

**Purpose:** Lifecycle and state management of workspaces—the primary context isolation unit.

**Responsibilities:**
- Workspace creation, cloning, archival, deletion
- Workspace state serialization and deserialization
- Isolation boundary enforcement (namespaces, mounts, network)
- Workspace resource quotas and limits
- Workspace hibernation (serialize memory to disk) and wake
- Workspace sharing and collaboration orchestration

**Workspace Model:**
```
Workspace {
  workspace_id: Uuid
  name: String
  description: String
  owner: IdentityId
  members: Vec<WorkspaceMember>
  root_mount: MountPath
  mount_namespace: NsFd
  network_namespace: NsFd
  pid_namespace: NsFd
  cgroup_path: CgroupPath
  resource_quota: ResourceQuota
  created_at: Timestamp
  state: INACTIVE | ACTIVE | HIBERNATED | ARCHIVED
  memory_snapshot_path: Option<PathBuf>
  ai_context_profile: ContextProfileId
  installed_capabilities: Vec<CapabilityId>
  tags: Vec<String>
}
```

**Lifecycle:**
```
CREATE → ACTIVATE ⇄ HIBERNATE → ARCHIVE → DELETE
                  ↓
               CLONE
```

**Public APIs:**
```protobuf
service WorkspaceManager {
  rpc CreateWorkspace(CreateWorkspaceRequest) returns (Workspace);
  rpc ActivateWorkspace(ActivateRequest) returns (ActivationHandle);
  rpc HibernateWorkspace(HibernateRequest) returns (HibernateResult);
  rpc DeleteWorkspace(DeleteRequest) returns (google.protobuf.Empty);
  rpc CloneWorkspace(CloneRequest) returns (Workspace);
  rpc ArchiveWorkspace(ArchiveRequest) returns (ArchiveResult);
  rpc GetWorkspace(GetRequest) returns (Workspace);
  rpc ListWorkspaces(ListRequest) returns (ListWorkspacesResponse);
  rpc UpdateWorkspace(UpdateRequest) returns (Workspace);
  rpc WatchWorkspaceEvents(WatchRequest) returns (stream WorkspaceEvent);
}
```

---

### 4. Filesystem Service (`services/filesystem`)

**Purpose:** Virtual filesystem abstraction with capabilities, snapshots, and indexing hooks.

**Responsibilities:**
- Path-based virtual filesystem (overlay, union, bind mounts)
- File access mediation via capability tokens
- Atomic snapshot creation and restore (Btrfs/ZFS send/receive)
- Per-file metadata store (extended attributes, tags, security labels)
- File watcher and change event dispatch
- Content-addressable storage dedup
- Encrypted folder support (fscrypt integration)

**File Capability Model:**
```
FileCapabilityToken {
  workspace_id: Uuid
  path: GlobPattern
  operations: READ | WRITE | EXECUTE | CREATE | DELETE | CHMOD | CHOWN | ACL
  validity_window: TimeRange
  delegation_allowed: bool
  revoked: bool
}
```

**Public APIs:**
```protobuf
service FilesystemService {
  rpc OpenFile(OpenRequest) returns (FileHandle);
  rpc ReadFile(ReadRequest) returns (stream FileChunk);
  rpc WriteFile(WriteRequest) returns (WriteResult);
  rpc StatFile(StatRequest) returns (FileMetadata);
  rpc ListDirectory(ListDirRequest) returns (stream DirectoryEntry);
  rpc CreateDirectory(MkdirRequest) returns (google.protobuf.Empty);
  rpc DeleteFile(DeleteRequest) returns (google.protobuf.Empty);
  rpc MoveFile(MoveRequest) returns (google.protobuf.Empty);
  rpc CopyFile(CopyRequest) returns (CopyResult);
  rpc CreateSnapshot(SnapshotRequest) returns (SnapshotId);
  rpc RestoreSnapshot(RestoreRequest) returns (RestoreResult);
  rpc WatchPath(WatchPathRequest) returns (stream FilesystemEvent);
  rpc GetFileMetadata(MetaRequest) returns (FileMetadata);
  rpc SetFileMetadata(SetMetaRequest) returns (google.protobuf.Empty);
  rpc SearchByMetadata(SearchMetaRequest) returns (stream FileHandle);
}
```

**Security:**
- OpenFile always requires a FileCapabilityToken; no ambient read/write authority
- Symlinks are always resolved within the workspace root; escape attempts are rejected
- SUID/SGID bits are ignored except on explicitly whitelisted paths

---

### 5. Network Service (`services/network`)

**Purpose:** All network functionality—routing, firewall, VPN, proxy, DNS, and API.

**Responsibilities:**
- Per-workspace network namespace and virtual interfaces
- Firewall rules (nftables backend) with capability-based allow lists
- VPN tunnel management (WireGuard, OpenVPN)
- HTTP/HTTPS proxy with content inspection capability
- DNS resolver with DoH/DoT, per-workspace DNS policies
- Network QoS and traffic shaping
- Network capability mediation (which hosts may be contacted)

**Network Capability Model:**
```
NetworkCapability {
  direction: INBOUND | OUTBOUND | BIDIRECTIONAL
  protocol: TCP | UDP | ICMP | ANY
  remote: IpCidrSet | HostnameGlob | "*"
  port: PortRange
  workspace_id: Option<Uuid>
  rate_limit_bps: Option<u64>
  time_window: TimeRange
}
```

**Public APIs:**
```protobuf
service NetworkService {
  rpc ConfigureInterface(IfaceConfig) returns (IfaceStatus);
  rpc AddFirewallRule(FirewallRule) returns (RuleId);
  rpc RemoveFirewallRule(RuleId) returns (google.protobuf.Empty);
  rpc ListFirewallRules(ListRulesRequest) returns (ListRulesResponse);
  rpc StartVpn(VpnConfig) returns (VpnHandle);
  rpc StopVpn(VpnId) returns (google.protobuf.Empty);
  rpc ConfigureDns(DnsConfig) returns (google.protobuf.Empty);
  rpc SetNetworkCapability(NetCapRequest) returns (CapabilityToken);
  rpc GetNetworkStats(StatsRequest) returns (NetworkStats);
  rpc WatchNetworkEvents(WatchRequest) returns (stream NetworkEvent);
  rpc TestConnectivity(ConnectivityTest) returns (TestResult);
}
```

---

### 6. Identity Manager (`services/identity`)

**Purpose:** User, device, and service identity management with strong cryptographic authentication.

**Responsibilities:**
- Identity creation and lifecycle (user, service, device, AI agent)
- Authentication: WebAuthn, password, hardware key, TPM attestation
- Key management: Ed25519 signing keys, Curve25519 encryption keys, recovery keys
- Identity federation: OAuth2, OIDC, SAML (future)
- Biometric authentication orchestration (fingerprint, face)
- Session management (short-lived session tokens from long-lived identities)

**Identity Model:**
```
Identity {
  id: IdentityId
  type: USER | SERVICE | DEVICE | AGENT | WORKSPACE
  display_name: String
  public_keys: Vec<PublicKey>
  credentials: Vec<Credential>
  created_at: Timestamp
  locked: bool
  recovery_keys: Vec<RecoveryKeyHash>
  attributes: HashMap<String, String>
}

Session {
  id: SessionId
  identity_id: IdentityId
  workspace_id: Option<Uuid>
  expires_at: Timestamp
  last_used_at: Timestamp
  auth_level: u8 (0-4, 4 = hardware key + biometric)
  scope: Vec<CapabilityToken>
}
```

**Public APIs:**
```protobuf
service IdentityManager {
  rpc CreateIdentity(CreateIdentityRequest) returns (Identity);
  rpc DeleteIdentity(DeleteRequest) returns (google.protobuf.Empty);
  rpc Authenticate(AuthRequest) returns (Session);
  rpc VerifySession(Session) returns (SessionInfo);
  rpc RevokeSession(RevokeRequest) returns (google.protobuf.Empty);
  rpc AddCredential(AddCredRequest) returns (Credential);
  rpc RemoveCredential(RemoveCredRequest) returns (google.protobuf.Empty);
  rpc ListIdentities(ListRequest) returns (ListIdentitiesResponse);
  rpc InitiateRecovery(RecoveryRequest) returns (RecoveryChallenge);
  rpc CompleteRecovery(RecoveryResponse) returns (Identity);
  rpc LockIdentity(LockRequest) returns (google.protobuf.Empty);
  rpc UnlockIdentity(UnlockRequest) returns (Session);
}
```

---

### 7. Security Service (`services/security`)

**Purpose:** System integrity, attestation, policy enforcement, and intrusion detection.

**Responsibilities:**
- File integrity monitoring (FIM) with cryptographic hashing
- TPM 2.0 quote generation and verification
- Runtime integrity measurement (IMA appraisal)
- Intrusion detection via eBPF hooks (exec, socket, filesystem)
- Secure Boot chain verification
- Threat intelligence integration (opt-in)

**Public APIs:**
```protobuf
service SecurityService {
  rpc GetIntegrityReport(IntegrityRequest) returns (IntegrityReport);
  rpc GenerateTpmQuote(TpmQuoteRequest) returns (TpmQuote);
  rpc VerifyAttestation(Attestation) returns (VerificationResult);
  rpc ScanForThreats(ScanRequest) returns (ThreatReport);
  rpc WatchSecurityEvents(WatchRequest) returns (stream SecurityEvent);
  rpc GetSystemSecurityState(StateRequest) returns (SecurityState);
  rpc HardenSystem(HardenRequest) returns (HardenResult);
}
```

---

### 8. Device Manager (`services/device`)

**Purpose:** Hardware device enumeration, driver binding, and device permission mediation.

**Responsibilities:**
- udev-based device enumeration and hotplug detection
- Device driver binding and firmware loading
- Device capability tokens (which processes may access which devices)
- GPU passthrough configuration for VMs
- USB device filtering and authorization
- Power state management (suspend/resume per device)

**Public APIs:**
```protobuf
service DeviceManager {
  rpc ListDevices(ListRequest) returns (ListDevicesResponse);
  rpc GetDeviceInfo(DeviceId) returns (DeviceInfo);
  rpc RequestDeviceAccess(DeviceAccessRequest) returns (DeviceCapability);
  rpc ReleaseDeviceAccess(DeviceId) returns (google.protobuf.Empty);
  rpc ConfigureGpuPassthrough(GpuConfig) returns (google.protobuf.Empty);
  rpc AuthorizeUsbDevice(UsbAuthRequest) returns (google.protobuf.Empty);
  rpc WatchDeviceEvents(WatchRequest) returns (stream DeviceEvent);
  rpc SetDevicePowerState(PowerRequest) returns (google.protobuf.Empty);
}
```

---

### 9. Graphics Service (`services/graphics`)

**Purpose:** GPU scheduling, buffer management, and Wayland compositor support.

**Responsibilities:**
- GPU time-slicing and process scheduling (DRM leases)
- Buffer allocation, sharing (DMA-BUF), and lifecycle
- Display configuration (outputs, modes, scaling via KMS)
- Wayland protocol negotiation and security filtering
- Hardware video encode/decode capability mediation

**Public APIs:**
```protobuf
service GraphicsService {
  rpc ListGpus(ListRequest) returns (ListGpusResponse);
  rpc AllocateGpuMemory(GpuMemRequest) returns (BufferHandle);
  rpc ReleaseGpuMemory(BufferHandle) returns (google.protobuf.Empty);
  rpc ConfigureDisplay(DisplayConfig) returns (DisplayStatus);
  rpc ListDisplays(ListRequest) returns (ListDisplaysResponse);
  rpc RequestWaylandSocket(SocketRequest) returns (SocketHandle);
  rpc GetGpuStats(StatsRequest) returns (GpuStats);
}
```

---

### 10. State Manager (`services/state`)

**Purpose:** Global state reconciliation, CRDT sync, durable persistence.

**Responsibilities:**
- System-wide key-value state store (RocksDB backend)
- CRDT-based multi-node state reconciliation (future cloud mode)
- State subscription and change notification
- Periodic snapshot and write-ahead logging
- Cross-workspace shared state (authorized)

**State Model:**
```
StateEntry {
  key: String (hierarchical, e.g. "workspace/<id>/config/theme")
  value: bytes (protobuf, JSON, or raw)
  revision: u64 (monotonic)
  tombstone: bool
  last_modified: Timestamp
  modified_by: IdentityId
  crdt_encoding: Option<CrdtType>
}
```

**Public APIs:**
```protobuf
service StateManager {
  rpc GetState(GetRequest) returns (StateEntry);
  rpc SetState(SetRequest) returns (Revision);
  rpc DeleteState(DeleteRequest) returns (google.protobuf.Empty);
  rpc WatchState(WatchRequest) returns (stream StateChange);
  rpc ListState(ListRequest) returns (stream StateEntry);
  rpc Transaction(StateTransaction) returns (TransactionResult);
  rpc CompareAndSwap(CasRequest) returns (CasResult);
  rpc SyncWithPeer(SyncRequest) returns (SyncResult);
}
```

---

### 11. Notification Service (`services/notification`)

**Purpose:** System notification delivery and user attention management.

**Responsibilities:**
- Notification creation with priority levels
- Multi-channel delivery (UI popup, email, SMS, push—opt-in)
- Notification grouping, summarization, AI-driven triage
- User-defined notification rules
- Notification history and search

**Public APIs:**
```protobuf
service NotificationService {
  rpc SendNotification(Notification) returns (NotificationId);
  rpc DismissNotification(DismissRequest) returns (google.protobuf.Empty);
  rpc UpdateNotification(UpdateRequest) returns (Notification);
  rpc ListNotifications(ListRequest) returns (ListNotifResponse);
  rpc MarkAsRead(MarkRequest) returns (google.protobuf.Empty);
  rpc RegisterChannel(ChannelConfig) returns (ChannelId);
  rpc WatchNotifications(WatchRequest) returns (stream NotificationEvent);
}
```

---

### 12. Search Service (`services/search`)

**Purpose:** Unified semantic and full-text search across all workspace data.

**Responsibilities:**
- Full-text search (SQLite FTS5 backend)
- Semantic vector search (Qdrant backend)
- Hybrid search (BM25 + vector similarity)
- Federated search across external plugins
- Search result ranking and personalization
- AI-powered result summarization

**Public APIs:**
```protobuf
service SearchService {
  rpc Search(SearchQuery) returns (SearchResponse);
  rpc SemanticSearch(SemanticQuery) returns (SemanticResponse);
  rpc HybridSearch(HybridQuery) returns (HybridResponse);
  rpc IndexDocument(IndexRequest) returns (DocId);
  rpc RemoveFromIndex(RemoveRequest) returns (google.protobuf.Empty);
  rpc GetSearchStats(StatsRequest) returns (SearchStats);
  rpc WatchIndexEvents(WatchRequest) returns (stream IndexEvent);
}
```

---

### 13. Indexing Service (`services/indexing`)

**Purpose:** Automatic file watching, metadata extraction, and embedding generation.

**Responsibilities:**
- File system watcher (inotify, recursive)
- Metadata extraction (document, image, audio EXIF, code AST)
- Text content extraction (PDF, DOCX, code, OCR via Tesseract)
- Embedding generation request dispatch
- Incremental re-indexing on file change
- Index backpressure and throttling

**Public APIs:**
```protobuf
service IndexingService {
  rpc RegisterWatchPath(WatchRequest) returns (WatchHandle);
  rpc UnregisterWatchPath(WatchHandle) returns (google.protobuf.Empty);
  rpc TriggerIndex(IndexRequest) returns (IndexResult);
  rpc GetIndexingStatus(StatusRequest) returns (IndexingStatus);
  rpc PauseIndexing(WorkspaceId) returns (google.protobuf.Empty);
  rpc ResumeIndexing(WorkspaceId) returns (google.protobuf.Empty);
  rpc ExtractMetadata(ExtractRequest) returns (ExtractedMetadata);
  rpc WatchIndexingEvents(WatchRequest) returns (stream IndexingEvent);
}
```

---

### 14. Telemetry Service (`services/telemetry`)

**Purpose:** Opt-in performance metrics, usage statistics, and error reporting.

**Responsibilities:**
- Metrics collection (Prometheus-compatible exposition format)
- Distributed tracing (OpenTelemetry, W3C Trace Context)
- Error report aggregation and symbolicated crash stacks
- Usage counter aggregation (opt-in, privacy-preserving differential noise)
- Anonymized upload to CognyxOS analytics (opt-in, explicit consent)

**Public APIs:**
```protobuf
service TelemetryService {
  rpc EmitMetric(Metric) returns (google.protobuf.Empty);
  rpc StartSpan(SpanStart) returns (SpanHandle);
  rpc EndSpan(SpanEnd) returns (google.protobuf.Empty);
  rpc RecordError(ErrorReport) returns (ErrorId);
  rpc GetMetricsSnapshot(SnapshotRequest) returns (MetricsSnapshot);
  rpc SetConsent(ConsentConfig) returns (google.protobuf.Empty);
  rpc GetConsent(ConsentRequest) returns (ConsentConfig);
}
```

**Privacy Guarantee:**
- Default: OFF. All collection requires explicit per-category consent.
- No data leaves the device without user opt-in per-transmission.
- Uploaded data is stripped of all identifiers and has differential privacy noise added.
- User may purge all locally-stored telemetry at any time.

---

### 15. Logging Service (`services/logging`)

**Purpose:** Structured log aggregation, querying, and rotation.

**Responsibilities:**
- Structured log ingestion (JSON lines, protobuf)
- Per-service log buffering and batching
- Log storage (SQLite hot, compressed cold storage)
- Log query interface (filter by level, module, correlation_id, etc.)
- Automatic rotation and retention policy enforcement
- Log integrity (hash chain for security-critical logs)

**Log Entry Model:**
```
LogEntry {
  timestamp: Timestamp (TAI64N)
  level: TRACE | DEBUG | INFO | WARN | ERROR | FATAL | AUDIT
  module: String
  correlation_id: Option<Uuid>
  causation_id: Option<Uuid>
  message: String
  fields: HashMap<String, Value>
  stack_trace: Option<String>
  workspace_id: Option<Uuid>
  identity_id: Option<IdentityId>
  hash_chain_prev: Option<Sha256>
}
```

**Public APIs:**
```protobuf
service LoggingService {
  rpc EmitLog(LogEntry) returns (google.protobuf.Empty);
  rpc QueryLogs(LogQuery) returns (stream LogEntry);
  rpc WatchLogs(WatchRequest) returns (stream LogEntry);
  rpc RotateLogs(RotateRequest) returns (RotateResult);
  rpc GetRetentionPolicy(PolicyRequest) returns (RetentionPolicy);
  rpc SetRetentionPolicy(RetentionPolicy) returns (google.protobuf.Empty);
  rpc VerifyIntegrity(IntegrityRequest) returns (IntegrityReport);
}
```

---

### 16. Config Service (`services/config`)

**Purpose:** Hierarchical schema-driven configuration management.

**Responsibilities:**
- Layered config: defaults → system → workspace → user
- Schema validation (JSON Schema)
- Config change notification
- Config rollback and version history
- Config import/export

**Config Model:**
```
ConfigLayer = DEFAULTS | SYSTEM | WORKSPACE | USER

ConfigEntry {
  key: String
  value: serde_json::Value
  layer: ConfigLayer
  schema: Option<SchemaId>
  last_modified: Timestamp
  modified_by: IdentityId
  version: u64
}
```

**Public APIs:**
```protobuf
service ConfigService {
  rpc GetConfig(GetRequest) returns (ConfigValue);
  rpc SetConfig(SetRequest) returns (google.protobuf.Empty);
  rpc ResetConfig(ResetRequest) returns (google.protobuf.Empty);
  rpc WatchConfig(WatchRequest) returns (stream ConfigChange);
  rpc ValidateConfig(ValidateRequest) returns (ValidationResult);
  rpc ListConfigKeys(ListRequest) returns (stream ConfigKey);
  rpc RollbackConfig(RollbackRequest) returns (google.protobuf.Empty);
  rpc ImportConfig(ImportRequest) returns (ImportResult);
  rpc ExportConfig(ExportRequest) returns (ConfigExport);
}
```

---

### 17. Update Manager (`services/update`)

**Purpose:** Atomic, signed system updates with rollback capability.

**Responsibilities:**
- Update channel management (stable, beta, nightly, custom)
- Delta update download and verification (Ed25519 signed)
- Atomic A/B partition swap (OSTree-based)
- Update rollback on boot failure (systemd-brownout detection)
- Workspace data migration scripts
- Firmware update orchestration (fwupd)

**Public APIs:**
```protobuf
service UpdateManager {
  rpc CheckForUpdates(CheckRequest) returns (UpdateInfo);
  rpc DownloadUpdate(DownloadRequest) returns (stream DownloadProgress);
  rpc ApplyUpdate(ApplyRequest) returns (UpdateHandle);
  rpc RollbackUpdate(RollbackRequest) returns (google.protobuf.Empty);
  rpc GetUpdateStatus(StatusRequest) returns (UpdateStatus);
  rpc SetUpdateChannel(ChannelRequest) returns (google.protobuf.Empty);
  rpc GetUpdateHistory(HistoryRequest) returns (UpdateHistory);
  rpc WatchUpdateEvents(WatchRequest) returns (stream UpdateEvent);
}
```

---

## State Management Architecture

### State Layering Model

```
┌──────────────────────────────────────────────────────────────┐
│                    Ephemeral State (RAM)                      │
│  AI Context Cache | LLM KV Cache | GPU Buffers | Sockets     │
├──────────────────────────────────────────────────────────────┤
│                 Hot State (Local NVMe SSD)                    │
│  SQLite DBs | Qdrant Vector Store | RocksDB KV | WALs        │
├──────────────────────────────────────────────────────────────┤
│                 Warm State (Local Disk)                       │
│  Workspace FS Snapshots | Rotated Logs | App State           │
├──────────────────────────────────────────────────────────────┤
│                 Cold State (Archival)                         │
│  Compressed Workspaces | Old Logs | Long-term Backups        │
├──────────────────────────────────────────────────────────────┤
│                 Remote State (Cloud, Optional)                │
│  Encrypted Backup | Distributed Workspaces | Shared Memory   │
└──────────────────────────────────────────────────────────────┘
```

### Write-Ahead Log Protocol

Every state mutation:
1. Client sends mutation request with expected precondition
2. Service serializes mutation to WAL entry
3. WAL fsynced to disk
4. In-memory state updated
5. ACK returned to client
6. Periodically: checkpoint snapshot created, WAL truncated

Recovery: On restart, replay WAL from last checkpoint to rebuild in-memory state.

---

## Process Management

### Supervision Tree

All processes are organized into a strict supervision tree with explicit failure policies:

```
Root Supervisor (one_for_all)
├── Message Bus (restart: always)
├── Core Services Supervisor (one_for_one)
│   ├── Logging (restart: always)
│   ├── Config (restart: always)
│   ├── Identity (restart: always, 5s delay)
│   ├── Capability (restart: always, 5s delay)
│   └── Security (restart: always, 5s delay)
├── System Services Supervisor (rest_for_one)
│   ├── Process Manager (restart: on-failure, max 5)
│   ├── State Manager (restart: on-failure, max 5)
│   ├── Filesystem (restart: on-failure, max 3)
│   ├── Scheduler (restart: on-failure, max 3)
│   ├── Network (restart: on-failure, max 3)
│   ├── Device (restart: on-failure, max 3)
│   └── Graphics (restart: on-failure, max 3)
├── AI Runtime Supervisor (one_for_one)
│   ├── LLM Engine (restart: on-failure, degrade after 3)
│   ├── Planning (restart: on-failure, degrade after 3)
│   ├── Vector Store (restart: on-failure, degrade after 3)
│   └── Agent Orchestrator (restart: on-failure, degrade after 3)
└── UI Supervisor (one_for_one)
    ├── Compositor (restart: on-failure)
    └── Shell (restart: on-failure)
```

### Failure Escalation

If a service exceeds its restart budget:
1. **Supervisor attempts clean restart** with state replay
2. **Degraded mode** - dependent services announce reduced functionality
3. **User alert** - notification + shell banner
4. **Safe mode** (worst case) - only core services + shell restart option

---

## Filesystem Architecture

### Directory Layout (Mounted)

```
/
├── sys/                    Read-only system partition (dm-verity protected)
│   ├── bin/                CognyxOS binaries
│   ├── lib/                System libraries
│   ├── services/           Service binaries and manifests
│   ├── models/             Default AI models
│   └── config/             System defaults (immutable)
├── state/                  Stateful system partition (authenticated)
│   ├── services/           Per-service state (sqlite, rocksdb, etc.)
│   ├── audit/              Audit logs (append-only, hash-chained)
│   ├── users/              Per-user config and state
│   └── updates/            Update staging
├── workspaces/             Workspace mount point
│   ├── <id>_<name>/        One mount per active workspace
│   │   ├── home/           User home directory within workspace
│   │   ├── apps/           Installed app state
│   │   ├── .cognyx/        AI memory, index, cache (auto-managed)
│   │   └── .snapshots/     Read-only snapshot subvolumes
├── devices/                Device mount point (USB, external)
├── tmp/                    Ephemeral (tmpfs, cleaned on reboot)
└── run/                    Runtime state (sockets, PID files, tmpfs)
```

### Encryption Layers

1. **Full Disk Encryption (FDE):** LUKS2 + Argon2id key derivation, TPM2 auto-unlock with PIN fallback
2. **Home Directory Encryption:** fscrypt (per-directory, user key)
3. **Audit Log Encryption:** Append-only, signed, encrypted with identity recovery key
4. **Swap Encryption:** Random ephemeral key on every boot

---

## Networking Architecture

### Network Namespace Topology

```
Host Netns
├── br-host (bridge, 10.0.0.1/24)
│   ├── wg-cognyx (WireGuard interface, VPN routes)
│   └── veth-<workspace-id> (one veth pair per workspace)
│
Workspace Netns (isolated per workspace)
├── lo (127.0.0.1/8, ::1/128)
└── eth0 (10.0.0.<N>/24, default route via bridge)
    └── Firewall: Only allows explicitly-granted NetCapability
```

### Zero-Trust Network Rules

- **Default deny:** Workspaces have NO network access by default
- **Capability granularity:** Egress permissions per CIDR, per hostname glob, per port, per protocol
- **DNS filtering:** Per-workspace DNS policy (whitelist, blacklist, DoH upstream)
- **Transparent proxy:** HTTP(S) traffic optionally routed through inspection proxy (user consent required)

---

## Identity & Authentication

### Authentication Level Ladder

| Level | Auth Factor | Capabilities Granted |
|-------|-------------|----------------------|
| 0 | Anonymous (no auth) | None except system UI |
| 1 | Username + Password | Basic workspace access, no destructive ops |
| 2 | + TOTP / Recovery Code | Full workspace access, destructive ops |
| 3 | + WebAuthn / Hardware Key | Capability delegation, config changes |
| 4 | + Biometric Confirmation | Key operations, identity changes, updates |

### Session Flow

```
User Authenticates → Identity Manager validates factors → AuthLevel determined
    → Session created with initial capabilities → Session token (short-lived JWT, EdDSA signed)
    → User action requires elevated capability → Step-up auth prompt → Session AuthLevel raised
```

---

## Device & Graphics Management

### Device Capability Flow

```
Hotplug event → Device Manager enumerates → Sysfs metadata read
    → User notification "Device <X> connected, grant access to Workspace Y?"
    → User approves → DeviceCapability token minted
    → Token assigned to workspace processes
    → Process opens device file → LSM verifies token presence
```

### GPU Scheduling

- **Time-sliced sharing:** GPU time divided across processes with per-process weights
- **DRM Lease:** Exclusive GPU access for VMs (passthrough subset)
- **VRAM accounting:** Per-process VRAM limits via cgroup + DRM cgroup controller
- **Emergency eviction:** If GPU OOM, lowest-priority process buffers evicted

---

## Update & Lifecycle Management

### A/B Partition Layout (OSTree)

```
Disk Partitions:
  p1: EFI System Partition (ESP) - 1GB
  p2: Boot - 2GB (kernel/initramfs for A and B)
  p3: Sys-A - 32GB (active system image)
  p4: Sys-B - 32GB (inactive system image)
  p5: State - (rest of disk, LUKS encrypted)
    └── LVM: workspaces, audit, services, users
```

**Update Flow:**
1. Delta downloaded, verified (Ed25519 + SHA-256)
2. Delta applied to inactive Sys-B via OSTree
3. Bootloader updated to try Sys-B, fallback to Sys-A on failure
4. Reboot
5. systemd brownout monitor verifies services healthy for 2 min
6. If healthy: commit Sys-B as active, Sys-A becomes rollback target
7. If unhealthy: automatic reboot, bootloader falls back to Sys-A

---

## Telemetry, Logging, & Observability

### Three Pillars

| Pillar | Backend | Retention | Export |
|--------|---------|-----------|--------|
| Metrics | Prometheus (embedded) | 7 days hot, 30 days warm | Prometheus Remote Write (opt-in) |
| Traces | OpenTelemetry Collector | 3 days hot | OTLP gRPC (opt-in) |
| Logs | SQLite + Compressed Zstd | 30 days hot, 1 year warm | N/A (local only) |

### Trace Context Propagation

Every message on the bus carries:
- `trace_id` (W3C Trace Context compliant)
- `span_id`
- `correlation_id` (user-initiated action identifier)
- `causation_id` (message that directly caused this one)

This enables:
- Full distributed tracing across modules
- User action replay and audit reconstruction
- AI decision provenance (why was this action taken?)
