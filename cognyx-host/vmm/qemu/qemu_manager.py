"""
CognyxOS QEMU/KVM Integration Layer

Manages QEMU process lifecycle, KVM device access, and VM configuration.
"""

import os
import subprocess
import json
from typing import Dict, List, Optional
from dataclasses import dataclass, asdict
from enum import Enum


class VMState(Enum):
    STOPPED = "stopped"
    RUNNING = "running"
    PAUSED = "paused"
    SUSPENDED = "suspended"


@dataclass
class CPUConfig:
    cores: int = 4
    threads: int = 2
    sockets: int = 1
    model: str = "host"
    pinning: List[int] = None  # Dedicated CPU cores
    
    def __post_init__(self):
        if self.pinning is None:
            self.pinning = []


@dataclass
class MemoryConfig:
    size_mb: int = 8192
    hugepages: bool = True
    ballooning: bool = False
    max_size_mb: int = 16384


@dataclass
class DiskConfig:
    path: str
    format: str = "qcow2"
    cache: str = "none"
    io: str = "native"
    discard: bool = True
    snapshot: bool = False


@dataclass
class NetworkConfig:
    model: str = "virtio"
    mac: str = ""
    bridge: str = "cognyx0"
    vhost: bool = True
    queues: int = 4


@dataclass
class GPUConfig:
    passthrough: bool = False
    vfio_pci: str = ""  # PCI address (e.g., "01:00.0")
    vgpu: bool = False
    framebuffer: bool = True  # VirtIO-GPU if not passthrough


@dataclass
class TPMConfig:
    enabled: bool = True
    type: str = "emulator"  # emulator or passthrough
    device: str = "/dev/tpm0"


@dataclass
class VMSpec:
    uuid: str
    name: str
    cpu: CPUConfig
    memory: MemoryConfig
    disks: List[DiskConfig]
    network: NetworkConfig
    gpu: GPUConfig = None
    tpm: TPMConfig = None
    uefi: bool = True
    secure_boot: bool = True
    
    def __post_init__(self):
        if self.gpu is None:
            self.gpu = GPUConfig()
        if self.tpm is None:
            self.tpm = TPMConfig()


