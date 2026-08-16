# Known Limitations (validated 2026-08-16 IST)

1. Planner emits application.search then open(application_id) then keyboard.type for the Golden sentence. Live GUI proof still requires an interactive desktop and `COGNYX_GUI_TEST=1`.
2. Windows 11 Store/WinUI Notepad is single-instance and may reuse personal tabs. GUI tests therefore spawn classic System32 Notepad into `C:\CognyxOSTestWorkspace\` and fail closed (`TEST_TARGET_UNSAFE`) on ambiguity. Leftover golden-test windows must be closed manually.
3. PATH listing capped at 256 files/dir; search has exact-exe fallback.
4. Playwright not installed; BROWSER=UNAVAILABLE. No package install performed.
5. Workspace unit tests still use InMemoryFilesystem; host backend exists for Windows dedicated root only.
6. Plugins are IN-PROCESS, WASM NOT IMPLEMENTED.
7. Workers are local registry only, WAN NOT VERIFIED.
8. Memory default is IN-PROCESS; optional JSON under the dedicated workspace memory dir if enable_disk_persist is called.
9. Doctor probes virt; this host is not virt-healthy (ok: false).
10. keyboard.type and application.open are Allow by default.
11. No signed installer, no cargo-audit (CARGO_AUDIT=NOT_AVAILABLE).
12. Docker daemon / Hyper-V as reported by doctor (UNAVAILABLE / PERMISSION_DENIED / NOT_VERIFIED).
13. LinuxRuntime.execute_command and Windows PowerShell automation adapters are still formatted strings; gateway no longer treats them as success.
