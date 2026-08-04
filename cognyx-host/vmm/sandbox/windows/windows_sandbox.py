"""
CognyxOS Windows Sandbox Implementation

Provides Windows VM configuration with UEFI, TPM 2.0, VirtIO drivers,
and optimized settings for Windows 10/11 execution.
"""

from typing import Dict, List, Optional
from dataclasses import dataclass
import uuid as uuid_lib

from ..qemu.qemu_manager import VMSpec, CPUConfig, MemoryConfig, DiskConfig, NetworkConfig, GPUConfig, TPMConfig


@dataclass
class WindowsVersion:
    name: str
    uefi_required: bool = True
    tpm_required: bool = True
    secure_boot: bool = True
    min_cpu_cores: int = 2
    min_memory_mb: int = 4096
    virtio_drivers: bool = True


WINDOWS_VERSIONS = {
    "windows10": WindowsVersion(
        name="Windows 10",
        uefi_required=True,
        tpm_required=False,  # Optional for Win10
        secure_boot=True,
        min_cpu_cores=2,
        min_memory_mb=4096
    ),
    "windows11": WindowsVersion(
        name="Windows 11",
        uefi_required=True,
        tpm_required=True,  # Required for Win11
        secure_boot=True,
        min_cpu_cores=2,
        min_memory_mb=4096
    ),
    "windows_server_2022": WindowsVersion(
        name="Windows Server 2022",
        uefi_required=True,
        tpm_required=False,
        secure_boot=True,
        min_cpu_cores=2,
        min_memory_mb=4096
    )
}


class WindowsSandboxFactory:
    """Creates optimized Windows VM configurations."""
    
    def __init__(self, base_storage_path: str = "/var/lib/cognyx/vms"):
        self.base_storage_path = base_storage_path
        self.virtio_iso_path = "/usr/share/virtio-win/virtio-win.iso"
        self.windows_iso_path = "/var/lib/cognyx/isos/windows.iso"
    
    def create_sandbox_spec(
        self,
        version: str = "windows11",
        vm_name: Optional[str] = None,
        cpu_cores: int = 4,
        memory_mb: int = 8192,
        disk_size_gb: int = 100,
        gpu_passthrough: bool = False,
        gpu_pci: Optional[str] = None,
        network_bridge: str = "cognyx0",
        enable_tpm: bool = True,
        secure_boot: bool = True
    ) -> VMSpec:
        """Create a Windows sandbox VM specification."""
        
        if version not in WINDOWS_VERSIONS:
            raise ValueError(f"Unknown Windows version: {version}")
        
        win_version = WINDOWS_VERSIONS[version]
        
        # Validate requirements
        if cpu_cores < win_version.min_cpu_cores:
            raise ValueError(f"Windows {version} requires minimum {win_version.min_cpu_cores} CPU cores")
        
        if memory_mb < win_version.min_memory_mb:
            raise ValueError(f"Windows {version} requires minimum {win_version.min_memory_mb}MB RAM")
        
        if win_version.tpm_required and not enable_tpm:
            raise ValueError(f"Windows {version} requires TPM 2.0")
        
        # Generate UUID and name
        vm_uuid = str(uuid_lib.uuid4())
        if vm_name is None:
            vm_name = f"windows-{version}-{vm_uuid[:8]}"
        
        # CPU Configuration with CPU pinning optimization
        cpu_config = CPUConfig(
            cores=cpu_cores,
            threads=1,
            sockets=1,
            model="host",
            pinning=[]  # Will be set by scheduler based on availability
        )
        
        # Memory Configuration with hugepages
        memory_config = MemoryConfig(
            size_mb=memory_mb,
            hugepages=True,
            ballooning=False,  # Disable for Windows stability
            max_size_mb=memory_mb
        )
        
        # Disk Configuration
        disk_path = f"{self.base_storage_path}/{vm_name}/disk.qcow2"
        disk_config = [
            DiskConfig(
                path=disk_path,
                format="qcow2",
                cache="none",
                io="native",
                discard=True,
                snapshot=False
            ),
            # VirtIO drivers ISO
            DiskConfig(
                path=self.virtio_iso_path,
                format="raw",
                cache="none",
                io="native",
                discard=False,
                snapshot=True
            ),
            # Windows installation ISO
            DiskConfig(
                path=self.windows_iso_path,
                format="raw",
                cache="none",
                io="native",
                discard=False,
                snapshot=True
            )
        ]
        
        # Network Configuration
        network_config = NetworkConfig(
            model="virtio",
            mac=self._generate_mac(),
            bridge=network_bridge,
            vhost=True,
            queues=4
        )
        
        # GPU Configuration
        gpu_config = GPUConfig(
            passthrough=gpu_passthrough,
            vfio_pci=gpu_pci if gpu_passthrough else "",
            vgpu=False,
            framebuffer=not gpu_passthrough
        )
        
        # TPM Configuration (Required for Windows 11)
        tpm_config = TPMConfig(
            enabled=enable_tpm and win_version.tpm_required,
            type="emulator",  # Use software TPM (swtpm)
            device="/dev/tpm0"
        )
        
        # Create VM Spec
        spec = VMSpec(
            uuid=vm_uuid,
            name=vm_name,
            cpu=cpu_config,
            memory=memory_config,
            disks=disk_config,
            network=network_config,
            gpu=gpu_config,
            tpm=tpm_config,
            uefi=win_version.uefi_required,
            secure_boot=secure_boot
        )
        
        return spec
    
    def _generate_mac(self) -> str:
        """Generate random MAC address for VirtIO network."""
        import random
        mac = [0x52, 0x54, 0x00] + [random.randint(0x00, 0xff) for _ in range(3)]
        return ":".join(f"{b:02x}" for b in mac)
    
    def prepare_storage(self, vm_name: str, disk_size_gb: int) -> str:
        """Prepare disk storage for Windows VM."""
        import subprocess
        import os
        
        vm_dir = f"{self.base_storage_path}/{vm_name}"
        os.makedirs(vm_dir, exist_ok=True)
        
        disk_path = f"{vm_dir}/disk.qcow2"
        
        # Create qcow2 disk
        subprocess.run([
            "qemu-img", "create", "-f", "qcow2",
            disk_path, f"{disk_size_gb}G"
        ], check=True)
        
        return disk_path
    
    def get_virtio_drivers_info(self) -> Dict:
        """Get VirtIO driver installation information."""
        return {
            "iso_path": self.virtio_iso_path,
            "drivers": [
                "NetKVM (Network)",
                "Viostor (Storage)",
                "Vioscsi (SCSI)",
                "Balloon (Memory)",
                "Vioserial (Serial)",
                "Qxl (Graphics)",
                "Vioinput (Input)",
                "Viogpudo (GPU)"
            ],
            "installation_order": [
                "Viostor (during Windows install)",
                "NetKVM (after first boot)",
                "Balloon (optional)",
                "Qxl/Viogpudo (graphics)",
                "Vioserial (integration)"
            ]
        }


