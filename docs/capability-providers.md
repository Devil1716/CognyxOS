# Capability providers

A `CapabilityProvider` declares its provider and runtime ids, priority, definitions, health, and async execution. `CapabilityProviderContext` contains a normalized request and selected runtime. The layer selects healthy providers by priority and can advance past retryable provider errors.

`AdapterProvider` is the contract provider for Linux, Windows, macOS, and containers. `LocalFilesystemProvider` is the only Phase 4 provider that currently performs real I/O; it confines every path to its explicitly configured root.
