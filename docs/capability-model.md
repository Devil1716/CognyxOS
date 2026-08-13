# Capability model

`CapabilityDefinition` has a stable id and semantic version, description, JSON input/output schemas, permissions/resources, runtime support, security/risk levels, idempotency, timeout, and audit policy. `CapabilityResult` always includes request, capability, runtime, status, normalized error, timing, provider, metadata, artifacts, and side effects.

Idempotency is explicit: reads are `ReadOnly`; writes are `NonIdempotent`; deletes are `Destructive`. The Planner and Recovery Engine can use this metadata without OS knowledge.
