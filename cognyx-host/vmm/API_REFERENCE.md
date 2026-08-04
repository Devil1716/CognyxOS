# CognyxOS VMM API Reference

## Overview
The Virtual Machine Manager (VMM) provides hardware-accelerated virtualization for Windows, macOS, and Linux sandboxes with GPU passthrough, resource isolation, and snapshot capabilities.

## Module Structure

```
vmm/
├── qemu/
│   └── qemu_manager.py      # QEMU/KVM integration
├── gpu/
│   └── gpu_passthrough.py   # VFIO-PCI GPU management
├── sandbox/
│   ├── windows/
│   │   └── windows_sandbox.py   # Windows VM factory
│   └── macos/
│       └── macos_sandbox.py     # macOS VM factory
├── snapshot/
│   └── snapshot_engine.py   # Snapshots & migration
├── lifecycle/
│   └── lifecycle_api.py     # Unified lifecycle management
└── README.md                # This file
```

## Core Components

### 1. QEMUManager (`qemu/qemu_manager.py`)

Manages QEMU process lifecycle and KVM device access.

**Key Classes:**
- `QEMUManager` - Process management
- `KVMManager` - Low-level KVM device access
- `VMSpec` - VM configuration dataclass
- `CPUConfig`, `MemoryConfig`, `DiskConfig`, `NetworkConfig`, `GPUConfig`, `TPMConfig`

**Example:**
```python
from vmm.qemu.qemu_manager import QEMUManager, VMSpec, CPUConfig, MemoryConfig

qemu_mgr = QEMUManager()

spec = VMSpec(
    uuid="550e8400-e29b-41d4-a716-446655440000",
    name="my-vm",
    cpu=CPUConfig(cores=4, pinning=[4,5,6,7]),
    memory=MemoryConfig(size_mb=8192),
    disks=[...],
    network=NetworkConfig()
)

qemu_mgr.create_vm(spec)
```

### 2. GPUPassthroughEngine (`gpu/gpu_passthrough.py`)

Manages VFIO-PCI device assignment and vGPU slicing.

**Key Classes:**
- `GPUPassthroughEngine` - Main coordination engine
- `VFIOManager` - VFIO-PCI binding
- `vGPUManger` - NVIDIA/Intel vGPU management
- `VirtIOGPUManager` - Paravirtualized graphics

**Example:**
```python
from vmm.gpu.gpu_passthrough import GPUPassthroughEngine, GPUMode

engine = GPUPassthroughEngine()

# Assign GPU to VM
engine.assign_gpu(
    vm_uuid="550e8400-e29b-41d4-a716-446655440000",
    pci_address="01:00.0",
    mode=GPUMode.PASSTHROUGH
)

# Verify isolation
isolation = engine.check_isolation(vm_uuid)
```

### 3. WindowsSandboxManager (`sandbox/windows/windows_sandbox.py`)

Creates optimized Windows VM configurations with UEFI, TPM 2.0, and VirtIO drivers.

**Supported Versions:**
- Windows 10
- Windows 11 (requires TPM 2.0)
- Windows Server 2022

**Example:**
```python
from vmm.sandbox.windows import WindowsSandboxManager

mgr = WindowsSandboxManager()

spec = mgr.create(
    version="windows11",
    vm_name="win11-dev",
    cpu_cores=4,
    memory_mb=8192,
    disk_size_gb=100,
    gpu_passthrough=True,
    gpu_pci="01:00.0"
)
```

### 4. MacOSSandboxManager (`sandbox/macos/macos_sandbox.py`)

Creates macOS VM configurations for Apple Silicon (legal) and Intel (EULA restricted).

**Supported Versions:**
- macOS Sonoma (ARM64, Apple Silicon only)
- macOS Ventura (ARM64, Apple Silicon only)
- macOS Monterey (ARM64, Apple Silicon only)

**Legal Compliance Check:**
```python
from vmm.sandbox.macos import MacOSSandboxManager

mgr = MacOSSandboxManager()

compliance = mgr.factory.check_legal_compliance("macos_sonoma")
if not compliance["compliant"]:
    print(f"Cannot create: {compliance['reason']}")
```

### 5. SnapshotEngine (`snapshot/snapshot_engine.py`)

Provides VM snapshot, restore, suspend, resume, and migration.

**Snapshot Types:**
- `LIVE` - Running VM state (RAM + disk)
- `COLD` - Stopped VM disk only
- `EXTERNAL` - LVM/ZFS integration

