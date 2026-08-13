# CognyxOS Developer Guide

> **Document ID:** DEV-002
> **Version:** 1.0.0
> **Status:** Phase 0 - Approved
> **Last Updated:** 2026-08-01
> **Owner:** Developer Experience Team

---

## Table of Contents

1. [Getting Your Development Environment Set Up](#getting-your-development-environment-set-up)
2. [Repository Layout](#repository-layout)
3. [Building the Project](#building-the-project)
4. [Running Locally](#running-locally)
5. [Debugging](#debugging)
6. [Creating a New Service](#creating-a-new-service)
7. [Adding a New Capability](#adding-a-new-capability)
8. [Writing Protocol Buffers](#writing-protocol-buffers)
9. [Testing Workflows](#testing-workflows)
10. [Code Review Checklist](#code-review-checklist)

---

## Getting Your Development Environment Set Up

### Prerequisites

| Component | Minimum Version | Install Command (Linux - apt-based) |
|-----------|-----------------|-------------------------------------|
| Linux Kernel | 6.8.0 | LTS kernel backport |
| Rust | 1.80 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| protoc | 27.0 | `apt install protobuf-compiler` |
| Node.js | 20 LTS | `volta install node@20` |
| pnpm | 9.x | `corepack enable && corepack prepare pnpm@9 --activate` |
| Python | 3.12 | `apt install python3.12` + uv (`curl -LsSf https://astral.sh/uv/install.sh \| sh`) |
| Podman | 5.0 | `apt install podman` (rootless mode mandatory) |
| QEMU/KVM | 8.2 | `apt install qemu-kvm libvirt-daemon-system` |
| Docker CLI | 26.x | Optional, for building containers |
| Wasmtime | 21.x | `curl https://wasmtime.dev/install.sh -sSf \| bash` |

### Additional Developer Tools (Recommended)

```bash
# Rust tools
cargo install cargo-audit cargo-vet cargo-flamegraph cargo-insta cargo-expand \
  wasm-pack trunk wasm-bindgen-cli

# TypeScript tools
pnpm add -g turbo tsx @bufbuild/buf

# Debugging
apt install lldb perf bpftool bpftrace strace ltrace
```

### First-Time Setup

```bash
# 1. Clone monorepo
git clone git@github.com:cognyxos/cognyxos.git
cd cognyxos

# 2. Bootstrap toolchains
./scripts/dev/bootstrap.sh    # Installs all pinned toolchains via rustup + pnpm + uv
./scripts/dev/generate-schemes.sh  # Compiles .proto → Rust/TS/Python types

# 3. Run local verification
./scripts/dev/check-env.sh   # Verifies all prerequisites installed

# 4. Install git hooks (commit message format, lint pre-push)
./scripts/dev/install-git-hooks.sh
```

---

## Repository Layout

```
cognyxos/                    # Monorepo root
├── kernel/                  # Kernel abstractions (Rust crate)
├── services/                # Layer 1 services (each = one Cargo crate)
│   └── <service-name>/      # e.g. services/workspace
│       ├── src/             # Rust source
│       ├── tests/           # Integration tests
│       └── Cargo.toml
├── runtime/                 # Layer 2 (AI Runtime) + Layer 4 (App/Container/VM)
├── agents/                  # Agent implementations
├── ui/                      # Layer 6: UI (React/Tauri apps, pnpm workspaces)
├── sdk/                     # Public SDKs (Rust/TS/Python/C++)
├── plugins/                 # First-party plugins (Wasm)
├── workspaces/              # Workspace templates
├── storage/                 # Storage backends (vector/KV/blob)
├── security/                # Sandbox, permission, crypto implementations
├── platform/                # Platform-specific (Linux/Windows/Android/macOS/Cloud)
├── proto/                   # Source of truth: Protocol Buffers
│   ├── messages/            # common.proto, bus.proto
│   └── services/            # core_services.proto, ai_services.proto, etc.
├── interfaces/              # High-level interface definitions (Rust traits, TS interfaces)
├── api/                     # API gateways (gRPC/REST/GraphQL)
├── system/                  # Boot, init, recovery, power
├── devtools/                # Developer utilities: debugger, profiler, emulator
├── tests/                   # Cross-crate integration, perf, security, e2e tests
├── scripts/                 # Dev/build/deploy/test shell scripts (Rust-based)
├── build/                   # Build system orchestration
├── deploy/                  # Deployment targets (K8s, ISO, Docker)
├── config/                  # Default configs
├── docs/                    # Architecture + API + security documentation
├── Cargo.toml               # Rust workspace root (all crates listed here)
├── pnpm-workspace.yaml      # TypeScript workspace root
├── pyproject.toml           # Python workspace (uv)
├── turbo.json               # Turborepo orchestration
├── Vision.md                # Root documentation
├── Architecture.md
├── Security.md
└── <other root docs>
```

---

## Building the Project

### Full System Build

```bash
# Build everything (release with LTO for final artifacts)
./scripts/build/full.sh --release

# Build only Rust components (default = debug)
cargo build --workspace

# Build only UI
cd ui && pnpm install && turbo run build

# Build protobuf (regenerate if you modified /proto/**/*.proto)
./scripts/build/gen-proto.sh
```

### Incremental Builds

```bash
# Build only workspace-manager service
cargo build -p cognyx-workspace-manager

# Run just workspace-manager unit tests
cargo test -p cognyx-workspace-manager

# Format + lint
cargo fmt --all && cargo clippy --workspace -- -D warnings
```

---

## Running Locally

### Development Mode (On Your Existing Linux Box)

CognyxOS runs as user-space services on top of your existing Linux in development mode. All kernel features are required but your host is the kernel HAL.

```bash
# 1. Build all services in debug
cargo build --workspace

# 2. Start message bus (root-like privileges needed for namespaces; use user namespaces where possible)
./target/debug/cognyxos-bus --config config/system/bus.dev.toml &

# 3. Start all core services via supervisor
./target/debug/cognyxos-supervisor --profile dev

# 4. Start UI shell (separate terminal)
cd ui/shell && pnpm run dev

# 5. Local development portal:
#    http://localhost:5173 → AI Shell UI
#    http://localhost:3000 → Developer Dashboard (logs, metrics, bus inspect)
```

### VM Emulation (Full CognyxOS Experience)

```bash
# Build bootable ISO
./scripts/build/iso.sh

# Launch in QEMU/KVM with 4 cores, 8GB RAM
./scripts/dev/run-qemu.sh ./build/artifacts/cognyxos-dev.iso
```

---

## Debugging

### Standard Debug Session Workflow

```bash
# 1. Launch service with debug log + tokio console
RUST_LOG=cognyx_workspace_manager=trace ./target/debug/cognyx-workspace-manager

# 2. Inspect bus message traffic live
./target/debug/cognyx-bus-inspector --filter workspace.*

# 3. Attach lldb to running service
lldb -p $(pgrep -f cognyx-workspace-manager)

# 4. System-wide perf trace (kernel + user)
perf record -F 99 -a -g -- sleep 10 && perf report

# 5. bpftrace for eBPF-level observability
bpftrace ./devtools/profiler/scripts/process_exec.bt
```

### Common Pitfalls

| Symptom | Root Cause | Fix |
|---------|-----------|-----|
| `PermissionDenied` from bus | Missing capability token in your gRPC metadata | Add `cap` metadata via SDK client builder |
| Namespace clone fails with `EPERM` | Not running with user namespaces or kernel < 6.8 | Enable user namespaces: `sysctl -w user.max_user_namespaces=65536` |
| Wasm plugin won't instantiate | Missing `wit` world imports match exactly | Use `cognyx-plugin-sdk doctor ./myplugin.wasm` |

---

## Creating a New Service

### Step-by-Step Template

```bash
# 1. Use the service scaffolder
./scripts/dev/scaffold-service.sh <service-name> <team-owner> <criticality>
# Example: ./scripts/dev/scaffold-service.sh billing finance HIGH

# 2. Output created:
services/<service-name>/
├── Cargo.toml              # Pre-configured with bus, config, telemetry deps
├── src/
│   ├── lib.rs              # Service crate root
│   ├── service.rs          # gRPC impl + ModuleLifecycle impl
│   ├── error.rs            # thiserror enum
│   ├── types.rs            # Strong types
│   └── main.rs             # Binary entry point
├── tests/service_test.rs   # gRPC integration test harness
└── README.md               # Pre-filled contract template

# 3. Edit proto/services/your_service.proto
#    Use existing patterns from core_services.proto

# 4. Regenerate protos
./scripts/build/gen-proto.sh

# 5. Register service in system/services/service_manifest.toml
#    Add to dependency graph so supervisor starts it correctly
```

### Mandatory Checklist Before Marking Service Complete

- [ ] Implements `ModuleLifecycle` trait (start/shutdown/health_check)
- [ ] Has ≥ 80% unit test coverage (CI gate)
- [ ] Has integration test via `cognyx-test-harness`
- [ ] All error paths use concrete error type (no anyhow in lib)
- [ ] Emits: metrics, tracing spans, structured logs
- [ ] Capability requirements documented and validated
- [ ] Security review for any new capability namespace additions

---

## Adding a New Capability

### Process

1. **Design Document** (1-page mini-ADR):
   - Operation namespace, resource pattern, HITL default
   - Security implications
   - Existing capabilities it composes with

2. **Add to Permissions.md** Table of operations

3. **Register in capability registry:**
   ```
   security/permissions/capability_registry.toml
   ```

4. **Wire up validation in Policy Engine (Rego policy if special rules needed)**

5. **Tests:**
   - Token mint + validate round-trip
   - Delegation chain works up to max depth
   - Revocation cascades correctly
   - HITL prompt triggered correctly

---

## Writing Protocol Buffers

### Style Rules (Superset of Google's Protobuf Style Guide)

1. **Never reuse field numbers.** When deprecating a field, use `reserved`.

```protobuf
// ✅ Good
message Foo {
  reserved 2, 15, 9 to 11;
  reserved "old_field";
  string new_field = 3;
}

// ❌ Bad - reuse of field 2 after removing it causes corruption!
message Foo { string different_purpose = 2; }
```

2. **Package naming:** `cognyx.{module_path}.v{major_version}`
3. **Services:** One service per `.proto` file, with clear separation into query vs mutations.
4. **Dates/Times:** Always `google.protobuf.Timestamp`, never `uint64` seconds.
5. **Durations:** Always `google.protobuf.Duration`, never `uint64` ms.
6. **Request metadata:** Every request message has a `common.RequestMetadata meta = 1` and `common.CapabilityToken cap = 2` as fields 1 and 2.

---

## Testing Workflows

### Run the Test Pyramid Locally

```bash
# 1. Unit tests (fast, local)
cargo test --workspace --lib --bins

# 2. Integration tests (Testcontainers spins up services)
cargo test --workspace --test '*' --features integration-tests

# 3. Security tests (nightly, can be run locally)
cargo test --workspace -p cognyx-security-tests

# 4. Performance benchmarks
cargo bench -p cognyx-message-bus

# 5. End-to-end (full stack in VM, takes ~20 min)
./tests/e2e/run-local.sh

# 6. Fuzzing (on CI, long-running; locally use single corpus)
cargo fuzz run message_parse -max_total_time=60
```

---

## Code Review Checklist

Every PR must pass before merge. Approver ticks each item.

### Architecture Review (1 of the 3 Architecture Council members)

- [ ] Aligns with Vision.md + Architecture.md principles
- [ ] Communication patterns (bus patterns only; no backdoor IPC)
- [ ] No shared memory; no cross-module calls bypassing bus
- [ ] Security model respected (new capabilities approved, ambient authority introduced?)

### Engineering Correctness (Any Senior Engineer)

- [ ] Compiles: `cargo build`, `cargo clippy -- -D warnings`, `cargo fmt --check`
- [ ] Tests pass locally + CI green
- [ ] Error handling follows standards; no bare `.unwrap()` in non-test code
- [ ] Uses strong types; no `&str` IDs, no generic `Value` when schema exists
- [ ] Structured logging, metrics, tracing spans added
- [ ] All public APIs documented with Rustdoc / TSDoc
- [ ] Proto schema version-safe (no field reuse, correct reserved usage)

### Documentation

- [ ] New modules have README.md with contract per standard
- [ ] New APIs added to docs/api/APISpecifications.md
- [ ] Permissions.md updated for any new capability namespace
- [ ] CHANGELOG updated per Conventional Commits
