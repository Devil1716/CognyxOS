# Computer Control Capabilities

CognyxOS Phase 5 provides real computer-use capabilities through the universal
capability layer. Every operation passes through the Permission Engine and
Capability Gateway.

## Capability Hierarchy

### Interaction Priority (Always follow this order)

1. **Accessibility APIs** (Windows UI Automation, AT-SPI, macOS AXUIElement)
   — Semantic interaction with named controls. No coordinates.
2. **Application APIs** — Application-specific automation (browser DOM via CDP)
3. **OS-level input simulation** — SendInput (Windows), xdotool (Linux)
   — Only when no semantic mechanism exists. Always audited.
4. **CAPABILITY_UNAVAILABLE** — Never fake a result.

## Key Capabilities

### screen.capture
Captures the display as a PNG image.
- Windows: GDI BitBlt (primary monitor)
- Linux: wl-screenshot (Wayland) or xwd (X11)
- macOS: ScreenCaptureKit
- Output: `{"image_b64": str, "width": int, "height": int, "format": "png", "timestamp_ms": int}`

### screen.read
Reads screen content as structured text without requiring OCR or vision model.
- Implementation hierarchy:
  1. Accessibility tree (UIA/AT-SPI/AXUIElement)
  2. Application text API
  3. OCR provider (if configured)
  4. Vision model provider (if configured)
  5. CAPABILITY_UNAVAILABLE

### keyboard.type
Types unicode text into the focused control.
- Windows: KEYEVENTF_UNICODE via SendInput
- Linux: AT-SPI SetValue or xdotool type
- macOS: CGEventCreateKeyboardEvent

### keyboard.press
Presses a named key (Enter, Tab, F1, etc.).
- Maps key names to platform virtual key codes

### mouse.click / mouse.move
Coordinate-based input. Always audited. Prefer accessibility actions.

## Application Integration Flow

```
application.open
  → process discovered (process.list)
  → window discovered (window.list)
  → window.focus
  → UI tree available (screen.read / window.inspect)
  → keyboard.type / mouse.click / button.invoke
```

## Permission Requirements

| Capability | Default Permission |
|---|---|
| screen.capture | Allow |
| keyboard.type | Allow |
| mouse.click | Allow |
| window.focus | Allow |
| window.close | UserApprovalRequired |
| clipboard.read | UserApprovalRequired |
| clipboard.write | UserApprovalRequired |
| application.close | UserApprovalRequired |
