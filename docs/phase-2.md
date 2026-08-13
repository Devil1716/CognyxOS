# CognyxOS Phase 2 Architecture & Execution Overview

> **Document ID:** ARCH-PHASE2-OVERVIEW  
> **Version:** 1.0.0  
> **Status:** Implemented  

---

## 1. System Architecture Diagram

```mermaid
graph TD
    A[CognyxOS Host Linux Kernel] --> B[Process Manager / Supervisor]
    B --> C[cognyx-bus IPC Daemon]
    C --> D[cognyx-runtime-manager]
    
    subgraph Execution Runtimes
        D --> E[Linux Native Runtime]
        D --> F[Windows VM Runtime - QEMU/KVM]
        D --> G[macOS VM Runtime - Local/Remote]
        D --> H[Container Runtime - Docker/containerd]
    end
    
    subgraph Subsystems
        F --> I[VM Storage Manager - qcow2/raw]
        F --> J[Virtual Network Manager - NAT/Bridge]
        F --> K[Guest Agent / vsock]
        D --> L[Resource Manager & Scheduler]
    end
```

## 2. Core Components & Responsibilities
- **Virtualization Abstraction (`cognyx-virtualization`):** Platform-independent interface (`VirtualMachineManager`, `VirtualMachineConfig`, `VirtualMachineSnapshot`) supporting production `KvmBackend` and test `MockBackend`.
- **Unified Runtime Interface (`cognyx-execution`):** Common `ExecutionRuntime` trait enabling capability discovery (`can_perform(capability)`).
- **Multi-OS Support:** Native Linux, Windows VM, macOS VM (Apple compliance/remote worker), Container execution.
- **Resource Management:** Real-time quota enforcement & reservation tracking.