**Example:**
```python
from vmm.snapshot import SnapshotEngine, SuspendResumeManager

engine = SnapshotEngine()
suspend_mgr = SuspendResumeManager(engine)

# Create live snapshot
snapshot = engine.create_live_snapshot(
    vm_uuid="550e8400-e29b-41d4-a716-446655440000",
    name="pre-update"
)

# Suspend VM
suspend_snapshot = suspend_mgr.suspend(vm_uuid)

# Resume
suspend_mgr.resume(suspend_snapshot.id)
```

### 6. VMLifecycleManager (`lifecycle/lifecycle_api.py`)

Unified lifecycle management for all VM types.

**Lifecycle Events:**
- CREATED, STARTING, STARTED
- STOPPING, STOPPED
- PAUSED, RESUMED
- SUSPENDED, DESTROYED, ERROR

**Example:**
```python
from vmm.lifecycle import VMLifecycleManager, VMType

lifecycle = VMLifecycleManager()

# Register callbacks
lifecycle.register_callback(LifecycleEvent.STARTED, lambda uuid, inst: print(f"Started: {uuid}"))

# Create VM
vm = lifecycle.create_vm(
    vm_type=VMType.WINDOWS,
    name="win11-test",
    version="windows11"
)

# Start
lifecycle.start_vm(vm.uuid)

# Get metrics
metrics = lifecycle.get_metrics(vm.uuid)

# Stop and destroy
lifecycle.stop_vm(vm.uuid)
lifecycle.destroy_vm(vm.uuid)
```

## Resource Isolation

### CPU Pinning
```python
cpu_config = CPUConfig(
    cores=4,
    pinning=[4, 5, 6, 7]  # Dedicated physical cores
)
```

### Memory Hugepages
```python
memory_config = MemoryConfig(
    size_mb=8192,
    hugepages=True,  # Use 2MB pages
    prealloc=True    # Pre-allocate at startup
)
```

### Cgroups v2 Limits
Applied automatically based on VM spec:
- `cpu.max` - CPU time limits
- `memory.max` - Memory limits
- `io.max` - I/O bandwidth limits

## Security Model

### IOMMU Isolation
- All GPU passthrough requires IOMMU
- Devices isolated by IOMMU group
- DMA remapping enabled

### VFIO-PCI Binding
- Devices bound to vfio-pci driver
- Host cannot access assigned devices
- Secure device handoff to VM

### TPM 2.0 Emulation
- Software TPM (swtpm) for Windows 11
- Isolated per-VM TPM state
- Secure boot support

## Performance Targets

| Metric | Target |
|--------|--------|
| Cold Boot | < 10s |
| Snapshot Boot | < 2s |
| GPU Overhead | < 5% |
| Memory Overhead | < 2% |
| Network Throughput | Near-native (VirtIO) |
| Disk I/O | Near-native (VirtIO-blk) |

## APIs Summary

| Operation | Method | Description |
|-----------|--------|-------------|
| Create VM | `create_vm()` | Instantiate new VM |
| Start VM | `start_vm()` | Boot VM |
| Stop VM | `stop_vm()` | Graceful shutdown |
| Pause VM | `pause_vm()` | Freeze execution |
| Resume VM | `resume_vm()` | Unfreeze execution |
| Suspend VM | `suspend_vm()` | Save state to disk |
| Destroy VM | `destroy_vm()` | Delete VM and resources |
| Snapshot | `create_live_snapshot()` | Capture VM state |
| Restore | `restore_snapshot()` | Restore from snapshot |
| Migrate | `migrate_outgoing()` | Live migrate to host |

## Error Handling

All methods return boolean success or raise exceptions:

```python
try:
    success = lifecycle.start_vm(vm_uuid)
    if not success:
        print("Failed to start VM")
except RuntimeError as e:
    print(f"Error: {e}")
```

## Monitoring

Metrics available via `get_metrics()`:
- CPU usage percentage
- Memory usage bytes
- Disk I/O bytes/sec
- Network I/O bytes/sec
- VM state

## Integration Points

### With Agent Kernel
- Lifecycle events published to event bus
- Capability runtime queries VM state
- Intent execution triggers VM operations

### With Storage Layer
- Disk images stored in `/var/lib/cognyx/vms/`
- Snapshots in `/var/lib/cognyx/snapshots/`
- Supports LVM, ZFS, btrfs backends

### With Network Layer
- Bridge networking via `cognyx0`
- NAT for internet access
- Firewall rules per VM

## Future Extensions

- Android sandbox support
- Cloud runtime integration
- Remote execution federation
- WASM-based lightweight sandboxes