class WindowsSandboxManager:
    """Manages Windows sandbox lifecycle."""
    
    def __init__(self):
        self.factory = WindowsSandboxFactory()
        self.active_sandboxes: Dict[str, VMSpec] = {}
    
    def create(self, version: str = "windows11", **kwargs) -> VMSpec:
        """Create a new Windows sandbox."""
        spec = self.factory.create_sandbox_spec(version=version, **kwargs)
        
        # Prepare storage
        disk_size = kwargs.get("disk_size_gb", 100)
        self.factory.prepare_storage(spec.name, disk_size)
        
        self.active_sandboxes[spec.uuid] = spec
        return spec
    
    def destroy(self, vm_uuid: str) -> bool:
        """Destroy a Windows sandbox."""
        if vm_uuid not in self.active_sandboxes:
            return False
        
        # Cleanup storage
        spec = self.active_sandboxes[vm_uuid]
        import shutil
        import os
        
        vm_dir = f"{self.factory.base_storage_path}/{spec.name}"
        if os.path.exists(vm_dir):
            shutil.rmtree(vm_dir)
        
        del self.active_sandboxes[vm_uuid]
        return True
    
    def list_sandboxes(self) -> List[Dict]:
        """List all Windows sandboxes."""
        return [
            {
                "uuid": spec.uuid,
                "name": spec.name,
                "cpu_cores": spec.cpu.cores,
                "memory_mb": spec.memory.size_mb,
                "disk_count": len(spec.disks),
                "gpu_passthrough": spec.gpu.passthrough,
                "tpm_enabled": spec.tpm.enabled
            }
            for spec in self.active_sandboxes.values()
        ]


# Example usage
if __name__ == "__main__":
    manager = WindowsSandboxManager()
    
    # Create Windows 11 sandbox
    spec = manager.create(
        version="windows11",
        vm_name="win11-dev",
        cpu_cores=4,
        memory_mb=8192,
        disk_size_gb=100,
        gpu_passthrough=False,
        enable_tpm=True
    )
    
    print(f"Created Windows sandbox: {spec.name}")
    print(f"UUID: {spec.uuid}")
    print(f"CPU Cores: {spec.cpu.cores}")
    print(f"Memory: {spec.memory.size_mb}MB")
    print(f"TPM Enabled: {spec.tpm.enabled}")
    print(f"UEFI: {spec.uefi}")
    
    # List sandboxes
    sandboxes = manager.list_sandboxes()
    print(f"\nActive Sandboxes: {sandboxes}")
    
    # Get VirtIO info
    virtio_info = manager.factory.get_virtio_drivers_info()
    print(f"\nVirtIO Drivers: {virtio_info['drivers']}")
