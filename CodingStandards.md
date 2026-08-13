# CognyxOS Coding Standards

> **Document ID:** DEV-001
> **Version:** 1.0.0
> **Status:** Phase 0 - Approved
> **Last Updated:** 2026-08-01
> **Owner:** Engineering Standards Team

---

## Table of Contents

1. [General Principles](#general-principles)
2. [Rust Standards](#rust-standards)
3. [TypeScript Standards](#typescript-standards)
4. [Python Standards](#python-standards)
5. [C++ Standards](#c-standards)
6. [Markdown Documentation Standards](#markdown-documentation-standards)
7. [Testing Standards](#testing-standards)
8. [Logging Standards](#logging-standards)
9. [Naming Conventions](#naming-conventions)
10. [Error Handling](#error-handling)
11. [Dependency Management](#dependency-management)
12. [Versioning & Releases](#versioning--releases)

---

## General Principles

These apply to ALL languages in the CognyxOS codebase.

### Tenets

1. **Correctness > Performance > Readability > Brevity**
   Code is read 100x more often than written. Performance only matters when profiled. Correctness is non-negotiable.

2. **No Surprises (Principle of Least Astonishment)**
   Functions, classes, modules behave EXACTLY as their name and signature promise. Side effects must be documented explicitly.

3. **Explicit is Better Than Implicit**
   No magic. No global state. No implicit dependencies. Pass parameters explicitly.

4. **Secure by Default**
   The insecure option is never the default. The code path the developer takes without thinking is the secure code path.

5. **Observable by Construction**
   No module reaches production without: tracing, structured logs, metrics, and health checks.

---

## 2. Rust Standards

### Toolchain

| Tool | Enforced | Configuration |
|------|----------|---------------|
| Rust Version | 1.80 MSRV | rust-toolchain.toml pinned |
| Edition | 2021 | Cargo.toml `edition = "2021"` |
| Clippy | CI: `deny(warnings)` | `.cargo/config.toml` clippy cfg |
| Rustfmt | CI enforced | `rustfmt.toml` committed |
| Miri | UB tests | CI on critical paths only |
| Cargo-vet | Supply chain | `supply-chain/` audits committed |
| Cargo-audit | CVEs | CI deny ≥ HIGH severity |

### Code Style

```rust
// ✅ Modules: snake_case, types: CamelCase, consts: SCREAMING_SNAKE_CASE
pub mod workspace_manager;

pub struct WorkspaceHandle {
    // Private fields by default. Public getters only.
    id: Uuid,
    state: WorkspaceState,
}

// ✅ Error enums: thiserror, no anyhow in library code
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("workspace {0} not found")]
    NotFound(Uuid),

    #[error("workspace {id} in state {state:?}, cannot activate")]
    InvalidStateTransition { id: Uuid, state: WorkspaceState },

    #[error("i/o error")]
    Io(#[from] std::io::Error),
}

// ✅ Result type alias per crate
pub type Result<T> = std::result::Result<T, WorkspaceError>;

// ✅ Fallible constructors are ::try_new(), never ::new() which panics
impl WorkspaceHandle {
    pub async fn try_new(config: WorkspaceConfig) -> Result<Self> { /* ... */ }
}

// ✅ Async: use async_trait sparingly; prefer static dispatch where possible
#[async_trait]
pub trait RuntimeHost: Send + Sync + 'static {
    async fn start(&self, spec: WorkloadSpec) -> Result<WorkloadHandle>;
}
```

### Unsafe Policy

- **ZERO `unsafe` blocks in production crates** are allowed without a formal safety proof + 2 reviewer signoffs + written comment with:
  1. Preconditions for `unsafe`
  2. Why the author believes it's sound
  3. Why `unsafe` cannot be avoided

### Crate Structure Guidelines

```
crate-name/
├── src/
│   ├── lib.rs          // Public API surface, pub re-exports
│   ├── error.rs        // Error enum (thiserror)
│   ├── types.rs        // Core data types (no logic)
│   ├── service.rs      // gRPC/API implementation
│   ├── backend/        // Implementation
│   │   ├── mod.rs
│   │   ├── trait.rs    // Trait definitions
│   │   └── impl_*.rs   // Implementations (one file per strategy)
│   └── test_util.rs    // Test helpers (feature flag: test-util)
├── tests/              // Integration tests
├── benches/            // Criterion benchmarks
├── Cargo.toml
└── README.md
```

### Dependency Rules

1. Prefer `std` + `tokio` + the crates in `/Cargo.toml [workspace.dependencies]`
2. No adding a new crate to workspace without: 10k+ GH stars, maintained, no `unsafe` audit passes
3. Avoid:
   - `anyhow` in libraries (use concrete error types)
   - `serde_json::Value` (use strong types unless truly dynamic)
   - Proc macros that obscure control flow

---

## 3. TypeScript Standards

### Toolchain

| Tool | Version | Purpose |
|------|---------|---------|
| TypeScript | 5.5+ | Language strict mode |
| pnpm | 9+ | Package manager (workspace mode, no npm/yarn allowed) |
| TurboRepo | 2+ | Build orchestration |
| ESLint | Flat Config | `eslint.config.js` strict |
| Prettier | 3+ | Formatting |
| Vitest | 2+ | Test runner + coverage |
| Vite | 5+ | Bundler for UI packages |
| tsc-strict | Strictest | `strict: true`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes` |

### tsconfig.base.json

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "exactOptionalPropertyTypes": true,
    "noImplicitOverride": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "verbatimModuleSyntax": true,
    "skipLibCheck": false
  }
}
```

### Code Style

```typescript
// ✅ Discriminated unions for state, not any/unknown hacks
type WorkspaceState =
  | { status: "inactive" }
  | { status: "activating"; startedAt: Date }
  | { status: "active"; handle: WorkspaceHandle }
  | { status: "error"; error: AppError };

// ✅ Branded types for IDs (avoid stringly-typed)
declare const WorkspaceIdBrand: unique symbol;
export type WorkspaceId = string & { readonly __brand: typeof WorkspaceIdBrand };
export function WorkspaceId(value: string): WorkspaceId {
  if (!UUID_REGEX.test(value)) throw new TypeError("Invalid WorkspaceId");
  return value as WorkspaceId;
}

// ✅ Async functions return Result<T, E> via neverthrow, not throw
import { ok, err, Result } from "neverthrow";

async function activateWorkspace(id: WorkspaceId): Promise<Result<WorkspaceHandle, WorkspaceError>> {
  const resp = await apiClient.post(..., { id });
  if (!resp.ok) return err(mapError(resp));
  return ok(resp.data);
}

// ✅ No direct null checks. Use Result / Option (from neverthrow or purify)
```

### React Components (UI Layer)

```tsx
// ✅ Composition over HOCs / render props / magic hooks
type Props = Readonly<{
  workspace: Workspace;
  onActivate: (id: WorkspaceId) => Promise<void>;
  isLoading?: boolean;
  "aria-label"?: string;
}>;

// ✅ Server components by default; "use client" explicitly where needed
// ✅ Small, focused components; >200 lines = refactor
export function WorkspaceCard({ workspace, onActivate, isLoading, ...rest }: Props) {
  // ✅ Destructure early; never re-read props
  // ✅ Use Tailwind; custom CSS only if utility impossible
  return (
    <div
      role="button"
      tabIndex={0}
      onClick={() => !isLoading && onActivate(workspace.id)}
      onKeyDown={(e) => (e.key === "Enter" && !isLoading && onActivate(workspace.id))}
      className="rounded-lg border p-4 transition hover:bg-neutral-100 dark:hover:bg-neutral-800 disabled:opacity-50"
      aria-busy={isLoading}
      {...rest}
    >
      <h3 className="font-semibold">{workspace.name}</h3>
      <StatusBadge state={workspace.state} />
    </div>
  );
}
```

---

## 4. Python Standards

### Toolchain

| Tool | Version | Purpose |
|------|---------|---------|
| Python | 3.12 | Runtime (match minimum version) |
| Rye / uv | Latest | Dependency + venv management (pip/setuptools/pipenv BANNED) |
| Ruff | Latest | Lint + format (flake8, isort, black combined) |
| mypy | Strict | `--strict` mode |
| PyTest | 8+ | Test runner + coverage ≥ 80% |
| PyO3 | 0.22+ | Rust bindings (NOT CPython extensions by hand) |

### Code Style

```python
# ✅ PEP 701 f-strings; dataclasses or pydantic for models
from __future__ import annotations

from dataclasses import dataclass
from typing import Annotated, NewType, Never, Protocol

import pydantic

WorkspaceId = NewType("WorkspaceId", str)

class WorkspaceConfig(pydantic.BaseModel):
    model_config = pydantic.ConfigDict(frozen=True, extra="forbid")

    id: WorkspaceId
    name: str
    memory_limit_bytes: Annotated[int, pydantic.Field(gt=0, le=2**40)]

# ✅ Protocols > ABC where possible; composition over inheritance
class RuntimeHost(Protocol):
    async def start(self, spec: WorkloadSpec) -> WorkloadHandle: ...
    async def stop(self, handle: WorkloadHandle, timeout: float) -> None: ...

# ✅ Always raise concrete exception types; no bare raise Exception
class WorkspaceError(Exception): ...
class WorkspaceNotFoundError(WorkspaceError): ...
```

---

## 5. C++ Standards

C++ is ONLY permitted in hardware abstraction, GPU/driver code, or linking to C++ libraries that cannot be wrapped cleanly otherwise. All new components default to Rust.

| Rule | Value |
|------|-------|
| Standard | C++20 minimum |
| Compiler | GCC 13 or Clang 17+ |
| Build System | CMake + Conan / vcpkg |
| Memory | `std::unique_ptr`/`std::shared_ptr`, NO raw `new`/`delete` |
| Undefined Behavior | `-fsanitize=address,undefined` in CI Debug builds |
| Exceptions | Disabled in embedded contexts; enabled with std::expected otherwise |
| Safety | BANNED: `reinterpret_cast`, `const_cast`, C-style casts, `volatile` |

---

## 6. Markdown Documentation Standards

### Format Rules

1. One sentence per line. (Cleaner diffs; no line-break chaos.)
2. Headings: `# H1` only in root docs; `## H2` first in sections; maximum depth 4.
3. Tables always have headers; escape pipes `\|` inside cells.
4. Code blocks specify language: ```rust, ```tsx, ```protobuf, etc.
5. Links always use reference form when reused 3+ times.
6. Frontmatter required for all docs in `/docs/**`:

```markdown
---
id: DEV-001
title: Coding Standards
version: 1.0.0
status: approved
owner: Engineering Standards
last_updated: 2026-08-01
---
```

---

## 7. Testing Standards

### Test Matrix Required Per Module

| Test Type | Mandatory For | CI Gate |
|-----------|---------------|---------|
| **Unit Tests** | All modules | ≥ 80% line coverage; 100% of public API |
| **Property Tests** | Parsers, encoders, planners, policy engine | Proptest / Arbitrary |
| **Integration Tests** | Services, APIs | gRPC end-to-end with Testcontainers |
| **Fuzz Tests** | Parser, IPC, message bus, protocol deserialization | Continuous AFL++ / libFuzzer |
| **Security Tests** | Permission system, audit log, sandbox boundaries | Pen test suite |
| **Performance Tests** | Hot paths: Message bus, FS, Planner, LLM routing | Benchmark regression ≤ 5% |
| **Chaos Tests** | Failover, module crash, network partition (Phase 2+) | Nightly only |

### Test Rules

1. **No flaky tests.** A test that fails intermittently is deleted, rewritten, or quarantined within 24 hours.
2. **No network access in unit tests.** Use mocks / wiremock.
3. **Deterministic tests.** If a test uses time or randomness, it accepts a deterministic seed.
4. **Test naming:** `test_<function>_<input/condition>_<expected_outcome>` (snake_case in all languages).

---

## 8. Logging Standards

### Structured Logging Everywhere

ALL log entries carry: timestamp, level, module, correlation_id, causation_id, workspace_id (when applicable).

### Rust: tracing

```rust
// ✅ Use tracing instrument macros for spans on every async function
#[tracing::instrument(
    name = "workspace.activate",
    skip_all,
    fields(
        workspace_id = %config.id,
        memory_limit = config.memory_limit_bytes
    ),
    err(level = "warn")
)]
pub async fn activate(config: WorkspaceConfig) -> Result<WorkspaceHandle> {
    info!("starting workspace activation");
    // ...
}
```

### TypeScript: pino

```typescript
import pino from "pino";
export const logger = pino({
  level: process.env.LOG_LEVEL ?? "info",
  formatters: { bindings: () => ({}) },
  base: undefined,
  timestamp: pino.stdTimeFunctions.isoTime,
});

// ✅ Pass a child logger with correlation to sub-tasks
const subLogger = logger.child({ correlationId, workspaceId });
```

### Level Semantics

| Level | When to use | Who sees? |
|-------|-------------|-----------|
| **TRACE** | Function entry/exit, variable dumps | Developers only (disabled by default) |
| **DEBUG** | Interesting internal state; not errors | Debug builds / verbose flag |
| **INFO**  | Milestones of normal operation (started, completed) | Default enabled |
| **WARN**  | Recoverable problem; retried; user unaffected | Admin dashboards |
| **ERROR** | User-visible failure; task incomplete. | Alerts, paging (when rate high) |
| **FATAL** | Cannot continue; crash incoming. | ALWAYS paging on-call. |
| **AUDIT** | Security events (see Security.md) | NEVER disabled; hash chained. |

---

## 9. Naming Conventions

### Global Cross-Language Rules

1. **IDs = `{Entity}Id`** - Always opaque, never serial numbers, never auto-increment ints (UUID v7 or string IDs).
2. **Timestamps = `*_at` suffix, always absolute UTC, never "time" alone.**
   - ✅: `created_at`, `deadline_at` (protobuf Timestamp / `DateTime<Utc>` / `Date` in UTC)
   - ❌: `time`, `creationTime`, `created`
3. **Durations = `*_duration_ms`** - Milliseconds explicit, no ambiguous "period" / "timeout".
4. **Booleans = yes/no questions prefix.** `is_active`, `has_permission`, `should_retry`. Never `active`, `retry`, `permission`.
5. **Count fields = plural.** `files`, `retries`, NOT `file_count`.

### API Conventions

| RPC Naming Pattern | Meaning |
|--------------------|---------|
| `Get{Entity}` | Fetch one by ID |
| `List{Entities}` | Paginated list with filters |
| `Create{Entity}` | Create, return entity |
| `Update{Entity}` | Partial update (PATCH semantics) |
| `Delete{Entity}` | Soft or hard delete |
| `Watch{Entities}` | Server stream of changes |
| `{Verb}{Entity}` | Action: `ActivateWorkspace`, `HibernateWorkspace` |

---

## 10. Error Handling

### The Error Gospel

1. **Never swallow errors.** If you catch/log and return OK, you have lied to the caller and destroyed the diagnostic trail. The ONLY valid swallow is explicitly annotated with a safety proof.
2. **Prefer Result types to exceptions** in all languages that support them without ergonomic cost (Rust, TS/neverthrow, Python 3.12 +).
3. **Error = (Code, Message, UserAction, Context).** All four pieces required for any error reaching user-facing layers.
4. **Two levels of error message.** One for developers (English, technical, stacktrace). One for users (i18n key, no technical jargon, suggested action).

---

## 11. Dependency Management

### Crates (Rust)

```toml
# Cargo.toml: workspace dependencies defined ONCE in root Cargo.toml
[workspace.dependencies]
tokio = { version = "1.40", features = ["full"] }
uuid = { version = "1.10", features = ["v7", "serde"] }
# Members use: uuid = { workspace = true }
```

### pnpm Workspaces (TS)

```yaml
# pnpm-workspace.yaml
packages:
  - "ui/*"
  - "sdk/typescript"
  - "api/grpc/*"
```

### Supply Chain Hardening

1. All external dependencies pinned with checksums (Cargo.lock, pnpm-lock.yaml).
2. Lock files committed; CI verifies checksum integrity.
3. Weekly automated dependency updates via Renovate/Dependabot with auto-test.
4. HIGH CVEs in dependencies = BLOCKED RELEASE until patched.

---

## 12. Versioning & Releases

See Versioning.md for full scheme. Summary for developers:

- **SemVer 2.0.** Breaking changes = MAJOR bump.
- **Protobuf:** Field numbers are sacred. NEVER reuse.
  - Add = backward compatible.
  - Delete = mark RESERVED, never add new field at old number.
- **APIs:** Additive only across minors. Breaking changes require v2 API surface + 2 version deprecation window.
