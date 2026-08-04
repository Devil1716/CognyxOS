"""
CognyxOS VMM Lifecycle APIs

Provides unified lifecycle management for all VM types with
create, start, stop, pause, resume, destroy, and monitor operations.
"""

import os
import json
import time
from typing import Dict, List, Optional, Callable
from dataclasses import dataclass
from enum import Enum
from datetime import datetime

from .qemu.qemu_manager import QEMUManager, VMSpec, VMState
from .sandbox.windows.windows_sandbox import WindowsSandboxManager
from .sandbox.macos.macos_sandbox import MacOSSandboxManager
from .snapshot.snapshot_engine import SnapshotEngine, SuspendResumeManager


class VMType(Enum):
    LINUX = "linux"
    WINDOWS = "windows"
    MACOS = "macos"
    ANDROID = "android"
    CUSTOM = "custom"


class LifecycleEvent(Enum):
    CREATED = "created"
    STARTING = "starting"
    STARTED = "started"
    STOPPING = "stopping"
    STOPPED = "stopped"
    PAUSED = "paused"
    RESUMED = "resumed"
    SUSPENDED = "suspended"
    DESTROYED = "destroyed"
    ERROR = "error"


@dataclass
class VMInstance:
    uuid: str
    name: str
    vm_type: VMType
    state: VMState
    spec: VMSpec
    created_at: str
    started_at: Optional[str] = None
    metadata: Dict = None


@dataclass
class LifecycleCallback:
    event: LifecycleEvent
    callback: Callable[[str, VMInstance], None]


