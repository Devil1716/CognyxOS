# ADR 0004: Platform-neutral process identity and capability tokens

Status: Accepted. Date: 2026-07-11.

## Decision

Core code receives only an `ExecutionIdentity` contract: `user`, `process`, `session`, `service`, `integrity_level`, `authentication_evidence`, and a stable opaque `principal_id`. It also receives a `CapabilityToken` contract with issuer, subject, audience, scoped grants, issue/expiry time, nonce, and revocation generation. It never receives Windows SIDs, access tokens, handles, Unix UIDs, or macOS audit tokens.

The Windows adapter derives identity from the process token, session, and service-control context; it maps Windows integrity/elevation to the platform-neutral levels `standard`, `elevated`, `system`. Future Linux and macOS adapters map their native credentials to the same values. The mapping is lossy by design: native authorization remains inside the adapter and raw primitives do not cross IPC or persistence boundaries.

Capability tokens are locally minted, audience-bound, short-lived, non-transferable, and checked against current revocation state on every protected action. Privilege escalation creates a new approved execution context and never mutates an existing identity. Administrative and system-critical operations use the permission workflow from the security contract.

## Consequences

This preserves one authorization API across all deployment targets. Phase 2 must implement the contract and Windows adapter behind tests, while Linux/macOS stay interface-complete placeholders.
