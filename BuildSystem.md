# CognyxOS Build System

> **Document ID:** DEV-005
> **Version:** 1.0.0
> **Status:** Phase 0 - Approved
> **Last Updated:** 2026-08-01
> **Owner:** Release Engineering Team

---

## Table of Contents

1. [Build System Philosophy](#build-system-philosophy)
2. [Monorepo Tooling Stack](#monorepo-tooling-stack)
3. [Rust Build Workflow](#rust-build-workflow)
4. [TypeScript Build Workflow](#typescript-build-workflow)
5. [Python Build Workflow](#python-build-workflow)
6. [Protocol Buffer Code Generation](#protocol-buffer-code-generation)
7. [Reproducible / Deterministic Builds](#reproducible--deterministic-builds)
8. [ISO Image & OS Image Pipeline](#iso-image--os-image-pipeline)
9. [Container Build Pipeline](#container-build-pipeline)
10. [Wasm Plugin Build Pipeline](#wasm-plugin-build-pipeline)
11. [CI/CD Pipeline (GitHub Actions / BuildKite)](#cicd-pipeline-github-actions--buildkite)
12. [Caching Strategy](#caching-strategy)
13. [Build Artifacts Layout](#build-artifacts-layout)
14. [Version Injection](#version-injection)

---

## Build System Philosophy

1. **Reproducible first.** Given identical source + git hash, the build output must be byte-for-byte identical. Two engineers on two continents running the same build get SHA-identical artifacts.
2. **Incremental and fast.** `cargo check` on full monorepo < 30s on modern laptop. UI builds < 20s.
3. **Single source of truth.** No duplicate config. All build configuration lives in the 4 files at repo root: `Cargo.toml`, `pnpm-workspace.yaml`, `turbo.json`, `Makefile.toml` (or `Justfile`).
4. **Hermetic builds.** Nothing depends on globally-installed tools except the bootstrap script. All toolchains (Rust, Node, Python, protoc, wasm-tools) are installed pinned by the project and downloaded automatically on first build.
5. **Zero surprise side effects.** `./scripts/build/full.sh --release` produces artifacts in `/build/artifacts/` and modifies nothing outside of `/build/`, `/target/`, and caches in `$HOME/.cache/cognyxos-build/`.

---

## 2. Monorepo Tooling Stack

| Tool | Purpose | Configuration |
|------|---------|---------------|
| **TurboRepo** | Cross-language build orchestration, dependency graph, caching | `turbo.json` |
| **Cargo (workspace)** | Rust crates compilation, test, clippy | Root `Cargo.toml` + per-crate |
| **pnpm** | TypeScript package manager (strict hoisting, workspaces) | `pnpm-workspace.yaml`, `.npmrc` |
| **Vite 5** | UI bundler (esbuild + rollup, optimized for React) | Per-UI-package `vite.config.ts` |
| **uv** | Python workspace & dependency management | `pyproject.toml` monorepo root |
| **buf** | Protocol Buffer lint + format + breaking change detection | `buf.yaml`, `buf.gen.yaml` |
| **protolock** | Protobuf field # lock to prevent accidental reuse | `proto.lock` |
| **Just** | Cross-platform task runner (modern make replacement) | `justfile` |
| **Nix (optional)** | Fully-hermetic devshell (all toolchains pinned with flake.nix) | `flake.nix`, `flake.lock` |
| **sccache** | Shared Rust compilation cache across machines / CI | Env var: `RUSTC_WRAPPER=sccache` |
| **Dprint** | Cross-language formatter (Rust=rustfmt, TS=prettier, Markdown=prettier, Protobuf=clang-format) | `.dprint.json` |

---

## 3. Rust Build Workflow

### Cargo Workspace

Root `Cargo.toml` declares ALL crates:

```toml
[workspace]
resolver = "2"
members = [
    "kernel/*",
    "services/*",
    "runtime/*",
    "runtime/ai/*",
    "sdk/rust",
    "security/*",
    "storage/*",
    "system/*",
    "devtools/*",
    "tests/*",
]

# ONE place to set shared dependency versions
[workspace.dependencies]
tokio = { version = "1.40", features = [
    "full", "tracing", "rt-multi-thread", "signal",
] }
tonic = "0.12"
prost = "0.13"
thiserror = "1.0"
serde = { version = "1", features = ["derive"] }
# ... 100+ crates all pinned here
uuid = { version = "1.10", features = ["v7", "serde"] }

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.80"
license = "AGPL-3.0-or-later"
repository = "https://github.com/cognyxos/cognyxos"
```

### Per-Crate `Cargo.toml` Shortcut

```toml
[package]
name = "cognyx-workspace-manager"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
uuid = { workspace = true }
cognyx-message-bus = { path = "../../kernel/ipc" }   # Inter-crate dependency = path
```

### Build Profiles

```toml
[profile.dev]
opt-level = 1
debug = true
overflow-checks = true

[profile.release]
opt-level = 3
lto = "fat"          # Link-time optimization (reduces size + increases perf)
codegen-units = 1    # Deterministic builds + best perf
panic = "abort"
strip = true
overflow-checks = true

[profile.test]
opt-level = 2
overflow-checks = true
```

---

## 4. TypeScript Build Workflow

### pnpm + Turborepo

```
# pnpm-workspace.yaml
packages:
  - "ui/*"
  - "ui/components"
  - "ui/shell"
  - "ui/compositor"
  - "sdk/typescript"
  - "api/grpc/ts-client"
  - "plugins/applications/*"
```

TurboRepo Pipeline:
```json
{
  "$schema": "https://turbo.build/schema.json",
  "tasks": {
    "gen-proto": { "outputs": ["src/gen/**"] },
    "build": {
      "dependsOn": ["^build", "gen-proto"],
      "outputs": ["dist/**", ".next/**", "build/**"]
    },
    "test": {
      "dependsOn": ["build"],
      "outputs": ["coverage/**"]
    },
    "lint": {},
    "typecheck": { "dependsOn": ["^build"] }
  }
}
```

---

## 5. Python Build Workflow

- `uv` workspace manages all Python packages (plugins, SDK, tooling)
- `pyproject.toml` at root with PEP 735 dependency groups
- Ruff for lint/format (single binary replaces isort + black + flake8 + pyupgrade)
- mypy strict mode on SDK packages

---

## 6. Protocol Buffer Code Generation

`buf` is the canonical tool. Single `buf.gen.yaml`:

```yaml
version: v1
inputs:
  - directory: proto
plugins:
  # Rust
  - plugin: buf.build/community/neoeinstein-prost
    out: interfaces/rust/gen
    opt:
      - bytes=Bytes
      - compile_well_known_types
  - plugin: buf.build/community/neoeinstein-tonic
    out: interfaces/rust/gen
    opt:
      - compile_well_known_types
  # TypeScript
  - plugin: buf.build/community/stephenh-ts-proto
    out: interfaces/typescript/gen
    opt:
      - env=both
      - useOptionals=messages
  # Python
  - local: protoc-gen-mypy
    out: sdk/python/cognyx_sdk/_gen
  # Go (future)
  - local: protoc-gen-go
    out: api/grpc/go-client
  # Documentation
  - plugin: buf.build/community/pseudomuto-doc
    out: docs/api/generated
    opt:
      - markdown,README.md
```

Build: `buf generate` (outputs are checked in for offline build capability; CI verifies they match input proto).

---

## 7. Reproducible / Deterministic Builds

### Byte-for-byte identical output, every time:

1. **Source date epoch** (`SOURCE_DATE_EPOCH`) = timestamp of latest git commit. Set in all scripts; embedded in binaries by Rustc/linker.
2. **`Cargo.lock`, `pnpm-lock.yaml`, `uv.lock`** committed; CI fails if mismatch with declared deps.
3. **Nix flake** provides fully-pinned toolchain: rustc 1.80.0 (exact commit), node 20.14.0, clang 18, binutils 2.42, etc.
4. **Strip, reorder, normalize** ELF binaries. Use `llvm-objcopy` + `strip --remove-section=.note.gnu.build-id` then re-add deterministic build-id.
5. **`diffoscope`** CI job on release compares nightly build vs. previous with same hash; reports any divergence.

### Reproducibility Verification Procedure:

```bash
# Machine A build
./scripts/build/full.sh --release --deterministic
sha256sum build/artifacts/cognyxos-0.1.0-x86_64.iso > /tmp/machineA.sha

# Machine B build (different OS, different timezone)
./scripts/build/full.sh --release --deterministic --from-hash <same commit>
sha256sum build/artifacts/cognyxos-0.1.0-x86_64.iso > /tmp/machineB.sha

# Must match EXACTLY
diff /tmp/machineA.sha /tmp/machineB.sha || FAILED "Non-deterministic build!"
```

---

## 8. ISO Image & OS Image Pipeline

Build flow for bootable installer ISO:

```
┌──────────────────────────────────────┐
│  1. Compile all Rust crates → ELF     │
│  2. Compile UI (pnpm turbo build)     │
│  3. Compile Wasm plugins              │
└──────────────────┬───────────────────┘
                   ▼
┌──────────────────────────────────────┐
│  4. Build initramfs (cpio, zstd)      │
│     - Busybox userland                │
│     - cognyxos-init (PID 1)           │
│     - Kernel modules, firmware        │
│     - Signed kernel + initramfs for   │
│       Secure Boot                     │
└──────────────────┬───────────────────┘
                   ▼
┌──────────────────────────────────────┐
│  5. Build OSTree root filesystem      │
│     - sys/ partition (read-only,      │
│       dm-verity merkle tree)          │
│     - Default state partition layout  │
└──────────────────┬───────────────────┘
                   ▼
┌──────────────────────────────────────┐
│  6. Assemble ISO (xorriso)            │
│     - EFI ESP partition (systemd-boot)│
│     - Boot entries                    │
│     - OSTree repo in /sysroot         │
│     - ISO hybris (BIOS boot + UEFI)   │
└──────────────────┬───────────────────┘
                   ▼
┌──────────────────────────────────────┐
│  7. Sign & Attest                     │
│     - Ed25519 sign OSTree commits     │
│     - Generate SHA256 manifest        │
│     - Sigstore transparency log entry │
│     - Produce .iso + .sha256 + .sig   │
└──────────────────────────────────────┘
```

### Image Output Types

| Artifact | Format | Use Case |
|----------|--------|----------|
| **Full Install ISO** | Hybrid ISO 9660 (.iso) | Bare-metal install on PC/laptop |
| **Cloud Image** | QCOW2 (KVM) + VHDX (Hyper-V) + AMI (AWS) | Cloud VMs, K8s nodes |
| **Raspberry Pi Image** | .img (rpi-imager flashable) | ARM SBCs |
| **WSL2 Tarball** | .tar.gz import | Windows Subsystem for Linux |
| **Framework Laptop OEM** | .raw, pre-imaged | Pre-installed hardware |

---

## 9. Container Build Pipeline

Per-service `Containerfile` (Dockerfile syntax, build with `buildah` podman-docker, rootless):

```dockerfile
# syntax=docker/dockerfile:1.7-labs
# =============================================
# Stage 1: Build (only has build toolchain)
# =============================================
FROM ghcr.io/cognyxos/build-rust:1.80 AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY services/workspace-manager ./services/workspace-manager/
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build -p cognyx-workspace-manager --release

# =============================================
# Stage 2: Runtime (minimal distroless-like)
# =============================================
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
COPY --from=builder /build/target/release/cognyx-workspace-manager /usr/local/bin/
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/cognyx-workspace-manager"]
```

**Container Standards:**
- All images pass `dockle` container linter (CIS benchmarks)
- All images signed with Cosign (Sigstore)
- SBOM (SPDX) generated + attached to every image
- 0 Critical / High CVEs on merge (grype scan gated)

---

## 10. Wasm Plugin Build Pipeline

For Rust plugins:
```bash
# Cargo.toml crate-type cdylib
rustup target add wasm32-wasip2
cargo build --release --target wasm32-wasip2
wasm-tools component new target/wasm32-wasip2/release/plugin.wasm \
  -o plugin-component.wasm
wasm-opt -Os plugin-component.wasm -o plugin-opt.wasm
cognyx plugin sign plugin-opt.wasm --key dev-key.ed25519
```

CI: verifies declared host imports = declared capabilities (audit mismatch = FAIL).

---

## 11. CI/CD Pipeline

### GitHub Actions / BuildKite Pipeline Stages

Every PR runs these 11 stages in order. Fail-fast: any stage failure halts pipeline.

```
STAGE 1 ───────────────────────────────────────────────
│ Setup: Checkout, install toolchains (from cache),
│ verify checksums, verify protolock
├──────────────────────────────────────────────────────
│ Time: 30-60 seconds (cached)

STAGE 2 ───────────────────────────────────────────────
│ Format + Lint + Static Analysis
│ - cargo fmt --check
│ - cargo clippy --workspace -- -D warnings
│ - pnpm run lint (ESLint)
│ - ruff check + ruff format --check
│ - buf lint + buf format --diff + buf breaking
├──────────────────────────────────────────────────────
│ Time: 3-5 min

STAGE 3 ───────────────────────────────────────────────
│ Type Checking
│ - cargo check (w/ test)
│ - pnpm typecheck (tsc --noEmit)
│ - mypy strict on SDK
├──────────────────────────────────────────────────────
│ Time: 2-4 min (cache)

STAGE 4 ───────────────────────────────────────────────
│ Protobuf Generation Verification
│ - buf generate → compare against checked-in files
│ - Fail if generated code stale
├──────────────────────────────────────────────────────
│ Time: 30 sec

STAGE 5 ───────────────────────────────────────────────
│ Unit Tests
│ - cargo test --lib --bins (each crate in parallel)
│ - pnpm vitest run --coverage
│ - uv run pytest tests/unit
├──────────────────────────────────────────────────────
│ Time: 5-10 min

STAGE 6 ───────────────────────────────────────────────
│ Property Tests + Fuzzing (short run, corpus reuse)
│ - cargo proptest -j 8 (each fuzz target, 10 sec)
├──────────────────────────────────────────────────────
│ Time: 5 min

STAGE 7 ───────────────────────────────────────────────
│ Integration Tests (Testcontainers spins up services)
│ - gRPC end-to-end test for each service
│ - Capability/security scenarios
│ - AI Runtime mock inference
├──────────────────────────────────────────────────────
│ Time: 10-15 min

STAGE 8 ───────────────────────────────────────────────
│ Build (Release)
│ - cargo build --workspace --release
│ - pnpm turbo run build --filter=...[HEAD~1] (changed)
├──────────────────────────────────────────────────────
│ Time: 20-40 min

STAGE 9 ───────────────────────────────────────────────
│ Security Scans
│ - cargo audit (deny HIGH/CRITICAL CVEs)
│ - cargo vet (supply chain attestation)
│ - grype CVE scan on container images
│ - semgrep custom rules (security hotspots)
├──────────────────────────────────────────────────────
│ Time: 5 min

STAGE 10 ────────────────────────────────────────────────
│ Performance Regression Benchmarks (hot paths only)
│ - Cognyx message bus throughput/latency benchmark
│ - Vector store query benchmark
│ - Fail PR if regression > 5% p95 vs baseline
├──────────────────────────────────────────────────────
│ Time: 10 min

STAGE 11 ────────────────────────────────────────────────
│ Reproducibility Check
│ - 2x independent build of ISO; diff sha256
├──────────────────────────────────────────────────────
│ Time: 30 min (only merge queue + release branches)

MERGE (only after ALL 11 stages green)
```

---

## 12. Caching Strategy

Multi-level cache for < 1 min incremental builds:

| Cache Layer | Backend | Hit Rate Target |
|-------------|---------|------------------|
| L1: Local filesystem caches | `target/`, `node_modules/` | 95% dev rebuild |
| L2: sccache (Rust object files) | Redis (shared CI) + local disk | 80% CI build |
| L3: TurboRepo Remote Cache | S3-compatible (R2) | 90% UI build |
| L4: ccache / clang cache | Local disk (C++ deps only) | 90% |
| L5: Container image layers (buildah) | Registry cache | 80% container rebuilds |

---

## 13. Build Artifacts Layout

```
build/
├── packages/                 # Per-release package repos
│   ├── deb/                  # Debian packages (.deb, apt repo)
│   ├── rpm/                  # Fedora/RHEL packages
│   ├── arch/                 # Arch Linux packages
│   └── flatpak/              # Flatpak bundles for apps
├── artifacts/                # Shippable images + binaries
│   ├── x86_64/
│   │   ├── cognyxos-<ver>-x86_64.iso
│   │   ├── cognyxos-<ver>-x86_64.iso.sha256
│   │   ├── cognyxos-<ver>-x86_64.iso.sig
│   │   ├── cognyxos-cloud-image-<ver>-x86_64.qcow2
│   │   ├── sbom-x86_64-<ver>.spdx.json
│   │   └── release-notes-<ver>.md
│   └── aarch64/
│       └── (same structure)
├── debug/
│   ├── <ver>-<arch>-debuginfo.tar.zst    # DWARF debug symbols (stripped from release)
│   └── breakpad-symbols/                 # Crash symbolicated stack info
└── cache/                     # Cached deps, objects, toolchains
```

---

## 14. Version Injection

All version information injected at build time via env vars (never hardcoded in src):

| Variable | Set By | Usage |
|----------|--------|-------|
| `CARGO_PKG_VERSION` | Cargo | Crate version |
| `COGNYX_OS_VERSION` | Just/turbo pipeline script | Full SemVer `0.1.0-rc.3+build12345` |
| `COGNYX_GIT_HASH` | CI, `git rev-parse HEAD` (7 chars) | Displayed in UI, in logs, crash reports |
| `COGNYX_GIT_COMMIT_DATE` | CI | `git log -1 --format=%cI` |
| `COGNYX_BUILD_DATE` | CI | ISO-8601 timestamp of build start |
| `COGNYX_CHANNEL` | CI branch mapping | `nightly`/`beta`/`stable`/`lts` |
| `COGNYX_SIGNING_KEY_ID` | Release pipeline only | Key fingerprint used for signing artifacts |

Embedded via env!() in Rust, Vite `import.meta.env` in UI.
