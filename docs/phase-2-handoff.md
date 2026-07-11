# Phase 2 handoff

Implementation must proceed in this order:

1. Create the versioned protobuf workspace and Buf validation/generation integration described in [ADR 0002](adr/0002-protobuf-generation-with-buf.md).
2. Implement shared envelope/error contracts and the Windows process-identity adapter contract from [ADR 0004](adr/0004-process-identity-abstraction.md) and [platform contracts](contracts/platform-contracts.md).
3. Implement the SQLite WAL event-store adapter, migrations, integrity checks, snapshot/replay path, and backup interfaces from [ADR 0003](adr/0003-sqlite-event-store.md).
4. Implement the composition root, configuration validation, structured logging, registry, and lifecycle coordinator exactly in [bootstrap](runtime/bootstrap.md) and [lifecycle](runtime/lifecycle.md).
5. Implement authenticated local IPC and event bus contracts from [IPC and registry](contracts/ipc-and-registry.md) and [events](contracts/events.md).
6. Implement health/diagnostics, scheduler admission, and metrics contracts from [operations](runtime/operations.md).

Do not begin agents, tools, plugins, models, memory, or desktop features until steps 1–6 pass their contract, compatibility, failure, and Windows adapter tests.
