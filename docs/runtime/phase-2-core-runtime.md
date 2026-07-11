# Phase 2 core runtime

## Architecture

`Runtime` is the sole bootstrap and shutdown coordinator. It follows the approved order: configuration, logging, dependency graph, registry, durable event bus, local IPC, scheduler, health checks, then `Running`. Shutdown closes scheduler admission, emits the shutdown event, stops IPC, and transitions to `Stopped`.

| Module | Public interface | Example | Troubleshooting and testing |
| --- | --- | --- | --- |
| `runtime` | `Runtime.start()`, `Runtime.stop()` | `runtime = Runtime(); runtime.start()` | Inspect `runtime_info()`; covered by startup/API test. |
| `lifecycle` | `LifecycleCoordinator.transition()` | transition to `Starting` only after `Initializing` | Invalid transitions raise `LifecycleError`; state graph is tested. |
| `container` | `Container.register()`, `resolve()`, `scope()` | register a singleton contract at composition root | Missing bindings and cycles raise `DependencyResolutionError`; lifetimes are tested. |
| `registry` | `register()`, `discover()`, `report_health()` | discover `events` with version `1.1` | A mismatch raises `VERSION_INCOMPATIBLE`; discovery is tested. |
| `events` | `publish()`, `subscribe()`, `replay()` | publish `org.cognyx.system.booted` | Events must use the approved namespace; persistence/filter/replay are tested. |
| `scheduler` | `start()`, `schedule()`, `cancel()`, `shutdown()` | schedule `ScheduledTask(work)` | Admission must be open; execution and pause are tested. |
| `health` | `register()`, `liveness()`, `ready()` | register required dependency checks | A failed required check blocks readiness. |
| `ipc` | `LocalApi.start()`, `endpoint` | `GET /health` with boot bearer token | Loopback-only development API; authentication is tested. |
| `diagnostics` | `snapshot()`, `metrics()` | `runtime.diagnostics.metrics()` | Sanitized operational state only. |
| `console` | `render_startup()` | `print(render_startup(runtime))` | `READY` appears only after `Running`. |
| `plugins` | `PluginManager.discover()`, `register()` | loader validates API version | No plugins ship in Phase 2. |

## Local development API

After startup, `Runtime.runtime_info()` reports the loopback endpoint and boot ID. Use the boot ID as the capability token:

```powershell
curl.exe -H "Authorization: Bearer <boot-id>" http://127.0.0.1:<port>/health
```

Supported local endpoints: `/health`, `/ready`, `/metrics`, `/runtime`, `/services`, `/events`, and `/diagnostics`. The adapter is local-development-only; production IPC stays governed by the approved Buf/protobuf strategy.

## Testing notes

```powershell
python -m pytest python/cognyx_runtime/tests -q
python -m ruff check python/cognyx_runtime
```

Current coverage is 91%. This phase intentionally excludes agents, memory, models, tools, browser work, and UI.
