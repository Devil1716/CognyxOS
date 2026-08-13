# CognyxOS Runtime API & gRPC Specification

> **Document ID:** ARCH-PHASE2-API  
> **Version:** 1.0.0  

---

## 1. gRPC Service Definition (`RuntimeManagerService`)
Defined in `proto/services/runtime_services.proto`.

### Methods
- `CreateRuntime`: Spawns VM or container configuration.
- `StartRuntime` / `StopRuntime` / `PauseRuntime` / `ResumeRuntime` / `DeleteRuntime`: Controls execution state.
- `CreateSnapshot` / `RestoreSnapshot`: Manages state checkpoints.
- `GetMetrics`: Fetches CPU, RAM, disk, and network telemetry.
- `CheckNetworkPolicy`: Evaluates inter-runtime communication rules.
