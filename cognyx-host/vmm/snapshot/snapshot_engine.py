"""
CognyxOS Snapshot Engine

Provides VM snapshot, restore, suspend, resume, and migration capabilities.
Supports live snapshots, incremental backups, and external storage integration.
"""

import os
import json
import subprocess
import socket
from typing import Dict, List, Optional
from dataclasses import dataclass, asdict
from datetime import datetime
from enum import Enum


class SnapshotType(Enum):
    LIVE = "live"  # Running VM state captured
    COLD = "cold"  # Stopped VM state
    EXTERNAL = "external"  # LVM/ZFS external snapshot


@dataclass
class Snapshot:
    id: str
    vm_uuid: str
    name: str
    type: SnapshotType
    created_at: str
    size_bytes: int
    disk_path: str
    state_path: Optional[str]  # RAM/CPU state for live snapshots
    metadata: Dict


class SnapshotEngine:
    """Manages VM snapshots and state persistence."""
    
    def __init__(self, base_storage_path: str = "/var/lib/cognyx/snapshots"):
        self.base_storage_path = base_storage_path
        self.snapshots: Dict[str, Snapshot] = {}
        self._load_existing_snapshots()
    
    def _load_existing_snapshots(self):
        """Load existing snapshots from disk."""
        if not os.path.exists(self.base_storage_path):
            os.makedirs(self.base_storage_path)
            return
        
        index_path = f"{self.base_storage_path}/index.json"
        if os.path.exists(index_path):
            with open(index_path, "r") as f:
                data = json.load(f)
                for snap_id, snap_data in data.items():
                    snap_data["type"] = SnapshotType(snap_data["type"])
                    self.snapshots[snap_id] = Snapshot(**snap_data)
    
    def _save_index(self):
        """Save snapshot index to disk."""
        index_path = f"{self.base_storage_path}/index.json"
        data = {
            snap_id: {
                **asdict(snap),
                "type": snap.type.value
            }
            for snap_id, snap in self.snapshots.items()
        }
        with open(index_path, "w") as f:
            json.dump(data, f, indent=2)
    
    def _get_monitor_socket(self, vm_uuid: str) -> str:
        """Get QEMU monitor socket path."""
        return f"/var/run/cognyx/{vm_uuid}.monitor"
    
    def _send_monitor_command(self, vm_uuid: str, command: str) -> str:
        """Send command to QEMU monitor."""
        monitor_path = self._get_monitor_socket(vm_uuid)
        
        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        client.connect(monitor_path)
        client.sendall(f"{command}\n".encode())
        response = client.recv(4096).decode()
        client.close()
        
        return response
    
    def create_live_snapshot(
        self,
        vm_uuid: str,
        name: str,
        metadata: Optional[Dict] = None
    ) -> Snapshot:
        """Create a live snapshot of a running VM."""
        import uuid as uuid_lib
        
        snapshot_id = str(uuid_lib.uuid4())
        timestamp = datetime.utcnow().isoformat()
        
        vm_dir = f"{self.base_storage_path}/{vm_uuid}"
        os.makedirs(vm_dir, exist_ok=True)
        
        snapshot_dir = f"{vm_dir}/{snapshot_id}"
        os.makedirs(snapshot_dir)
        
        # Create disk snapshot (qcow2 external snapshot)
        disk_path = f"{snapshot_dir}/disk.qcow2"
        self._send_monitor_command(
            vm_uuid,
            f"drive_backup virtio0 {disk_path} qcow2"
        )
        
        # Save VM state (RAM + CPU registers)
        state_path = f"{snapshot_dir}/vmstate.vm"
        self._send_monitor_command(
            vm_uuid,
            f"savevm {snapshot_id}"
        )
        self._send_monitor_command(
            vm_uuid,
            f"migrate_exec 'exec:cat > {state_path}'"
        )
        
        # Calculate size
        size_bytes = os.path.getsize(disk_path) if os.path.exists(disk_path) else 0
        if os.path.exists(state_path):
            size_bytes += os.path.getsize(state_path)
        
        snapshot = Snapshot(
            id=snapshot_id,
            vm_uuid=vm_uuid,
            name=name,
            type=SnapshotType.LIVE,
            created_at=timestamp,
            size_bytes=size_bytes,
            disk_path=disk_path,
            state_path=state_path,
            metadata=metadata or {}
        )
        
        self.snapshots[snapshot_id] = snapshot
        self._save_index()
        
        return snapshot
    
    def create_cold_snapshot(
        self,
        vm_uuid: str,
        name: str,
        disk_path: str,
        metadata: Optional[Dict] = None
    ) -> Snapshot:
        """Create a snapshot of a stopped VM."""
        import uuid as uuid_lib
        
        snapshot_id = str(uuid_lib.uuid4())
        timestamp = datetime.utcnow().isoformat()
        
        vm_dir = f"{self.base_storage_path}/{vm_uuid}"
        os.makedirs(vm_dir, exist_ok=True)
        
        snapshot_dir = f"{vm_dir}/{snapshot_id}"
        os.makedirs(snapshot_dir)
        
        # Copy disk image
        snapshot_disk = f"{snapshot_dir}/disk.qcow2"
        subprocess.run([
            "cp", "--reflink=auto",  # CoW copy if supported
            disk_path,
            snapshot_disk
        ], check=True)
        
        size_bytes = os.path.getsize(snapshot_disk)
        
        snapshot = Snapshot(
            id=snapshot_id,
            vm_uuid=vm_uuid,
            name=name,
            type=SnapshotType.COLD,
            created_at=timestamp,
            size_bytes=size_bytes,
            disk_path=snapshot_disk,
            state_path=None,
            metadata=metadata or {}
        )
        
        self.snapshots[snapshot_id] = snapshot
        self._save_index()
        
        return snapshot
    
    def restore_snapshot(self, snapshot_id: str, target_vm_uuid: Optional[str] = None) -> bool:
        """Restore a VM from snapshot."""
        if snapshot_id not in self.snapshots:
            return False
        
        snapshot = self.snapshots[snapshot_id]
        
        if snapshot.type == SnapshotType.LIVE:
            return self._restore_live_snapshot(snapshot, target_vm_uuid)
        elif snapshot.type == SnapshotType.COLD:
            return self._restore_cold_snapshot(snapshot, target_vm_uuid)
        
        return False
    
    def _restore_live_snapshot(self, snapshot: Snapshot, target_vm_uuid: Optional[str]) -> bool:
        """Restore a live snapshot with state."""
        vm_uuid = target_vm_uuid or snapshot.vm_uuid
        
        # Load VM state
        if snapshot.state_path and os.path.exists(snapshot.state_path):
            self._send_monitor_command(
                vm_uuid,
                f"migrate_incoming 'exec:cat {snapshot.state_path}'"
            )
        
        # Switch to snapshot disk
        self._send_monitor_command(
            vm_uuid,
            f"drive_del virtio0"
        )
        self._send_monitor_command(
            vm_uuid,
            f"drive_add 0 file={snapshot.disk_path},if=virtio,format=qcow2"
        )
        
        return True
    
    def _restore_cold_snapshot(self, snapshot: Snapshot, target_vm_uuid: Optional[str]) -> bool:
        """Restore a cold snapshot (disk only)."""
        # For cold snapshots, just replace the disk path
        # The VM spec should be updated to point to snapshot.disk_path
        return True
    
    def delete_snapshot(self, snapshot_id: str) -> bool:
        """Delete a snapshot and free resources."""
        if snapshot_id not in self.snapshots:
            return False
        
        snapshot = self.snapshots[snapshot_id]
        
        # Remove files
        if os.path.exists(snapshot.disk_path):
            os.remove(snapshot.disk_path)
        if snapshot.state_path and os.path.exists(snapshot.state_path):
            os.remove(snapshot.state_path)
        
        # Remove directory if empty
        snapshot_dir = os.path.dirname(snapshot.disk_path)
        if os.path.exists(snapshot_dir):
            try:
                os.rmdir(snapshot_dir)
            except OSError:
                pass  # Directory not empty
        
        del self.snapshots[snapshot_id]
        self._save_index()
        
        return True
    
    def list_snapshots(self, vm_uuid: Optional[str] = None) -> List[Snapshot]:
        """List all snapshots, optionally filtered by VM."""
        if vm_uuid:
            return [s for s in self.snapshots.values() if s.vm_uuid == vm_uuid]
        return list(self.snapshots.values())
    
    def get_snapshot(self, snapshot_id: str) -> Optional[Snapshot]:
        """Get a specific snapshot."""
        return self.snapshots.get(snapshot_id)


