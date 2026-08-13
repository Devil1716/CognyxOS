# Capability security

The existing Phase 3 `PermissionEngine` remains the authorization authority. The gateway rejects both `DENY` and `USER_APPROVAL_REQUIRED` requests before dispatch. Definitions expose permissions, resource needs, risk, security level, audit policy, and idempotency; audit events are emitted for registration, request, start, completion, failure, timeout, and provider health changes.

Phase 5 adds explicit approval requirements for destructive filesystem mutations, process start/stop, terminal execution, application close, and clipboard access. The native terminal provider only runs a configured executable allowlist and uses an argument vector—not a shell.

```mermaid
flowchart TD
  R[Capability request] --> P{Permission Engine}
  P -->|ALLOW| E[Provider execution]
  P -->|DENY| D[Normalized PERMISSION_DENIED]
  P -->|APPROVAL REQUIRED| A[Normalized USER_APPROVAL_REQUIRED]
  E --> O[Audit and normalized result]
```
