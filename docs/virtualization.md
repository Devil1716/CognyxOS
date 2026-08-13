# CognyxOS Virtualization Abstraction & KVM Backend

> **Document ID:** ARCH-PHASE2-VIRTUALIZATION  
> **Version:** 1.0.0  

---

## 1. Virtual Machine Lifecycle

```mermaid
stateDiagram-v2
    [*] --> Stopped
    Stopped --> Starting : start_vm()
    Starting --> Running : QEMU process active
    Running --> Paused : pause_vm()
    Paused --> Running : resume_vm()
    Running --> Stopping : stop_vm() / shutdown_vm()
    Stopping --> Stopped
    Running --> Failed : QEMU Crash
    Failed --> Stopped
```

## 2. Abstraction Interface
- `VirtualMachineManager`: Platform-agnostic manager.
- `VirtualizationBackend`: Interface implemented by `KvmBackend` (QEMU/KVM with UEFI, TPM, VirtIO) and `MockBackend` (CI/test simulator).