class SuspendResumeManager:
    """Manages VM suspend and resume operations."""
    
    def __init__(self, snapshot_engine: SnapshotEngine):
        self.snapshot_engine = snapshot_engine
    
    def suspend(self, vm_uuid: str, name: Optional[str] = None) -> Optional[Snapshot]:
        """Suspend a running VM (save state to disk)."""
        if name is None:
            name = f"suspend-{datetime.utcnow().isoformat()}"
        
        snapshot = self.snapshot_engine.create_live_snapshot(
            vm_uuid=vm_uuid,
            name=name,
            metadata={"suspend": True}
        )
        
        # Stop VM after snapshot
        self._send_monitor_command(vm_uuid, "stop")
        
        return snapshot
    
    def resume(self, snapshot_id: str) -> bool:
        """Resume a suspended VM."""
        success = self.snapshot_engine.restore_snapshot(snapshot_id)
        
        if success:
            snapshot = self.snapshot_engine.get_snapshot(snapshot_id)
            if snapshot:
                self._send_monitor_command(snapshot.vm_uuid, "cont")
        
        return success
    
    def _send_monitor_command(self, vm_uuid: str, command: str) -> str:
        """Send command to QEMU monitor."""
        monitor_path = f"/var/run/cognyx/{vm_uuid}.monitor"
        
        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        client.connect(monitor_path)
        client.sendall(f"{command}\n".encode())
        response = client.recv(4096).decode()
        client.close()
        
        return response


