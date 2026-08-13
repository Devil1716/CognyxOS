# Accessibility APIs

CognyxOS Phase 5 prioritizes accessibility-tree-based interaction over
coordinate-based input. This enables:
- More reliable automation (controls are addressed by name, not pixels)
- Accessibility for users with disabilities
- Resilience to UI layout changes

## Interaction Priority

1. **Accessibility tree** (UIA / AT-SPI / AXUIElement) — semantic controls
2. **Application API** (browser DOM via CDP, IDE APIs)
3. **OCR** — text recognition from screenshot (requires OCR provider)
4. **Vision model** — visual element detection (requires vision provider)
5. **Screen coordinates** — pixel-level fallback, always audited
6. **CAPABILITY_UNAVAILABLE** — never fake

## Windows: UI Automation

- API: `IUIAutomation` (COM interface via `windows` crate)
- Discovers: windows, controls, buttons, text fields, menus
- Actions: Invoke, SetValue, Focus, Expand, Select
- Thread model: COM STA — all UIA calls use `spawn_blocking`

## Linux: AT-SPI2

- API: D-Bus accessible-event protocol
- Discovers: GTK, Qt, and other accessible applications
- Actions: activate, set-text, focus
- Requires: at-spi2-core package

## macOS: AXUIElement

- API: Carbon Accessibility API / ApplicationServices
- Discovers: any app that enables NSAccessibility
- Actions: AXPress, AXSetValue, AXFocus
- Requires: TCC Accessibility permission

## screen.read Implementation

`screen.read` never requires a vision model. It returns structured text from the
accessibility tree:

```json
{
  "elements": [
    {"name": "File", "role": "MenuItem", "value": null, "state": "normal"},
    {"name": "document content", "role": "Document", "value": "Hello World"}
  ],
  "text": "File Edit View\nHello World",
  "source": "accessibility_tree"
}
```

If no accessible window exists: `CAPABILITY_UNAVAILABLE`.
