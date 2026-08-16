# System validation report (Phase 12)

Host: Windows (this machine). Date: 2026-08-14 IST.

## VAL-001 (bash fake success)

**Before:** `CapabilityGateway::execute_capability` for non-universal caps formatted  
`Executed capability 'bash' on target '' via runtime 'sim-backend-' (args: [])` with `success: true`.

**After:** no lookup and no real process execute → `success: false`, error contains `CAPABILITY_UNAVAILABLE`. No `sim-backend` in the production path.

Existing test `tests/e2e/tests/system_validation.rs::legacy_non_universal_capability_must_not_fake_success` **PASS unchanged**.

Unit test `test_capability_gateway_pipeline` now expects `CAPABILITY_UNAVAILABLE` (LinuxRuntime.execute_command is a formatted string, not a real process).

Planner nodes that still emit `bash` fail honestly via `dispatch_node_execution`.

## Production kernel wiring

`ui/shell/src/main.rs` constructs `AgentKernelAdapter` wrapping `AgentKernelServer`. It does not mention `RecordingKernel`. RecordingKernel remains for `ui/shell/tests/phase8.rs` and the e2e shell-approval test (TEST ONLY).

## Runtime identity

On Windows, `AgentKernelServer::new` registers `WindowsRuntime::host()` as `windows-host`. Providers receive that id. `application.search` / `application.list` `assigned_runtime_id` does not contain `linux`.

## Doctor

`Diagnostic.status: HealthStatus`. Virtualization `ok` is false when not verified. Docker daemon down → UNAVAILABLE. Hyper-V elevation failure → PERMISSION_DENIED.

## Workspace

Dedicated root `C:\CognyxOSTestWorkspace`. HostFilesystem create/read/write PASS. Missing linux-host / macos-host → `RUNTIME_UNAVAILABLE`.

## Golden path

REAL: Shell → AgentKernelServer.submit_task.  
REAL: kernel gateway `application.search` notepad, Windows runtime id.  
PLANNER STILL CANNOT: NL → search → open by application_id → focus → type → close. Prompt containing "application" plans `application.open` with command `winget`/`echo`, which fails honestly (`ApplicationNotFound`). Full GUI sequence remains the ignored `e2e_open_notepad_and_type_hello_cognyxos`.
