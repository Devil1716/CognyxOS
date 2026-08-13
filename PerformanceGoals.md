# CognyxOS Performance Goals & Non-Functional Requirements

> **Document ID:** PERF-001
> **Version:** 1.0.0
> **Status:** Phase 0 - Approved
> **Last Updated:** 2026-08-01
> **Owner:** Performance Engineering Team

---

## Table of Contents

1. [Performance Philosophy](#performance-philosophy)
2. [Boot Time Targets](#boot-time-targets)
3. [Memory Limits](#memory-limits)
4. [Latency Goals](#latency-goals)
5. [Throughput Goals](#throughput-goals)
6. [Scalability Targets](#scalability-targets)
7. [Fault Tolerance & Availability](#fault-tolerance--availability)
8. [Security Goals](#security-goals)
9. [Offline Operation Requirements](#offline-operation-requirements)
10. [Cloud Synchronization](#cloud-synchronization)
11. [GPU & Hardware Acceleration](#gpu--hardware-acceleration)
12. [Architecture Support (x86 / ARM / RISC-V)](#architecture-support-x86--arm--risc-v)
13. [Future Mobile & XR Support](#future-mobile--xr-support)
14. [Energy Efficiency](#energy-efficiency)
15. [Internationalization & Accessibility](#internationalization--accessibility)
16. [Measurement Methodology](#measurement-methodology)

---

## Performance Philosophy

CognyxOS has non-negotiable performance contract: **users never wait for the OS.** If an AI operation takes time, the UI responds immediately with progress. If a subsystem is overwhelmed, it degrades gracefully rather than freezing.

Three pillars guide all performance work:

1. **Perceived Latency < Absolute Latency.** If the system cannot respond in <16ms, it responds with a spinner, skeleton UI, or optimistic update in <16ms.
2. **Graceful Degradation Under Resource Pressure.** Never OOM-kill user work. Shed load in reverse priority: background indexing → telemetry → AI features → UI animations → core services.
3. **Zero Surprise Degradation.** All slow paths emit metrics and visible "System under load" indicator with exact cause.

---

## 2. Boot Time Targets

Boot = Power-on to interactive AI shell with default workspace loaded.

| Target Hardware | Cold Boot (Laptop DC Power) | Cold Boot (NVMe SSD) | Hibernation Resume | S3 Sleep Resume |
|-----------------|--------------------------|----------------------|--------------------|-----------------|
| **Reference Laptop (i7-13th, 16GB, NVMe)** | < 8s | < 6s | < 2s | < 500ms |
| **High-End Laptop (i9-14th, 64GB)** | < 6s | < 4s | < 1.5s | < 300ms |
| **Mid-Tier ARM Laptop (Snapdragon X Elite)** | < 5s | < 4s | < 1.5s | < 300ms |
| **Entry-Level (i5, 8GB, SATA SSD)** | < 15s | < 12s | < 3s | < 1s |
| **Min Spec (Celeron, 4GB, eMMC)** | < 30s | < 25s | < 5s | < 2s |

**Milestones:**
- T+6 months (Phase 1): Within 2× targets (≤ 16s reference)
- T+12 months (Phase 2): Meet targets exactly
- T+24 months (Phase 4): Exceed targets by 20%

---

## 3. Memory Limits

All measurements = fresh boot, default configuration, one empty workspace active, AI model loaded (3B quantized).

| Scenario | RAM Usage Target (RSS) | Notes |
|----------|---------------------|-------|
| **Idle, no apps, no AI** | < 384 MB | Headless mode, shell disabled |
| **Idle, AI 3B model loaded** | < 2.5 GB | Default config |
| **Idle, AI 7B Q4 model loaded** | < 5.5 GB | Recommend 8GB minimum for 7B |
| **Light Use** (browser + 1 workspace + chat) | < 6 GB | 16 GB system minimum comfortably |
| **Heavy Use** (2 VMs + 5 workspaces + 70B distributed) | < 20 GB | 32 GB system minimum |
| **Minimum System RAM** | 4 GB | Degraded mode, 1B model only |
| **Recommended System RAM** | 16 GB | Sweet spot, 7B model + apps |
| **Optimal System RAM** | 32 GB | 70B distributed inference + VMs |

**Per-Module Memory Budgets (Phase 1 GA):**
- Message Bus: < 32 MB
- Logging + Audit: < 16 MB
- Identity + Capability: < 24 MB
- Filesystem service cache: < 64 MB
- Workspace Manager: < 48 MB
- AI Runtime (without models): < 128 MB
- Vector Store (Qdrant, empty): < 256 MB
- UI Shell: < 192 MB (React + Tauri, optimized)
- Graphics + Compositor: < 96 MB

---

## 4. Latency Goals

Latency at p95, p99, p99.9 measured on reference hardware.

| Operation | p50 | p95 | p99 | p99.9 |
|-----------|-----|-----|-----|-------|
| **Message Bus IPC (same node)** | < 1 µs | < 5 µs | < 10 µs | < 50 µs |
| **Capability Validation** | < 10 µs | < 50 µs | < 200 µs | < 1 ms |
| **File Open (capability-checked)** | < 50 µs | < 200 µs | < 500 µs | < 2 ms |
| **Workspace Activation** | < 200 ms | < 500 ms | < 1 s | < 2 s |
| **Workspace Switch (UI)** | < 50 ms | < 200 ms | < 500 ms | < 1 s |
| **AI Chat - First Token (3B local)** | < 100 ms | < 300 ms | < 500 ms | < 1 s |
| **AI Chat - First Token (7B local)** | < 200 ms | < 500 ms | < 1 s | < 2 s |
| **AI Text Generation Throughput** | > 50 tok/s | > 30 tok/s | > 20 tok/s | > 10 tok/s |
| **Vector Search (k=20, 1M vectors)** | < 10 ms | < 30 ms | < 50 ms | < 100 ms |
| **Search Query (UI → Results)** | < 50 ms | < 200 ms | < 500 ms | < 1 s |
| **App Launch (native)** | < 100 ms | < 300 ms | < 500 ms | < 1 s |
| **Container App Launch** | < 1 s | < 2 s | < 4 s | < 8 s |
| **VM Boot (Linux)** | < 5 s | < 10 s | < 15 s | < 30 s |
| **VM Boot (Windows 11)** | < 20 s | < 40 s | < 60 s | < 90 s |
| **UI Interaction (click → paint)** | < 8 ms | < 16 ms | < 32 ms | < 48 ms |
| **HITL Permission Prompt** | < 50 ms | < 100 ms | < 250 ms | < 500 ms |
| **Notification Popup** | < 20 ms | < 50 ms | < 100 ms | < 250 ms |
| **Boot to Interactive Shell** (cold) | | < 8s | < 10s | < 15s |

---

## 5. Throughput Goals

| Metric | Minimum Target | Ideal Target |
|--------|---------------|-------------|
| **Message Bus Messages/sec** (same node, 64-byte payloads) | 500,000 msg/s | 1,000,000 msg/s |
| **IPC Bulk Transfer** (zero-copy memfd) | 20 GB/s | Bandwidth limit of RAM |
| **Filesystem Read (NVMe passthrough)** | 7 GB/s saturate hardware | Hardware limit |
| **Filesystem Write (NVMe)** | 5 GB/s saturate hardware | Hardware limit |
| **Vector Ingest Rate** (Qdrant, single node) | 10,000 vec/sec | 50,000 vec/sec |
| **Container Image Pull** (gzipped, cached layers) | 500 MB/s | 2 GB/s |
| **LLM Inference (7B Q4, single GPU)** | 80 tok/s user | 150+ tok/s |
| **Web Request Processing** (REST gateway) | 50,000 req/s | 200,000 req/s |

---

## 6. Scalability Targets

| Dimension | Single Node (Phase 1-3) | Multi-Node Cluster (Phase 6+) |
|-----------|----------------------|------------------------------|
| **Concurrent users / identities** | 1 local user + 10 remote | 100,000+ |
| **Active Workspaces** | 100 | 100,000+ |
| **Simultaneous Running Containers** | 64 | 10,000+ (K8s) |
| **Running VMs** | 16 (with IOMMU/SR-IOV) | 1,000+ |
| **Running AI Agents** | 256 | 1,000,000+ |
| **In-flight Plan Steps (Task Queue)** | 10,000 | 1,000,000 |
| **Vector Store Index Size** | 100M vectors | 10B+ vectors |
| **Audit Log Throughput** | 50,000 entries/s | 1M entries/s |
| **Online Nodes (Cluster Mode)** | N/A | 1,024-node sharded cluster |

---

## 7. Fault Tolerance & Availability

### Single Node

| Failure Type | Recovery Time (Goal) | User Experience |
|-------------|---------------------|-----------------|
| Plugin crash (Wasm) | < 10 ms restart | Transparent; user sees 1-2 frame glitch at worst |
| Container crash | < 500 ms (restart policy) | Depends on app; shows restart banner |
| Service crash (non-critical) | < 1 s restart + state replay | Degraded indicator; no data loss |
| Core service crash (CRITICAL) | < 2 s failover (backup instance) | Brief spinner; user notified of recovery |
| Message Bus crash | < 1 s dual-redundant failover | System-wide pause; then resume transparent |
| GPU driver fault/reset | < 3 s (reset without reboot) | Display flicker; running AI inference resumes |
| Kernel panic / Watchdog reset | < 30 s hardware reset | Unsaved AI state discarded; filesystem snapshots used to recover work |
| Power loss | N/A (hardware) | Journal recovery + fsck; last 1s writes may be lost (FUA used where possible) |

### Availability SLOs (Phase 5 Enterprise)

| Deployment Target | Availability SLA | Data Durability SLO |
|-------------------|------------------|---------------------|
| Single user desktop | Best effort; no SLA | 11 9's (ZFS/Btrfs RAID-1 mirror) |
| Enterprise workstation | 99.9% uptime | 12 9's + daily backups |
| Cloud cluster (Phase 6+) | 99.95% (≤ 4.38 hrs downtime/year) | 13 9's + geo-redundant replicas |
| Managed CognyxOS Cloud | 99.99% (≤ 52 min/year) | 14 9's + 3 AZ replicas |

---

## 8. Security Goals

See Security.md for full architecture. Summary goals:

| Area | Target |
|------|--------|
| **Capability Validation** | 100% of messages on bus are validated. No bypasses. |
| **Sandbox Escape Resistance** | ≥ 3 independent layers required to escape (defense in depth) |
| **Audit Trail Coverage** | 100% of security-relevant events; hash-chain integrity 100% |
| **Default Attack Surface Reduction** | Default install: 0 open inbound ports; 0 SUID binaries; seccomp on every process |
| **TPM 2.0 Use** | Measured Boot, LUKS auto-unlock, AIK remote attestation, sealed identity keys |
| **Full-Disk Encryption** | ON by default with Argon2id; cannot be disabled without Level-4 auth |
| **AI Prompt Injection Resistance** | ≥ 99.9% success in standard benchmark suites (JailbreakBench, HarmBench) |
| **Confused Deputy Mitigation** | Zero known capability-side-channel exploits in audit; capability tokens always carry resource scope |
| **CVE Response SLA** | CRITICAL CVEs in core OS: ≤ 24 hours from public disclosure → patch pushed to stable |

---

## 9. Offline Operation Requirements

1. **100% Functional Without Network.**
   - Local AI inference: yes.
   - Local file search: yes.
   - Local workspace management: yes.
   - VMs, containers, plugins without network caps: yes.

2. **No Feature Requires Cloud Unless Opt-in.**
   - Cloud AI inference is OPTIONAL. User toggles per-request.
   - Cloud backup is OPTIONAL. User configures.
   - User account is NOT required for local operation. Guest mode supported.

3. **Graceful Transition Between Offline ↔ Online.**
   - Network outages: work continues, local-only mode banner.
   - Reconnection: queued syncs replay in causal order, CRDT merge.
   - No "account verification" server calls that brick the system offline.

---

## 10. Cloud Synchronization (Optional)

| Feature | Latency Target | Consistency Model |
|---------|---------------|-------------------|
| Workspace state sync | < 1 s across devices (LAN) | CRDT strong eventual |
| Large file sync | Delta upload; 1 Gbps saturate bandwidth | Last-writer-wins per file |
| AI memory sync | < 5s | CRDT semantic merge |
| Cross-device work resume | < 20 s to full state | Snapshot + incremental delta |
| Encrypted off-site backup | Background; low priority | Full + incremental, BKDR hash integrity |

**Non-Negotiable:** Zero-knowledge encryption for all sync. CognyxOS Cloud or any third-party storage provider MUST never see plaintext. Keys held only by user identity, recoverable only via recovery seed.

---

## 11. GPU & Hardware Acceleration

| Feature | Support |
|---------|---------|
| **AI Inference Acceleration** | CUDA (NVIDIA), ROCm (AMD), Metal (macOS guests), QNN (Snapdragon), oneAPI (Intel) |
| **GPU Scheduling** | Per-user, per-workspace, per-process GPU time-slicing + DRM leases + SR-IOV |
| **Graphics API** | Vulkan 1.3 (primary), OpenGL 4.6 (compat), Direct3D 12 (VM guests only) |
| **Video Encode/Decode** | VA-API, NVENC/NVDEC, QSV, V4L2 Request API |
| **Compute Offload** | OpenCL 3.0, SYCL 2020, CUDA in containers via CDI |
| **DPU / SmartNIC (Phase 6+)** | Network + storage + vector search offload |

---

## 12. Architecture Support

### Primary (Tier 1 - Tested Every Commit)

| Architecture | Minimum Target | Optimization | Status |
|-------------|---------------|-------------|--------|
| **x86_64 (Intel/AMD)** | Haswell (AVX2) | AVX-512, AMX for AI inference, Zen5 tuning | Phase 1 GA |
| **AArch64 (ARM64)** | ARM v8.2-A (SVE) | SVE2, NEON, Snapdragon X optimizations | Phase 1 (beta), Phase 2 (GA) |

### Secondary (Tier 2 - Tested Daily)

| Architecture | Notes |
|-------------|-------|
| **RISC-V 64 GC (RV64GC)** | Vector extension v1.0; VisionFive 2 & Unmatched boards | Phase 3 experimental |

### Future (Tier 3 - Experimental)

| Architecture | Timeline |
|-------------|----------|
| **loongarch64** | Phase 4+ |
| **PowerPC64le (OpenPOWER)** | Phase 5+ enterprise server/cloud |

---

## 13. Future Mobile & XR Support

### CognyxOS Mobile (Phase 7)

| Requirement | Target |
|-------------|--------|
| **Base OS footprint** | < 4 GB system partition, < 512 MB idle RAM |
| **Battery life** | ≥ 14 hours screen-on time (reference phone 4500 mAh) |
| **Android App Runtime** | AOSP-compatible; CognyxOS permission bridge to Android runtime |
| **Cellular** | Modem sandboxed; IMS voice via capability-secured daemon |
| **Wake-from-idle AI** | NPU-based always-on AI assistant; < 1.5%/hr idle drain |

### CognyxOS Spatial / XR (Phase 7+)

| Requirement | Target |
|-------------|--------|
| **Frame Pacing** | 90/120 FPS synchronized to display, no reprojection artifacts |
| **Motion-to-Photon Latency** | < 20 ms end-to-end |
| **Workspace as Infinite 3D Canvas** | Unlimited windows, spatial AI entity presence |
| **Eye/Hand Tracking** | Privacy-preserving on-device processing only |

---

## 14. Energy Efficiency

Laptop battery life goals, real-world usage, 75Wh battery:

| Usage Pattern | Target Battery Life |
|---------------|--------------------|
| **Light use** (documents + browsing + AI occasionally) | ≥ 14 hours |
| **Productivity** (dev/IDE + containers + AI chat 50 tok/s) | ≥ 8 hours |
| **Heavy AI** (7B model continuous inference) | ≥ 4 hours |
| **Suspend** (S0ix / S3 idle) | ≥ 30 days standby, < 0.5%/hour drain |

Energy savings strategies:
- Big.LITTLE scheduler: UI on E-cores, AI inference on P-cores, NPU/GPU offload
- Race-to-idle: background work bursts; aggressive idle state entry
- AI compute dynamic clocking: adjust model quantization based on battery% + user preference
- eBPF-based usage patterns → per-workspace idle freeze when not in use

---

## 15. Internationalization & Accessibility

### I18N / L10N

- **First-class Unicode 15+** everywhere: UTF-8 on disk, UTF-16 in Windows compat only
- **Bidirectional text:** Arabic + Hebrew UI & terminal
- **Complex scripts shaping:** HarfBuzz-backed rendering (Indic, Thai, etc.)
- **Ship with ≥ 40 language packs** in GA release
- **Date/Time/Number/Calendar locale:** ICU74+

### Accessibility (WCAG 2.2 AA at minimum; AAA target for UI shell)

- Screen reader: Orca + built-in AI narration (AI can read aloud and summarize UI)
- Keyboard-navigable everything
- High contrast, reduced motion, text scaling 200% without breakage
- Voice control (via AI) as primary input option
- Colorblind-safe default palettes

---

## 16. Measurement Methodology

All benchmarks follow strict methodology for reproducible results:

1. **Hardware reference fleet** (results reported alongside hardware exact model)
2. **Fixed kernel config,** mitigations=auto (no cheating by disabling Spectre/Meltdown)
3. **No background daemons,** no GUI animations disabled
4. **30 runs per metric**; report p50, p95, p99, p99.9, min, max, stdev
5. **Temperature stabilized.** Benchmarks only run after CPU/GPU temps within ±3°C of ambient for 10 minutes.
6. **Open source benchmark harness** (`tests/performance/`) so results are independently verifiable.
7. **Automated per-PR benchmark regression detection** for hot paths: >5% regression on p95 = automatic PR block, with flamegraph diff.