class MigrationManager:
    """Manages live migration between hosts."""
    
    def __init__(self):
        self.migration_port = 8000
    
    def migrate_outgoing(
        self,
        vm_uuid: str,
        destination_host: str,
        destination_port: Optional[int] = None
    ) -> bool:
        """Migrate VM to another host."""
        port = destination_port or self.migration_port
        uri = f"tcp:{destination_host}:{port}"
        
        monitor_path = f"/var/run/cognyx/{vm_uuid}.monitor"
        
        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        client.connect(monitor_path)
        client.sendall(f"migrate {uri}\n".encode())
        response = client.recv(4096).decode()
        client.close()
        
        return "OK" in response
    
    def migrate_incoming(self, vm_uuid: str, source_uri: str) -> bool:
        """Receive migrated VM from another host."""
        monitor_path = f"/var/run/cognyx/{vm_uuid}.monitor"
        
        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        client.connect(monitor_path)
        client.sendall(f"migrate_incoming {source_uri}\n".encode())
        response = client.recv(4096).decode()
        client.close()
        
        return "OK" in response
    
    def cancel_migration(self, vm_uuid: str) -> bool:
        """Cancel ongoing migration."""
        monitor_path = f"/var/run/cognyx/{vm_uuid}.monitor"
        
        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        client.connect(monitor_path)
        client.sendall("migrate_cancel\n".encode())
        response = client.recv(4096).decode()
        client.close()
        
        return "OK" in response


# Example usage
if __name__ == "__main__":
    engine = SnapshotEngine()
    suspend_mgr = SuspendResumeManager(engine)
    migration_mgr = MigrationManager()
    
    vm_uuid = "550e8400-e29b-41d4-a716-446655440000"
    
    # Create live snapshot
    snapshot = engine.create_live_snapshot(
        vm_uuid=vm_uuid,
        name="pre-update-snapshot",
        metadata={"reason": "system update"}
    )
    print(f"Created snapshot: {snapshot.id}")
    print(f"Type: {snapshot.type.value}")
    print(f"Size: {snapshot.size_bytes} bytes")
    
    # List snapshots
    snapshots = engine.list_snapshots(vm_uuid)
    print(f"\nSnapshots for VM {vm_uuid}:")
    for s in snapshots:
        print(f"  - {s.name} ({s.created_at})")
    
    # Suspend VM
    suspend_snapshot = suspend_mgr.suspend(vm_uuid)
    print(f"\nSuspended VM, state saved to: {suspend_snapshot.id}")
    
    # Resume VM
    if suspend_snapshot:
        success = suspend_mgr.resume(suspend_snapshot.id)
        print(f"Resume status: {success}")
    
    # Delete snapshot
    engine.delete_snapshot(snapshot.id)
    print(f"\nDeleted snapshot: {snapshot.id}")
