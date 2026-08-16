# Phase 7: Unified Workspace

**Status:** IMPLEMENTED (logical workspace + in-memory runtime filesystems)  
**Last Updated:** 2026-08-14

## Overview

Phase 7 adds a unified workspace so files, applications, tasks, artifacts,
runtimes, and agents appear as one environment. Physical storage remains on
execution runtimes. Agents address logical paths such as `/Workspace/Documents`.

```
COGNYX WORKSPACE
        │
 Files / Apps / Tasks / Artifacts
        │
 Workspace Graph
        │
 Linux / Windows / macOS / Container / Remote
```

## What this phase does

- Workspace model (`Workspace`, items, references, permissions, metadata)
- Logical filesystem layout under `/Workspace`
- Portable file references (workspace, item, runtime, physical, checksum, version)
- Cross-runtime copy/move/sync/read/write/version/restore
- Phase 6 artifacts ingested as first-class workspace objects
- Unified metadata search (not semantic memory; that is Phase 10)
- Session state (active workspace, apps, tasks, agents, recent files)
- Conflict detection that refuses silent overwrite
- Security via existing `PermissionEngine` plus workspace ACLs
- Recovery via workspace checkpoints (no secrets)

## What this phase does not claim

- Not hardware-tested against real Windows/macOS VMs
- Cross-OS copies in tests use in-memory runtime filesystems behind `RuntimeRegistry`
- No production simulated providers
- Semantic memory is out of scope (Phase 10)
- Shell UI is out of scope (Phase 8)

## Integration

| Concern | Existing component reused |
|---|---|
| Permission checks | `cognyx-agent-core::PermissionEngine` |
| Runtime presence | `cognyx-execution::RuntimeRegistry` |
| Agent bus events | `cognyx-agent-core::AgentEventPublisher` |
| Artifacts | ingested from Phase 6 artifact identity fields |

No duplicate runtime, capability, scheduler, or permission systems.

## Tests

`services/workspace/tests/phase7.rs` covers create/read, Linux↔Windows copy,
Linux→macOS when attached, artifacts, conflicts, versioning, restore,
permission denial, runtime unavailable, and checkpoint recovery.

## Next

Phase 8: AI-native desktop / Cognyx Shell, using this workspace API and the
existing Agent Kernel. Do not add a second execution engine.
