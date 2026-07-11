# ADR 0002: Versioned protobuf contracts generated with Buf

Status: Accepted. Date: 2026-07-11.

## Context

The runtime needs one language-neutral service contract for Rust, Python, and TypeScript. Two viable approaches were assessed.

| Approach                                           | Benefits                                                                        | Costs                                                                                |
| -------------------------------------------------- | ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| Direct `protoc` invoked independently per language | familiar and minimal tooling                                                    | divergent plugins, non-deterministic developer setup, weak compatibility enforcement |
| Buf-managed protobuf modules and generation        | deterministic generation, linting, breaking-change checks, remote/local plugins | one additional tool and configuration                                                |

## Decision

Use Buf as the contract toolchain. Source files live under `protocols/cognyx/<domain>/v1/*.proto`; each package is `cognyx.<domain>.v1`. Generated code is ignored by Git and emitted to `generated/{rust,python,typescript}` by `buf generate`. Generated code is never hand-edited and is not imported across domain boundaries except through published language adapter packages.

The root `buf.yaml` defines one workspace and `buf.gen.yaml` pins generator versions. The build command will invoke `buf lint`, `buf breaking --against <main>`, and `buf generate` before compiling service adapters. This is a Phase 2 build integration task, not a runtime implementation task.

## Compatibility policy

Packages receive a new `vN` directory for wire-incompatible changes. Within `v1`, fields are only added with new positive field numbers; field numbers and enum values are never reused; removed fields are reserved; required fields are prohibited; `oneof` changes require compatibility review. Consumers must ignore unknown fields. Deprecated fields/methods are annotated and retained for at least two minor releases or 180 days. CI blocks lint or breaking changes absent a major package bump and ADR.

## Consequences

Contracts remain reviewable and compatible before services exist. Buf becomes a documented bootstrap prerequisite for contributors who change protocol sources; ordinary application development consumes generated packages only.
