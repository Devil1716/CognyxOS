# Runtime bootstrap specification

The bootstrap coordinator is the only component allowed to start or stop managed runtime services. It executes this dependency order and records every transition with the boot correlation ID.

```mermaid
sequenceDiagram
  participant B as Bootstrap coordinator
  participant C as Configuration
  participant D as DI composition root
  participant L as Logging
  participant R as Service registry
  participant E as Event store/bus
  participant I as IPC host
  participant S as Scheduler
  B->>C: Load, migrate, validate configuration
  B->>D: Build dependency graph
  B->>L: Configure structured logging
  B->>R: Start registry (not ready)
  B->>E: Recover event store and start bus
  B->>I: Start authenticated local IPC
  B->>S: Start scheduler (not accepting work)
  B->>R: Register services and dependency health
  B->>B: Run readiness checks
  B-->>B: Publish SystemBooted; state = Running
```

1. **Boot:** create a boot ID, establish minimal crash-safe logging, and acquire the singleton runtime lock.
2. **Configuration:** layer, migrate, validate, and resolve secret references. Invalid configuration fails closed before service creation.
3. **Dependency injection:** construct the complete directed dependency graph, rejecting missing bindings/cycles before starting any service.
4. **Logging:** enable the structured/audit logger and propagate the boot correlation ID.
5. **Registry and event store:** restore persistent state first. The registry is reachable only internally until readiness.
6. **IPC and scheduler:** start authenticated IPC, then scheduler workers in paused/admission-closed mode.
7. **Health and ready:** register services in dependency order, run readiness checks, open scheduler admission, publish `SystemBooted`, and mark `Running`.

Failure before the event store is healthy stops immediately and preserves diagnostics. Failure of an optional service enters `Degraded`; failure of a required security, registry, store, or IPC service enters `Failed` and performs safe shutdown. Retries are limited to declared transient dependencies with exponential backoff and jitter; configuration/authorization/compatibility failures do not retry. A restart uses a fresh boot ID, durable cursors, and checkpoint recovery—not in-memory state.

Safe shutdown closes admission, publishes `SystemShutdown`, drains in-flight work to its deadline, checkpoints scheduler/event cursors, unregisters services in reverse dependency order, flushes logs/audit records, then releases the lock. Timeout escalates from cooperative cancellation to isolated process termination only after durable state is preserved.
