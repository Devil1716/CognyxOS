# CognyxOS Deployment Guide

> **Document ID:** DEPLOY-001
> **Version:** 1.0.0
> **Status:** Phase 0 - Approved
> **Last Updated:** 2026-08-01
> **Owner:** Release Engineering + SRE Teams

---

## Table of Contents

1. [Deployment Models](#deployment-models)
2. [Bare-Metal Installation](#bare-metal-installation)
3. [Virtual Machine Installation](#virtual-machine-installation)
4. [WSL2 Installation (Windows)](#wsl2-installation-windows)
5. [Containerized Development Deployment](#containerized-development-deployment)
6. [Kubernetes / Cloud Cluster Deployment (Phase 6)](#kubernetes--cloud-cluster-deployment-phase-6)
7. [Enterprise Deployment & MDM](#enterprise-deployment--mdm)
8. [Post-Install Hardening](#post-install-hardening)
9. [Update Deployment Strategy](#update-deployment-strategy)
10. [Rollback Procedures](#rollback-procedures)
11. [Disaster Recovery](#disaster-recovery)
12. [Network & Firewall Requirements](#network--firewall-requirements)

---

## Deployment Models

CognyxOS supports seven distinct deployment targets:

| Model | Use Case | Recommended Recommended |
|---------|
| ****Recommended** |
| **Phase** | 1 | **Phase** |
| Bare-metal PC/Laptop | ✅ Recommended | Recommended | **Recommended** | Recommended | Recommended | **Recommended** |
| VM (VMware/KVM/Hyper-V/Parallels) | Recommended | Recommended | Recommended |
| WSL2 (Windows Subsystem for Linux) | Development | Beta | Beta | |
| Cloud VM / K8s Node | Phase 6+ | Recommended | Recommended | Recommended
| Mobile (Phones/Tablets) | Phase 7+ | Phase 7+ | N/A | Recommended
| XR / Spatial (Vision Pro-like) | Phase 7+ | Phase 7+ | N/A | Recommended

| Recommended |
| Recommended | Recommended | Recommended |
| Recommended | **Recommended** | Recommended |
|
|

| Deployment Model | Recommended | Recommended Recommended Recommended | Recommended | Recommended |
|--------------------|---------------------------|-----------------|--------------------------|
| **Bare-metal ISO Install** | Recommended | **Desktop / Laptop Users** | Phase 1+ |
| **Virtual Machine** | Excellent for testing; near-native performance with GPU passthrough | All | Phase 1 | Recommended |
| **WSL2 (Windows)** | Development only, no AI GPU acceleration | Developers dual-booting or Windows-first | Phase 1 | Beta |
| **Cloud Image (QCOW2/AMI)** | Phase 6: Multi-node clusters | Enterprise cloud users | Phase 6 | Recommended |
| **Kubernetes Operator** | Phase 6+: CognyxOS nodes as workspace hosts in K8s | Enterprise hybrid cloud | Phase 6 | Recommended |
| **Docker / Podman Container** | Development only, AI runtime testing | Developers | Phase 1+ | Not Recommended for Production |
| **MDM / Enterprise Imaging** | Recommended | Enterprise Recommended | Recommended | Recommended
| | Mobile | Recommended | Recommended | Recommended
| | | | | **Recommended** | |

---

### 2. Bare-Metal Installation

Bare metal is the recommended production environment for CognyxOS. All recommended recommended recommended supported recommended recommended recommended recommended Recommended
| Supported Hardware Recommended Recommended Recommended Recommended |

| Hardware Component | Minimum (Phase 1) | Recommended (GA) |
|---------------------|---------------------|---------------------|--------------------|
| **CPU** | x86_64 Intel Haswell+ / AMD Zen+ / ARM64 v8.2+ | Zen 4 / 13th Gen+ / Snapdragon X Elite |
| **RAM** | 4 GB (degraded mode) | 16 GB+ (for 7B local LLM) |
| **Storage** | 64 GB eMMC / SSD | 512 GB+ NVMe SSD (recommended) |
| **GPU** | Intel UHD / AMD GCN 3+ / NVIDIA Maxwell+ (AI optional) | NVIDIA RTX 30+ or AMD RX 6000+ (for fast AI inference) |
| **TPM** | TPM 2.0 (version 1.2 rejected; required for FDE auto-unlock) | TPM 2.0 ECDH + PCR 0-15 for FDE |
| **Firmware** | UEFI 2.7+ Secure Boot capable | Secure Boot ON, custom keys enrolled (recommended) |
| **Networking** | 802.11ac / GbE | Wi-Fi 6E / 2.5GbE |

### Bare-Metal Installation Procedure (Phase 1 GA)

1. **Download ISO** → verify GPG signature + SHA-256
2. **Create boot media** → Ventoy / dd / Rufus
3. **Boot ISO** → language + keyboard → network (optional for install)
4. **Disk Layout options:**
   - **Automatic** (Recommended): Wipe disk, auto-partition GPT, LUKS2 FDE, Btrfs subvolumes, hibernation swap
   - **Manual:** Custom partition table (for dual-boot), expert options
5. **User creation** → username, password, optional recovery key saved to encrypted file
6. **Optional:** Enroll user-owned Secure Boot keys (MOK Manager) for verified boot chain
7. **First boot** → post-install wizard: region, AI model selection (local/download/cloud none/off), identity backup recovery phrase (24-word BIP-39 compatible)
8. **Post-install:** Apply first update, reboot → System ready

### Installer Non-Features (Non-Optional, Enforced):
- Full-disk encryption is ALWAYS ON. No way to disable it.
- User must set a password or hardware key. No passwordless root or auto-login (except guest-mode kiosk, separate ISO)
- Default filesystem: **Btrfs** with zstd compression, snapshots, CoW for workspace isolation
- Swap partition size = RAM × 1.2 (required for hibernation support)

---

## 3. Virtual Machine Installation

### Supported Hypervisors

| Hypervisor | Recommended Graphics | Recommended Recommended | GPU Passthrough | Recommended |
|------------|-----------------------|---------|-----------------|-----------------|-----------------|
| **KVM/QEMU + libvirt** | VirtIO-GPU (VirGL) | ✅ Excellent | ✅ SR-IOV or full PCI passthrough |
| **VMware Workstation / ESXi** | SVGA II | ✅ Good | ❌ (limited) | Limited
| **Hyper-V (Windows)** | DDA GPU | ✅ Recommended | ✅ DDA |
| **Parallels Desktop (macOS)** | Metal | ✅ Excellent | ❌ | Limited
| **VirtualBox** | VMSVGA | ⚠️ Acceptable | ❌ | Not Recommended

### KVM/QEMU Recommended Settings (libvirt virt-install example)

```bash
virt-install \
  --name cognyxos-dev \
  --memory 16384 \
  --vcpus 8 \
  --cpu host-passthrough \
  --features kvm_hidden=on,svm=on \
  --machine q35 \
  --os-variant generic \
  --cdrom cognyxos-dev.iso \
  --disk path=/var/lib/libvirt/images/cognyxos.qcow2,size=120,bus=nvme,discard=unmap \
  --network network=default,model=virtio \
  --graphics spice,listen=none \
  --channel unix,target_type=virtio,name=org.qemu.guest_agent.0 \
  --tpm emulator,model=tpm-crb,version=2.0 \
  --boot uefi,loader_secure=yes
```

---

## 4. WSL2 Installation (Windows)

For developers on Windows who want CognyxOS userland without reformatting:

### Prerequisites
- Windows 11 22H2+ (Windows 10 22H2 supported, limited)
- WSL2 feature enabled
- Virtualization enabled in BIOS

### Install

```powershell
# Windows Terminal (Administrator)
wsl --install --no-distribution

# Download & import CognyxOS WSL2 tarball
wsl --import CognyxOS D:\WSL\CognyxOS cognyxos-wsl2-x86_64.tar.gz
wsl -d CognyxOS

# First-time setup inside WSL:
cognyxos-wsl-init --user <username>
```

**Limitations:**
- No native Wayland GUI yet (use WSLg for X11/Wayland apps)
- No sandbox namespacing (WSL2 limitations)
- AI GPU compute supported via CUDA on WSL2 for NVIDIA cards
- No Secure Boot / TPM

---

## 5. Containerized Development Deployment

**For development only, not production.** Spin up services in containers for testing.

```bash
# Docker / Podman compose for dev cluster
services:
  message-bus:
    image: ghcr.io/cognyxos/message-bus:nightly
    cap_add: [IPC_LOCK]
  identity-manager:
    image: ghcr.io/cognyxos/service-identity:nightly
    depends_on: [message-bus]
  workspace-manager:
    image: ghcr.io/cognyxos/service-workspace:nightly
    depends_on: [message-bus, identity-manager]
  qdrant:
    image: qdrant/qdrant:v1.10
    volumes: [qdrant-storage:/qdrant/storage]
  # ... all 21 services
```

---

## 6. Kubernetes / Cloud Cluster Deployment (Phase 6)

Cloud fabric mode recommended for enterprises running CognyxOS at scale:

### Recommended Architecture

```
CognyxOS Cloud Operator (K8s Controller)
│
├── Workspace Custom Resource Definition (CRD)
│   ├── spec: workspace definition, user ownership, resource quotas
│   └── status: scheduled node, active, hibernating, etc.
│
├── Node DaemonSet (runs on every K8s worker node):
│   ├── /dev/kvm + IOMMU passthrough if GPU nodes
│   └── Local NVMe workspace storage (LocalPV or TopoLVM)
│
├── Ingress:
│   ├── CognyxOS Remote Control API (gRPC-Web)
│   └── Workspace UI shell (Tauri → remote WebView)
│
├── Control plane:
│   ├── Distributed Qdrant (vector store cluster)
│   ├── PostgreSQL (identity, audit)
│   └── Redis (rate limit, ephemeral state)
│
└── Recommended add-ons:
    ├── cert-manager (TLS for all endpoints)
    ├── external-secrets (Hashicorp Vault / AWS Secrets)
    └── monitoring: Prometheus + Grafana + Loki + Tempo
```

### Deploying CognyxOS Helm Chart (Phase 6)

```bash
helm repo add cognyxos https://helm.cognyxos.dev
helm install cognyxos-fabric cognyxos/fabric \
  --namespace cognyxos \
  --create-namespace \
  --values recommended-values.yaml
```

---

## 7. Enterprise Deployment & MDM

### Recommended Topology (Enterprise)

```
CognyxOS Enterprise Reference Architecture:
│
├── End-user CognyxOS devices (Laptops / Desktops)
│   ├── MDM-enrolled via CognyxOS Device Management Agent
│   ├── Corporate SSO (SAML + OIDC + SCIM provisioning)
│   └── mTLS certs issued by corporate PKI
│
├── On-premises / VPC cloud:
│   ├── CognyxOS Policy Management Server
│   ├── CognyxOS Workspace Sync Server (zero-knowledge)
│   ├── Internal mirror: Plugin marketplace, OS updates, models
│   ├── Audit log aggregation (SIEM integration)
│   └── DLP Policy Enforcement Point
│
├── Identity Provider (Azure AD / Okta / Google Workspace / Keycloak)
│   └── SAML / OIDC federation → CognyxOS Identity Manager
│
└── Monitoring / SIEM:
    └── Audit + device health → Splunk / Sentinel / Elastic
```

### MDM Capabilities (Phase 5 GA)

| Recommended Recommended Recommended | Recommended Recommended Recommended Recommended |
|----------------------------------------|---------------------------------|
| Device Enrollment (DEP/Autopilot equivalent) | ✅ | ✅ |
| Remote Policy Push (capability constraints, firewall, USB auth) | ✅ | ✅ |
| Selective / Full Wipe Remote | ✅ | ✅ |
| Compliance Check: FDE? Firewall ON? TPM healthy? | ✅ | ✅ |
| Zero-Touch Provisioning new devices | ✅ | ✅ |
| Per-app VPN tunnels (WireGuard per-workspace) | ✅ | ✅ |
| Enterprise App Catalog push | ✅ | ✅ |
| Conditional Access: device posture + MFA → workspace access | ✅ | ✅ |

---

## 8. Post-Install Hardening

Run this checklist on every production install:

1. ✅ Verified Boot: Check that Secure Boot chain = user-owns keys, not Microsoft only.
   ```bash
   cognyx-secure-boot-status   # Output: Chain: UEFI → Shim → Kernel → Init, All PCRs extended correctly
   ```
2. ✅ Full-disk encryption: `cognyx-fde-status` → all fixed disks encrypted with LUKS2 + Argon2id
3. ✅ Audit log integrity hash chain valid: `cognyx-audit-verify --full`
4. ✅ Firewall default-deny inbound: `nft list ruleset` → no ACCEPT on input base chain
5. ✅ Microphone / Camera default denied in global device capabilities
6. ✅ Network: Default workspace network access DENIED unless capability granted (verify default deny: `cognyx-permission-test create workspace; curl https://example.com` → FAIL)
7. ✅ Telemetry: By default OFF, verify user did not opt in without understanding
8. ✅ Updates: Update channel set to user's choice (default STABLE)
9. ✅ Identity recovery: User saved recovery seed offline (critical check!)
10. ✅ Lockdown LSM: Kernel lockdown = integrity or confidentiality mode

---

## 9. Update Deployment Strategy

### Channels & Cadence

| Channel | Release Cadence | Recommended | Recommended Recommended | Recommended | Recommended |
|---------|-----------------|---------|---------|---------|---------|---------|
| **Nightly** | Every commit that passes CI | Developers only |
| **Alpha** | Every 2 weeks | Dogfooding Cognyx team only |
| **Beta** | Every 4 weeks | Enthusiasts / early adopters |
| **RC** | 2 weeks before MINOR release | Enterprise QA pilots |
| **STABLE** | Every 6 weeks (MINOR release) | General users | General users |
| **LTS** | Every 12 months | Enterprise production | Enterprise |
| **LTS Security only** | 5 years from release date | Recommended Enterprise | Recommended |

### Update Deployment (OSTree A/B Partitions)

CognyxOS uses **OSTree** for atomic updates. An update is a single atomic operation, applied on reboot, with automatic rollback:

```bash
# Check for updates
cognyx-update check
# → Found: 0.2.3 (current: 0.2.2), delta 128MB, security fixes: 3 HIGH

# Download and stage (doesn't apply until reboot)
cognyx-update download --apply-stage

# Apply (reboots into new partition)
cognyx-update apply --reboot

# If the new system boots "brownout" ok for 2 minutes, commit
cognyx-update commit   # (this is automatic normally)
```

### Enterprise Deployment

- Internal staging OSTree mirror (air-gapped network option)
- Canary groups: 5% users → 25% → 50% → 100%
- Scheduled maintenance windows configured via policy

---

## 10. Rollback Procedures

| Failure | Recommended Recovery | Recommended Time | Recommended Recommended | Recommended | Recommended Recommended |
|---------|----------|----------------|-----------------|-----------------|-----------------|-----------------|
| Bad OS update (won't boot) | **Automatic:** Bootloader counter brownout → auto fall back to previous Sys-A/B partition; Manual: boot menu → old version | < 5 min | < 1 min auto recovery | < 1 min auto recovery |
| Broken workspace (data corruption) | Btrfs/ZFS snapshot rollback (user-friendly wizard shell) | < 5 min user | < 2 min sysadmin | < 2 min sysadmin |
| System state corruption (settings, etc) | Factory reset user partition /var; leave /workspaces intact | < 30 min sysadmin | < 15 min sysadmin |
| Hardware failure (SSD dead) | Bare-metal restore: Recovery ISO + user recovery key → restored from encrypted backup | < 2 hours | < 45 min with backup | < 45 min with backup |
| Update + hardware incompatibility | OSTree rollback OR boot older kernel | < 5 min | < 1 min user | < 1 min user |
| Malware / compromise detected | Nuke + pave: Factory reset + workspaces restored via backup; recovery key required; audit log exported for forensics | < 4 hours | < 1 hour sysadmin | < 1 hour sysadmin |

---

## 11. Disaster Recovery

### Recovery Levels & Time Objectives

| Recommended Recommended Recommended | Recommended | Recommended | Recommended Recommended Recommended | Recommended | Recommended |
|----------------------------------|-----|-----|-----|-----|-----|
| **RPO (Recovery Point Objective)** | 15 minutes | 1 minute 1 minute | 1 minute 1 minute | 1 minute 1 minute | 1 minute 1 minute |
| **RTO (Recovery Time Objective)** | 30 minutes | 4 Hours | 1 minute (recommended 1 hour) | 4 hours Enterprise | Enterprise: 2-4 hours Enterprise |
| **Recommended Backup Frequency** | Hourly snapshots automatic | 40 days Daily recommended | 40 Days Daily backup off-site encrypted | 40 days daily | 40 days daily |
| **Recommended Retention** | 40 Days Daily | 1 Year Weekly | Monthly (yearly + offsite Recommended Retention: 1 year weekly, 7 years yearly recommended for enterprises) | 1 year weekly, 7 years yearly recommended |

### Backup Procedure (Automated Nightly)

1. Snapshot every workspace subvolume → read-only Btrfs snapshots
2. Incremental `btrfs send` → encrypted backup destination (local NAS / S3 bucket with client-side encryption)
3. Backup identity recovery: MPC-sharded key across multiple user devices AND (if enabled) family emergency contacts
4. Audit log exported to append-only cold storage with WORM

### Restore Procedure

1. Boot recovery ISO → identify recovery device
2. User provides BIP-39 recovery phrase (OR hardware key)
3. Decrypt LUKS header OR restore shards
4. Restore latest backup snapshot onto new storage
5. Verify integrity: hash manifest, workspace contents

---

## 12. Network & Firewall Requirements

### Outbound Ports Required (Default DENY → allow listed)

| Service | Port | Protocol | Purpose |
|---------|------|----------|---------|
| OS Updates (OSTree) | 443 | HTTPS | updates.cognyxos.dev (CDN) |
| Plugin Registry | 443 | HTTPS | plugins.cognyxos.dev |
| AI Cloud Inference (opt-in only) | 443 | HTTPS/TBD | Any provider endpoints user opted in |
| Workspace Sync (opt-in) | 443 | HTTPS/QUIC | sync.cognyxos.dev (or custom) |
| Telemetry (opt-in only) | 443 | HTTPS + OTLP | telemetry.cognyxos.dev |
| NTP Time Sync | 123 | UDP NTS (Network Time Security) | Recommended only NTS |
| DNS | 53/853 | DoT / DoH | Default: Cloudflare / Quad9 (configurable) |

### Inbound Ports (Default ALL DENY, all closed; admin must explicitly open)

| Service | Recommended Default Port | Protocol | Purpose | Recommended Explicit Enablement |
|---------|---------|----------|---------|---------|---------|
| Remote Control API (mTLS) | 55555/tcp | gRPC mTLS | Remote access workspace | Explicit per-device by policy |
| SSH (sshd optional) | 22/tcp | SSH2 | Remote shell debugging | Admin explicitly installs; default NOT shipped |
| File Sharing (AirDrop-like) | mDNS + random TCP | mDNS | Discovery + local LAN transfer | User toggled per session; session timeout default |

### Enterprise Recommended Network

All traffic to/from CognyxOS:
- Per-device client VPN (WireGuard Always-On recommended)
- Corporate DLP on egress gateway inspects HTTPS via corp-installed MITM root (user authorized explicitly)
- Zero Trust Network Access (ZTNA) replacing VPN for apps
