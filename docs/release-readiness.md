# Release Readiness

Decision: **NOT READY** for any release channel.

Phase 12 fixed VAL-001 and connected the production shell to `AgentKernelServer`. That is necessary, not sufficient, for a release.

Still blocking:

- Planner cannot turn NL into application.search then open(application_id) then keyboard.type.
- Playwright missing; browser UNAVAILABLE.
- Docker / Hyper-V not healthy on this host (doctor virtualization ok: false).
- cargo-audit not installed (CARGO_AUDIT=NOT_AVAILABLE).
- No signed installer.
- Plugins IN-PROCESS (WASM NOT IMPLEMENTED).
- Workers local only (WAN NOT VERIFIED).
- Memory default IN-PROCESS.
- Notepad GUI e2e remains ignored unless run interactively.

Follow-up: clippy on the workspace; cargo-audit still missing.
