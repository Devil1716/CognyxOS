# Phase 13.5: deterministic Windows GUI validation harness

Phase 13 already plans and executes `"Open Notepad and type Hello CognyxOS"` through the production Shell → Agent Kernel path. The remaining gap was proving that live typing hit a **test-owned** Notepad instance rather than a reused Windows 11 window.

Phase 13.5 does **not** change the planner, IntentEngine, scheduler, permission engine, or production Shell wiring.

## What changed

- `cognyx_capability::gui_test` — test workspace, protected-path/title filters, golden document helpers. Inactive unless `COGNYX_GUI_TEST=1`.
- `NativeApplicationProvider::application.open` — in GUI test mode only: attach `C:\CognyxOSTestWorkspace\CognyxOS-Golden-Test.txt`, spawn classic System32 Notepad for Notepad apps, accept only a new HWND with the golden title marker.
- `WindowsWindowProvider` — `window.focus` / `window.inspect` report `focused`.
- `WindowsKeyboardProvider` — honors `window_id` on type and hotkey; in GUI test mode refuses to type if focus cannot be verified.
- `tests/e2e/tests/common/mod.rs` — `GuiHarness` used by the Golden Test and hardware Notepad tests.
- Docs: `windows-gui-testing.md`, `golden-test.md`, this file.

## What did not change

- Planner graph remains `application.search` → `application.open` (`${step-1.applications[0].application_id}`) → `keyboard.type` with dynamic `window_id`.
- Production Shell still uses `AgentKernelAdapter` → `AgentKernelServer`.
- No RecordingKernel, MockKernel, MockApplicationProvider, MockKeyboardProvider, or sim-backend on the Golden path.
- No `taskkill` / “close all Notepad windows”.

## Failure mode

Ambiguous or personal targets fail closed:

```
TEST_TARGET_UNSAFE
```

Cleanup that cannot re-prove ownership reports:

```
CLEANUP_REQUIRES_MANUAL_INTERVENTION
```

## Acceptance

See `docs/golden-test.md` and `docs/windows-gui-testing.md`. Phase 13.5 PASS requires a dedicated test environment, a proven owned HWND, real keyboard through CapabilityGateway, independent verification of `Hello CognyxOS`, and cleanup of only test-owned resources.
