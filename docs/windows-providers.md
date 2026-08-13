# Windows Native Providers

**Status:** REAL, NOT-HARDWARE-TESTED (Phase 5A)

All Windows providers are gated with `#[cfg(target_os = "windows")]` and use
real Win32 APIs via the `windows` Rust crate.

## Provider Overview

### WindowsScreenCaptureProvider
- **Capability:** `screen.capture`
- **API:** Win32 GDI (BitBlt, GetDC, GetDIBits)
- **Status:** REAL
- **Input:** `{}` (no parameters required for primary monitor)
- **Output:** `{"image_b64": string, "width": int, "height": int, "format": "png", "timestamp_ms": int}`
- **Permission:** screen.capture (Allow by default)

### WindowsWindowProvider
- **Capability:** window.list, window.inspect, window.focus, window.close, window.minimize, window.maximize, window.move, window.resize, window.activate
- **API:** Win32 (EnumWindows, ShowWindow, SetForegroundWindow, SetWindowPos, PostMessage)
- **Status:** REAL
- **Notes:** UI Automation preferred over coordinate interaction

### WindowsKeyboardProvider
- **Capability:** keyboard.type, keyboard.press, keyboard.hotkey
- **API:** Win32 SendInput (KEYEVENTF_UNICODE for text, VK_* for keys)
- **Status:** REAL
- **Permission:** keyboard.type/press/hotkey (Allow by default)
- **Security:** All keystrokes are audited. No kernel-level injection.

### WindowsMouseProvider
- **Capability:** mouse.move, mouse.click, mouse.double_click, mouse.right_click, mouse.scroll
- **API:** Win32 SendInput (MOUSEEVENTF_*)
- **Status:** REAL
- **Notes:** Coordinate-based fallback. UIA preferred where available.
- **Audit:** All coordinate-based actions emit `mouse.coordinate_input` side effect

### WindowsClipboardProvider
- **Capability:** clipboard.read, clipboard.write
- **API:** PowerShell Get-Clipboard / Set-Clipboard
- **Status:** REAL
- **Permission:** Both require explicit grant (UserApprovalRequired by default)

## Registration

All providers are registered in `CapabilityGateway::new()` under `#[cfg(target_os = "windows")]`.
Runtime ID: `host-windows-1`

## Security Notes

- `keyboard.type` sends unicode characters via `KEYEVENTF_UNICODE` (no raw VK injection for text)
- `mouse.click` at coordinates is auditable via side_effects
- Screen capture uses GDI (accessible to any process with DISPLAY access)
- No UI Automation pattern bypass — all operations go through PermissionEngine
