# Golden Test

Sentence:

```
Open Notepad and type Hello CognyxOS
```

## Required path

```
USER
 → CognyxShell
 → AgentKernelAdapter
 → AgentKernelServer
 → IntentEngine
 → Planner
 → ExecutionGraph
 → GraphScheduler
 → CapabilityGateway
 → PermissionEngine
 → RuntimeRegistry
 → WindowsApplicationProvider (GUI-test owned Notepad)
 → WindowsWindowProvider
 → WindowsKeyboardProvider
 → "Hello CognyxOS"
 → verification
 → task COMPLETED
 → Shell result
```

This path must **not** use RecordingKernel, MockKernel, mock providers, sim-backend, or InMemoryFilesystem for the typed result.

## Isolation

The ignored hardware Golden Test (`golden_shell_open_notepad_and_type_hello_cognyxos` in `tests/e2e/tests/phase13.rs`) calls `GuiHarness::prepare()`, which sets `COGNYX_GUI_TEST=1` and creates `C:\CognyxOSTestWorkspace\CognyxOS-Golden-Test.txt`.

`application.open` then launches a test-owned classic Notepad window for that file. Keyboard input uses the `window_id` returned by open (planner binding). Verification reads the owned window (clipboard via gateway, then the golden file). Cleanup closes only that HWND.

## Failure conditions

The test fails (safely) if:

- the application or window cannot be uniquely identified
- the target path is unknown, protected, or outside the test workspace
- focus cannot be verified
- keyboard action or text cannot be verified
- multiple golden-test windows remain ambiguous
- a leftover `CognyxOS-Golden-Test` window already exists (manual close required)

Do not treat the Golden Test as PASS unless this exact sentence ran through the production Shell and `Hello CognyxOS` was independently verified in the test-owned instance.

## How to run

Always-on planner/auth tests:

```
cargo test -p cognyx-e2e --test phase13
```

Live GUI Golden Test (interactive desktop):

```
cargo test -p cognyx-e2e --test phase13 -- --include-ignored --nocapture --test-threads=1
```
