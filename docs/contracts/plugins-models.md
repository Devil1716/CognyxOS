# Plugin and model provider specifications

## Plugins

Plugin manifests are signed, versioned documents containing `plugin_id`, version, compatible core/API range, publisher, entry points, capability descriptors, dependencies with version ranges, requested permissions, configuration schema, integrity hash, and signature. Installation verifies signature, hash, dependency graph, compatibility, and user approval before activation. Updates stage in a separate directory, validate the same checks, run migration only with explicit approval, then atomically switch; failed updates roll back.

Lifecycle: `discovered → verified → installed → configured → initialized → active → draining → stopped → removed`. Plugins are sandboxed with least-privilege capability tokens, scoped filesystem storage, per-plugin configuration, resource limits, and no direct platform/secret access. They publish/subscribe only to declared, authorized event types. Unsigned plugins are prohibited in production; development mode requires a clearly recorded local override. Compatibility follows the platform contract policy.

## Model providers

Providers (Ollama, llama.cpp, vLLM, OpenVINO, ONNX Runtime, and future adapters) implement one contract: discovery, capability report, load, unload, inference, streaming, cancellation, health, and local telemetry. Provider-specific options are held in a validated opaque `provider_options` object and may not alter common response semantics.

A model descriptor contains provider/model IDs, version, format, local path reference, supported capabilities, context limits, hardware requirements, and content classification. Inference accepts a request ID, capability, normalized input, parameters, deadline, cancellation token, and correlation metadata. Streaming emits ordered chunks and exactly one terminal status. Load/unload are idempotent. Health reports readiness/degraded/unavailable and never sends telemetry off-device; telemetry is local, opt-in diagnostic data only. Model bytes and prompts are never placed in event payloads or normal logs.
