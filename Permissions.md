# CognyxOS Permission System

> **Document ID:** SEC-002
> **Version:** 1.0.0
> **Status:** Phase 0 - Approved
> **Last Updated:** 2026-08-01
> **Owner:** Security Architecture Team

---

## Table of Contents

1. [Permission System Overview](#permission-system-overview)
2. [Capability Taxonomy](#capability-taxonomy)
3. [Capability Namespaces](#capability-namespaces)
4. [Role-Based Capability Composition](#role-based-capability-composition)
5. [Delegation Model](#delegation-model)
6. [Human-in-the-Loop Triggers](#human-in-the-loop-triggers)
7. [Consent Management](#consent-management)
8. [Policy Engine Rules (Rego/OPA)](#policy-engine-rules-regoopa)
9. [Permission UI Patterns](#permission-ui-patterns)
10. [Common Workflows](#common-workflows)

---

## Permission System Overview

CognyxOS uses a **pure object-capability (object-cap) model**, not ACLs, not RBAC as the primary mechanism. Permissions are not attached to users or groups—they are unforgeable tokens that must be explicitly acquired, carried, and presented with each operation.

### Why Not ACLs / RBAC Alone?

| Model | Handles AI Delegation? | Handles Composition? | Confused Deputy Safe? | Audit Trail? |
|-------|------------------------|----------------------|-----------------------|--------------|
| ACLs (Unix Style) | ❌ No | ❌ No | ❌ Prone | Partial |
| RBAC | ⚠️ With great effort | ⚠️ Hierarchical only | ❌ Prone | Partial |
| **Object Capabilities** | ✅ Native delegation | ✅ Natural composition | ✅ Provably safe | ✅ Every use explicit |

**Conclusion:** Capabilities are the only model where AI agents can safely delegate to each other without confused deputy attacks. We do combine capabilities with:
- **Roles** = Preset bundles of capabilities (for UX convenience)
- **Policies** = Negative guardrails (deny even if capability present, e.g., "never at 3am")

---

## Capability Taxonomy

### Classification by Lifespan

| Capability Type | Lifespan | Use Case | Example |
|-----------------|----------|----------|---------|
| **One-Shot** | Single use only (max_uses=1) | Destructive actions, user-approved one-offs | Delete single file |
| **TTL-Limited** | Valid_until timestamp only | Multi-step tasks within bounded time | 10-minute file read batch for AI task |
| **Session** | Tied to a user session (logout = revoke) | Interactive app usage | Terminal session fs access |
| **Persistent** | No expiration (revocable on demand) | Long-term user-approved trust | Backup app write access to backup dir |
| **Permanent** | Require Level 4 auth to revoke | Extremely rare | User's own shell has total workspace authority |

### Classification by Scope

| Capability Type | Scope Granularity | Examples |
|-----------------|-------------------|----------|
| **Exact Object** | Single resource | `filesystem.delete:/workspaces/123/a.txt` |
| **Glob Pattern** | Shell-style globs | `filesystem.read:/workspaces/123/docs/**/*.md` |
| **Wildcard** | All of namespace (rare, heavily restricted) | `notification.send:*` |
| **Parameterized** | Constraints beyond path + op | `network.outbound:api.github.com:443/tcp` |
| **Abstract** | No resource, just operation | `ai.generate_text:gpt4` (no "file" resource) |

### Classification by Authority Direction

```
┌─────────────────────────────────────────────────────────┐
│ INWARD CAPABILITIES (what can be done TO a resource)     │
│ Example: fs.read granted BY the filesystem owner        │
├─────────────────────────────────────────────────────────┤
│ OUTWARD CAPABILITIES (what a module can do)              │
│ Example: process.spawn capability held BY an AI agent   │
├─────────────────────────────────────────────────────────┤
│ DELEGATION CAPABILITIES (authority to pass other caps)  │
│ Example: "may delegate fs.read caps within /docs/"      │
└─────────────────────────────────────────────────────────┘
```

---

## Capability Namespaces

Full, formal namespace definitions. Every operation name in the system is in exactly one namespace.

### Namespace: `filesystem`

| Operation | Resource Pattern | HITL Default | Brief Description |
|-----------|------------------|--------------|-------------------|
| `filesystem.read` | Path Glob | No | Read file contents, list directories, stat metadata |
| `filesystem.write` | Path Glob | Yes (first grant) | Write, truncate, append to files |
| `filesystem.create` | Path Glob | No | Create new files or directories |
| `filesystem.delete` | Exact or Glob | **Always Yes** | Delete files or directories |
| `filesystem.exec` | Exact Path | Yes | Execute a binary file |
| `filesystem.chmod` | Exact or Glob | Yes | Change POSIX permissions |
| `filesystem.chown` | Exact or Glob | **Always Yes** | Change file ownership |
| `filesystem.link_hard` | Glob | Yes | Create hard links (potential privilege escalation path) |
| `filesystem.link_sym` | Glob | Yes | Create symlinks (potential path traversal) |
| `filesystem.xattr_write` | Glob | No | Write extended attributes |
| `filesystem.snapshot_create` | Workspace ID | No | Create Btrfs/ZFS snapshot |
| `filesystem.snapshot_restore` | Snapshot ID | **Always Yes** | Restore snapshot (destructive overwrite) |

### Namespace: `process`

| Operation | Resource Pattern | HITL Default | Brief Description |
|-----------|------------------|--------------|-------------------|
| `process.spawn` | Binary path glob, workspace ID | Yes (first per binary) | Create new process |
| `process.kill` | PID or PID glob within workspace | No | Send SIGTERM/SIGKILL |
| `process.signal` | PID, signal number | Yes for SIGKILL/SIGSTOP | Send arbitrary signal |
| `process.ptrace` | PID or Workspace | **Always Yes** | Debug, inspect, modify process memory |
| `process.setrlimit` | Workspace ID, resource type | Yes | Change resource limits |
| `process.setpriority` | PID or Workspace | No | Change nice/priority |
| `process.namespace_enter` | Namespace type + ID | **Always Yes** | Enter another sandbox's namespace |

### Namespace: `network`

| Operation | Resource Pattern | HITL Default | Brief Description |
|-----------|------------------|--------------|-------------------|
| `network.outbound` | CIDR / Host Glob : PortRange / Protocol | Yes (first per host) | Initiate outbound connections |
| `network.inbound` | Port / Protocol | **Always Yes** | Accept inbound connections (open ports) |
| `network.listen` | Port, IP bind addr | **Always Yes** | Bind and listen on a socket |
| `network.raw` | Interface name | **Always Yes** | Raw packet access |
| `network.firewall_modify` | Rule set ID | **Always Yes** | Add/remove firewall rules |
| `network.tap` | Interface name | **Always Yes** | Create TUN/TAP interfaces |
| `network.proxy_set` | Proxy URL pattern | Yes | Configure HTTP/SOCKS proxy |
| `network.vpn_manage` | VPN profile ID | Yes | Start/stop/configure VPN tunnels |
| `network.dns_set` | Resolver list | **Always Yes** | Change DNS servers |

### Namespace: `device`

| Operation | Resource Pattern | HITL Default | Brief Description |
|-----------|------------------|--------------|-------------------|
| `device.open_read` | Device path / class | Yes (first per device) | Read-only open device node |
| `device.open_write` | Device path / class | **Always Yes** | Write to device |
| `device.ioctl` | Device path, ioctl number range | **Always Yes** | Device-specific ioctls |
| `device.usb_authorize` | USB VendorID:ProductID | **Always Yes** | Allow USB device to be visible |
| `device.gpu_submit` | GPU ID, command class | No | Submit GPU commands |
| `device.gpu_passthrough` | GPU ID, VM ID | **Always Yes** | Exclusive GPU lease to VM |
| `device.camera` | Camera index | **Always Yes** | Access camera device |
| `device.microphone` | Microphone index | **Always Yes** | Access audio input |
| `device.bluetooth` | Device MAC pattern | **Always Yes** | Bluetooth scan/connect |
| `device.location` | Provider ID | **Always Yes** | Obtain geolocation |

### Namespace: `window`

| Operation | Resource Pattern | HITL Default | Brief Description |
|-----------|------------------|--------------|-------------------|
| `window.create` | Count, size bounds, title pattern | No | Create application windows |
| `window.resize` | Window ID pattern | No | Resize/move existing windows |
| `window.close` | Window ID pattern | Yes for non-self windows | Close windows not owned by caller |
| `window.screenshot` | Window ID or "screen" | **Always Yes** | Capture window/screen contents |
| `window.input_inject` | Window ID | **Always Yes** | Inject synthetic mouse/keyboard input |
| `window.set_focus` | Window ID | No | Change input focus |
| `window.fullscreen` | Window ID | Yes for first time | Toggle fullscreen |
| `window.global_shortcut` | Key combination | **Always Yes** | Register global hotkeys |

### Namespace: `workspace`

| Operation | Resource Pattern | HITL Default | Brief Description |
|-----------|------------------|--------------|-------------------|
| `workspace.create` | N/A | No | Create new workspace |
| `workspace.delete` | Workspace ID | **Always Yes** | Delete workspace + all data permanently |
| `workspace.activate` | Workspace ID | No | Activate (mount, start services) |
| `workspace.hibernate` | Workspace ID | No | Hibernate to disk |
| `workspace.clone` | Source Workspace ID | Yes (first clone) | Deep copy workspace |
| `workspace.share` | Workspace ID, Recipient Identity | **Always Yes** | Grant another user access |
| `workspace.export` | Workspace ID, Destination | **Always Yes** | Export workspace as archive (data exfiltration!) |
| `workspace.modify_limits` | Workspace ID, Resource Type | Yes | Change memory/CPU/disk limits |

### Namespace: `ai`

| Operation | Resource Pattern | HITL Default | Brief Description |
|-----------|------------------|--------------|-------------------|
| `ai.generate_text` | Model ID | No | Generate text with LLM (cost consideration) |
| `ai.generate_image` | Model ID | Yes (first use) | Generate images |
| `ai.generate_audio` | Model ID | Yes (first use) | Generate/synthesize audio |
| `ai.tool_use` | Tool name pattern | Yes per dangerous tool | AI to call tools autonomously |
| `ai.memory_write` | Memory type, scope | No | Write to semantic memory |
| `ai.memory_read` | Memory type, scope | No | Read from semantic memory |
| `ai.memory_delete` | Memory ID glob | Yes | Delete items from memory |
| `ai.agent_spawn` | Agent type | **Always Yes** | Spawn new AI agent |
| `ai.agent_delegate` | Agent ID, cap scope | **Always Yes** | Delegate capabilities to an agent |
| `ai.cloud_inference` | Model ID | **Always Yes** | Send prompt data to remote inference API |

### Namespace: `system`

| Operation | Resource Pattern | HITL Default | Brief Description |
|-----------|------------------|--------------|-------------------|
| `system.update` | Channel | **Always Yes** | Apply OS updates |
| `system.reboot` | N/A | **Always Yes** | Reboot machine |
| `system.shutdown` | N/A | **Always Yes** | Power off machine |
| `system.suspend` | N/A | No | Suspend to RAM |
| `system.hibernate` | N/A | No | Hibernate to disk |
| `system.factory_reset` | N/A | **Always Yes (multi-conf)** | Wipe state partition, reset to default |
| `system.time_set` | N/A | Yes | Set system clock |
| `system.locale_set` | N/A | No | Change language/locale |

---

## Role-Based Capability Composition

Roles are UX conveniences—**preset bundles of capabilities** assigned via policy. They do NOT bypass the object-cap model.

### Built-In Roles

```toml
# role: workspace.owner (auto-granted to user who created workspace)
[[role.capabilities]]
namespace = "filesystem"
operations = ["read","write","create","delete","exec","snapshot_create"]
resource = "/workspaces/{{workspace_id}}/**"
ttl = "session"
hitl_override = false  # Owner bypasses HITL for non-destructive ops

[[role.capabilities]]
namespace = "process"
operations = ["spawn","kill","signal"]
resource = "workspace={{workspace_id}},binary=/sys/bin/**"
ttl = "session"

[[role.capabilities]]
namespace = "workspace"
operations = ["activate","hibernate","clone","modify_limits"]
resource = "{{workspace_id}}"
ttl = "permanent"

# role: workspace.contributor
[[role.capabilities]]
namespace = "filesystem"
operations = ["read","write","create"]
resource = "/workspaces/{{workspace_id}}/**"
ttl = "session"

# role: workspace.viewer (read-only)
[[role.capabilities]]
namespace = "filesystem"
operations = ["read"]
resource = "/workspaces/{{workspace_id}}/**"
ttl = "session"

# role: ai.assistant.default (auto-granted to primary AI assistant within workspace)
[[role.capabilities]]
namespace = "filesystem"
operations = ["read"]
resource = "/workspaces/{{workspace_id}}/**"
ttl = "ttl:1h"
hitl_enforced = true  # AI NEVER bypasses HITL on destructive ops

[[role.capabilities]]
namespace = "ai"
operations = ["generate_text","memory_read","memory_write"]
resource = "*"
ttl = "session"

[[role.capabilities]]
namespace = "search"
operations = ["*"]
resource = "workspace={{workspace_id}}"
ttl = "session"
```

**Critical Rule:** Roles with `hitl_enforced = true` CANNOT be used to bypass human-in-the-loop prompts. Roles with `hitl_override = true` are ONLY granted to authenticated user identities (never to agents, apps, or plugins).

---

## Delegation Model

### Delegation Rules (Formal)

Given capability C held by principal P:

1. **Delegation is EXPLICIT ONLY.** No implicit delegation of parent's capabilities to children. P must call `CapabilityService.Delegate()`.
2. **Constraints are MONOTONICALLY REDUCING.** Delegated capability C' must be strictly ≤ C on all constraint dimensions:
   - C'.resource ⊆ C.resource (glob subset test)
   - C'.operations ⊆ C.operations
   - C'.valid_until ≤ C.valid_until
   - C'.max_uses ≤ C.max_uses
   - C'.delegation_depth < C.delegation_depth
   - C'.rate_limit ≤ C.rate_limit
   - C'.workspace_id = C.workspace_id
3. **Delegation depth is bounded (default max = 3).** Prevents deep surprise chains.
4. **Delegation REVOCABLE by ANY ancestor in the chain.** P can revoke C' even if P delegated to Q who delegated to R.
5. **All delegation events trigger audit log entries** at NOTICE severity or above.

### Delegation Example Scenario

```
User U (workspace owner) holds C_full:
  fs.read/write/delete:/ws/1/** (permanent)

U delegates to AI Assistant A:
  C_assistant: fs.read,write:/ws/1/** TTL=2h, depth=1, HITL enforced
  (Note: no delete permission; only subset; TTL shorter)

A wants to delegate to sub-agent B for a task:
  C_subagent: fs.read:/ws/1/reports/*.csv TTL=10min, depth=0, max_uses=50
  (Even more restricted. Depth 0 means B cannot further delegate.)

Attempt to violate (A tries to grant delete to B):
  DENIED. A's token lacks delete operation; monotonic reduction fails.
```

---

## Human-in-the-Loop Triggers

HITL is a **mandatory pause** in any capability-mediated action to obtain explicit user confirmation. Triggers are evaluated in order.

### Trigger Evaluation Order

```
For each capability use:
  1. Is token marked hitl_enforced=true? → PAUSE (can't be bypassed)
  2. Is operation in namespace.operation HITL default "Always Yes"? → PAUSE
  3. Is actor identity type = AGENT or PLUGIN? → PAUSE (agents never auto-perform destructive ops)
  4. Does the role grant hitl_override=true for this exact operation? → PROCEED
  5. Is there a valid user consent record (consented this exact op + resource in last 30 days)? → PROCEED
  6. Default → PAUSE for consent
```

### HITL Prompt Categories

| Prompt Type | UI Modal | Auto-Expire | Example |
|-------------|----------|-------------|---------|
| **Information Only** | Non-blocking toast | N/A | "AI is reading your project files" |
| **Single Object Approval** | Modal with object preview, Accept/Deny | 10 min | "Allow AI to delete file x.pdf?" |
| **Scope Approval** | Modal listing exact resource glob + ops, Accept/Deny/Scope-Narrow | 30 days if "Always" | "Allow Terminal to write to ~/Documents/**?" |
| **Destructive Action** | Red-modal, 3-second delay, confirm by re-typing action word | This action only | "DELETE workspace Project-4? Type DELETE to confirm." |
| **Auth-Level Escalation** | Hardware key / biometric / TOTP challenge | This action only | "Change system DNS servers? Verify with hardware key." |
| **Data Exfiltration** | Modal showing data size + destination + consent to leave machine | This action only | "Export workspace (4.2 GB) to remote server backup.example.com?" |

---

## Consent Management

### Consent Record Format

```
ConsentRecord {
  consent_id: Uuid
  user_identity: IdentityId
  actor: IdentityId          // who the consent is FOR (app, agent, plugin id)
  operation: String          // e.g., "filesystem.read"
  resource_pattern: String   // e.g., "/ws/1/docs/**"
  scope: ONCE | SESSION | DURATION(seconds) | PERMANENT
  granted_at: Timestamp
  auth_level_used: u8       // what auth level user confirmed with
  context_prompt_hash: Sha256 // hash of UI text shown to user (non-repudiation)
  revoked: bool
  revoked_at: Option<Timestamp>
  revoked_reason: Option<String>
}
```

### Consent Management APIs

```protobuf
service ConsentManager {
  rpc GrantConsent(GrantRequest) returns (ConsentRecord);
  rpc RevokeConsent(RevokeRequest) returns (google.protobuf.Empty);
  rpc QueryConsent(QueryRequest) returns (stream ConsentRecord);
  rpc PromptForConsent(PromptRequest) returns (ConsentDecision);
  rpc GetConsentHistory(HistoryRequest) returns (stream ConsentRecord);
  rpc ClearAllConsentForActor(ActorId) returns (google.protobuf.Empty);
}
```

---

## Policy Engine Rules (Rego/OPA)

Capabilities are **positive** permissions. The Policy Engine adds **negative guardrails**—rules that deny even when a valid capability is present.

### Built-in Policy Ruleset (Rego Pseudocode)

```rego
package cognyxos.policy

# DENY 1: Rate limiting (prevents brute force attacks)
deny[{"reason": "rate_limit_exceeded", "details": dl}] if {
    count([e | e := data.audit.entries[_]
            e.actor == input.actor
            e.timestamp > now() - 60_000_000_000
            e.event_type == "CAP_USE"
            e.operation == input.operation]) > input.rate_limit_ops_per_sec * 60
}

# DENY 2: No destructive operations between 02:00-05:00 local unless explicit override
deny[{"reason": "time_window_restricted", "details": "maintenance_window"}] if {
    ns := split(input.operation, ".")[0]
    destructive_ops := {"delete","snapshot_restore","factory_reset","hibernate"}
    split(input.operation, ".")[1] in destructive_ops
    ns != "ai"  # AI ops OK
    hour := input.local_time_hour
    hour >= 2
    hour < 5
    not input.hitl_override_level >= 3  # Level 3 explicit user override OK
}

# DENY 3: Data exfiltration pattern detector
deny[{"reason": "potential_data_exfiltration", "details": pattern}] if {
    input.operation == "network.outbound"
    input.actor.type == "AGENT"
    data.audit.agent_bandwidth[input.actor.id].last_hour > 10_000_000  # >10MB/hr
    not some record in data.consent:
        record.actor == input.actor.id
        record.scope == "PERMANENT"
        record.operation == "network.outbound"
}

# DENY 4: New network hosts (no prior consent for) from unsigned plugins
deny[{"reason": "unsigned_plugin_network_unknown_host"}] if {
    input.actor.type == "PLUGIN"
    input.actor.verified == false
    input.operation == "network.outbound"
    host := extract_host(input.resource)
    count([c | c := data.consent[_]
           c.actor == input.actor.id
           glob.match(c.resource_pattern, false, input.resource)]) == 0
}

# DENY 5: No agent may self-escalate its auth level
deny[{"reason": "agent_self_escalation_attempt"}] if {
    input.actor.type == "AGENT"
    input.operation in ["system.update","workspace.share","identity.add_credential"]
}

# ALLOW by default (capability token has been validated; this is just guardrail layer)
default allow = true
```

### Custom Policy Support

Enterprise customers and power users can add custom Rego policies stored in `/config/system/policy/`:
- Loaded via OPA discovery service
- Hot reloaded (0 downtime)
- Hash-changed validated against admin identity

---

## Permission UI Patterns

### Permission Center Dashboard

A single place in the UI shell for users to:
1. **View all granted capabilities** by:
   - Actor (app / agent / plugin)
   - Resource (file paths, network hosts, devices)
   - Operation type
2. **Revoke any capability** (with cascading revocation of all delegated descendants)
3. **View audit trail** for each permission: "who used what, when, against what resource"
4. **Manage consent** records: revoke session/permanent consents, inspect prompt hash provenance
5. **Policy editor**: Create custom rules (advanced mode)

### Just-In-Time Permission UX

When the AI or an app hits a missing capability:

```
┌─────────────────────────────────────────────────────────────┐
│  ⚠️  Action Blocked - Permission Required                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Assistant needs permission to:                             │
│    ✦ Read files matching: /workspace/Finances/q3/*.xlsx     │
│    ✦ (12 files, 8.4 MB total)                               │
│                                                             │
│  Why? "To extract revenue numbers from Q3 spreadsheets"     │
│  [Show Plan Details ↓]                                      │
│                                                             │
│  Grant for:                                     [This Task ▾]│
│    ○ Just this file set (one-shot)                          │
│    ○ Next 10 minutes                                        │
│    ● This session only (until logout)  ← recommended        │
│    ○ Permanently for this workspace                         │
│                                                             │
│  [Deny]                  [Review & Approve with Hardware Key]│
└─────────────────────────────────────────────────────────────┘
```

---

## Common Workflows

### Workflow 1: New Application Installation

1. User downloads `com.example.app-1.2.3.bundle`
2. Bundle signature verified → App manifest shows **declared capability requirements**
3. Installer UI displays:
   - All required capabilities (in red if high-risk)
   - All optional capabilities (user can individually allow/deny)
   - Justification text (from app developer) per capability
4. User approves subset → Installation proceeds
5. On first launch: capabilities minted with declared + user-approved subset

### Workflow 2: AI Assistant Needs Network Access

1. User: "Summarize the articles in this Hacker News page I'm viewing"
2. AI Planner: Step 1 = fetch webpage content → requires `network.outbound` to `news.ycombinator.com`
3. No cached consent → HITL Prompt
4. Prompt includes: "AI wants to fetch content from news.ycombinator.com (87 KB expected)" + one-shot/session/permanent options
5. User picks one-shot → Capability minted, one-use max
6. Fetch completes → Token auto-invalidated → Audit log + consent record created

### Workflow 3: Cross-Workspace Data Copy

1. User: "Copy the design folder from the Brand workspace to Project Aurora"
2. Planner:
   - Need `filesystem.read:/ws/brand/design/**` in Brand workspace
   - Need `filesystem.write.create:/ws/aurora/design/**` in Aurora workspace
   - Need **cross-workspace transfer capability** (workspace.export + workspace.import)
3. HITL Destructive/Exfiltration Prompt:
   - "Transfer 2.3 GB (1,847 files) FROM Brand → TO Aurora?"
   - Shows preview, option to exclude large files
4. User confirms Level 3 auth → transfer begins
5. Audit log: CAP_USE for cross-workspace transfer + SHA-256 of transfer manifest
