# CognyxOS System Validation Report

Date: 2026-08-14
Host: Windows (DESKTOP-8FSAOT2), rustc 1.99.0-nightly (008fa22ce 2026-07-25)
Tree: C:\Users\DaRkAngeL\Desktop\cognyxos
Verdict: NOT a complete operating system.

Subsystems work in isolation. They are not wired as one USER to SHELL to KERNEL to RUNTIME to APPLICATION loop. Failures are kept.

## What actually works (REAL)

- Universal path: process.list, application.list/search/open, window.list/focus/close, keyboard.type, clipboard write/read
- Notepad: C:\WINDOWS\notepad.exe pid 28964 opened, focused, typed, closed
- Clipboard token COGNYXOS-VALIDATION-TOKEN round-tripped
- filesystem.delete and clipboard.read without grant: USER_APPROVAL_REQUIRED
- window.teleport not success; screen.read CAPABILITY_UNAVAILABLE
- Workspace in-memory FS; shell does not execute OS actions itself
- Kernel submit_task handle in 19 ms then plans and dispatches
- Long-term memory owner-scoped delete; sample plugin lifecycle; local worker heartbeat auth
- Hardening rejects production plus nightly; secret redact refuses leak

## Incomplete

- Shell main.rs uses RecordingKernel, not AgentKernelServer. Golden path MOCKED.
- Workspace InMemoryFilesystem, not Hyper-V/Docker VMs
- Plugins in-process, not Wasm; workers local, not WAN
- Playwright missing; Docker daemon down; Hyper-V unelevated; cargo-audit missing
- No signed installer

## Failures

| ID | Failure | Class |
|---|---|---|
| VAL-001 | Gateway success for bash via sim-backend with no provider | DEFECT fake success |
| VAL-002 | Original notepad test used target notepad not application_id | test vs API; fixed search-then-open |
| VAL-003 | keyboard.type needed input.text; gateway now maps it | glue hole mapped |
| VAL-004 | PATH listing 256/dir; search exact-exe fallback added | completeness |
| VAL-005 | cargo test --workspace aborted at VAL-001; later crates not run | process gap |
| VAL-006 | Doctor ok:true for virtualization while untested | misleading health |

Test legacy_non_universal_capability_must_not_fake_success is kept failing.

## Mocked vs real

Notepad/clipboard/process/window: REAL. Browser: NOT RUN. Workspace FS: MOCKED. Shell golden path: MOCKED. Kernel handle: REAL in-process. Multi-agent/plugins/workers: IN-PROCESS. bash: FAKE SUCCESS.

NativeApplicationProvider stamps runtime_id host-linux-1 even when launching Windows Notepad.

See phase-acceptance-matrix.md, security-validation.md, release-readiness.md, next-phase-plan.md.

No commit was created. Proposed message:

validate CognyxOS 1-11 on Windows and record real vs mocked seams

Keep the bash fake-success test failing. Map keyboard/window gateway inputs so host computer-use can actually run. Search PATH when application.search would otherwise miss notepad.exe. Document that the shell still uses RecordingKernel.

## Tooling (follow-up)

- cargo clippy --workspace --all-targets --offline --target-dir target-val: exit 0 (warnings only)
- cargo test --workspace --exclude cognyx-e2e --offline --target-dir target-val: REST_EXIT 0
- cognyx-e2e system_validation: 15 passed, 1 failed (VAL-001 bash fake success, kept)
- cargo-audit: not installed
