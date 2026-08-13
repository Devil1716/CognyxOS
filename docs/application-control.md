# Application Control

Application control capabilities allow the agent to discover, launch, and
interact with applications through the universal capability layer.

## Discovery

Applications are discovered dynamically from PATH (all platforms).
No applications are hardcoded.

```
application.list → NativeApplicationProvider.discover() → PATH executable scan
```

## Lifecycle Integration

```
application.open → spawn process
  → process.list → find by PID
  → window.list → find by process ID  
  → window.focus → bring to foreground
  → screen.read / window.inspect → accessibility tree
  → keyboard.type / browser.click → interact
  → application.close → terminate
```

## Capabilities

| Capability | Description | Permission |
|---|---|---|
| application.list | List all discoverable applications | Allow |
| application.search | Search by name | Allow |
| application.inspect | Get application metadata | Allow |
| application.open | Launch application | Allow |
| application.close | Terminate application | UserApprovalRequired |
| application.focus | Bring to foreground | Allow |
| application.status | Get running status | Allow |

## Output Fields

```json
{
  "application_id": "app:/path/to/exe",
  "name": "notepad",
  "display_name": "Notepad",
  "executable": "/path/to/notepad.exe",
  "version": null,
  "runtime_id": "host-windows-1",
  "process_id": 12345,
  "window_ids": ["hwnd:67890"],
  "status": "running",
  "capabilities": ["application.open", "application.close"]
}
```
