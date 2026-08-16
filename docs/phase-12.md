# Phase 12: System validation (honesty + production wiring)

**Status:** IMPLEMENTED on this Windows host (2026-08-14 IST)  
**Classification key:** REAL + HARDWARE VERIFIED / REAL + INTEGRATION VERIFIED / MOCKED / IN-MEMORY / UNAVAILABLE / NOT VERIFIED

## What landed

1. **VAL-001** — `CapabilityGateway` no longer formats a simulated success for `bash` (or any capability without a real provider). Missing lookup and no real process execute → `success: false`, `CAPABILITY_UNAVAILABLE`. `sim-backend` is gone from the production path.
2. **Production shell** uses `AgentKernelAdapter` → `AgentKernelServer`. `RecordingKernel` is TEST ONLY.
3. **Runtime identity** comes from `native_host_runtime_id()` / `WindowsRuntime::host()` (`windows-host` on this machine). Native application providers no longer stamp `host-linux-1` on Windows.
4. **Doctor** reports a `HealthStatus`. Virtualization is `ok: false` unless Docker or Hyper-V was actually probed healthy. Never `ok: true` for an untested virt backend.
5. **Real workspace filesystem** (`HostFilesystem`) scoped to `C:\CognyxOSTestWorkspace`. `InMemoryFilesystem` remains for unit tests. Missing Linux/macOS runtime → `RUNTIME_UNAVAILABLE` (not a silent in-memory fallback).
6. **Golden path** submits via the real kernel and drives the same gateway the kernel uses for `application.search`. Planner still cannot emit a full search→open→type→close graph; those nodes fail honestly.

## Evidence

| Item | Result | Class |
|---|---|---|
| VAL-001 `legacy_non_universal_capability_must_not_fake_success` | PASS | REAL + INTEGRATION VERIFIED |
| Shell `main.rs` / `AgentKernelAdapter` | PASS (no RecordingKernel in production) | REAL + INTEGRATION VERIFIED |
| `windows-host` on application.search/list | PASS | REAL + HARDWARE VERIFIED (PATH discovery) |
| Doctor virtualization `ok` | false unless probed healthy | REAL + INTEGRATION VERIFIED |
| Host FS create/read/write under dedicated root | PASS | REAL + HARDWARE VERIFIED |
| Cross-OS Linux/macOS without runtime | `RUNTIME_UNAVAILABLE` | REAL + INTEGRATION VERIFIED |
| Notepad GUI type Hello CognyxOS | `#[ignore]` (existing e2e_phase5) | NOT VERIFIED this run |
| Playwright / browser | BROWSER=UNAVAILABLE | UNAVAILABLE |
| Plugins | in-process | MOCKED / IN-PROCESS (WASM NOT IMPLEMENTED) |
| Workers | local registry | REAL + INTEGRATION VERIFIED (local); WAN NOT VERIFIED |
| Memory | in-process + optional JSON persist | IN-PROCESS (persist path available, not default) |
| cargo-audit | not installed | UNAVAILABLE (`CARGO_AUDIT=NOT_AVAILABLE`) |

## What this phase does not claim

- Not a full NL → application.open/keyboard.type planner rewrite (Phase 3 frozen).
- Not a signed installer, not WAN workers, not Wasm plugins, not a hardware-tested hypervisor.
- Notepad GUI remains `#[ignore]` unless run with `--include-ignored` on an interactive desktop.
