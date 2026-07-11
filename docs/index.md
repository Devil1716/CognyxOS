# CognyxOS Engineering Foundation

CognyxOS is developed first on Windows and deployed later to Linux. Phase 1 establishes reusable infrastructure only: no AI, agents, desktop features, or runtime automation.

## Repository layout

- `apps/` contains user-facing shells: desktop, launcher, settings, and runtime.
- `core/` reserves product-domain modules for later phases.
- `packages/` contains reusable TypeScript and Rust contracts.
- `python/cognyx_runtime/` contains platform-neutral backend infrastructure.
- `configs/` contains environment defaults.
- `tests/` contains end-to-end test scaffolding.

Every operating-system interaction goes through a platform contract. The Windows adapter is the reference implementation; Linux and macOS adapter placeholders prevent coupling to a single host OS.
