# Platform contracts

This specification is normative for every CognyxOS subsystem. Contracts are technology-neutral; implementations may use Python, Rust, or TypeScript but MUST preserve these semantics.

## Naming and identity

- Use reverse-domain IDs: `org.cognyx.<domain>.<name>` for services, plugins, capabilities, and event types.
- APIs use `lower_snake_case` fields, `PascalCase` type names, RFC 4122 UUIDv7 identifiers, and UTC RFC 3339 timestamps with millisecond precision.
- Contract documents, schemas, and packages use semantic versioning. A contract ID is immutable once published.
- All externally visible messages carry `contract_version` as `major.minor`.

## Compatibility and evolution

Major changes are incompatible; minor changes may add optional fields, event types, enum values only where consumers must tolerate unknown values, and optional capabilities. Patch changes correct prose or schemas without semantic change.

Producers MUST retain a supported major version for the documented deprecation window (minimum two minor releases or 180 days, whichever is longer). Consumers MUST ignore unknown optional fields and preserve correlation metadata. Required-field removal, semantic reinterpretation, ID reuse, and changing a success response to an error are breaking changes.

Schema changes require: an ADR for a major version, compatibility fixtures, migration guidance, a `deprecated_at` date, and an owner. Interfaces use additive methods or a new versioned interface; never alter a published method signature in place.

## Cross-platform boundary

Core modules depend on contracts only. OS-specific functionality is supplied by a platform adapter selected in the composition root: Windows is the reference adapter; Linux and macOS adapters are separately versioned placeholders. Raw OS handles, paths, error codes, credentials, and native API types MUST NOT cross a contract boundary.

## Common envelope

Every message uses the following base envelope. Sensitive metadata and payload fields are classified before persistence or forwarding.

| Field              | Type              | Requirement                               |
| ------------------ | ----------------- | ----------------------------------------- |
| `message_id`       | UUIDv7            | required, globally unique                 |
| `contract_version` | string            | required                                  |
| `correlation_id`   | UUIDv7            | required; shared by a user-initiated flow |
| `causation_id`     | UUIDv7            | optional; preceding message/event         |
| `timestamp`        | RFC 3339 UTC      | required                                  |
| `source`           | reverse-domain ID | required                                  |
| `tenant_scope`     | string            | local profile/workspace scope             |
| `classification`   | `public           | internal                                  | sensitive | restricted` | required |

## Contract governance

The Architecture Council owns published contracts. Each contract has a named maintainer, ADR references, compatibility tests, and a change log. Exceptions require a time-boxed ADR and cannot become implicit precedent.
