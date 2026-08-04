"""
CognyxOS VM Factory

Purpose: Create, manage, and destroy virtual machines for different
runtime environments (Linux, Windows, macOS, Android).

Features:
- QEMU/KVM-based full virtualization
- Firecracker micro-VM support
- GPU passthrough configuration
- VSOCK communication setup
- Automatic resource allocation
"""

import subprocess
import json
import uuid
from pathlib import Path
from dataclasses import dataclass, field
from typing import Optional, Dict, Any, List
from enum import Enum


class VMType(Enum):
    LINUX = "linux"
    WINDOWS = "windows"
    MACOS = "macos"
    ANDROID = "android"
    FIRECRACKER = "firecracker"


@dataclass
class VMConfig:
    vm_id: str
    vm_type: VMType
    vcpus: int
    memory_mb: int
    disk_path: str
    disk_size_gb: int
    gpu_passthrough: bool = False
    gpu_device_idx: Optional[int] = None
    network_bridge: Optional[str] = None
    vsock_cid: int = 0
    extra_args: Dict[str, Any] = field(default_factory=dict)


class VMFactory:
    """Factory for creating and managing CognyxOS VMs."""
    
    def __init__(self, storage_pool_path: str = "/cognyx-pool/vms"):
        self.storage_pool = Path(storage_pool_path)
        self.vms: Dict[str, VMConfig] = {}
        self.running_vms: Dict[str, subprocess.Popen] = {}
        
    def create_vm(self, vm_type: VMType, vm_id: Optional[str] = None,
                  vcpus: int = 2, memory_mb: int = 4096,
                  disk_size_gb: int = 50, gpu_passthrough: bool = False,
                  **kwargs) -> VMConfig:
        """
        Create a new VM configuration.
        
        Reasoning: Different runtime types have different requirements.
        Windows needs TPM, macOS needs specific CPU features, etc.
        """
        if vm_id is None:
            vm_id = f"{vm_type.value}-{uuid.uuid4().hex[:8]}"
        
        # Validate VM ID format
        if not vm_id.replace("-", "").replace("_", "").isalnum():
            raise ValueError("Invalid VM ID format")
        
        # Create disk image path
        disk_path = self.storage_pool / vm_id / "disk.qcow2"
        disk_path.parent.mkdir(parents=True, exist_ok=True)
        
        # Create qcow2 disk image
        subprocess.run([
            "qemu-img", "create", "-f", "qcow2",
            "-o", "cluster_size=2M", "-o", "preallocation=metadata",
            str(disk_path), f"{disk_size_gb}G"
        ], check=True)
        
        # Assign VSOCK CID
        vsock_cid = self._allocate_vsock_cid()
        
        # Build VM configuration
        config = VMConfig(
            vm_id=vm_id,
            vm_type=vm_type,
            vcpus=vcpus,
            memory_mb=memory_mb,
            disk_path=str(disk_path),
            disk_size_gb=disk_size_gb,
            gpu_passthrough=gpu_passthrough,
            vsock_cid=vsock_cid,
            extra_args=kwargs
        )
        
        # Add type-specific configurations
        if vm_type == VMType.WINDOWS:
            config.extra_args.update({
                "tpm": True,
                "secure_boot": True,
                "uefi": "OVMF_CODE.fd",
            })
        elif vm_type == VMType.MACOS:
            config.extra_args.update({
                "cpu_model": "Penryn",
                "kernelsupport": True,
                "machine_type": "q35",
            })
        elif vm_type == VMType.ANDROID:
            config.extra_args.update({
                "kernel": "/usr/share/android/kernel.img",
                "initrd": "/usr/share/android/ramdisk.img",
                "cmdline": "androidboot.hardware=qemu androidboot.serialno=EMULATOR",
            })
        elif vm_type == VMType.FIRECRACKER:
            config.extra_args.update({
                "microvm": True,
                "jailer": True,
            })
        
        self.vms[vm_id] = config
        return config
    
    def start_vm(self, vm_id: str, iso_path: Optional[str] = None) -> subprocess.Popen:
        """
        Start a VM with QEMU/KVM.
        
        Reasoning: QEMU provides full hardware emulation needed
        for Windows and macOS guests with GPU passthrough support.
        """
        if vm_id not in self.vms:
            raise ValueError(f"VM {vm_id} does not exist")
        
        if vm_id in self.running_vms:
            raise RuntimeError(f"VM {vm_id} is already running")
        
        config = self.vms[vm_id]
        
        # Build QEMU command
        cmd = [
            "qemu-system-x86_64",
            "-name", f"cognyx-{vm_id}",
            "-machine", "q35,accel=kvm",
            "-smp", str(config.vcpus),
            "-m", str(config.memory_mb),
            "-drive", f"file={config.disk_path},format=qcow2,if=virtio,cache=none",
            "-netdev", f"tap,id=net0,ifname=tap-{vm_id},script=no,downscript=no",
            "-device", "virtio-net-pci,netdev=net0,mac=52:54:00:12:34:56",
            "-device", "vhost-vsock-pci,guest-cid={}".format(config.vsock_cid),
            "-display", "none",
            "-daemonize",
        ]
        
        # Add ISO for installation
        if iso_path:
            cmd.extend([
                "-drive", f"file={iso_path},media=cdrom,format=raw",
                "-boot", "d"
            ])
        
        # Windows-specific options
        if config.vm_type == VMType.WINDOWS:
            cmd.extend([
                "-device", "tpm-tis,tpmdev=tpm0",
                "-chardev", "socket,id=tpm0,path=/dev/vtpm0",
                "-global", "ICH9-LPC.disable_s3=1",
            ])
            
            if config.extra_args.get("secure_boot"):
                cmd.extend([
                    "-drive", f"if=pflash,format=raw,file=/usr/share/OVMF/OVMF_CODE.secboot.fd,readonly",
                    "-drive", f"if=pflash,format=raw,file=/usr/share/OVMF/OVMF_VARS.secboot.fd",
                ])
        
        # macOS-specific options
        elif config.vm_type == VMType.MACOS:
            cmd.extend([
                "-cpu", config.extra_args.get("cpu_model", "Penryn"),
                "-device", "isa-applesmc,osk=ourhardworkbythesewordsguardedpleasedontsteal(c)AppleComputerInc",
                "-smbios", "type=2",
            ])
        
        # GPU passthrough
        if config.gpu_passthrough and config.gpu_device_idx is not None:
            # VFIO PCI device assignment
            cmd.extend([
                "-device", "vfio-pci,host=00:{}.0,multifunction=on".format(
                    config.gpu_device_idx
                ),
                "-device", "vfio-pci,host=00:{}.1".format(config.gpu_device_idx),
            ])
        
        # Firecracker micro-VM mode
        if config.vm_type == VMType.FIRECRACKER:
            return self._start_firecracker_vm(config)
        
        # Start QEMU process
        process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.running_vms[vm_id] = process
        
        return process
    
    def stop_vm(self, vm_id: str, force: bool = False) -> None:
        """Stop a running VM."""
        if vm_id not in self.running_vms:
            return
        
        process = self.running_vms[vm_id]
        
        if force:
            process.kill()
        else:
            # Graceful shutdown via QEMU monitor
            subprocess.run([
                "qemu-monitor", "-p", f"/var/run/cognyx-{vm_id}.sock",
                "-c", "system_powerdown"
            ])
            process.wait(timeout=60)
        
        del self.running_vms[vm_id]
    
    def destroy_vm(self, vm_id: str) -> None:
        """Destroy VM and all associated resources."""
        # Stop if running
        if vm_id in self.running_vms:
            self.stop_vm(vm_id, force=True)
        
        if vm_id in self.vms:
            config = self.vms[vm_id]
            
            # Delete disk image
            disk_path = Path(config.disk_path)
            if disk_path.exists():
                disk_path.unlink()
            
            # Release VSOCK CID
            self._release_vsock_cid(config.vsock_cid)
            
            del self.vms[vm_id]
    
    def snapshot_vm(self, vm_id: str, snapshot_name: str) -> str:
        """Create VM snapshot using storage pool."""
        if vm_id not in self.vms:
            raise ValueError(f"VM {vm_id} does not exist")
        
        from storage.pool_manager import StoragePoolManager
        manager = StoragePoolManager()
        
        snapshot_path = manager.create_snapshot(vm_id, snapshot_name)
        return snapshot_path
    
    def _allocate_vsock_cid(self) -> int:
        """Allocate unique VSOCK CID."""
        # CIDs 0-2 are reserved, 3-100 for host services
        # Use 101+ for VMs
        existing_cids = {cfg.vsock_cid for cfg in self.vms.values()}
        
        for cid in range(101, 0xFFFFFFFF):
            if cid not in existing_cids:
                return cid
        
        raise RuntimeError("No available VSOCK CIDs")
    
    def _release_vsock_cid(self, cid: int) -> None:
        """Release VSOCK CID back to pool."""
        pass  # Simple implementation - just allow reuse
    
    def _start_firecracker_vm(self, config: VMConfig) -> subprocess.Popen:
        """Start Firecracker micro-VM."""
        # Firecracker requires specific JSON configuration
        firecracker_cfg = {
            "boot-source": {
                "kernel_image_path": config.extra_args.get("kernel", "/usr/share/firecracker/vmlinux"),
                "boot_args": config.extra_args.get("cmdline", "console=ttyS0 reboot=k panic=1 pci=off")
            },
            "drives": [
                {
                    "drive_id": "rootfs",
                    "path_on_host": config.disk_path,
                    "is_root_device": True,
                    "partuuid": None,
                    "is_read_only": False
                }
            ],
            "network-interfaces": [
                {
                    "iface_id": "eth0",
                    "host_dev_name": f"tap-{config.vm_id}"
                }
            ],
            "vsock": {
                "guest_cid": config.vsock_cid,
                "uds_path": f"/var/run/cognyx-{config.vm_id}.vsock"
            },
            "machine-config": {
                "vcpu_count": config.vcpus,
                "mem_size_mib": config.memory_mb,
                "ht_enabled": False
            }
        }
        
        # Write configuration
        cfg_path = Path(f"/tmp/firecracker-{config.vm_id}.json")
        with open(cfg_path, 'w') as f:
            json.dump(firecracker_cfg, f, indent=2)
        
        # Start Firecracker
        cmd = ["firecracker", "--api-sock", f"/tmp/firecracker-{config.vm_id}.api"]
        process = subprocess.Popen(
            cmd, stdin=open(cfg_path), stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        
        return process
    
    def list_vms(self) -> List[VMConfig]:
        """List all configured VMs."""
        return list(self.vms.values())
    
    def get_vm_status(self, vm_id: str) -> Dict[str, Any]:
        """Get VM status information."""
        if vm_id not in self.vms:
            return {"error": "VM not found"}
        
        config = self.vms[vm_id]
        is_running = vm_id in self.running_vms
        
        return {
            "vm_id": vm_id,
            "vm_type": config.vm_type.value,
            "status": "running" if is_running else "stopped",
            "vcpus": config.vcpus,
            "memory_mb": config.memory_mb,
            "disk_path": config.disk_path,
            "vsock_cid": config.vsock_cid,
            "gpu_passthrough": config.gpu_passthrough,
        }


# Example usage
if __name__ == "__main__":
    factory = VMFactory()
    
    # Create Windows VM with GPU passthrough
    # win_vm = factory.create_vm(
    #     vm_type=VMType.WINDOWS,
    #     vm_id="windows-dev-001",
    #     vcpus=4,
    #     memory_mb=8192,
    #     disk_size_gb=100,
    #     gpu_passthrough=True,
    #     gpu_device_idx=1
    # )
    
    # Start VM with installation ISO
    # factory.start_vm("windows-dev-001", iso_path="/isos/windows11.iso")
    
    print("VM Factory initialized")
