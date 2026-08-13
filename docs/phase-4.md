# Phase 4: Universal Capability Layer

Phase 4 introduces `cognyx-capability`, a runtime-neutral execution boundary below the existing Capability Gateway. Phase 1–3 interfaces remain unchanged: the gateway still authorizes requests, RuntimeRegistry remains the runtime source of truth, and the Agent Kernel never selects an OS adapter.

```mermaid
flowchart TD
  K[Agent Kernel] --> G[Existing Capability Gateway]
  G --> P[Permission Engine]
  G --> U[Universal Capability Layer]
  U --> R[Capability Registry]
  R --> L[Linux adapter]
  R --> W[Windows adapter]
  R --> M[macOS adapter]
  R --> C[Container adapter]
  L & W & M & C --> N[Normalized CapabilityResult]
```

Provider resolution uses a compatible capability version, runtime hint, health state, and provider priority. The gateway only uses the universal layer for registered Phase 4 names; legacy Phase 1–3 capability handling remains frozen.

```mermaid
sequenceDiagram
  participant K as Agent Kernel
  participant G as Capability Gateway
  participant P as Permission Engine
  participant U as Capability Layer
  participant A as OS Adapter
  K->>G: CapabilityRequest
  G->>P: authorize
  P-->>G: ALLOW / DENY / APPROVAL_REQUIRED
  G->>U: authorized universal request
  U->>A: selected provider
  A-->>U: native or simulated result
  U-->>G: normalized result
```

See the focused documents in this directory for contracts, providers, security, and versioning.
