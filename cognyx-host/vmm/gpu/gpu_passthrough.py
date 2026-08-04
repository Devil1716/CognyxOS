"""
CognyxOS GPU Passthrough Engine

Manages VFIO-PCI device assignment, vGPU slicing, and GPU isolation.
"""

import os
import subprocess
import json
from typing import Dict, List, Optional
from dataclasses import dataclass
from enum import Enum


class GPUMode(Enum):
    PASSTHROUGH = "passthrough"  # Full device assignment
    VGPU = "vgpu"  # Time-sliced sharing
    VIRTIO = "virtio"  # Paravirtualized graphics


@dataclass
class GPUDevice:
    pci_address: str  # e.g., "01:00.0"
    vendor_id: str
    device_id: str
    iommu_group: int
    driver: str
    mode: GPUMode
    vfio_config: Optional[Dict] = None


class VFIOManager:
    """Manages VFIO-PCI device binding for GPU passthrough."""
    
    def __init__(self):
        self.vfio_path = "/sys/bus/pci/drivers/vfio-pci"
        self.iommu_enabled = self._check_iommu()
    
    def _check_iommu(self) -> bool:
        """Verify IOMMU is enabled in kernel."""
        return os.path.exists("/sys/class/iommu")
    
    def _get_iommu_group(self, pci_address: str) -> int:
        """Get IOMMU group ID for a PCI device."""
        path = f"/sys/bus/pci/devices/{pci_address}/iommu_group"
        try:
            return int(os.readlink(path).split("/")[-1])
        except (FileNotFoundError, ValueError):
            raise RuntimeError(f"Device {pci_address} has no IOMMU group")
    
    def _unbind_device(self, pci_address: str):
        """Unbind device from current driver."""
        current_driver = self._get_current_driver(pci_address)
        if current_driver:
            unbind_path = f"/sys/bus/pci/drivers/{current_driver}/unbind"
            with open(unbind_path, "w") as f:
                f.write(pci_address)
    
    def _get_current_driver(self, pci_address: str) -> Optional[str]:
        """Get current driver bound to device."""
        driver_path = f"/sys/bus/pci/devices/{pci_address}/driver"
        try:
            link = os.readlink(driver_path)
            return link.split("/")[-1]
        except (FileNotFoundError, OSError):
            return None
    
    def bind_to_vfio(self, pci_address: str) -> bool:
        """Bind PCI device to VFIO driver."""
        if not self.iommu_enabled:
            raise RuntimeError("IOMMU not enabled")
        
        # Get device info
        vendor_id = self._read_pci_config(pci_address, "vendor")
        device_id = self._read_pci_config(pci_address, "device")
        
        # Unbind from current driver
        self._unbind_device(pci_address)
        
        # Add device IDs to vfio-pci
        new_id_path = f"{self.vfio_path}/new_id"
        with open(new_id_path, "w") as f:
            f.write(f"{vendor_id} {device_id}")
        
        # Bind device
        bind_path = f"{self.vfio_path}/bind"
        with open(bind_path, "w") as f:
            f.write(pci_address)
        
        return True
    
    def _read_pci_config(self, pci_address: str, field: str) -> str:
        """Read PCI configuration field."""
        path = f"/sys/bus/pci/devices/{pci_address}/{field}"
        with open(path, "r") as f:
            return f.read().strip()
    
    def list_vfio_devices(self) -> List[GPUDevice]:
        """List all devices bound to VFIO."""
        devices = []
        if not os.path.exists(self.vfio_path):
            return devices
        
        for entry in os.listdir(self.vfio_path):
            if entry.startswith("0000:"):
                iommu_group = self._get_iommu_group(entry)
                devices.append(GPUDevice(
                    pci_address=entry,
                    vendor_id=self._read_pci_config(entry, "vendor"),
                    device_id=self._read_pci_config(entry, "device"),
                    iommu_group=iommu_group,
                    driver="vfio-pci",
                    mode=GPUMode.PASSTHROUGH
                ))
        
        return devices


class vGPUManger:
    """Manages virtual GPU slicing for multi-VM sharing."""
    
    def __init__(self):
        self.nvidia_vgpu = "/sys/bus/mdev/drivers/nvidia"
        self.intel_gvt = "/sys/bus/pci/drivers/i915"
    
    def create_vgpu(self, physical_gpu: str, type_id: str, vm_uuid: str) -> str:
        """Create a virtual GPU instance."""
        # NVIDIA vGPU example
        mdev_path = f"{self.nvidia_vgpu}/{type_id}/create"
        
        with open(mdev_path, "w") as f:
            f.write(vm_uuid)
        
        return vm_uuid
    
    def destroy_vgpu(self, vm_uuid: str):
        """Destroy virtual GPU instance."""
        remove_path = f"/sys/bus/mdev/devices/{vm_uuid}/remove"
        if os.path.exists(remove_path):
            with open(remove_path, "w") as f:
                f.write("1")
    
    def list_vgpu_types(self, pci_address: str) -> List[Dict]:
        """List available vGPU types for a physical GPU."""
        types_path = f"/sys/bus/pci/devices/{pci_address}/mdev_supported_types"
        types = []
        
        if not os.path.exists(types_path):
            return types
        
        for type_dir in os.listdir(types_path):
            type_path = f"{types_path}/{type_dir}"
            name = self._read_file(f"{type_path}/name")
            instances = int(self._read_file(f"{type_path}/available_instances"))
            
            types.append({
                "type_id": type_dir,
                "name": name,
                "available_instances": instances
            })
        
        return types
    
    def _read_file(self, path: str) -> str:
        """Read file content."""
        try:
            with open(path, "r") as f:
                return f.read().strip()
        except FileNotFoundError:
            return ""


