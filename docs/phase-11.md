# Phase 11: Production hardening + release

**Status:** IMPLEMENTED (hardening crate + doctor binary; not a full distro installer)  
**Last Updated:** 2026-08-14

## What landed

- `SecretStore` rejects secrets in logs/history/backups
- Validated config for development/testing/staging/production
- Release channels: development, nightly, beta, stable
- Backup/restore that refuses plaintext secrets
- `cognyx doctor` diagnostics (host, virt, workspace, memory, plugins, security)
- Update + rollback that will not apply a failed health check
- First-boot step list
- Health aggregation

## Threat model (summary)

See `docs/threat-model.md`. Covered: malicious agent/plugin, compromised
runtime/worker, prompt/tool/browser injection, filesystem/network/credential
theft, privilege escalation, supply chain. Controls: least privilege,
capability gateway, plugin scopes, worker auth, secret isolation.

## What this phase does not claim

- Not a signed OS installer for physical machines
- Not hardware-tested GPU/VM isolation
- `cargo audit` may be unavailable; report it if missing
- No production simulated providers were added

## Next

Full system test (TEST 1–10) against the real kernel/shell path where
hardware allows. Do not mark RELEASE READY until those pass on real
runtimes.
