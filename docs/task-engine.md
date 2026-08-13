# CognyxOS Task Engine Architecture & Lifecycle

> **Document ID:** ARCH-PHASE3-TASK-ENGINE  
> **Version:** 1.0.0  

---

## 1. Task Lifecycle State Machine

```mermaid
stateDiagram-v2
    [*] --> CREATED
    CREATED --> PLANNING
    PLANNING --> READY
    READY --> RUNNING
    RUNNING --> WAITING
    WAITING --> RUNNING
    RUNNING --> BLOCKED
    BLOCKED --> RUNNING
    RUNNING --> PAUSED
    PAUSED --> RUNNING
    RUNNING --> FAILED
    FAILED --> RECOVERING
    RECOVERING --> PLANNING
    RECOVERING --> RUNNING
    RUNNING --> COMPLETED
    RUNNING --> CANCELLED
    COMPLETED --> [*]
    CANCELLED --> [*]
```

## 2. Persistent Task Schema
Every task contains:
- `task_id`
- `intent_id`
- `parent_task_id`
- `status` (11 states)
- `priority`
- `constraints`
- `required_capabilities`
- `plan`
- `execution_graph`
- `assigned_runtime`
- `checkpoint`
- `result`
- `error`
- `timestamps`
- `retry_count`
