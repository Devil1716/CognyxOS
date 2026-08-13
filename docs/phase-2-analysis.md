# CognyxOS Phase 2 Integration & Repository Analysis

> **Document ID:** ARCH-PHASE2-ANALYSIS
> **Version:** 1.0.0
> **Status:** Completed
> **Author:** Principal Systems Engineer

---

## 1. Overview & Architectural Boundaries

Phase 2 builds the **Hypervisor + Multi-OS Execution Runtime** layer directly on top of the Phase 1 Linux host foundation.

### Core Principle
- **Linux Host Ownership:** The underlying Linux host owns hardware, kernel, drivers, storage, networking, GPU, USB, memory, CPU scheduling, and virtualization primitives (`kvm`, `cgroups v2`, `namespaces`, `seccomp`).
- **No Phase 1 Rewrites:** Phase 1 components (`cognyx-bus`, `cognyx-supervisor`, `cognyx-proto`, `cognyx-sdk`, system service scaffolding) remain intact and are directly reused as the foundation.
- **Isolated Execution Environments:** Phase 2 introduces platform-independent virtualization abstractions and runtime interfaces for Native Linux, Containers, Windows VMs, macOS VMs (local/remote), and Remote Runtimes.

---

## 2. Phase 1 Reuse Matrix

| Phase 1 Component | Location | Phase 2 Reuse Pattern |
| :--- | :--- | :--- |
| **Message Bus Daemon** | `services/bus` | Central IPC broker for runtime control RPCs, gRPC streams, and pub/sub lifecycle events. |
| **System Supervisor** | `system/supervisor` | Manages runtime service daemons (`cognyx-runtime-manager`, `cognyx-storage-manager`, `cognyx-network-manager`). |
| **Protobuf Schemas** | `proto/` | Extended with `proto/services/runtime_services.proto` for runtime management gRPC APIs. |
| **Protobuf Codegen Crate** | `proto/gen/rust` | Updated `build.rs` to generate Rust gRPC client/server code for runtime services using `protoc-bin-vendored`. |
| **Rust SDK** | `sdk/rust` | `BusClient` utilized by CLI tool (`cognyx`) and runtime engines to communicate with `cognyx-bus`. |
| **Service Scaffolding** | `services/` | Integrated with process, security, network, storage, and telemetry daemons. |

---

## 3. Phase 2 Component Specifications

### 3.1 Virtualization Abstraction (`runtime/virtualization`)
- **Traits & Interfaces:**
  - `VirtualMachineManager`, `VirtualMachine`, `VirtualMachineConfig`, `VirtualMachineState`, `VirtualMachineSnapshot`, `VirtualMachineNetwork`, `VirtualMachineStorage`, `VirtualMachineDevice`, `VirtualMachineResource`.
- **Backend Abstraction:**
  - `VirtualizationBackend` trait implemented by `KvmBackend` (production QEMU/KVM driver) and `MockBackend` (CI/unit testing without nested virtualization).

### 3.2 Unified Execution Runtime (`runtime/execution`)
- **Common Interface:** `ExecutionRuntime` implemented by `LinuxRuntime`, `WindowsRuntime`, `MacOSRuntime`, `ContainerRuntime`, and `RemoteRuntime`.
- **Runtime Metadata & Capability Discovery:** Runtime ID, type, status, capabilities, resources, health, latency, location, security level, and tool registry.

### 3.3 Container Runtime (`runtime/container`)
- `ContainerRuntime` wrapping `DockerContainerBackend` / `containerd` / `Podman` and `MockContainerBackend`.
- Full container lifecycle: `create`, `start`, `stop`, `restart`, `pause`, `resume`, `delete`, `exec`, `logs`, `inspect`, `metrics`, volume mounts, GPU access.

### 3.4 Windows Runtime (`runtime/windows`)
- Modular separation into 5 distinct components:
  1. `WindowsVmManager` (VM lifecycle)
  2. `WindowsGuestCommunication` (gRPC / VirtIO-vsock)
  3. `WindowsAppAutomation` (Win32 / PowerShell / UI automation)
  4. `WindowsFilesystemBridge` (Shared folder / virtio-fs)
  5. `WindowsCapabilityAdapter` (Permission translation)

### 3.5 macOS Runtime (`runtime/macos`)
- Architectural support for Apple licensing compliance:
  - `MacOSRuntime`, `MacOSExecutionBackend`, `LocalMacBackend` (local hypervisor when on Apple hardware), `RemoteMacBackend` (authorized remote Mac host).

### 3.6 Resource Management (`runtime/resources`)
- `ResourceManager`, `ResourceScheduler`, `ResourceQuota`, `ResourceReservation`, `ResourceMetrics`.
- Tracks CPU, RAM, GPU, VRAM, Storage, Network, USB, Processes, VMs, and Containers.

### 3.7 Storage Foundation (`runtime/storage`)
- `VMStorageManager`: Support for qcow2, raw, virtual disks, CoW snapshots, disk resizing, cloning, image versioning.
- Centralized configuration: `/var/lib/cognyxos/images`, `/var/lib/cognyxos/disks`, `/var/lib/cognyxos/snapshots`.

### 3.8 Virtual Networking (`runtime/network`)
- `VirtualNetworkManager`: NAT, bridge, isolated networks, firewall rules, DNS, service discovery, policy evaluator (`can_communicate(runtime_a, runtime_b)`).

### 3.9 Guest Communication (`runtime/guest`)
- Decoupled modules: `GuestControl`, `GuestFileSystem`, `GuestProcess`, `GuestNetwork`, `GuestMetrics`, `GuestAutomation`.

### 3.10 Developer CLI (`devtools/cli`)
- CLI binary `cognyx` supporting `cognyx runtime list`, `create`, `start`, `stop`, `inspect`, `snapshot`, `restore`, `metrics`, `capabilities`.

---

## 4. Risks & Limitations

1. **Host Virtualization Hardware:** Running full QEMU/KVM hardware virtualization requires KVM kernel modules (`/dev/kvm`). In CI or non-nested VM environments, `MockBackend` handles tests.
2. **macOS Licensing:** Virtualizing macOS locally requires Apple-branded hardware. `RemoteMacBackend` allows delegating execution to a physical Mac worker over network IPC.
3. **Windows Guest Automation:** Full UI automation requires the Windows guest agent to be running inside the Windows VM.
