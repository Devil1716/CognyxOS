# Phase Acceptance Matrix

PASS / PARTIAL / FAIL / NOT TESTED. PASS means real effects where the spec requires them.

| Phase | Status | Evidence | Gaps |
|---|---|---|---|
| 1 Kernel | PARTIAL | submit_task handle; production shell uses AgentKernelServer | planner nodes still not a full OS loop |
| 2 Intent | PARTIAL | used in kernel submit | no isolated eval |
| 3 Planner | PASS | create_plan compile_plan get_ready_nodes; NL search → open(application_id) → type | close still a separate prompt |
| 4 Capabilities | PARTIAL | real process/app/window/keyboard/clipboard; VAL-001 honest bash failure; windows-host id | screen.read unavailable; browser UNAVAILABLE |
| 5 Computer use | PARTIAL | REAL list/search/clipboard/windows; GUI Notepad isolated under COGNYX_GUI_TEST | browser NOT RUN (Playwright missing) |
| 6 Multi-agent | PARTIAL | e2e_phase6 | in-process only |
| 7 Workspace | PARTIAL | in-memory contract + REAL HostFilesystem under dedicated root | no Linux/macOS hardware FS |
| 8 Shell | PARTIAL | production AgentKernelAdapter; does not execute OS itself | no GPU compositor |
| 9 Workers | PARTIAL | local heartbeat auth | WAN NOT VERIFIED |
| 10 Memory | PARTIAL | delete real; optional JSON persist | default IN-PROCESS |
| 10.5 Plugins | PARTIAL | sample plugin lifecycle | IN-PROCESS, WASM NOT IMPLEMENTED |
| 11 Hardening | PARTIAL | prod+nightly reject; secret redact; doctor honest virt | no installer; cargo-audit missing |
| 12 Validation | PARTIAL | VAL-001 PASS; shell to kernel PASS; windows-host PASS; doctor virt not ok | GUI notepad ignored unless --include-ignored |
| 13 NL planning | PASS | search → open → type; dynamic application_id/window_id; no bash fallback | live GUI proof is Phase 13.5 |
| 13.5 GUI harness | PASS | dedicated workspace, owned HWND, live Golden Test verified Hello CognyxOS | leftover personal Notepad must remain untouched |

Overall: still not a complete OS. Phase 12 makes the connected path honest. PARTIAL as a host capability toolkit plus a real kernel submit path.