class VMLifecycleManager:
    """Unified lifecycle management for all VM types."""
    
    def __init__(self, base_storage_path: str = "/var/lib/cognyx/vms"):
        self.base_storage_path = base_storage_path
        self.qemu_mgr = QEMUManager()
        self.windows_mgr = WindowsSandboxManager()
        self.macos_mgr = MacOSSandboxManager()
        self.snapshot_engine = SnapshotEngine()
        self.suspend_mgr = SuspendResumeManager(self.snapshot_engine)
        
        self.instances: Dict[str, VMInstance] = {}
        self.callbacks: Dict[LifecycleEvent, List[Callable]] = {}
        self._load_existing_instances()
    
    def _load_existing_instances(self):
        """Load existing VM instances from disk."""
        index_path = f"{self.base_storage_path}/instances.json"
        if os.path.exists(index_path):
            with open(index_path, "r") as f:
                data = json.load(f)
                for uuid, inst_data in data.items():
                    inst_data["vm_type"] = VMType(inst_data["vm_type"])
                    inst_data["state"] = VMState(inst_data["state"])
                    self.instances[uuid] = VMInstance(**inst_data)
    
    def _save_instances(self):
        """Save instance index to disk."""
        index_path = f"{self.base_storage_path}/instances.json"
        os.makedirs(os.path.dirname(index_path), exist_ok=True)
        
        data = {
            uuid: {
                **{k: v for k, v in inst.__dict__.items() if k != "spec"},
                "vm_type": inst.vm_type.value,
                "state": inst.state.value
            }
            for uuid, inst in self.instances.items()
        }
        
        with open(index_path, "w") as f:
            json.dump(data, f, indent=2)
    
    def register_callback(self, event: LifecycleEvent, callback: Callable):
        """Register a callback for lifecycle events."""
        if event not in self.callbacks:
            self.callbacks[event] = []
        self.callbacks[event].append(callback)
    
    def _emit_event(self, event: LifecycleEvent, vm_uuid: str, instance: VMInstance):
        """Emit a lifecycle event to registered callbacks."""
        if event in self.callbacks:
            for callback in self.callbacks[event]:
                try:
                    callback(vm_uuid, instance)
                except Exception as e:
                    print(f"Callback error for {event}: {e}")
    
    def create_vm(
        self,
        vm_type: VMType,
        name: str,
        spec: Optional[VMSpec] = None,
        **kwargs
    ) -> VMInstance:
        """Create a new VM instance."""
        import uuid as uuid_lib
        
        vm_uuid = str(uuid_lib.uuid4())
        timestamp = datetime.utcnow().isoformat()
        
        # Create type-specific configuration
        if vm_type == VMType.WINDOWS:
            version = kwargs.get("version", "windows11")
            spec = self.windows_mgr.factory.create_sandbox_spec(version=version, vm_name=name, **kwargs)
        elif vm_type == VMType.MACOS:
            version = kwargs.get("version", "macos_sonoma")
            spec = self.macos_mgr.factory.create_sandbox_spec(version=version, vm_name=name, **kwargs)
        elif vm_type == VMType.LINUX:
            # Use provided spec or create default
            if spec is None:
                from .qemu.qemu_manager import CPUConfig, MemoryConfig, DiskConfig, NetworkConfig
                spec = VMSpec(
                    uuid=vm_uuid,
                    name=name,
                    cpu=CPUConfig(cores=4),
                    memory=MemoryConfig(size_mb=8192),
                    disks=[DiskConfig(path=f"{self.base_storage_path}/{name}/disk.qcow2")],
                    network=NetworkConfig()
                )
        else:
            raise ValueError(f"Unsupported VM type: {vm_type}")
        
        # Prepare storage
        if hasattr(spec, "disks") and spec.disks:
            disk_path = spec.disks[0].path
            os.makedirs(os.path.dirname(disk_path), exist_ok=True)
            
            if not os.path.exists(disk_path):
                import subprocess
                disk_size = kwargs.get("disk_size_gb", 100)
                subprocess.run([
                    "qemu-img", "create", "-f", "qcow2",
                    disk_path, f"{disk_size}G"
                ], check=True)
        
        # Create instance record
        instance = VMInstance(
            uuid=vm_uuid,
            name=name,
            vm_type=vm_type,
            state=VMState.STOPPED,
            spec=spec,
            created_at=timestamp,
            metadata=kwargs.get("metadata", {})
        )
        
        self.instances[vm_uuid] = instance
        self._save_instances()
        
        self._emit_event(LifecycleEvent.CREATED, vm_uuid, instance)
        
        return instance
    
    def start_vm(self, vm_uuid: str, timeout: int = 60) -> bool:
        """Start a VM."""
        if vm_uuid not in self.instances:
            return False
        
        instance = self.instances[vm_uuid]
        
        if instance.state == VMState.RUNNING:
            return True
        
        self._emit_event(LifecycleEvent.STARTING, vm_uuid, instance)
        
        # Start via QEMU manager
        success = self.qemu_mgr.create_vm(instance.spec)
        
        if success:
            instance.state = VMState.RUNNING
            instance.started_at = datetime.utcnow().isoformat()
            self._save_instances()
            
            # Wait for VM to be ready
            start_time = time.time()
            while time.time() - start_time < timeout:
                if self.is_vm_ready(vm_uuid):
                    break
                time.sleep(1)
            
            self._emit_event(LifecycleEvent.STARTED, vm_uuid, instance)
        else:
            instance.state = VMState.STOPPED
            self._emit_event(LifecycleEvent.ERROR, vm_uuid, instance)
        
        return success
    
    def stop_vm(self, vm_uuid: str, force: bool = False, timeout: int = 30) -> bool:
        """Stop a VM."""
        if vm_uuid not in self.instances:
            return False
        
        instance = self.instances[vm_uuid]
        
        if instance.state == VMState.STOPPED:
            return True
        
        self._emit_event(LifecycleEvent.STOPPING, vm_uuid, instance)
        
        success = self.qemu_mgr.stop_vm(vm_uuid, force=force)
        
        if success:
            instance.state = VMState.STOPPED
            self._save_instances()
            self._emit_event(LifecycleEvent.STOPPED, vm_uuid, instance)
        else:
            self._emit_event(LifecycleEvent.ERROR, vm_uuid, instance)
        
        return success
    
    def pause_vm(self, vm_uuid: str) -> bool:
        """Pause a running VM."""
        if vm_uuid not in self.instances:
            return False
        
        instance = self.instances[vm_uuid]
        
        if instance.state != VMState.RUNNING:
            return False
        
        success = self.qemu_mgr.pause_vm(vm_uuid)
        
        if success:
            instance.state = VMState.PAUSED
            self._save_instances()
            self._emit_event(LifecycleEvent.PAUSED, vm_uuid, instance)
        
        return success
    
    def resume_vm(self, vm_uuid: str) -> bool:
        """Resume a paused VM."""
        if vm_uuid not in self.instances:
            return False
        
        instance = self.instances[vm_uuid]
        
        if instance.state != VMState.PAUSED:
            return False
        
        success = self.qemu_mgr.resume_vm(vm_uuid)
        
        if success:
            instance.state = VMState.RUNNING
            self._save_instances()
            self._emit_event(LifecycleEvent.RESUMED, vm_uuid, instance)
        
        return success
    
    def suspend_vm(self, vm_uuid: str) -> bool:
        """Suspend a VM (save state to disk)."""
        if vm_uuid not in self.instances:
            return False
        
        instance = self.instances[vm_uuid]
        
        snapshot = self.suspend_mgr.suspend(vm_uuid)
        
        if snapshot:
            instance.state = VMState.SUSPENDED
            instance.metadata["suspend_snapshot"] = snapshot.id
            self._save_instances()
            self._emit_event(LifecycleEvent.SUSPENDED, vm_uuid, instance)
            return True
        
        return False
    
    def resume_from_suspend(self, vm_uuid: str) -> bool:
        """Resume a suspended VM."""
        if vm_uuid not in self.instances:
            return False
        
        instance = self.instances[vm_uuid]
        
        if "suspend_snapshot" not in instance.metadata:
            return False
        
        snapshot_id = instance.metadata["suspend_snapshot"]
        success = self.suspend_mgr.resume(snapshot_id)
        
        if success:
            instance.state = VMState.RUNNING
            del instance.metadata["suspend_snapshot"]
            self._save_instances()
            self._emit_event(LifecycleEvent.RESUMED, vm_uuid, instance)
        
        return success
    
    def destroy_vm(self, vm_uuid: str, force: bool = False) -> bool:
        """Destroy a VM and cleanup resources."""
        if vm_uuid not in self.instances:
            return False
        
        instance = self.instances[vm_uuid]
        
        # Stop if running
        if instance.state == VMState.RUNNING:
            if not self.stop_vm(vm_uuid, force=force):
                return False
        
        # Cleanup storage
        if hasattr(instance.spec, "disks") and instance.spec.disks:
            import shutil
            disk_path = instance.spec.disks[0].path
            vm_dir = os.path.dirname(disk_path)
            
            if os.path.exists(vm_dir):
                shutil.rmtree(vm_dir)
        
        # Remove instance record
        del self.instances[vm_uuid]
        self._save_instances()
        
        self._emit_event(LifecycleEvent.DESTROYED, vm_uuid, instance)
        
        return True
    
    def is_vm_ready(self, vm_uuid: str) -> bool:
        """Check if VM is ready (OS booted)."""
        # Implementation would check via agent or network
        # For now, assume ready after start
        return True
    
    def get_vm_state(self, vm_uuid: str) -> Optional[VMState]:
        """Get current VM state."""
        if vm_uuid not in self.instances:
            return None
        return self.instances[vm_uuid].state
    
    def list_vms(self, vm_type: Optional[VMType] = None) -> List[VMInstance]:
        """List all VMs, optionally filtered by type."""
        if vm_type:
            return [inst for inst in self.instances.values() if inst.vm_type == vm_type]
        return list(self.instances.values())
    
    def get_vm(self, vm_uuid: str) -> Optional[VMInstance]:
        """Get a specific VM instance."""
        return self.instances.get(vm_uuid)
    
    def get_metrics(self, vm_uuid: str) -> Dict:
        """Get VM performance metrics."""
        if vm_uuid not in self.instances:
            return {}
        
        # Would integrate with QEMU stats
        return {
            "uuid": vm_uuid,
            "state": self.instances[vm_uuid].state.value,
            "cpu_usage": 0.0,
            "memory_usage": 0.0,
            "disk_io": 0.0,
            "network_io": 0.0
        }


