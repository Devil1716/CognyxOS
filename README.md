# CognyxOS

**An AI Agent Runtime and Desktop Environment**

CognyxOS is **NOT a traditional operating system**. It is an AI-native runtime platform and desktop environment that runs on top of existing operating systems (Linux, Windows, macOS).

## What CognyxOS Is

✅ **AI Agent Runtime** - Framework for building and orchestrating AI agents  
✅ **Desktop Environment** - React-based UI for monitoring and controlling agents  
✅ **Virtualization Manager** - Python wrappers for QEMU/KVM VM management  
✅ **Capability Abstraction Layer** - Unified interface for cross-platform operations  
✅ **Configuration Framework** - YAML-based system configuration  

## What CognyxOS Is NOT

❌ A bootable operating system  
❌ A custom kernel  
❌ A replacement for Linux/Windows/macOS  
❌ A hypervisor (uses existing KVM/QEMU)  

## Architecture

CognyxOS follows a 6-layer architecture (see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)):

```
Layer 6: Applications (React desktop, agent tools)
    ↓
Layer 5: Execution Runtimes (Python VMM, capability adapters)
    ↓
Layer 4: Capability Runtime (Protocol buffers, gRPC)
    ↓
Layer 3: Agent Kernel (Orchestration - planned)
    ↓
Layer 2: Virtualization (QEMU/KVM via libvirt)
    ↓
Layer 1: Host OS (Linux/Windows/macOS)
    ↓
Layer 0: Hardware
```

## Quick Start

### Prerequisites

- Node.js 24+
- Python 3.12+
- pnpm 10+

### Installation

```bash
pnpm install
python -m pip install -e ./python/cognyx_runtime[test]
```

### Run Tests

```bash
pnpm test
```

### Development

```bash
cd apps/desktop
pnpm dev
```

## Project Structure

```
cognyx-os/
├── apps/                    # Applications
│   └── desktop/             # React dashboard UI
├── cognyx-host/             # Host integration (Linux kernel modules, VMM)
│   ├── kernel/              # Linux kernel module stubs
│   ├── vmm/                 # VM lifecycle management
│   └── virt/                # Virtualization platform
├── core/                    # Core services (planned)
├── docs/                    # Architecture documentation
├── packages/                # Shared libraries
└── python/                  # Python runtime
    └── cognyx_runtime/      # Runtime foundation
```

## Current Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| Desktop UI | ✅ Complete | React dashboard with mock metrics |
| Python Runtime | ✅ Complete | Configuration, logging, events, IPC |
| VMM Scripts | ✅ Complete | QEMU/KVM lifecycle management |
| Kernel Modules | ⚠️ Stubs | Cannot compile without kernel build |
| Agent System | ❌ Planned | Architecture defined, not implemented |
| Bootloader | ❌ N/A | Uses host OS bootloader |

## Testing

```bash
# Run all tests
pytest python/cognyx_runtime/tests/

# Coverage report
pytest --cov=cognyx_runtime
```

Current test coverage: **91%**

## Documentation

- [Architecture Specification](docs/ARCHITECTURE.md)
- [Security Model](cognyx-os/_infra/security/SECURITY_MODEL.md)
- [Event Bus Design](cognyx-os/_infra/event-bus/EVENT_BUS_ARCHITECTURE.md)
- [Observability](cognyx-os/_infra/observability/OBSERVABILITY_ARCHITECTURE.md)
- [Capability Protocol](cognyx-os/layer4-capability-runtime/protobuf/capabilities.proto)

## Roadmap

### Phase 1 (Complete)
- ✅ Architectural specification
- ✅ Desktop UI prototype
- ✅ Python runtime foundation
- ✅ VMM management scripts

### Phase 2 (In Progress)
- 🔄 Capability runtime implementation
- 🔄 Agent kernel orchestration
- 🔄 Cross-platform adapters

### Phase 3 (Planned)
- Agent swarm implementation
- Distributed execution
- Memory and learning systems

## License

Apache 2.0

## Contributing

This is an open-source project. Contributions welcome!

---

**Note:** CognyxOS requires an existing operating system to run. It is not a standalone OS.
