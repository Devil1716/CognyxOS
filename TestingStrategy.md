# CognyxOS Testing Strategy

> **Document ID:** QA-001
> **Version:** 1.0.0
> **Status:** Phase 0 - Approved
> **Last Updated:** 2026-08-01
> **Owner:** Quality Assurance + SRE Teams

---

## Table of Contents

1. [Testing Philosophy](#testing-philosophy)
2. [Test Pyramid](#test-pyramid)
3. [Unit Tests](#unit-tests)
4. [Property Tests](#property-tests)
5. [Integration Tests](#integration-tests)
6. [Fuzz Testing](#fuzz-testing)
7. [End-to-End Tests](#end-to-end-tests)
8. [Security / Penetration Testing](#security--penetration-testing)
9. [Performance & Benchmarking](#performance--benchmarking)
10. [Chaos Engineering](#chaos-engineering)
11. [Concurrency & Correctness Tests](#concurrency--correctness-tests)
12. [Canary & Dogfood Testing](#canary--dogfood-testing)
13. [Testing Infrastructure & Tooling](#testing-infrastructure--tooching)
14. [Test Result Quality Gates](#test-result-quality-gates)
15. [Bug Tracking & Defect Triage](#bug-tracking--defect-triage)

---

## 1. Testing Philosophy

CognyxOS is security-critical, AI-orchestrated infrastructure. Our testing philosophy is built on one premise:

> **Any failure that can happen in production will happen in testing,** and failures that only happen in production are a test methodology bug, not a code bug.

Principles:
1. **Write the test FIRST (TDD) for bug fixes.** The bug isn't fixed until you have a test that fails before the fix and passes after.
2. **Deterministic tests are non-negotiable.** Flaky tests are quarantined within 24 hours.
3. **Every commit is tested.** No exceptions. Even documentation changes trigger docs link-check.
4. **Security tests block releases.** No critical/high security test failure is releasable.
5. **Test failure pauses the line.** A red main branch halts all merges until fixed.

---

## 2. Test Pyramid

CognyxOS follows an extended pyramid with security and performance layers.

```
                               ╱╲
                              ╱  ╲
                             ╱ AI ╲       ← AI Alignment & Safety Eval (2%)
                            ╱Tests ╲
                           ╱────────╲
                          ╱  Chaos   ╲      ← Chaos / Fault injection (2%)
                         ╱  Engineer  ╲
                        ╱──────────────╲
                       ╱   E2E / Full   ╲     ← Full OS boot + AI scenario (5%)
                      ╱     System       ╲
                     ╱────────────────────╲
                    ╱    Security/PenTest   ╲   ← Security tests (~5%)
                   ╱──────────────────────────╲
                  ╱     Integration Tests      ╲  ← Service-to-service (20%)
                 ╱  (Testcontainers, in-memory) ╲
                ╱────────────────────────────────╲
               ╱      Property + Fuzz Tests        ╲ ← Hot paths, parsers (10%)
              ╱──────────────────────────────────────╲
             ╱            Unit Tests                   ╲ ← Per crate (50%)
            ╱          (≥ 80% line, 100% public API)   ╲
           ╱──────────────────────────────────────────────╲
          ╱  Static Analysis / Lints / Type Checking        ╲ ← Every PR (6 layers)
         ╱────────────────────────────────────────────────────╲

        ┌──────────────────────────────────────────────────────┐
        │  COMPILE TIME: Rust type system, borrow checker,     │
        │  clippy, protolock, buf breaking, cargo audit, etc.  │
        └──────────────────────────────────────────────────────┘
```

---

## 3. Unit Tests

### Scope
Per module, per-crate, per-function. No cross-module calls; use mocks.

**Rust:** `cargo test` under each crate's `src/**/*.rs` with `#[cfg(test)]` modules + `tests/` integration folder.

**TypeScript:** Vitest, `__tests__/` sibling to file being tested.

**Coverage Gates (CI):**
- Minimum **80% line coverage** overall
- **100% public API coverage** (no public function untested)
- Minimum **80% branch coverage** for hot paths (bus, fs, security, permissions)

Coverage reports published per PR; regressions of >1% block the PR until tests added.

**Testing Rules:**
1. Use **parameterized tests** for boundary cases (rstest in Rust, vitest each in TS).
2. Mock the message bus; test a module in isolation with mock messages.
3. Use deterministic time. Never call `SystemTime::now()` directly; inject `Clock` trait.
4. All error paths tested, not just happy paths.

---

## 4. Property Tests

Property tests are mandatory for: parsers, encoders, serializers, IPC message decode, policy engine, HTN planner decompose.

### Toolchain
- **Rust:** `proptest` + `cargo-test-fuzz` (afl.rs + libfuzzer-sys)
- **TypeScript:** `fast-check`
- **Python:** `hypothesis`

### Example Properties For Message Bus:
```rust
proptest! {
  // Property: Encode then decode roundtrip = identity
  #[test]
  fn proto_encode_decode_roundtrip(msg in arb_message()) {
      let encoded = encode(&msg);
      let decoded = decode(&encoded).unwrap();
      assert_eq!(msg, decoded);
  }

  // Property: Malformed bytes never cause UB, always return Err
  #[test]
  fn garbage_bytes_decode_safely(bytes in any::<Vec<u8>>()) {
      let result = decode(&bytes);
      assert!(result.is_err() || /* valid */);
  }

  // Property: Messages never exceed declared max size
  #[test]
  fn payload_bounded(msg in arb_message()) {
      let size = encode(&msg).len();
      assert!(size < MAX_MESSAGE_SIZE);
  }
}
```

Run in CI: `proptest` with 10,000 iterations per target per PR; 1M iterations nightly.

---

## 5. Integration Tests

### Scope
2-5 services together using real binaries in containers / subprocesses. Real gRPC calls on real message bus with real capability validation.

### Tooling
- **Rust services:** Testcontainers-rs spawns per-service containers. Bus + 1 service = minimal config.
- **Temporary test workspaces:** Each test run gets a tempdir `/tmp/cognyx-test-<UUID>/` wiped after.
- **AI Runtime mock inference:** Return canned responses for planning/LLM calls; real inference separate test class.

### Example Test Cases
```rust
// Can create and activate a workspace end-to-end
#[testcontainers::test]
async fn test_workspace_lifecycle() {
    let bus = spawn_bus_container().await;
    let identity = spawn_identity(&bus).await;
    let workspace = spawn_workspace_manager(&bus).await;

    // 1. User authenticates
    let session = identity.login_user_alice().await.unwrap();

    // 2. Capability to create workspace minted
    let cap = mint_workspace_create_cap(&session).await;

    // 3. End-to-end: create → activate → verify state == ACTIVE
    let ws = workspace.create(ws_config(), cap.clone()).await.unwrap();
    workspace.activate(ws.id, cap).await.unwrap();
    let status = workspace.get_status(ws.id).await.unwrap();
    assert_eq!(status.state, WorkspaceState::ACTIVE);
}
```

---

## 6. Fuzz Testing

Fuzzing is mandatory for: IPC deserialization, filesystem driver ioctls, USB handling, image/audio decoders, capability token parsing, all network protocol parsing.

### Tooling
- Rust: `cargo fuzz` + libfuzzer, afl.rs for parallel fuzzing
- Coverage-guided + structure-aware

### Continuous Fuzzing (OSS-Fuzz Integration)
- 24/7 fuzzing cluster, daily reports
- New crash found → auto-files GitHub issue with repro, auto-triage security severity
- Continuous corpus back into fuzzers, corpus minimization weekly

Fuzzers cover:
- `fuzz_message_decode` → all valid/invalid messages to message bus parse
- `fuzz_cap_token_parse` → forged token attempts
- `fuzz_seccomp_filter` → 1B random syscall sequences, ensure sandbox survives
- `fuzz_wasm_plugin` → malformed WASM to wasmtime host, ensure no sandbox escape

---

## 7. End-to-End Tests

Full system boot + user scenario.

### Infrastructure
- CognyxOS ISO image booted in QEMU/KVM VM (real UEFI, real Secure Boot, virtual TPM 2.0)
- Scripted via expect + QMP (QEMU monitor protocol)
- Real local AI models downloaded (cached, 7B Q4) for true E2E

### Example Scenario Test Suites
| # | Scenario | Coverage |
|---|----------|----------|
| 1. | User creates workspace → types "Summarize Q3 docs" → AI reads 10 PDFs → writes summary.docx | AI + FS + Permissions + Shell |
| 2. | Start Windows 11 VM → launch Word via compatibility → type document → save → close | VM Manager + App Compat + FS |
| 3. | Install plugin "Markdown to PDF" → approve permissions → AI uses tool to convert | Plugin SDK + Security + Tool Registry |
| 4. | Create 5 workspaces → switch 1000 times → UI remains responsive, no memory leak | Workspace Manager + Memory Leak Detection |
| 5. | Pull large container → run PostgreSQL → verify SQL queries work end-to-end | Container Runtime + Network + FS |

Nightly only (1-2 hours). Release blockers: 0 failures of critical path E2E.

---

## 8. Security / Penetration Testing

### Continuous Automated Testing
- `semgrep` custom rules per commit (150 Cognyx-specific security hotspots)
- `cargo-audit` / `cargo-deny` for CVEs
- `grype` on every container
- Sandbox escape attempt battery: 1000 pre-built exploit attempts against each release
- AI prompt injection benchmark suite: JailbreakBench, HarmBench, garak, PromptFoo

### Periodic Manual Testing
- **External 3rd party pen test:** 1 per MINOR release cycle (Twice/year GA)
- **Red Team exercise:** Quarterly internal
- **Bug bounty program:** Always on, payout ladder per severity

### Pen Test Coverage Areas
1. Capability forgery / replay attacks
2. Sandbox escape via namespaces/seccomp/syscall confusion
3. IPC fuzzing → message bus crash / overflow
4. Cross-workspace data exfiltration side channels
5. TPM credential extraction
6. Supply chain attack vectors (updater, plugins, marketplace)
7. AI agent confused deputy scenarios
8. DMA attacks via Thunderbolt / FireWire

---

## 9. Performance & Benchmarking

**Hot path benchmarks run on every PR against baseline; >5% regression blocks merge.**

### Benchmark Infrastructure
- Bare-metal isolated benchmarking machine. No VMs for performance numbers.
- Thermal stabilization: CPU at constant temp before benchmarking (3-minute soak at load)
- 30 samples per benchmark, report p50/p95/p99/stdev
- `cargo-benchcmp` for Rust, `hyperfine` for CLI benchmarks, `k6` for REST/gRPC API benchmarks

### Core Benchmarks (always)
1. Message bus: 64-byte msg → 1M msg/sec throughput; <5µs p95 latency one-way
2. IPC: 4KB → 1GB memfd transfer throughput; zero-copy verified
3. Workspace activate/deactivate roundtrip: <500ms p95
4. AI first token latency (7B local): <300ms p95
5. Vector search (1M embeddings, k=20): <30ms p95
6. Sandbox spawn (wasm plugin): <5ms p95
7. File open capability-checked: <50µs p95

### Storage
- Benchmark results archived per commit; trend dashboard, anomaly detection auto-alerts

---

## 10. Chaos Engineering (Phase 2+)

### Nightly Chaos Suite (k6-operator + chaos-mesh in cluster mode, manual on single node)
Inject failures for 15-minute soak tests:
- Random process kill: kill one service per minute (message bus, workspace manager, etc.)
- Network: 500ms delay, 1% packet loss, partition
- Resource starvation: limit CPU to 1 core, limit RAM to 1GB, fill disk to 99%
- Bit rot: corrupt random bytes on filesystem (check Btrfs/ZFS recovers)

### Post-Chaos Assertions
- No silent data corruption. Audit log integrity remains valid.
- No capability tokens lost; user logged back in within 60s of recovery
- AI resumes current plan where it left off; no lost state

---

## 11. Concurrency & Correctness Tests

Tokio `--cfg loom` and shuttle loom tests for the trickiest concurrency:
```rust
// Loom model checking: explore ALL possible thread interleavings
#[cfg(loom)]
#[test]
fn message_bus_queue_concurrent_push_pop() {
    loom::model(|| {
        let bus = Arc::new(Bus::new());
        for _ in 0..4 {
            let bus = bus.clone();
            loom::thread::spawn(move || { for _ in 0..100 { bus.push(msg()); }});
        }
        for _ in 0..4 {
            let bus = bus.clone();
            loom::thread::spawn(move || { for _ in 0..100 { bus.pop(); }});
        }
    });
}
```

Looms on critical modules per-nightly; detect data races that unit tests miss.

---

## 12. Canary & Dogfood Testing

### Dogfood (Requirement for TSC)
- All CognyxOS engineers MUST run latest nightly on primary machine 4 days/week
- Crash reports auto-filed to GitHub Issues with detailed traces
- Daily dogfood blocker meeting: anything blocking dogfood → fix before any feature work

### Canary Releases
- 1% population → 5% → 20% → 50% → 100% per MINOR release
- Stop/rollback metrics: crash rate (>0.1% auto-stop), AI decision correctness regression, boot failures, workspace corruption
- Automatic rollback: canary group if crash rate > 5× baseline for 1 hour

---

## 13. Testing Infrastructure & Tooling

| Tool | Purpose |
|------|---------|
| **BuildKite / GitHub Actions** | CI runner, 500+ parallelism on self-hosted bare metal |
| **Testcontainers** | Spawns Postgres/Qdrant/services for integration tests |
| **cargo-nextest** | 2-5x faster Rust test execution with scheduling |
| **Vitest** | TS unit tests |
| **k6 + Tauri-driver** | E2E UI testing; real clicks on shell |
| **afl.rs / libfuzzer (cargo fuzz)** | Continuous fuzzing |
| **proptest / hypothesis / fast-check** | Property tests |
| **loom / shuttle** | Concurrency model checking |
| **SonarQube** | Static analysis, test coverage aggregation |
| **DefectDojo** | Security findings aggregation + deduplication |
| **OpenTelemetry tracing in tests** | Per-test trace in Jaeger; debug failures with waterfall |

---

## 14. Test Result Quality Gates (Merge Pipeline)

| Stage | Failure Type | Action |
|-------|--------------|--------|
| Compile + Clippy | Any error | ❌ Block PR |
| Unit tests | Any failure | ❌ Block PR |
| Property tests (short) | Any failure | ❌ Block PR |
| Coverage | < 80% line, or public API < 100% | ❌ Block PR, with exceptions |
| Static: semgrep / cargo-audit HIGH | New finding | ❌ Block PR until remediated |
| Proto breaking changes | buf breaking reports any | ❌ Block PR (major version ONLY with TSC) |
| Integration tests | Any failure | ❌ Block PR |
| Performance benchmarks | >5% regression p95 hot path | ❌ Block PR until perf justified or fixed |
| Fuzz (nightly only) | New crash | ⛔ Halts release train |
| Pen test (release only) | CRITICAL/HIGH unremediated | ⛔ Block release |
| E2E (nightly only) | Critical path failure | ⛔ Block release candidate |

---

## 15. Bug Tracking & Defect Triage

### Defect Severity Ladder

| Severity | Definition | SLA |
|----------|------------|-----|
| **P0 CRITICAL** | User data loss, security breach active in wild, widespread boot failure, AI dangerous action | 4 hours. Hotfix release within 24h. |
| **P1 HIGH** | Core functionality broken for most users, no workaround, sandbox escape proof of concept | 24h triage; fix in next PATCH (or MINOR within 2 weeks) |
| **P2 MEDIUM** | Workaround exists, broken UI, minor wrong behavior | Fix in next MINOR |
| **P3 LOW** | Cosmetic, docs, UX polish | Best-effort, community PRs welcome |

### Triage Process
- Weekly Triage Monday: all untriaged bugs assigned P0-P3 + owner
- Severity escalations managed by QA lead, override by Architecture Council at any time
