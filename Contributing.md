# CognyxOS Contributing Guide

> **Document ID:** DEV-003
> **Version:** 1.0.0
> **Status:** Phase 0 - Approved
> **Last Updated:** 2026-08-01
> **Owner:** Developer Experience Team

---

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Ways to Contribute](#ways-to-contribute)
3. [Contribution Workflow](#contribution-workflow)
4. [Pull Request Template](#pull-request-template)
5. [Code Review Guidelines](#code-review-guidelines)
6. [Reporting Security Issues](#reporting-security-issues)
7. [Reporting Bugs](#reporting-bugs)
8. [Feature Proposals](#feature-proposals)
9. [Community Resources](#community-resources)

---

## Code of Conduct

CognyxOS is developed in public by a global community. We are committed to a safe, inclusive, respectful environment.

### Our Standards

- Be welcoming and respectful.
- Be collaborative; debate ideas, not people.
- Assume good faith; ask clarifying questions before assuming malice.
- Use welcoming and inclusive language.

### Enforcement

Instances of abusive, harassing, or otherwise unacceptable behavior may be reported to the project maintainers at **conduct@cognyxos.dev**. All complaints will be reviewed and investigated promptly and fairly.

---

## Ways to Contribute

You don't need to write code to contribute meaningfully.

| Contribution Type | How |
|-------------------|-----|
| **Bug Reports** | Open GitHub issue with reproduction steps |
| **Documentation** | Improve docs in `/docs/**`, fix typos, expand examples |
| **Translations** | Help i18n the UI and documentation |
| **Testing** | Run nightly releases, file QA issues |
| **Performance** | Run benchmarks, submit flamegraphs, find regressions |
| **Code (Easy)** | Tags: `good-first-issue`, `help wanted` on GitHub |
| **Code (Hard)** | Larger features; write ADR + RFC first |
| **Security** | Participate in bug bounty (see Reporting Security Issues) |
| **Community** | Help other users on Discord / Discourse |

---

## Contribution Workflow

Follow these steps for code contributions. If you're new, pick an issue marked **good-first-issue**.

### Step 1: Fork & Clone

```bash
# 1. Fork on GitHub
# 2. Clone locally with your fork
git clone git@github.com:<your-username>/cognyxos.git
cd cognyxos

# 3. Add upstream remote so you can pull latest
git remote add upstream git@github.com:cognyxos/cognyxos.git
git fetch upstream

# 4. Create feature branch from upstream/main
git checkout -b feature/my-branch-name upstream/main
```

### Step 2: Make Changes + Test

```bash
# Make your changes
# Then:
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check

# For UI changes:
cd ui && pnpm install && turbo run build test lint
```

### Step 3: Commit Message Format (Conventional Commits)

```
<type>(<scope>): <short summary>
│       │             │
│       │             └─⫸ Summary in present tense. Not capitalized. No period at end.
│       │
│       └─⫸ Commit Scope: services/*, runtime/*, kernel/*, ui/*, sdk/*, docs, proto, scripts
│
└─⫸ Commit Type: build|ci|docs|feat|fix|perf|refactor|test|chore|sec
```

**Examples:**
```
feat(services/workspace): add archive/unarchive support for hibernated workspaces

fix(runtime/ai): LLM multiplexer routes to ONNX when GPU memory exhausted
  Closes: #1234

sec(security/sandbox): block userfaultfd syscall in default seccomp profile
  Security: CVE-2026-XXXX (if applicable)

docs(guides): expand section on workspace capability grants
```

### Step 4: Sign Your Work (DCO)

All commits require a **Developer Certificate of Origin (DCO)** signoff. This certifies you wrote (or have right to submit) the code.

```bash
git commit -s -m "feat(scope): description"
#         ^^^ adds: Signed-off-by: Your Name <your-email@example.com>
```

You must use your real name; pseudonymous contributions are permitted but signoff still required using an email you control.

### Step 5: Push and Open PR

```bash
git push -u origin feature/my-branch-name
# Then: Open Pull Request on GitHub from your fork → cognyxos/cognyxos:main
```

Fill in the PR template (below).

---

## Pull Request Template

```markdown
## Summary

<!-- One-paragraph high-level summary of what this PR changes. -->

## Motivation

<!-- Why are we doing this? Link to GitHub issue or RFC. -->

Fixes: # (issue number)
Closes: #
Related: #

## Changes

<!-- Bullet list of main changes. Keep brief; full diff tells the story. -->

- Implemented X
- Refactored Y to use Z
- Added tests for A, B, C

## Checklist

- [ ] My code follows the Coding Standards (see CodingStandards.md)
- [ ] I have added / updated tests (unit + integration where relevant)
- [ ] `cargo build` and `cargo test` pass locally
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] New public APIs have Rustdoc / TSDoc
- [ ] Protocol Buffers are backward compatible (no field reuse)
- [ ] Permissions.md updated if I added new capability namespaces
- [ ] Documentation updated if behavior changed
- [ ] My commits are signed off (DCO `git commit -s`)

## Breaking Changes

Does this PR introduce any breaking API changes?
- [ ] No
- [ ] Yes, and I have updated the major version markers accordingly

## How Has This Been Tested?

Describe the tests that you ran to verify your changes.

- Unit tests:
- Integration tests:
- Manual testing steps:
```

---

## Code Review Guidelines

### We Review For, In Priority Order

1. **Correctness & Security** - Does the code do what it says? Securely?
2. **Architecture Alignment** - Follows Vision.md + Architecture.md principles
3. **Maintainability** - Can a new engineer understand this in 15 minutes?
4. **Performance** - Are hot paths efficient? Is there obvious bloat?
5. **Style Niceties** - Formatting, naming, docs.

### What Reviewers Are NOT Allowed to Block On

- Personal style preferences not in CodingStandards.md ("I prefer X over Y" without a standard to point to)
- Architectural disagreements without reference to approved Architecture docs — raise separately as ADR
- Nit-level formatting issues (fixed by `cargo fmt`; CI should catch and auto-fix)

### Approver Requirements (merge gates)

| PR Size / Impact | Required Approvals |
|------------------|--------------------|
| Docs / Typos | 1 maintainer |
| Bug fix, test | 1 Senior Engineer |
| New feature, non-critical module | 2 Senior Engineers + 1 Architecture Council member |
| Critical service (bus, kernel, security) | 2 Senior Engineers + ALL 3 Architecture Council members + Security Team review |
| Protobuf / API breaking change | Architecture Council + TSC vote |

### Review Response SLA

- First review response within 2 business days for smaller PRs
- Architectural decisions: reviewed in weekly Architecture Council meeting

---

## Reporting Security Issues

**DO NOT open public GitHub issues for security vulnerabilities.** We run a private, coordinated disclosure process.

### How to Report

Email: **security@cognyxos.dev** (PGP key available on website, fingerprint: `...`)

Include in your report:
- Description of the vulnerability
- Step-by-step reproduction
- Affected versions / commit hash
- Expected vs actual behavior
- Proof-of-concept code (if available)
- Your assessment of impact and severity

### What Happens Next

1. Security Team acknowledges within **24 hours**
2. Initial severity assessment within **48 hours**
3. Private GitHub Security Advisory created; you are added as collaborator
4. Fix developed; CVE requested if applicable
5. Release + published disclosure on **first monthly release date after fix (90-day max disclosure window)**

### Bug Bounty Program

Yes, we run a formal bug bounty program with payouts:

| Severity | Payout Range | Example |
|----------|--------------|---------|
| Critical | $5,000 - $50,000+ | Root-level sandbox escape, TPM key extraction |
| High | $1,000 - $10,000 | Cross-workspace data leak, unsigned code exec |
| Medium | $100 - $2,000 | Permission bypass with user interaction required |
| Low | Up to $500 | UI-only bug, theoretical audit log inconsistency |

---

## Reporting Bugs

Use the GitHub Bug Report template. A good bug report contains:

```
## Summary
Short, clear description.

## Steps to Reproduce
1. On what hardware/OS?
2. What CognyxOS version / commit?
3. Exact actions:
   - Step 1: Create workspace 'X'
   - Step 2: Install plugin 'Y'
   - Step 3: Run AI command 'Z'

## Expected Behavior
What should happen?

## Actual Behavior
What actually happens? Screenshots / error logs welcome.

## Environment
- CognyxOS version:
- Kernel version:
- CPU / RAM / GPU:
- Installed plugins/apps:

## Logs
Run `cognyx-collect-logs` and attach generated tarball (sanitize anything private first).
```

---

## Feature Proposals

For non-trivial features (anything more than a small bugfix or single module tweak), propose via **RFC (Request for Comments)** before writing code.

### RFC Process

1. Copy `docs/architecture/0000-template.md` to `docs/architecture/NNNN-my-feature.md` (use next available 4-digit number)
2. Fill in: Problem, Proposal, Rationale, Drawbacks, Alternatives Considered, Unresolved Questions
3. Open PR with just the RFC document (no code yet)
4. Discussion period: minimum 1 week for community comment
5. Outcome:
   - **ACCEPT** → proceed to implementation
   - **REVISE** → address comments and resubmit
   - **REJECT** → closes with reasoning documented

Small improvements and obvious fixes can skip the RFC. When in doubt: ask in the `#dev` channel. You'll be told if an RFC is needed.

---

## Community Resources

| Resource | Location | Purpose |
|----------|----------|---------|
| Website | https://cognyxos.dev | Project homepage, docs portal |
| GitHub | https://github.com/cognyxos/cognyxos | Code, issues, PRs |
| Discord | https://chat.cognyxos.dev | Real-time dev/user chat |
| Discourse | https://forum.cognyxos.dev | Longer discussions, RFCs |
| Monthly Dev Call | YouTube + Calendar link | TSC meetings, open to public |
| Blog | https://cognyxos.dev/blog | Release notes, deep dives |
