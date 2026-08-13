# Linux Native Providers

**Status:** ARCHITECTURE STUB — NOT HARDWARE TESTED
**Note:** Development is on Windows. Linux providers require a Linux runtime to test.

## Display Server Detection

Linux providers must detect the display environment at runtime:

```rust
fn detect_display_server() -> DisplayServer {
    if env::var("WAYLAND_DISPLAY").is_ok() { return DisplayServer::Wayland; }
    if env::var("DISPLAY").is_ok() { return DisplayServer::X11; }
    DisplayServer::None
}
```

## Planned Provider Stack

| Capability | Primary Backend | Fallback |
|---|---|---|
| screen.capture | Wayland (wl-screenshot) | X11 (xwd/scrot) |
| keyboard.type | AT-SPI (atspi-send) | xdotool (X11 only) |
| mouse.click | AT-SPI | xdotool (X11 only) |
| window.list | AT-SPI + DBus (xdprop) | wmctrl |
| clipboard.read/write | wl-clipboard | xclip/xsel |
| application.list | .desktop entries (/usr/share/applications) | PATH discovery |
| browser.* | Playwright (shared with Windows) | chromiumoxide |

## Accessibility (AT-SPI)

Where AT-SPI2 is available:
- Discover accessible applications via `Atspi.get_desktop(0)`
- Read control names, types, states
- Invoke controls semantically
- Detect enabled/disabled/visible state

Prefer AT-SPI actions over coordinate-based input (same principle as Windows UIA).

## Capability Status

All Linux capabilities return `CAPABILITY_UNAVAILABLE` on non-Linux hosts.
On Linux, capabilities are UNAVAILABLE if the required display server is absent.

## Test Requirements

Linux hardware tests require:
- A Linux host (physical or VM)
- Wayland or X11 display server
- AT-SPI2 installed (for accessibility features)
- Playwright/Node.js for browser tests
