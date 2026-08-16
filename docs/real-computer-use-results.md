# Real Computer-Use Results (Windows host)

Date: 2026-08-14. Host: Windows.

## Notepad

Attempt 1: application.open target notepad -> ApplicationNotFound (must be discovered). Honest API, wrong test.

Attempt 2: application.search notepad succeeded.
Discovered app:C:\WINDOWS\notepad.exe and WindowsApps notepad.exe.
Opened C:\WINDOWS\notepad.exe process_id 28964.
window.list found Notepad. window.focus ok. keyboard.type Hello CognyxOS ok. window.close ok.
No leftover notepad process.
Classification: REAL. Typing success is SendInput after focus, not OCR of the buffer.

## Other

process.list REAL (tasklist). application.list REAL (PATH scan). window.list REAL. clipboard write/read token COGNYXOS-VALIDATION-TOKEN REAL.
clipboard.read and filesystem.delete without grant blocked.
screen.read unavailable. mouse not executed. screen.capture not executed.

Only Notepad was launched. No System32 writes.
