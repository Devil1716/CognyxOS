# Browser Automation

**Status:** REAL (requires Playwright/Node.js) — NOT HARDWARE TESTED

## Architecture

```
Universal Browser Capability (browser.*)
  ↓
UniversalBrowserProvider (cognyx-capability)
  ↓
Playwright CLI Backend (Node.js subprocess)
  ↓
Chromium / Firefox / WebKit
```

The provider is backend-independent: future providers (chromiumoxide CDP,
Firefox DevTools Protocol) can be plugged in without changing the
universal capability contract.

## Backend: Playwright CLI

Playwright is invoked via Node.js subprocess scripts.
Each browser capability generates a short Node.js script, executes it, and
parses the JSON output. This avoids requiring a persistent browser daemon.

**Installation:**
```bash
npm install -g playwright
npx playwright install chromium
```

**Detection:** The provider checks for Playwright at first use.
If unavailable: `CAPABILITY_UNAVAILABLE` (no fake result).

## Capabilities

| Capability | Input | Output |
|---|---|---|
| browser.open | `{"url": str}` | `{"session_id": str, "url": str}` |
| browser.navigate | `{"session_id": str, "url": str}` | `{"url": str, "title": str}` |
| browser.read | `{"session_id": str}` | `{"text": str, "title": str, "url": str}` |
| browser.click | `{"session_id": str, "selector": str}` | `{"clicked": bool}` |
| browser.type | `{"session_id": str, "selector": str, "text": str}` | `{"typed": bool}` |
| browser.screenshot | `{"session_id": str}` | `{"image_b64": str, "format": "png"}` |
| browser.close | `{"session_id": str}` | `{"closed": bool}` |
| browser.tabs | `{"session_id": str}` | `{"tabs": [...]}` |

## Security

- Runs headless by default
- Does NOT access saved passwords, cookies, or browser profiles
- Download/upload capabilities require explicit user authorization
- All browser sessions are tracked and auditable
- Filesystem access for downloads respects filesystem capability permissions

## Testing

Browser integration tests use a local test HTTP server (no external network access):
- Tests start a local server on a random port
- All tests run through the universal capability interface
- Tests skip gracefully if Playwright is not installed
