# ADR 0001: Local gRPC for service calls and durable events for facts

Status: Accepted. Date: 2026-07-11.

CognyxOS requires typed request/response, streaming, cancellation, offline operation, replay, and OS abstraction. We choose gRPC semantics over platform-local transport for service calls, and a durable local event log for immutable facts. Named pipes and Unix sockets are transport adapters, not public contracts. WebSockets are reserved for a future external gateway; shared memory requires a future ADR.

Consequences: every service has a versioned contract and registry entry; every event is idempotently consumable. There is no direct endpoint coupling and no generic message queue as a substitute for commands.
