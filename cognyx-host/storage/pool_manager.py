"""
CognyxOS Storage Pool Manager

Purpose: Manage ZFS/Btrfs storage pools for VM images, snapshots,
and atomic updates.

Features:
- Copy-on-write for instant snapshots
- Per-VM encrypted datasets
- Automatic pool expansion
- Compression and deduplication
"""

import subprocess
import json
from pathlib import Path
from dataclasses import dataclass
from typing import Optional, List
from enum import Enum


class PoolType(Enum):
    ZFS = "zfs"
    BTRFS = "btrfs"


@dataclass
class StoragePool:
    name: str
    pool_type: PoolType
    devices: List[str]
    size_bytes: int
    available_bytes: int
    compression: str
    encryption: bool


@dataclass
class VMDataset:
    vm_id: str
    dataset_path: str
    size_quota: int
    snapshot_count: int


class StoragePoolManager:
    """Manages storage pools for CognyxOS VMs."""
    
    def __init__(self, pool_name: str = "cognyx-pool"):
        self.pool_name = pool_name
        self.root_path = Path(f"/{pool_name}")
        
    def create_pool(self, devices: List[str], pool_type: PoolType = PoolType.ZFS,
                    compression: str = "lz4", encryption: bool = True) -> StoragePool:
        """
        Create a new storage pool from given devices.
        
        Reasoning: ZFS provides atomic snapshots essential for
        VM state management and A/B updates.
        """
        if pool_type == PoolType.ZFS:
            cmd = ["zpool", "create", "-f"]
            if encryption:
                cmd.extend(["-O", "encryption=aes-256-gcm", "-O", "keyformat=passphrase"])
            cmd.extend([self.pool_name] + devices)
            
            result = subprocess.run(cmd, capture_output=True, text=True)
            if result.returncode != 0:
                raise RuntimeError(f"Failed to create ZFS pool: {result.stderr}")
                
            # Set compression
            subprocess.run(["zfs", "set", f"compression={compression}", self.pool_name])
            
        elif pool_type == PoolType.BTRFS:
            cmd = ["mkfs.btrfs", "-f", "-d", "raid0", "-m", "raid1"] + devices
            result = subprocess.run(cmd, capture_output=True, text=True)
            if result.returncode != 0:
                raise RuntimeError(f"Failed to create Btrfs filesystem: {result.stderr}")
                
            # Mount the pool
            mount_point = Path(f"/mnt/{self.pool_name}")
            mount_point.mkdir(exist_ok=True)
            subprocess.run(["mount", devices[0], str(mount_point)])
        
        return self.get_pool_info()
    
    def get_pool_info(self) -> StoragePool:
        """Get current pool statistics."""
        if self._is_zfs():
            result = subprocess.run(
                ["zpool", "list", "-j", self.pool_name],
                capture_output=True, text=True
            )
            data = json.loads(result.stdout)
            pool_data = data["pools"][0]
            
            return StoragePool(
                name=self.pool_name,
                pool_type=PoolType.ZFS,
                devices=pool_data.get("spec", []),
                size_bytes=int(pool_data.get("size", 0)),
                available_bytes=int(pool_data.get("avail", 0)),
                compression=self._get_zfs_property("compression"),
                encryption=self._get_zfs_property("encryption") != "off"
            )
        else:
            # Btrfs fallback
            result = subprocess.run(
                ["btrfs", "filesystem", "usage", "-j", str(self.root_path)],
                capture_output=True, text=True
            )
            data = json.loads(result.stdout)
            
            return StoragePool(
                name=self.pool_name,
                pool_type=PoolType.BTRFS,
                devices=data.get("devices", []),
                size_bytes=data.get("total-size", 0),
                available_bytes=data.get("free", 0),
                compression="zstd",  # Default for Btrfs
                encryption=True  # Assume dm-crypt layer
            )
    
    def create_vm_dataset(self, vm_id: str, size_quota: int) -> VMDataset:
        """
        Create isolated dataset for a VM.
        
        Reasoning: Each VM gets its own dataset with quotas
        for resource isolation and easy snapshot management.
        """
        dataset_name = f"{self.pool_name}/vms/{vm_id}"
        
        if self._is_zfs():
            # Create dataset with quota
            subprocess.run([
                "zfs", "create",
                "-o", f"quota={size_quota}",
                "-o", "compression=lz4",
                "-o", "atime=off",
                dataset_name
            ], check=True)
            
            dataset_path = Path(f"/{dataset_name}")
        else:
            # Btrfs subvolume
            dataset_path = self.root_path / "vms" / vm_id
            dataset_path.mkdir(parents=True, exist_ok=True)
            subprocess.run([
                "btrfs", "quota", "enable", str(dataset_path)
            ], check=True)
            subprocess.run([
                "btrfs", "qgroup", "limit", str(size_quota), str(dataset_path)
            ], check=True)
        
        return VMDataset(
            vm_id=vm_id,
            dataset_path=str(dataset_path),
            size_quota=size_quota,
            snapshot_count=0
        )
    
    def create_snapshot(self, vm_id: str, snapshot_name: str) -> str:
        """
        Create instant snapshot of VM dataset.
        
        Reasoning: Copy-on-write enables instant snapshots
        with minimal overhead for rollback and cloning.
        """
        timestamp = snapshot_name or "auto"
        
        if self._is_zfs():
            snapshot_path = f"{self.pool_name}/vms/{vm_id}@{timestamp}"
            subprocess.run(["zfs", "snapshot", snapshot_path], check=True)
            return snapshot_path
        else:
            # Btrfs subvolume snapshot
            source = self.root_path / "vms" / vm_id
            snapshot = self.root_path / "snapshots" / vm_id / timestamp
            snapshot.parent.mkdir(parents=True, exist_ok=True)
            subprocess.run([
                "btrfs", "subvolume", "snapshot", str(source), str(snapshot)
            ], check=True)
            return str(snapshot)
    
    def rollback_snapshot(self, vm_id: str, snapshot_name: str) -> None:
        """Rollback VM to previous snapshot."""
        if self._is_zfs():
            snapshot_path = f"{self.pool_name}/vms/{vm_id}@{snapshot_name}"
            subprocess.run(["zfs", "rollback", snapshot_path], check=True)
        else:
            # Btrfs: delete current and rename snapshot
            current = self.root_path / "vms" / vm_id
            snapshot = self.root_path / "snapshots" / vm_id / snapshot_name
            
            subprocess.run(["btrfs", "subvolume", "delete", str(current)], check=True)
            subprocess.run([
                "btrfs", "subvolume", "snapshot", str(snapshot), str(current)
            ], check=True)
    
    def destroy_vm_dataset(self, vm_id: str) -> None:
        """Destroy VM dataset and all snapshots."""
        if self._is_zfs():
            subprocess.run([
                "zfs", "destroy", "-r", f"{self.pool_name}/vms/{vm_id}"
            ], check=True)
        else:
            dataset = self.root_path / "vms" / vm_id
            snapshots = self.root_path / "snapshots" / vm_id
            
            if dataset.exists():
                subprocess.run([
                    "btrfs", "subvolume", "delete", str(dataset)
                ], check=True)
            if snapshots.exists():
                subprocess.run([
                    "btrfs", "subvolume", "delete", str(snapshots)
                ], check=True)
    
    def _is_zfs(self) -> bool:
        """Check if pool is ZFS."""
        result = subprocess.run(
            ["zpool", "list", self.pool_name],
            capture_output=True
        )
        return result.returncode == 0
    
    def _get_zfs_property(self, prop: str) -> str:
        """Get ZFS property value."""
        result = subprocess.run(
            ["zfs", "get", "-H", "-o", "value", prop, self.pool_name],
            capture_output=True, text=True
        )
        return result.stdout.strip() if result.returncode == 0 else ""
    
    def expand_pool(self, new_devices: List[str]) -> None:
        """Add new devices to existing pool."""
        if self._is_zfs():
            subprocess.run(
                ["zpool", "add", self.pool_name] + new_devices,
                check=True
            )
        else:
            # Btrfs: add device to filesystem
            for device in new_devices:
                subprocess.run(
                    ["btrfs", "device", "add", device, str(self.root_path)],
                    check=True
                )
            # Rebalance filesystem
            subprocess.run(
                ["btrfs", "balance", "start", str(self.root_path)],
                check=True
            )


# Example usage
if __name__ == "__main__":
    manager = StoragePoolManager("cognyx-pool")
    
    # Create pool with NVMe devices
    # pool = manager.create_pool(
    #     devices=["/dev/nvme0n1", "/dev/nvme1n1"],
    #     compression="lz4",
    #     encryption=True
    # )
    
    # Create VM dataset
    # vm_dataset = manager.create_vm_dataset("windows-vm-001", size_quota=50 * 1024**3)
    
    # Create snapshot before update
    # snapshot = manager.create_snapshot("windows-vm-001", "pre-update-20240115")
    
    print(f"Storage pool manager initialized for {manager.pool_name}")
