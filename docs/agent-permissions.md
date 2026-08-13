# Agent Permissions & Capability Scoping

**Status:** COMPLETE  

Every agent operates under a strict `AgentPolicy` defining its capability scope and permission inheritance model.

## Inheritance Policy: Default DENY

When a parent agent spawns a child agent:
1. The child defaults to `DENY` for all capabilities.
2. The child's capability scope is explicitly defined at spawn time.
3. The system checks `evaluate_permission_inheritance(parent, requested_cap)`:
   - If `parent.capabilities` does NOT contain `requested_cap`, spawning/granting fails with a `PrivilegeEscalation` error.

## Example Role Scopes

| Role | Allowed Capabilities | Explicitly Denied (Default) |
|---|---|---|
| `RESEARCHER` | `browser.read`, `browser.navigate`, `network.request`, `filesystem.read` | `filesystem.delete`, `terminal.execute`, `process.stop` |
| `COMPUTER_OPERATOR` | `application.open`, `screen.capture`, `keyboard.type`, `mouse.click`, `window.*` | `filesystem.delete`, `win32.powershell` |
| `FILE_OPERATOR` | `filesystem.read`, `filesystem.write`, `filesystem.list`, `filesystem.copy` | `terminal.execute`, `browser.navigate` |
| `BROWSER_OPERATOR` | `browser.open`, `browser.navigate`, `browser.read`, `browser.click`, `browser.type`, `browser.screenshot` | `filesystem.delete`, `process.stop` |
| `WRITER` | `filesystem.read`, `filesystem.write`, `doc.render` | `terminal.execute`, `network.request` |
