# CognyxOS

An AI-native operating system where the AI Agent is the Operating System. Users interact with **Intent**, not Linux. The system converts Intent into execution across multiple runtime environments.

## Architecture Overview

CognyxOS is built on a strict six-layer architecture:

```
Layer 6: Applications (Intent manifests, sandbox profiles)
    ↓
Layer 5: Execution Runtimes (Linux, Windows, macOS, Android, Cloud)
    ↓
Layer 4: Capability Runtime (Universal capability adapters)
    ↓
Layer 3: Agent Kernel (Intent parsing, scheduling, state management)
    ↓
Layer 2: Virtualization Platform (Micro-VMs, containers)
    ↓
Layer 1: Linux Host (Immutable, hardened kernel)
    ↓
Layer 0: Hardware (CPU, GPU, TPU, NPU, I/O)
```

## Directory Structure

```
cognyx-os/
├── layer0-hardware/           # Hardware abstraction specifications
├── layer1-linux-host/         # Immutable Linux host
│   ├── kernel-modules/        # GPU, network, storage drivers
│   └── security/              # SELinux, AppArmor, secure boot
├── layer2-virtualization/     # Hypervisor and VM management
│   ├── hypervisor/            # Firecracker, Kata Containers config
│   └── vm-images/             # Linux, Windows, macOS, Android images
├── layer3-agent-kernel/       # Core orchestration
│   ├── intent-parser/         # NLU, graph builder, validator
│   ├── state-manager/         # Session store, global state
│   ├── scheduler/             # DAG executor, priority queue
│   └── identity/              # Service identities, auth
├── layer4-capability-runtime/ # Universal capability adapters
│   ├── adapters/              # Vision, input, audio, system, etc.
│   ├── registry/              # Capability discovery, health
│   └── protobuf/              # Protocol definitions
├── layer5-execution-runtimes/ # OS-specific implementations
│   ├── linux/                 # X11, Wayland, DBus, systemd
│   ├── windows/               # UI Automation, PowerShell
│   ├── macos/                 # AppleScript, Accessibility API
│   ├── android/               # ADB, Instrumentation
│   └── cloud/                 # Browser automation, containers
├── layer6-applications/       # Application manifests
│   ├── manifests/             # YAML application definitions
│   └── sandbox/               # Sandbox profiles, policies
├── _infra/                    # Cross-cutting concerns
│   ├── event-bus/             # NATS JetStream configuration
│   ├── observability/         # OpenTelemetry, metrics, logging
│   ├── security/              # Zero trust, mTLS, OPA policies
│   └── config/                # Configuration management
├── docs/                      # Documentation
└── scripts/                   # Build and deployment scripts
```

## Key Documents

| Document | Description |
|----------|-------------|
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | Complete six-layer architecture specification |
| [EVENT_BUS_ARCHITECTURE.md](_infra/event-bus/EVENT_BUS_ARCHITECTURE.md) | NATS JetStream messaging infrastructure |
| [SECURITY_MODEL.md](_infra/security/SECURITY_MODEL.md) | Zero trust security architecture |
| [OBSERVABILITY_ARCHITECTURE.md](_infra/observability/OBSERVABILITY_ARCHITECTURE.md) | Tracing, metrics, and logging framework |
| [capabilities.proto](layer4-capability-runtime/protobuf/capabilities.proto) | Universal capability interface definition |

## Core Capabilities

The system exposes these universal capabilities to AI agents:

### Vision
- `read_screen()` - Capture screen content
- `detect_objects()` - Identify UI elements
- `ocr()` - Extract text from images

### Input
- `click(x, y)` - Mouse click at coordinates
- `type(text)` - Keyboard input
- `scroll(delta)` - Scroll operation
- `hotkey(keys)` - Keyboard shortcuts

### Audio
- `record_audio()` - Capture audio
- `play_audio()` - Play audio
- `speech_to_text()` - Transcribe speech
- `text_to_speech()` - Synthesize speech

### System
- `open_application(name)` - Launch applications
- `close_application()` - Terminate applications
- `list_processes()` - Enumerate processes

### Filesystem
- `read_file(path)` - Read file contents
- `write_file(path, content)` - Write files
- `list_directory(path)` - List directory contents

### Network
- `http_request()` - HTTP client
- `websocket_connect()` - WebSocket connections

### Clipboard
- `copy()` - Copy to clipboard
- `paste()` - Paste from clipboard

### Notifications
- `send_notification()` - Display notifications
- `listen_notifications()` - Monitor notifications

## Design Principles

1. **Intent-First**: Users express what they want; the system determines how
2. **Capability Abstraction**: Agents see only capabilities, never OS specifics
3. **Zero Trust**: Every component verified, authenticated, authorized
4. **Immutable Infrastructure**: Layers 0-2 are read-only except for updates
5. **Distributed by Design**: Local, cloud, and remote execution are first-class

## Communication Patterns

- **Internal IPC**: gRPC over Unix sockets, VSOCK for VMs
- **Event Bus**: NATS JetStream for async messaging
- **External APIs**: REST, GraphQL, WebSocket
- **Streaming**: gRPC streaming for real-time data

## Security Model

- **mTLS**: All service-to-service communication encrypted
- **Capability-Based Access Control**: Fine-grained permissions
- **Human-in-the-Loop**: Approval required for critical operations
- **Audit Everything**: All actions logged and traceable
- **Multiple Isolation Layers**: VM, container, process isolation

## Observability

- **Distributed Tracing**: OpenTelemetry across all layers
- **Metrics**: Prometheus with custom capability metrics
- **Logging**: Structured JSON logs with Loki aggregation
- **Session Recording**: Full session capture for debugging

## Supported Runtimes

| Runtime | Status | Implementation |
|---------|--------|----------------|
| Linux (Native) | Ready | X11/Wayland, DBus, systemd |
| Windows (VM) | Planned | UI Automation, PowerShell |
| macOS (VM) | Planned | AppleScript, Accessibility API |
| Android (VM) | Future | ADB, Instrumentation |
| Cloud | Future | Browser automation, containers |
| Remote | Future | Mesh networking, federation |

## Getting Started

This repository contains the architectural specification for CognyxOS. Implementation will follow this specification strictly.

### Prerequisites

- Understanding of distributed systems
- Familiarity with virtualization technologies
- Knowledge of security best practices

### Next Steps

1. Review the complete [architecture specification](docs/ARCHITECTURE.md)
2. Understand the [security model](_infra/security/SECURITY_MODEL.md)
3. Study the [capability interface](layer4-capability-runtime/protobuf/capabilities.proto)
4. Examine the [event bus design](_infra/event-bus/EVENT_BUS_ARCHITECTURE.md)
5. Review the [observability strategy](_infra/observability/OBSERVABILITY_ARCHITECTURE.md)

## License

Proprietary - All rights reserved

## Contributing

This is an internal architecture specification. External contributions are not accepted at this time.
