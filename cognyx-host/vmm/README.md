# CognyxOS Virtual Machine Manager (VMM)

## Overview
The VMM subsystem provides hardware-accelerated virtualization for Windows, macOS, and Linux sandboxes with GPU passthrough, resource isolation, and snapshot capabilities.

## Architecture Components

### 1. QEMU/KVM Integration
- **KVM Kernel Module**: Direct hardware virtualization via `/dev/kvm`
- **QEMU Process Manager**: Spawns isolated QEMU processes per VM
- **Virtio Drivers**: High-performance paravirtualized I/O
- **CPU Pinning**: Dedicated core allocation for real-time performance

### 2. GPU Passthrough Engine
- **VFIO-PCI**: Direct device assignment for NVIDIA/AMD/Intel GPUs
- **vGPU Support**: Time-sliced GPU sharing for multiple VMs
- **Driver Isolation**: Host never loads guest GPU drivers
- **Memory Mapping**: DMA remapping for secure GPU access

### 3. Sandbox Implementations
- **Windows Sandbox**: UEFI boot, TPM 2.0 emulation, VirtIO drivers
- **macOS Sandbox**: Apple Silicon virtualization (ARM64), Intel legacy support
- **Linux Sandbox**: Lightweight container-VM hybrid

### 4. Resource Isolation
- **Cgroups v2**: CPU, memory, I/O limits per VM
- **Namespaces**: PID, network, mount isolation
- **Seccomp-bpf**: System call filtering
- **AppArmor Profiles**: Mandatory access control

### 5. Shared Resources
- **Shared Memory**: ivshmem for low-latency host-guest communication
- **Shared Files**: 9p filesystem for efficient file exchange
- **Clipboard Sync**: Secure copy-paste bridge
- **Drag & Drop**: File transfer protocol

### 6. Snapshot Engine
- **Live Snapshots**: QEMU internal state + disk state
- **Incremental Backups**: Block-level change tracking
- **External Snapshots**: LVM/ZFS integration
- **Snapshot Chains**: Multi-generation restore

### 7. Suspend/Resume
- **State Serialization**: RAM + CPU registers + device state
- **Fast Resume**: Sub-second restoration
- **Migration Support**: Live migration between hosts
- **Power Management**: ACPI S3/S4 states

### 8. Fast Boot
- **UEFI OVMF**: Optimized firmware loading
- **Initrd Caching**: Pre-loaded kernel images
- **Parallel Device Init**: Concurrent hardware initialization
- **Snapshot Boot**: Instant restore from saved state

### 9. Lifecycle APIs
- **Create**: Define VM spec, allocate resources
- **Start**: Boot sequence with health checks
- **Stop**: Graceful shutdown with timeout
- **Pause**: Freeze execution without state loss
- **Destroy**: Cleanup resources securely
- **Monitor**: Real-time metrics streaming

## Security Model
- **Isolation**: Each VM runs in separate security context
- **Encryption**: Disk encryption (LUKS), memory encryption (AMD SEV)
- **Network Filtering**: ebtables/nftables per VM
- **Audit Logging**: All VMM operations logged

## Performance Targets
- **Boot Time**: < 2s (snapshot), < 10s (cold)
- **GPU Overhead**: < 5% vs bare metal
- **Memory Overhead**: < 2% per VM
- **I/O Throughput**: Near-native with VirtIO
