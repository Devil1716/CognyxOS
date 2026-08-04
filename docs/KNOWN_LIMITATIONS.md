# Known Limitations and Roadmap

**Last Updated:** August 4, 2025

This document explicitly lists what CognyxOS cannot do today and the realistic path forward.

---

## Current Limitations

### P0: Cannot Boot on Bare Metal

**Issue:** CognyxOS has no custom kernel and cannot boot independently.

**Evidence:**
- `cognyx-host/boot/grub.cfg` references non-existent `vmlinuz-cognyx`
- Kernel modules in `cognyx-host/kernel/` require Linux 6.8+ to load
- No bootloader implementation exists

**Workaround:** Run CognyxOS on top of existing Linux/Windows/macOS systems.

**Timeline:** Not planned. Project is an OS extension, not a replacement.

---

### P0: No Custom Memory Management

**Issue:** Relies entirely on host OS memory management.

**Evidence:**
- `memguard.c` is a Linux kernel module, not standalone MMU
- No page table implementation
- No virtual memory subsystem

**Impact:** Cannot provide memory isolation beyond what Linux provides.

---

### P0: No Process Scheduler

**Issue:** Uses Linux CFS scheduler exclusively.

**Evidence:**
- `cognyx_sched.c` extends CFS with priority hints only
- No independent process creation (`fork`/`exec` not implemented)
- No thread scheduling

**Impact:** Agent workloads scheduled by Linux, not CognyxOS.

---

### P1: Kernel Modules Cannot Compile

**Issue:** Kernel module code references undefined symbols.

**Evidence:**
```c
// cognyx_sched.c line 97
// scx_register_ops(&cognyx_sched_ops);  // COMMENTED OUT
```

**Fix Required:** Either complete BPF scheduler integration or remove kernel modules.

---

### P1: Empty Core Directories

**Issue:** 18 of 20 directories in `/workspace/core/` contain only `.gitkeep` files.

**Missing Components:**
- Agent implementations
- Planner logic
- Scheduler service
- Memory service
- Executor
- Observer
- All tools

**Timeline:** Phase 3 (planned)

---

### P2: Capability Runtime Incomplete

**Issue:** Only ~5 of 40+ defined capabilities have implementations.

**Defined but Not Implemented:**
- Most vision capabilities (`detect_objects`, advanced OCR)
- Audio processing (`speech_to_text`, `text_to_speech`)
- Advanced filesystem operations
- Network capabilities beyond basic HTTP
- USB passthrough
- Full accessibility API coverage

**Timeline:** Phase 2 (in progress)

---

### P2: No Cross-Platform Adapters

**Issue:** Windows and macOS capability adapters are stubs.

**Evidence:**
- `layer5-execution-runtimes/windows/` - empty
- `layer5-execution-runtimes/macos/` - empty
- Only Linux adapter has partial implementation

**Timeline:** Phase 2 (in progress)

---

### P2: No Distributed Execution

**Issue:** All execution is local-only.

**Missing:**
- Remote runtime discovery
- Cloud worker integration
- Mesh networking protocol
- Task migration/checkpointing

**Timeline:** Phase 3 (planned)

---

### P3: No Performance Benchmarks

**Issue:** Architecture claims "< 10μs IPC latency" but no benchmarks exist.

**Required:**
- IPC latency measurements
- VM startup time benchmarks
- GPU passthrough overhead tests
- Memory copy performance

**Timeline:** After Phase 2 completion

---

### P3: Limited Test Coverage

**Issue:** Only Python runtime has tests (91% coverage).

**Missing Tests:**
- VMM integration tests
- QEMU boot tests (CI job added but minimal)
- Capability adapter tests
- End-to-end agent workflows

**Timeline:** Ongoing

---

## What Works Today

✅ **Desktop UI** - React dashboard displays system metrics  
✅ **Python Runtime** - Configuration, logging, events, IPC, lifecycle  
✅ **VMM Scripts** - QEMU/KVM VM lifecycle management (untested)  
✅ **Documentation** - Comprehensive architecture specifications  
✅ **Protocol Definitions** - Well-defined protobuf capabilities  

---

## Realistic Roadmap

### Phase 2 (Q4 2025 - Q2 2026)
**Goal:** Complete core runtime functionality

- [ ] Implement all Linux capability adapters
- [ ] Add Windows capability adapters (via COM/Win32)
- [ ] Add macOS capability adapters (via AppleScript/AXUIElement)
- [ ] Complete agent kernel orchestration
- [ ] Implement planner and scheduler services
- [ ] Add comprehensive integration tests

**Deliverable:** Functional AI agent runtime on Linux/Windows/macOS

---

### Phase 3 (Q3 2026 - Q4 2027)
**Goal:** Agent swarm and distributed execution

- [ ] Implement Executive Agent
- [ ] Build specialist agents (Coder, Researcher, Browser, etc.)
- [ ] Add distributed task execution
- [ ] Implement memory and learning systems
- [ ] Add cloud worker support
- [ ] Build performance benchmark suite

**Deliverable:** Multi-agent system with cloud offloading

---

### Phase 4 (2028+)
**Goal:** Advanced capabilities

- [ ] Long-term memory with vector embeddings
- [ ] Self-improvement and reflection
- [ ] Advanced vision models integration
- [ ] Voice-native interaction
- [ ] Mobile device support

**Deliverable:** Mature AI operating environment

---

## What Will Never Happen

The following are **explicitly out of scope**:

❌ **Custom Kernel** - CognyxOS will always run on existing kernels  
❌ **Bare-Metal Boot** - No UEFI/BIOS bootloader planned  
❌ **Linux Replacement** - Embraces Linux as the foundation  
❌ **Hypervisor Development** - Uses KVM/QEMU, Firecracker  

---

## Honest Classification

CognyxOS is:
- ✅ An AI agent runtime framework
- ✅ A desktop environment for AI interaction
- ✅ A virtualization management layer
- ✅ A capability abstraction platform

CognyxOS is NOT:
- ❌ A standalone operating system
- ❌ A Linux distribution
- ❌ A kernel
- ❌ A hypervisor

---

## Contributing

Contributions are welcome, especially for:

1. **Capability Adapters** - Windows, macOS implementations
2. **Agent Implementations** - Specialist agents for specific tasks
3. **Testing** - Integration and E2E tests
4. **Documentation** - Tutorials, examples, API docs
5. **Performance** - Benchmarking and optimization

Please read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting PRs.

---

**This document is updated quarterly to reflect actual progress.**
