# CognyxOS VM Storage Architecture

> **Document ID:** ARCH-PHASE2-STORAGE  
> **Version:** 1.0.0  

---

## 1. Centralized Storage Layout
- `/var/lib/cognyxos/images`: Base OS installation ISOs and golden qcow2 templates.
- `/var/lib/cognyxos/disks`: Active qcow2/raw virtual disk drives.
- `/var/lib/cognyxos/snapshots`: Copy-on-Write (CoW) disk checkpoints.

## 2. CoW Cloning Workflow

```mermaid
sequenceDiagram
    participant Golden as Golden qcow2 Image
    participant Storage as VMStorageManager
    participant Disk as Active Copy-on-Write Disk
    
    Storage ->> Golden: Create backing store reference
    Storage ->> Disk: Allocate qcow2 overlay disk
    Disk -->> Storage: Ready for VM boot (< 100ms)
```
