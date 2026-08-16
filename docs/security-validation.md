# Security Validation

Passed:

- filesystem.delete without grant: USER_APPROVAL_REQUIRED
- path-traversal target on filesystem.delete: not success (approval, not a proven canonicalizer)
- clipboard.read without grant: USER_APPROVAL_REQUIRED
- window.teleport: DENIED / not success
- screen.read: CAPABILITY_UNAVAILABLE, not a fake screenshot
- VAL-001: bash with empty registry returns success:false and CAPABILITY_UNAVAILABLE (fixed Phase 12)
- worker heartbeat wrong token: AuthenticationFailure
- memory retrieve owner-scoped; delete removes the record
- disabled plugin cannot execute
- production plus nightly channel rejected
- secret redact of the live secret errors
- HostFilesystem refuses parent-dir components and paths outside the dedicated workspace root

Failed / dangerous / residual:

- application.open is Allow by default
- keyboard.type is Allow by default
- Doctor virtualization is honest (ok: false when not verified) but virt is not healthy
- Path traversal on real disk is approval-gated, not proven canonicalized for every provider

Not tested: Wasm escape, remote worker auth, cargo-audit, signed updates.
