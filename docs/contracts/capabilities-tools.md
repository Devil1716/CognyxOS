# Capabilities and tool contract

## Capability framework

A capability is a named, versioned, permission-scoped operation—not a raw API. Its descriptor is `{capability_id, version, risk_level, input_schema, output_schema, required_permissions, failure_modes, idempotency, audit_classification}`. Inputs and outputs are JSON-schema references. Capability invocation is authorized by the permission broker and produces an audit event.

| Domain     | Required capabilities                                 | Default risk                    |
| ---------- | ----------------------------------------------------- | ------------------------------- |
| Filesystem | `read`, `write`, `copy`, `move`, `delete`, `watch`    | confirmation for writes/deletes |
| Browser    | `navigate`, `download`, `fill_form`, `capture_screen` | confirmation for form/download  |
| Model      | `chat`, `embed`, `vision`, `transcribe`, `rerank`     | safe for local approved model   |

Failure modes are `validation_failed`, `permission_denied`, `not_supported`, `resource_unavailable`, `deadline_exceeded`, `cancelled`, and `internal`. Destructive operations require an explicit rollback declaration or `rollback: unsupported`; no operation may imply rollback.

## Tool contract

Every tool supplies metadata (`tool_id`, name, version, owner, capability IDs), manifest permissions, input/output schemas, health/readiness, initialization, validation, execution, optional rollback, shutdown, metrics, structured logs, and standard errors. Tools receive injected dependencies only; they cannot discover secrets or instantiate platform services. Execution must honor cancellation/deadline, emit a correlation ID, report an idempotency declaration, and never log secret input.

Tool lifecycle: `discovered → validated → initialized → ready → executing → ready → shutting_down → stopped`; failure transitions to `unhealthy`. A failed initialization is compensating and leaves no registered capability. Metrics are local-only counters, duration histograms, and failure-code totals.
