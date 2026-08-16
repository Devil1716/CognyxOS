# Cross-OS validation

This run is a **Windows host**.

- Windows native capabilities (process.list, application.list/search, clipboard, window.list): REAL + HARDWARE VERIFIED (where tests passed).
- Linux runtime not registered on this host. Requesting `linux-host` for workspace I/O returns `RUNTIME_UNAVAILABLE`. Class: REAL + INTEGRATION VERIFIED (honest failure). **Not** a silent `InMemoryFilesystem` success.
- macOS runtime not registered. Same `RUNTIME_UNAVAILABLE`.
- Phase 7 unit tests still attach in-memory linux-host / windows-vm / macos-vm **inside the test harness**. Those are IN-MEMORY contract tests, not hardware cross-OS proof.

Do not claim Linux or macOS hardware verification from this Windows run.
