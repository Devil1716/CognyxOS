# CognyxOS Host Kernel

Minimal Linux-based infrastructure layer providing hardware abstraction, virtualization support, and core services for the Agent Kernel.

## Directory Structure

```
cognyx-host/
├── boot/                    # Boot configuration and initramfs
│   ├── cmdline              # Kernel command line parameters
│   ├── initramfs.cpio.gz    # Minimal initramfs with essential tools
│   └── grub.cfg             # GRUB configuration for bare-metal boot
├── kernel/                  # Kernel modules and services
│   ├── scheduler/           # Intent-aware process scheduler extensions
│   │   └── cognyx_sched.c   # Custom scheduler for agent workloads
│   ├── memory/              # Memory management subsystems
│   │   ├── cgroup_config    # Cgroup v2 configuration for isolation
│   │   └── memguard.c       # Memory protection for VMs
│   ├── ipc/                 # Inter-process communication primitives
│   │   ├── virtio_ipc.c     # Virtio-based IPC for VMs
│   │   └── unix_sock_mgr.c  # Unix socket manager for local services
│   └── drivers/             # Hardware drivers (GPU, NIC, Storage)
│       ├── gpu/             # GPU passthrough and virtualization
│       │   ├── nvidia_vfio.c
│       │   └── amd_kfd.c
│       ├── nic/             # High-performance networking
│       │   └── dpdk_bind.c
│       └── storage/         # NVMe and storage controllers
│           └── nvme_virt.c
├── storage/                 # Storage subsystem
│   ├── pool_manager.py      # ZFS/Btrfs pool management
│   ├── snapshot_service.py  # Atomic snapshot creation
│   └── encryption.py        # LUKS/dm-crypt integration
├── network/                 # Networking stack
│   ├── bridge_manager.py    # Virtual bridge creation
│   ├── nat_rules.py         # NAT and port forwarding
│   └── firewall.py          # nftables-based firewall
├── virt/                    # Virtualization platform
│   ├── hypervisor/          # KVM/QEMU management
│   │   ├── vm_factory.py    # VM lifecycle management
│   │   ├── vsock_handler.py # VSOCK communication handler
│   │   └── gpu_passthrough.py # GPU device assignment
│   └── runtimes/            # Runtime-specific configurations
│       ├── linux_runtime.yaml
│       ├── windows_runtime.yaml
│       ├── macos_runtime.yaml
│       └── android_runtime.yaml
├── config/                  # System configuration
│   ├── host.conf            # Host-level settings
│   ├── security.conf        # Security policies
│   └── capabilities.yaml    # Capability definitions
└── scripts/                 # Build and deployment scripts
    ├── build_host.sh        # Build minimal host image
    ├── deploy_baremetal.sh  # Deploy to bare metal
    └── update_kernel.sh     # A/B kernel update script
```

## Core Components

### 1. Boot Layer
- **Purpose**: Minimal boot sequence to initialize hardware and launch virtualization platform
- **Implementation**: Custom initramfs with only essential drivers (NVMe, NIC, GPU VFIO)
- **Boot Time Target**: < 3 seconds from BIOS to hypervisor ready

### 2. Kernel Services
- **Custom Scheduler**: Extends CFS with intent-aware priority classes
- **Memory Guard**: Prevents VM escape through memory corruption
- **Virtio IPC**: High-speed communication between host and guest VMs

### 3. Storage Subsystem
- **Copy-on-Write**: ZFS or Btrfs for atomic snapshots
- **Encryption**: Full-disk encryption with per-VM keys
- **Pool Management**: Dynamic storage allocation for VMs

### 4. Networking Stack
- **Virtual Bridges**: Isolated networks per runtime type
- **NAT/Firewall**: Strict egress filtering with nftables
- **DPDK Support**: High-throughput packet processing

### 5. Virtualization Platform
- **KVM-Based**: Hardware-accelerated virtualization
- **GPU Passthrough**: VFIO-based device assignment
- **VSOCK**: Socket-based communication with guests
- **Micro-VMs**: Firecracker-compatible lightweight VMs

## Security Model
- Immutable root filesystem
- Signed kernel modules only
- SELinux/AppArmor enforced
- No user-space access to host kernel
- All VMs run in isolated namespaces

## Performance Targets
- VM startup: < 500ms
- IPC latency: < 10μs
- GPU passthrough overhead: < 5%
- Network throughput: Near line-rate

## Dependencies
- Linux Kernel 6.8+ (with KVM, VFIO, Virtio)
- QEMU 8.2+ (for full VMs)
- Firecracker (for micro-VMs)
- ZFS on Linux or Btrfs
- DPDK (optional, for high-performance networking)
