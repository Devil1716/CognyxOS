# macOS Native Providers

**Status:** ARCHITECTURE STUB — NO MAC HARDWARE AVAILABLE
**Note:** macOS providers compile but return CAPABILITY_UNAVAILABLE on non-macOS hosts.

## Planned APIs

| Capability | API |
|---|---|
| application.list/open | NSWorkspace.shared |
| window.list/focus | Accessibility API (AXUIElement) |
| screen.capture | ScreenCaptureKit (macOS 12.3+) |
| keyboard.type | CGEventCreateKeyboardEvent |
| mouse.click | CGEventCreateMouseEvent |
| clipboard.read/write | NSPasteboard.general |
| process.list | NSRunningApplication |

## Remote Mac Support

The existing Phase 2 RemoteMacBackend can be reused. The macOS capability provider
should support both:
- `LocalMacBackend` (running directly on macOS)
- `RemoteMacBackend` (Phase 2 remote execution channel)

If no Mac runtime is configured: CAPABILITY_UNAVAILABLE.

## Accessibility Requirements

macOS Accessibility API requires TCC (Transparency, Consent, Control) permissions.
The app must be granted Accessibility access in System Preferences → Privacy & Security.

## Test Requirements

- Physical Mac or macOS VM
- macOS 12.3+ (for ScreenCaptureKit)
- TCC Accessibility permission granted
- Playwright for browser tests
