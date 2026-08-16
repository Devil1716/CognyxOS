# Next Phase Plan

1. Planner: emit universal capabilities (application.search / open by application_id / keyboard.type / window.*) from NL. Do not restore fake bash success.
2. Run ignored Notepad GUI e2e on an interactive desktop and record REAL + HARDWARE VERIFIED.
3. Playwright only if a real local HTML navigate is required. Do not fake a browser. Do not install packages unless already in repo scripts.
4. Hardware runtimes (Docker, Hyper-V) only when in scope; until then keep doctor ok:false.
5. Plugin sandbox: Wasm or keep calling it IN-PROCESS.
6. Install cargo-audit and run a release-profile test.
7. Default memory persist under the dedicated workspace root if product wants durability.
8. Commit Phases 7-12 when the owner asks.

Do not add new architecture until the planner can drive the same gateway sequence the golden tests already use.
