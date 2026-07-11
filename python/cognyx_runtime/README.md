# Cognyx Runtime

The Phase 2 core runtime is a local platform supervisor, not an AI, desktop, memory, or agent subsystem.

## Start

```powershell
python -c "from cognyx_runtime import Runtime; from cognyx_runtime.console import render_startup; r=Runtime(); r.start(); print(render_startup(r)); r.stop()"
```

The local development API binds only to `127.0.0.1` and requires `Authorization: Bearer <boot_id>`. It exposes `/health`, `/ready`, `/metrics`, `/runtime`, `/services`, `/events`, and `/diagnostics`.

## Architecture and testing

`runtime.py` owns lifecycle transitions and boot order. `events.py` is a durable SQLite event log, `registry.py` is local discovery and version negotiation, `scheduler.py` handles cooperative background work, and `ipc.py` hosts the local development adapter. Run `python -m pytest python/cognyx_runtime/tests` for tests.
