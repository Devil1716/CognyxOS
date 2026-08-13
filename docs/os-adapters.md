# OS adapters

`LinuxCapabilityAdapter`, `WindowsCapabilityAdapter`, `MacOSCapabilityAdapter`, and `ContainerCapabilityAdapter` translate universal names to native API concepts. They are only usable through an `AdapterProvider` registered with the universal layer.

```mermaid
flowchart LR
  C[filesystem.read] --> U[Universal capability]
  U --> L[POSIX read]
  U --> W[Win32 file API]
  U --> M[Foundation FileManager]
  U --> X[Container filesystem]
```

Adapter responses are explicitly marked `simulated: true` until a native transport is configured and tested on that target OS.
