# Phase 13: natural-language execution planning

Phase 13 makes deterministic application requests executable through the existing Agent Kernel path.  The intent engine extracts an application entity and ordered actions; the planner emits one capability node per action; and the kernel validates then executes the dependency graph through the Capability Gateway.

No planner invokes the host. Runtime selection, permission decisions, and provider execution remain owned by RuntimeRegistry, PermissionEngine, and CapabilityGateway respectively.

Current real-provider behavior: `application.open` waits for a visible process window on Windows, focuses it, and returns `window_id` when one appears. Keyboard input follows open. Close plans bind `${step-2.window_id}` into `window.close`.

Phase 13.5 adds an opt-in GUI test mode (`COGNYX_GUI_TEST=1`) so live verification targets only `C:\CognyxOSTestWorkspace\`. See `docs/phase-13-5.md` and `docs/windows-gui-testing.md`. The planner is unchanged.