class QEMUManager:
    """Manages QEMU processes for VM lifecycle."""
    
    def __init__(self, kvm_device: str = "/dev/kvm"):
        self.kvm_device = kvm_device
        self.vm_processes: Dict[str, subprocess.Popen] = {}
        self.vm_states: Dict[str, VMState] = {}
        
    def _check_kvm(self) -> bool:
        """Verify KVM availability."""
        return os.path.exists(self.kvm_device) and os.access(self.kvm_device, os.R | os.W)
    
    def _build_qemu_command(self, spec: VMSpec) -> List[str]:
        """Build QEMU command line from VM spec."""
        cmd = [
            "qemu-system-x86_64",
            "-enable-kvm",
            "-name", f"Cognyx-{spec.name}",
            "-uuid", spec.uuid,
        ]
        
        # CPU Configuration
        cpu_str = f"{spec.cpu.model},topoext=on"
        cmd.extend(["-cpu", cpu_str])
        cmd.extend(["-smp", f"cores={spec.cpu.cores},threads={spec.cpu.threads},sockets={spec.cpu.sockets}"])
        
        if spec.cpu.pinning:
            pin_list = ",".join(map(str, spec.cpu.pinning))
            cmd.extend(["-pin", pin_list])
        
        # Memory Configuration
        mem_opts = f"size={spec.memory.size_mb}M"
        if spec.memory.hugepages:
            mem_opts += ",prealloc=on"
        if spec.memory.ballooning:
            mem_opts += f",max={spec.memory.max_size_mb}M"
            cmd.extend(["-device", "virtio-balloon-pci"])
        cmd.extend(["-m", mem_opts])
        
        # UEFI Firmware
        if spec.uefi:
            cmd.extend([
                "-drive", "if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE.fd",
                "-drive", "if=pflash,format=raw,file=/usr/share/OVMF/OVMF_VARS.fd",
            ])
        
        # Disk Configuration
        for idx, disk in enumerate(spec.disks):
            disk_opts = f"file={disk.path},format={disk.format}"
            disk_opts += f",cache={disk.cache},aio={disk.io}"
            if disk.discard:
                disk_opts += ",discard=unmap"
            if disk.snapshot:
                disk_opts += ",snapshot=on"
            cmd.extend(["-drive", disk_opts])
        
        # Network Configuration
        net_opts = f"tap,ifname=cognyx_{spec.uuid[:8]},script=no,downscript=no"
        if spec.network.vhost:
            net_opts += ",vhost=on,vhostfds=3"
        if spec.network.queues > 1:
            net_opts += f",queues={spec.network.queues}"
        cmd.extend(["-netdev", net_opts])
        cmd.extend(["-device", f"{spec.network.model}-pci,netdev=net0,mac={spec.network.mac}"])
        
        # GPU Configuration
        if spec.gpu.passthrough and spec.gpu.vfio_pci:
            cmd.extend(["-device", f"vfio-pci,host={spec.gpu.vfio_pci}"])
        else:
            cmd.extend(["-device", "virtio-vga-gl"])
        
        # TPM Configuration
        if spec.tpm.enabled:
            if spec.tpm.type == "emulator":
                cmd.extend(["-chardev", "socket,id=chrtpm,path=/var/run/swtpm/socket"])
                cmd.extend(["-tpmdev", "emulator,id=tpm0,chardev=chrtpm"])
                cmd.extend(["-device", "tpm-tis,tpmdev=tpm0"])
        
        # Monitoring & Control
        monitor_socket = f"/var/run/cognyx/{spec.uuid}.monitor"
        cmd.extend(["-monitor", f"unix:{monitor_socket},server,nowait"])
        
        # Serial Console
        cmd.extend(["-serial", "pty"])
        
        # Daemonize
        cmd.extend(["-daemonize"])
        
        return cmd
    
    def create_vm(self, spec: VMSpec) -> bool:
        """Create and start a VM."""
        if not self._check_kvm():
            raise RuntimeError("KVM not available")
        
        cmd = self._build_qemu_command(spec)
        
        try:
            process = subprocess.Popen(
                cmd,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                preexec_fn=os.setsid
            )
            self.vm_processes[spec.uuid] = process
            self.vm_states[spec.uuid] = VMState.RUNNING
            return True
        except Exception as e:
            print(f"Failed to start VM {spec.name}: {e}")
            return False
    
    def stop_vm(self, uuid: str, force: bool = False) -> bool:
        """Stop a running VM."""
        if uuid not in self.vm_processes:
            return False
        
        process = self.vm_processes[uuid]
        
        if force:
            process.kill()
        else:
            # Send ACPI shutdown signal via monitor
            self._send_monitor_command(uuid, "system_powerdown")
            # Wait for graceful shutdown
            try:
                process.wait(timeout=30)
            except subprocess.TimeoutExpired:
                process.kill()
        
        del self.vm_processes[uuid]
        self.vm_states[uuid] = VMState.STOPPED
        return True
    
    def pause_vm(self, uuid: str) -> bool:
        """Pause VM execution."""
        if uuid not in self.vm_processes:
            return False
        
        self._send_monitor_command(uuid, "stop")
        self.vm_states[uuid] = VMState.PAUSED
        return True
    
    def resume_vm(self, uuid: str) -> bool:
        """Resume paused VM."""
        if uuid not in self.vm_processes:
            return False
        
        self._send_monitor_command(uuid, "cont")
        self.vm_states[uuid] = VMState.RUNNING
        return True
    
    def _send_monitor_command(self, uuid: str, command: str) -> str:
        """Send command to QEMU monitor socket."""
        import socket
        monitor_path = f"/var/run/cognyx/{uuid}.monitor"
        
        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        client.connect(monitor_path)
        client.sendall(f"{command}\n".encode())
        response = client.recv(4096).decode()
        client.close()
        
        return response
    
    def get_vm_state(self, uuid: str) -> Optional[VMState]:
        """Get current VM state."""
        return self.vm_states.get(uuid)
    
    def list_vms(self) -> List[Dict]:
        """List all managed VMs with states."""
        return [
            {"uuid": uuid, "state": state.value}
            for uuid, state in self.vm_states.items()
        ]


class KVMManager:
    """Low-level KVM device management."""
    
    def __init__(self, device: str = "/dev/kvm"):
        self.device = device
        self.fd: Optional[int] = None
    
    def open(self) -> bool:
        """Open KVM device."""
        try:
            self.fd = os.open(self.device, os.O_RDWR)
            return True
        except OSError:
            return False
    
    def close(self):
        """Close KVM device."""
        if self.fd is not None:
            os.close(self.fd)
            self.fd = None
    
    def check_extension(self, ext_id: int) -> bool:
        """Check if KVM extension is available."""
        import ctypes
        libc = ctypes.CDLL("libc.so.6")
        result = libc.ioctl(self.fd, 0xC004AE03, ctypes.byref(ctypes.c_int(ext_id)))
        return result >= 0


# Example usage
if __name__ == "__main__":
    # Create VM specification
    spec = VMSpec(
        uuid="550e8400-e29b-41d4-a716-446655440000",
        name="windows-sandbox-1",
        cpu=CPUConfig(cores=4, threads=2, pinning=[4, 5, 6, 7]),
        memory=MemoryConfig(size_mb=8192, hugepages=True),
        disks=[
            DiskConfig(path="/var/lib/cognyx/vms/win1.qcow2", format="qcow2")
        ],
        network=NetworkConfig(mac="52:54:00:12:34:56"),
        gpu=GPUConfig(passthrough=True, vfio_pci="01:00.0"),
        tpm=TPMConfig(enabled=True, type="emulator")
    )
    
    # Initialize manager
    qemu_mgr = QEMUManager()
    
    # Start VM
    if qemu_mgr.create_vm(spec):
        print(f"VM {spec.name} started successfully")
    
    # Check state
    state = qemu_mgr.get_vm_state(spec.uuid)
    print(f"VM State: {state}")
    
    # List all VMs
    vms = qemu_mgr.list_vms()
    print(f"Active VMs: {vms}")
