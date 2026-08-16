# Phase 9: Distributed execution

**Status:** IMPLEMENTED (worker registry + remote runtime adapter)  
**Last Updated:** 2026-08-14

## Overview

Machines become execution workers. The Agent Kernel stays logically
centralized. `WorkerRegistry` **registers** workers into the existing
`RuntimeRegistry` via `RemoteWorkerRuntime`. It does not replace
RuntimeRegistry, the scheduler, or the capability gateway.

```
Agent Kernel → Capability Gateway → Runtime Registry
        → Worker Registry → Remote Worker → Native Runtime
```

## Features

- Worker identity, capabilities, resources, health, status, policy
- TLS-required communication flag (workers without TLS are rejected)
- Token authentication + principal authorization
- Heartbeat / disconnect / health
- Resource-aware selection (OS, RAM, GPU, latency, capability)
- Task assign / cancel
- Artifact transfer with checksum + encryption flag + resume
- Checkpoint via existing `CheckpointEngine`
- Migration: checkpoint → select replacement → restore assignment
- Duplicate destructive execution guard
- Non-transferable state is not migrated

## What this phase does not claim

- Not a real WAN TLS mesh or mTLS CA
- Not a decentralized consensus system
- Network failures are modeled in-process (disconnect/timeout), not against a physical LAN
- No production simulated cloud provider

## Tests

`runtime/worker/tests/phase9.rs` covers registration, discovery, heartbeat,
health, remote task, artifact transfer, worker failure + migration,
checkpoint restore, network/auth/authorization failure, resource
scheduling, and duplicate destructive blocking.

## Next

Phase 10: long-term memory. Extend Phase 3 WorkingMemory; do not rewrite it.
