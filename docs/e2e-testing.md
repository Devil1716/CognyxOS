# E2E testing (Phase 12)

## Always-on (not ignored)

`cargo test -p cognyx-e2e --test system_validation --offline --target-dir target-val`

Includes VAL-001, unauthorized filesystem.delete, unknown capability, process/application list, clipboard (Windows), window.list (Windows), workspace in-memory contract, RecordingKernel shell approvals (TEST ONLY), real-kernel submit, application.search runtime id, doctor.

## Hardware / interactive (ignored)

Phase 13.5 GUI Notepad tests (`phase13` golden/hardware and `e2e_open_notepad_and_type_hello_cognyxos`) use `GuiHarness`. They set `COGNYX_GUI_TEST=1`, open only `C:\CognyxOSTestWorkspace\CognyxOS-Golden-Test.txt`, and close only the owned HWND. Run with `--include-ignored` on an interactive desktop. Do not call Win32 from the test except through CapabilityGateway. See `docs/windows-gui-testing.md`.

`screen_capture_has_real_provider_on_windows` requires a real display DC.

Browser e2e requires Playwright; currently BROWSER=UNAVAILABLE.

## What is REAL vs planner-limited

- REAL: Shell → `AgentKernelAdapter` → `AgentKernelServer::submit_task` → gateway `dispatch_node_execution`.
- REAL: gateway universal sequence search/list/process/clipboard/window.
- REAL: NL `"Open Notepad and type Hello CognyxOS"` compiles to application.search → open(application_id) → keyboard.type. Live GUI proof is the ignored Phase 13.5 Golden Test.
