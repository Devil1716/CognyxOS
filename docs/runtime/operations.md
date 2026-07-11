# Health, diagnostics, scheduling, and metrics

## Health and diagnostics

The local-only diagnostic API exposes `liveness`, `readiness`, and `diagnostics` through authenticated IPC; it never listens on a network interface by default. Liveness answers whether the supervisor loop runs. Readiness answers whether all required dependencies, migrations, identity, event store, registry, and IPC are usable. Diagnostics returns sanitized component state, versions, health checks, active boot ID, recent error codes, and metric summaries.

Crash reporting writes a local redacted diagnostic bundle: configuration fingerprints (not secrets), last structured records, component health, stack traces, event/store integrity result, and version inventory. Debug mode increases local logging and requires explicit development configuration. Safe mode disables third-party plugins, background admission, and destructive capabilities. Recovery mode is read-only until storage/configuration integrity is confirmed.

## DI standard

The composition root owns registration; modules declare constructor dependencies against contracts. Registration occurs before any service starts. Lifetimes are: singleton (one runtime owner, e.g. registry), scoped (one request/task/session), and transient (new value per resolution). Singletons cannot depend on scoped services. Cycles fail startup with a dependency path; service-locator use is prohibited except in the composition root. Lazy dependencies use an explicit `Provider<T>` contract only where startup ordering would otherwise be cyclic, and must be documented.

## Scheduling policy

The scheduler has separate queues for service lifecycle work, interactive tasks, background jobs, and maintenance. Within a queue it uses priority `critical|high|normal|low`, deadlines, resource/capability constraints, and fair aging. Execution is cooperative: every task receives cancellation/deadline signals and must checkpoint at documented safe points. Retries use bounded exponential backoff with jitter and only declared retryable errors/idempotency keys. Queue admission stops in paused/degraded/safe modes according to policy. Starvation prevention ages waiting work and reserves bounded capacity for maintenance; critical work cannot bypass permission or resource quotas.

## Metrics standard

Metrics are local structured measurements with name, value, unit, timestamp, service/instance, boot ID, and correlation/trace IDs when available. Required metrics: `runtime.startup_duration_ms`, `runtime.memory_bytes`, `runtime.cpu_percent`, `ipc.latency_ms`, `tool.duration_ms`, `plugin.load_duration_ms`, `events.appended_total`, `events.replay_lag`, `scheduler.queue_depth`, `scheduler.wait_duration_ms`, and `health.score` (0–100 with component breakdown). Histograms capture latency; counters are monotonic per boot; gauges capture current state. High-cardinality IDs and user content are forbidden as labels. Default sampling is 100% for errors/audit, aggregate 10-second operational intervals, and configurable 1% tracing; raw metric retention is 30 days and rollups 365 days.