# Example usage
if __name__ == "__main__":
    lifecycle_mgr = VMLifecycleManager()
    
    # Register callbacks
    def on_created(vm_uuid, instance):
        print(f"VM {instance.name} created: {vm_uuid}")
    
    def on_started(vm_uuid, instance):
        print(f"VM {instance.name} started")
    
    lifecycle_mgr.register_callback(LifecycleEvent.CREATED, on_created)
    lifecycle_mgr.register_callback(LifecycleEvent.STARTED, on_started)
    
    # Create Windows VM
    win_vm = lifecycle_mgr.create_vm(
        vm_type=VMType.WINDOWS,
        name="win11-test",
        version="windows11",
        cpu_cores=4,
        memory_mb=8192,
        disk_size_gb=100
    )
    print(f"Created Windows VM: {win_vm.uuid}")
    
    # Start VM
    lifecycle_mgr.start_vm(win_vm.uuid)
    
    # Get state
    state = lifecycle_mgr.get_vm_state(win_vm.uuid)
    print(f"VM State: {state.value}")
    
    # Get metrics
    metrics = lifecycle_mgr.get_metrics(win_vm.uuid)
    print(f"Metrics: {metrics}")
    
    # Pause VM
    lifecycle_mgr.pause_vm(win_vm.uuid)
    
    # Resume VM
    lifecycle_mgr.resume_vm(win_vm.uuid)
    
    # Stop VM
    lifecycle_mgr.stop_vm(win_vm.uuid)
    
    # Destroy VM
    lifecycle_mgr.destroy_vm(win_vm.uuid)
    
    # List all VMs
    vms = lifecycle_mgr.list_vms()
    print(f"Total VMs: {len(vms)}")
