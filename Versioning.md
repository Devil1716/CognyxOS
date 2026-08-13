# CognyxOS Versioning Policy

> **Document ID:** DEV-004
> **Version:** 1.0.0
> **Status:** Phase 0 - Approved
> **Last Updated:** 2026-08-01
> **Owner:** Release Engineering Team

---

## Table of Contents

1. [Versioning Strategy Overview](#versioning-strategy-overview)
2. [Semantic Versioning (SemVer) 2.0 Compliance](#semantic-versioning-semver-20-compliance)
3. [OS Release Versioning](#os-release-versioning)
4. [API Versioning](#api-versioning)
5. [Protocol Buffer Schema Versioning](#protocol-buffer-schema-versioning)
6. [Capability Versioning](#capability-versioning)
7. [SDK & Library Versioning](#sdk--library-versioning)
8. [Release Channels](#release-channels)
9. [Deprecation Policy](#deprecation-policy)
10. [Backward Compatibility Guarantees](#backward-compatibility-guarantees)
11. [Concrete Versioning Examples](#concrete-versioning-examples)

---

## Versioning Strategy Overview

CognyxOS uses **Semantic Versioning 2.0** as the foundation, extended with explicit rules for:
- APIs (gRPC/REST/GraphQL)
- Protocol Buffer messages
- Capability tokens
- SDK/library releases
- Workspace data format compatibility

The golden rule: **If an upgrade breaks a documented public contract without providing at least two minor versions of deprecation warnings, it is a MAJOR version bump. No exceptions.**

---

## Semantic Versioning (SemVer) 2.0 Compliance

All releases follow `MAJOR.MINOR.PATCH` strictly:

| Bump | When | Example |
|------|------|---------|
| **MAJOR** | Any breaking change to any public contract, OR when removing previously-deprecated functionality. | 1.2.3 → 2.0.0 |
| **MINOR** | New functionality added in backward-compatible manner. Existing functionality deprecated but still works. Default for scheduled feature releases. | 1.2.3 → 1.3.0 |
| **PATCH** | Backward-compatible bug fixes, security patches only. No new features. No behavior changes beyond the bug fix. Security patches may be force-applied. | 1.2.3 → 1.2.4 |

### Pre-Release Labels

- `0.x.y` = Pre-GA. SemVer is relaxed but breaking changes bump MINOR in 0.x series.
- Suffixes in order: `0.1.0-alpha.1` → `0.1.0-beta.2` → `0.1.0-rc.1` → `0.1.0`
- Alpha: Feature complete is aspirational. Not even dogfood.
- Beta: Dogfoodable internally; features may still be cut.
- RC: Barring critical bugs, RC = release. Only security/crash fixes between RC and stable.

---

## OS Release Versioning

The full OS version (displayed in UI, kernel info, ISO filename) follows:

```
{SemVer}+{build.metadata}
```

Examples:
```
0.1.0+alpha.1.framework.12345     # Phase 1 MVP, alpha build 12345
0.3.0+socrates.20260801.3a4f8b2c   # Named release Socrates, 2026-08-01 build, commit 3a4f
1.0.0+descartes.gold               # GA 1.0 Gold release
```

### Build Metadata Fields
- Optional channel tag: `nightly`, `beta`, `stable`, `lts`
- Git commit short SHA (7 chars)
- Build date (YYYYMMDD)
- Build pipeline ID

---

## API Versioning

### gRPC Services (Canonical APIs)

Package = `cognyx.{module}.v{major}`. Example:
```protobuf
package cognyx.workspace.v1;   // Breaking changes → cognyx.workspace.v2
service WorkspaceService { ... }
```

**Rules:**
- A v2 service MAY run alongside v1 indefinitely.
- Clients connecting to `/cognyx.workspace.v1.WorkspaceService` are guaranteed stable for the MAJOR lifetime of v1.
- v1 and v2 share server implementation; gateway translates between them on edge.

### REST APIs

`/api/v{major}/{path}`. Example:
```
/api/v1/workspaces        # Works for entire v1.x lifetime
/api/v2/workspaces        # Introduced when v2 breaks contract
```

### GraphQL

Schema changes are additive only for MINOR bumps. BREAKING changes require new server endpoint `/graphql/v2` and dual-run support for ≥ 2 MINOR release cycles.

---

## Protocol Buffer Schema Versioning

Proto files are THE source of truth. Rules are NON-NEGOTIABLE.

### Allowed Changes (MINOR / PATCH compatible)

1. Adding new fields at new field numbers.
2. Marking fields as deprecated (never removing them!).
3. Adding optional `oneof` members at new indices.
4. Adding new enum values at the END of an enum declaration.
5. Adding new services / RPCs.
6. Relaxing constraint validation (never tightening).

### FORBIDDEN Changes (MAJOR-VERSION-ONLY, if ever)

1. **REUSING FIELD NUMBERS.** Ever. Even if a field was "never used in prod".
2. Removing fields.
3. Changing field types (e.g. `int32` → `int64`, or `string` → `bytes`).
4. Changing message names.
5. Reordering existing enum values or renaming them.
6. Adding new required fields to existing messages (fields MUST be optional or have sane defaults).
7. Tightening validation (e.g. reducing string max_length, adding regex that rejects existing values).

### Reserved Fields Enforcement

When deprecating a field, you MUST mark reserved in proto:

```protobuf
message Foo {
  reserved 2, 15, 9 to 11;
  reserved "old_name", "deprecated_field";

  string new_name = 3;   // NEW field at NEW number
}
```

CI protolock plugin blocks PRs violating any of these rules.

---

## Capability Versioning

Capabilities are part of the public contract; their format affects security.

```
Capability identifier format:
{namespace}.{operation}.v{major}
    ↓
filesystem.delete.v1
network.outbound.v1
```

Rules:
- Capability names (namespace + operation) are STABLE for a MAJOR version.
- If semantics of an operation change in breaking way, a NEW capability is introduced: `filesystem.delete.v2`.
- Old capability continues to function, with deprecation warnings logged, for minimum 2 MAJOR versions after introduction of v2.
- Policy engine maps v1→v2 behavior transparently where safe.

---

## SDK & Library Versioning

### Crate (Rust), npm Package (TS), PyPI (Python), etc.

Each individual package follows its OWN SemVer, independent of OS release version.

Breaking changes in an SDK bump that package's MAJOR. Example:
- `cognyx-plugin-sdk 0.3.0` introduces breaking manifest format change → bump to `cognyx-plugin-sdk 1.0.0`.

Compatibility matrix is maintained in `/docs/guides/sdk-compatibility-matrix.md`.

---

## Release Channels

| Channel | Version Cadence | Audience | Stability |
|---------|----------------|----------|-----------|
| **Nightly** | Every commit to `main` that passes CI | Core developers, testers | ❌ Can be broken, no upgrade path |
| **Alpha** | Every 2 weeks during active phase | Dogfooding engineers, early adopters | ⚠️ Data loss possible; expect migrations |
| **Beta** | Every 4 weeks, feature freeze last week | External enthusiasts | ✅ APIs largely stable; migration scripts provided |
| **Release Candidate (RC)** | Every milestone; one or more RCs per MINOR | Enterprise pilot, QA | ✅ No planned changes; only critical fixes |
| **Stable** | Every 6 weeks (MINOR) + as-needed PATCH | General users | ✅ Production ready |
| **LTS** | Annually; 5-year support window | Enterprise customers | ✅ Security + critical bugfixes only; no features |

### Channel Promotion

```
Nightly → Alpha → Beta → RC → Stable → LTS
```
No skipping. A build must bake in each channel for minimum N days with zero new HIGH severity regressions.

---

## Deprecation Policy

### The Three-Cycle Rule

When deprecating a public API/capability/proto field in version `X.Y.0`:
1. Cycle 1 (`X.Y.0`): Feature marked `@deprecated`. Still works. WARNING-level log on every use. UI shows deprecation banner.
2. Cycle 2 (`X.(Y+1).0`): Warning escalates to ERROR in log for new usages; existing usages still work with WARNING. Migration tool provided.
3. Cycle 3 (`X.(Y+2).0`): HARD ERROR. Attempted use fails explicitly. Removal in NEXT MAJOR.

The feature is physically REMOVED only at the next MAJOR version bump (so typically 3 MINOR cycles + 1 MAJOR = minimum 4 cycles heads-up).

### Deprecation Annotation Standards

**Rust:**
```rust
#[deprecated(since = "1.2.0", note = "Use WorkspaceService::GetWorkspaceV2 instead. See migration-guide-v1-to-v2.md")]
pub fn get_workspace(id: Uuid) -> Result<Workspace> { ... }
```

**Protobuf:**
```protobuf
rpc GetWorkspace(GetWorkspaceRequest) returns (Workspace) {
  option deprecated = true;
  option (cognyx.options).deprecation_replacement = "GetWorkspaceV2";
  option (cognyx.options).removal_version = "2.0.0";
}
```

**TypeScript:**
```typescript
/**
 * @deprecated since 1.2.0 - use getWorkspaceV2 instead
 * @see {@link https://docs.cognyxos.dev/migration/v1-to-v2}
 * @removal 2.0.0
 */
export function getWorkspace(id: WorkspaceId): Promise<Result<Workspace, Error>>
```

---

## Backward Compatibility Guarantees

### We Guarantee Backward Compatibility For:

| Area | Minimum Guaranteed Period |
|------|---------------------------|
| Stable gRPC vN API | 3 years from release of vN+1 |
| Stable REST vN API | 3 years from release of vN+1 |
| Protobuf messages with field #N | Permanent (reserved forever) |
| LTS release bug + security fixes | 5 years from LTS release |
| Plugin SDK vN (Rust/TS/Python) | 18 months after release of vN+1 |
| Capability identifiers `*.vN` | 2 MAJOR OS versions |

### We DO NOT Guarantee Backward Compatibility For:

| Area | Contract |
|------|----------|
| Nightly / Alpha builds | No guarantees. Data loss expected. |
| Internal APIs (any marked `@internal` or in `internal/` modules) | May change without notice. |
| CLI commands not listed in stable docs | Subject to change. |
| Performance characteristics | May change between MINOR releases (faster is always allowed; slower requires justifying doc). |
| Beta SDK pre-1.0 | Best-effort migration notes only. |

---

## Concrete Versioning Examples

### Scenario 1: Add new ListWorkspaces filtered query (non-breaking)
- `cognyx.workspace.v1` → no bump. Add new RPC `ListWorkspacesFiltered`.
- OS: 1.3.2 → **1.4.0** (new feature = MINOR)

### Scenario 2: Change `Workspace.name` maximum length from 255 to 1024 chars (relaxing validation)
- OS: **1.3.2 → 1.3.3** (PATCH-compatible; no code change required for clients)

### Scenario 3: Change `Workspace.name` maximum length from 255 to 64 chars (tightening validation)
- **BREAKS existing clients with longer names.**
- Proto: Add `WorkspaceV2` message, add `CreateWorkspaceV2` RPC in same package or new v2 package.
- OS: **1.3.2 → 1.4.0** + v1 deprecation log, migration tool provided.

### Scenario 4: Remove deprecated `OldCreateWorkspace` (3 cycles ago deprecated)
- OS: **1.x → 2.0.0** (MAJOR bump only)

### Scenario 5: Critical security vulnerability in seccomp default profile
- Backport fix to: current stable (new PATCH), previous 2 MINORs (new PATCH), and active LTS branches.
- Versions: 1.4.2 → **1.4.3**, 1.3.5 → **1.3.6**, 1.2.8 → **1.2.9**, LTS 1.0.12 → **1.0.13**
