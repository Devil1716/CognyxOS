# CognyxOS threat model (Phase 11)

Actors and controls. This is a design-time model, not a penetration-test report.

| Threat | Control |
|---|---|
| Malicious agent | Agent Kernel + PermissionEngine; no direct OS APIs |
| Malicious plugin | Declared permissions, scopes, quotas, no user-grant inheritance |
| Compromised runtime | RuntimeRegistry isolation; workspace checksums |
| Compromised worker | Token auth, TLS required, duplicate destructive guard |
| Malicious application/document | Capability gateway; approval for write/delete/execute |
| Prompt / tool / browser injection | Intents parsed by IntentEngine; tools only via gateway |
| Filesystem / network attack | Scoped workspace FS; plugin network allowlists |
| Credential theft | SecretStore; redact; no secrets in memory/logs/backups |
| Privilege escalation | No workspace/plugin bypass of PermissionEngine |
| Supply-chain | Plugin checksum verify, rollback, release channels |

Failed operations must never be reported as success.