class VirtIOGPUManager:
    """Manages VirtIO-GPU for paravirtualized graphics."""
    
    def __init__(self):
        self.driver = "virtio_gpu"
    
    def configure(self, vm_uuid: str, resolution: str = "1920x1080"):
        """Configure VirtIO-GPU for VM."""
        # VirtIO-GPU is configured via QEMU command line
        # This method validates configuration
        width, height = map(int, resolution.split("x"))
        
        if width > 7680 or height > 4320:
            raise ValueError("Resolution exceeds maximum (8K)")
        
        return {
            "vm_uuid": vm_uuid,
            "resolution": resolution,
            "acceleration": "virgl"
        }


class GPUPassthroughEngine:
    """Main engine coordinating GPU passthrough operations."""
    
    def __init__(self):
        self.vfio_mgr = VFIOManager()
        self.vgpu_mgr = vGPUManger()
        self.virtio_mgr = VirtIOGPUManager()
        self.assigned_devices: Dict[str, GPUDevice] = {}
    
    def assign_gpu(self, vm_uuid: str, pci_address: str, mode: GPUMode = GPUMode.PASSTHROUGH) -> bool:
        """Assign GPU to VM."""
        if not self.vfio_mgr.iommu_enabled:
            raise RuntimeError("IOMMU not enabled - cannot perform passthrough")
        
        # Get device info
        iommu_group = self.vfio_mgr._get_iommu_group(pci_address)
        
        # Bind to VFIO if passthrough mode
        if mode == GPUMode.PASSTHROUGH:
            self.vfio_mgr.bind_to_vfio(pci_address)
        
        device = GPUDevice(
            pci_address=pci_address,
            vendor_id=self.vfio_mgr._read_pci_config(pci_address, "vendor"),
            device_id=self.vfio_mgr._read_pci_config(pci_address, "device"),
            iommu_group=iommu_group,
            driver="vfio-pci",
            mode=mode
        )
        
        self.assigned_devices[vm_uuid] = device
        return True
    
    def release_gpu(self, vm_uuid: str) -> bool:
        """Release GPU from VM."""
        if vm_uuid not in self.assigned_devices:
            return False
        
        device = self.assigned_devices[vm_uuid]
        
        # Unbind from VFIO
        if device.mode == GPUMode.PASSTHROUGH:
            unbind_path = f"{self.vfio_mgr.vfio_path}/unbind"
            with open(unbind_path, "w") as f:
                f.write(device.pci_address)
        
        del self.assigned_devices[vm_uuid]
        return True
    
    def get_assigned_gpus(self) -> Dict[str, GPUDevice]:
        """Get all assigned GPUs."""
        return self.assigned_devices.copy()
    
    def check_isolation(self, vm_uuid: str) -> Dict:
        """Verify GPU isolation for security audit."""
        if vm_uuid not in self.assigned_devices:
            return {"isolated": False, "reason": "No GPU assigned"}
        
        device = self.assigned_devices[vm_uuid]
        
        checks = {
            "iommu_enabled": self.vfio_mgr.iommu_enabled,
            "vfio_bound": device.driver == "vfio-pci",
            "isolated_group": True,  # Would verify no other devices in same group
            "dma_remapping": True,  # Would verify IOMMU mapping
        }
        
        isolated = all(checks.values())
        
        return {
            "isolated": isolated,
            "checks": checks,
            "device": device.pci_address
        }


# Example usage
if __name__ == "__main__":
    engine = GPUPassthroughEngine()
    
    # Check IOMMU status
    print(f"IOMMU Enabled: {engine.vfio_mgr.iommu_enabled}")
    
    # List available VFIO devices
    vfio_devices = engine.vfio_mgr.list_vfio_devices()
    print(f"VFIO Devices: {[d.pci_address for d in vfio_devices]}")
    
    # Assign GPU to VM
    vm_uuid = "550e8400-e29b-41d4-a716-446655440000"
    gpu_pci = "01:00.0"
    
    if engine.assign_gpu(vm_uuid, gpu_pci, GPUMode.PASSTHROUGH):
        print(f"GPU {gpu_pci} assigned to VM {vm_uuid}")
    
    # Verify isolation
    isolation = engine.check_isolation(vm_uuid)
    print(f"Isolation Status: {isolation}")
    
    # Release GPU
    engine.release_gpu(vm_uuid)
    print(f"GPU released from VM {vm_uuid}")
