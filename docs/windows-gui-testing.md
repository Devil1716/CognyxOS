# Windows GUI testing

Phase 13.5 adds a fail-closed harness so live Notepad verification cannot touch personal documents.

This is **not** enabled for normal users. Production `application.open` is unchanged unless `COGNYX_GUI_TEST=1`.

## Test workspace

| Item | Value |
|---|---|
| Workspace | `C:\CognyxOSTestWorkspace\` |
| Golden document | `C:\CognyxOSTestWorkspace\CognyxOS-Golden-Test.txt` |
| Environment flag | `COGNYX_GUI_TEST=1` |
| Optional document override | `COGNYX_GUI_TEST_DOCUMENT` (must still be a `cognyxos-*.txt` under the workspace) |

The test creates the workspace and golden file. It never uses Desktop, Documents, Downloads, `.env`, or any existing user file.

## Safety boundaries

The harness and GUI-test `application.open` reject:

- titles/paths containing `.env`, credentials, secrets, password, token
- user Documents, Desktop, and Downloads
- any path outside `C:\CognyxOSTestWorkspace\`
- filenames that are not `cognyxos-*.txt`

Failure code: `TEST_TARGET_UNSAFE`. The test stops. It does not type, save, or close the ambiguous window.

## Process ownership

Windows 11 Store/WinUI Notepad is single-instance and may reuse personal tabs. When `COGNYX_GUI_TEST=1` and the discovered application is Notepad, `application.open` launches classic `%WINDIR%\System32\notepad.exe` with the golden document. That process is separate from any already-open WinUI Notepad.

Ownership is **not** “any `notepad.exe`”. The provider records:

1. every visible HWND present before spawn
2. the PID created by the test spawn
3. the HWND discovered after launch whose title contains `cognyxos-golden-test`

Pre-existing windows are ignored. Protected titles are ignored. If a unique new test-owned window cannot be proven, open fails with `TEST_TARGET_UNSAFE`.

## Window ownership

After open, subsequent operations use that exact `window_id` (`hwnd:<usize>`). Tests do not search for “Notepad” again.

Focus is verified through `window.focus` / `window.inspect` (`focused: true` plus the golden title). If focus cannot be verified, keyboard input is not sent.

## Text verification

Preferred order used by the harness:

1. prove the owned window is focused
2. `window.inspect` document text via UI Automation (Win11 Notepad does not expose a Win32 Edit control)
3. if needed, save only the golden file and read `C:\CognyxOSTestWorkspace\CognyxOS-Golden-Test.txt`
4. clipboard via CapabilityGateway (`ctrl+a`, `ctrl+c` with `window_id`) as a last resort

Clipboard is never read before ownership is proven.

## Save

If a save is required, only `Ctrl+S` on the owned window is used. The file must remain `C:\CognyxOSTestWorkspace\CognyxOS-Golden-Test.txt`. Contents are checked after save.

## Cleanup

Cleanup focuses and closes **only** the recorded owned HWND, and only if that HWND still has the golden title. Then it deletes only the golden test file if the test created it.

It never:

- closes arbitrary Notepad windows
- closes personal Notepad
- runs `taskkill /IM notepad.exe` or any process-wide kill

Leftover Notepad windows (Untitled, personal documents, previous failed runs) are detected and ignored. A leftover **golden-test** window at start fails with `TEST_TARGET_UNSAFE` and must be closed manually. If cleanup cannot re-prove ownership: `CLEANUP_REQUIRES_MANUAL_INTERVENTION`.

## Known Windows 11 Notepad behavior

- Store/WinUI Notepad may reuse one process and keep personal tabs in the same window.
- Typing into that window, or closing it, can affect personal documents. The harness therefore does not target it.
- Classic System32 Notepad is multi-instance and can own a dedicated HWND for `CognyxOS-Golden-Test.txt`.
- Window title is used as a **marker**, never as the sole identity. HWND + new-window + marker + non-protected path are required together.

## How to run

```
cargo test -p cognyx-e2e --test phase13 -- --include-ignored --nocapture --test-threads=1
```

`GuiHarness::prepare()` sets `COGNYX_GUI_TEST=1` for the process. Do not export the flag globally for interactive CognyxOS use.
