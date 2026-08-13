# Cross-OS Testing Matrix

**Updated:** 2026-08-13 (Phase 5)

## Test Categories

| Category | Description | Run By Default |
|---|---|---|
| UNIT | Individual component tests | Yes |
| CONTRACT | Cross-OS capability contract tests | Yes |
| INTEGRATION | Multi-component integration tests | Yes |
| HARDWARE | Real OS API tests (skip on CI) | No (requires real hardware) |
| END_TO_END | Full pipeline from intent to result | No (requires real environment) |

## Windows Hardware Test Status (Phase 5 — Development Machine)

| Test | Status | Notes |
|---|---|---|
| process.list returns real processes | HARDWARE-TESTED | |
| application.list is dynamic (PATH) | HARDWARE-TESTED | |
| application.open launches real process | HARDWARE-TESTED | |
| filesystem.write + read roundtrip | HARDWARE-TESTED | |
| terminal.execute allowlisted command | HARDWARE-TESTED | |
| terminal.execute rejects non-allowlisted | HARDWARE-TESTED | |
| screen.capture returns real image | NOT-YET | New in Phase 5A |
| window.list returns real windows | NOT-YET | New in Phase 5A |
| window.focus brings window to front | NOT-YET | New in Phase 5A |
| clipboard.write then read | NOT-YET | Provider exists |
| keyboard.type into application | NOT-YET | New in Phase 5A |
| mouse.click on UI element | NOT-YET | New in Phase 5A |
| UI Automation element discovery | NOT-YET | New in Phase 5A |
| browser.open + navigate + read | NOT-YET | New in Phase 5B |
| browser.click test button | NOT-YET | New in Phase 5B |
| Permission denial enforcement | HARDWARE-TESTED | |
| CAPABILITY_UNAVAILABLE returned | HARDWARE-TESTED | |
| USER_APPROVAL_REQUIRED blocks exec | HARDWARE-TESTED | |

## Linux Test Status

All Linux capabilities: **NOT-HARDWARE-TESTED** (development on Windows)
Linux runtime tests require a Linux host.

## macOS Test Status

All macOS capabilities: **NOT-HARDWARE-TESTED** (no Mac available)
macOS runtime tests require a physical Mac or macOS VM.

## Browser Test Status

| Test | Status | Notes |
|---|---|---|
| Playwright availability detection | NOT-YET | Requires npm install |
| browser.open local test page | NOT-YET | |
| browser.read text content | NOT-YET | |
| browser.click button | NOT-YET | |
| browser.type into input | NOT-YET | |
| browser.screenshot local page | NOT-YET | |

## Running Tests

```powershell
# All unit and contract tests
cargo test --workspace

# Windows hardware tests (on Windows)
cargo test -p cognyx-capability --test native_host -- --nocapture

# Browser integration tests (requires Playwright)
cargo test -p cognyx-capability --test browser_integration -- --nocapture

# E2E tests (marked #[ignore], run explicitly)
cargo test -p cognyx-e2e -- --include-ignored --nocapture

# Format check
cargo fmt --all -- --check

# Clippy
cargo clippy --workspace --all-targets

# Security audit
cargo audit
```
