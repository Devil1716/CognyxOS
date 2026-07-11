# Permissions, configuration, logging, and errors

## Permission framework

Risk levels are `safe`, `confirmation_required`, `administrative_approval`, and `system_critical`. A grant binds subject, capability, resource scope, constraints, issuer, expiry, session binding, and audit ID. Child operations may inherit only equal-or-narrower scope; no capability can escalate through composition. Temporary grants expire automatically, session grants die with the session, and revocation is immediate and checked before every use.

Approval requests show actor, capability, resource scope, consequences, expiry, and rollback availability. Administrative and system-critical approvals require an authenticated local administrator; system-critical operations additionally require a second explicit confirmation. All requests, decisions, use, expiry, and revocations are append-only audit events.

## Configuration standard

Configuration is versioned YAML or JSON, validated against JSON Schema, and layered in this order: built-in defaults → global → user → workspace → plugin → runtime → environment overrides. Later layers override only documented keys. Secrets are references to the OS secret store, never literal values. Each document has `config_version`; migrations are deterministic, reversible when possible, previewable, and backed up before application. Invalid configuration fails closed with field-level diagnostics.

## Logging standard

Logs are structured JSON with `timestamp`, `level`, `event`, `message`, `service_id`, `instance_id`, `correlation_id`, optional `trace_id`, `session_id`, `request_id`, duration/metric fields, and classification. Levels are `DEBUG`, `INFO`, `WARNING`, `ERROR`, `CRITICAL`. Security and audit events are separate append-only streams. Secrets, raw credentials, raw audio, private prompts, and unrestricted file content are forbidden in logs. Default retention: operational 30 days, security/audit 365 days; retention is configurable subject to policy. Trace propagation follows the common envelope.

## Error taxonomy and recovery

Errors use `{code, category, retryable, message, details, correlation_id}`. Categories: `SYSTEM`, `RUNTIME`, `AGENT`, `PLUGIN`, `TOOL`, `PERMISSION`, `MODEL`, `NETWORK`, `VALIDATION`, and `CONFLICT`. Codes are stable uppercase identifiers, e.g. `PERMISSION_DENIED`, `MODEL_UNAVAILABLE`, `PLUGIN_INCOMPATIBLE`, `DEADLINE_EXCEEDED`.

Recovery policy: validation and permission errors do not retry; network/resource errors may retry with bounded backoff; conflicts reload and reconcile; model/plugin/tool failures isolate the component and invoke declared compensation; system-critical failures transition the supervisor to safe shutdown. Clients receive sanitized details; full diagnostics remain local and access-controlled.
