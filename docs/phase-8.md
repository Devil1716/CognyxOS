# Phase 8: AI-native desktop / Cognyx Shell

**Status:** IMPLEMENTED (native egui desktop + shell API; not a hardware compositor)  
**Last Updated:** 2026-08-14

## Overview

The Cognyx shell is the user-facing OS interface. It is not an admin
dashboard and it is not an execution engine.

```
USER → Cognyx Shell → Agent Kernel → (existing Phase 3–6 path)
```

Natural-language commands go to the Agent Kernel through `KernelClient`.
The shell never plans tasks, dispatches capabilities, or talks to OS APIs
directly.

## Surface

- Native desktop window (`cognyx-shell`) with wallpaper, dock/taskbar, launcher, workspace search
- Command bar (`submit_intent`)
- Agent panel (task + agent tree + recover)
- Human approval: allow once / allow for task / deny (never silent)
- Computer-use observation of Phase 5 streams (display only)
- Unified window model (`window_id`, `application_id`, `runtime_id`, ...)
- Notifications with de-duplication
- Workspace search via Phase 7 `WorkspaceManager`

The GUI is a renderer for this surface. It still forwards every command through
`CognyxShell` → `AgentKernelAdapter` → `AgentKernelServer`. It does not plan,
schedule, or execute capabilities itself.

## Kernel integration

`KernelClient` is the only submit path. Production wiring forwards to
`AgentKernelServer` (`runtime/agent/kernel`). Tests use `RecordingKernel`,
which records prompts and does not execute capabilities.

`src/lib.rs` was added to the kernel crate so the existing `main.rs`
(`use cognyx_agent_kernel::AgentKernelServer`) can build as a library
consumers (the shell) can call. No Phase 3 behavior was rewritten.

## What this phase does not claim

- Not a hardware-accelerated compositor
- Not a full accessibility audit
- Live computer-use frames are observed, not captured by the shell
- RecordingKernel is a test double, not a production provider

## Tests

`ui/shell/tests/phase8.rs` covers launch, command-bar submit, task
progress, agent tree, approve/deny, Windows + browser observation,
workspace switch, file search, failure recovery, and notification
de-dupe.

## Next

Phase 9: distributed execution / workers. Do not replace RuntimeRegistry.
