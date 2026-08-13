# CognyxOS Plugin SDK Specification

> **Document ID:** SDK-001
> **Version:** 1.0.0
> **Status:** Phase 0 - Approved
> **Last Updated:** 2026-08-01
> **Owner:** Extensibility Team

---

## Table of Contents

1. [Plugin Model Overview](#plugin-model-overview)
2. [Extension SDK Architecture](#extension-sdk-architecture)
3. [Plugin Kinds](#plugin-kinds)
4. [WebAssembly Plugin Development (Rust)](#webassembly-plugin-development-rust)
5. [WebAssembly Plugin Development (TypeScript)](#webassembly-plugin-development-typescript)
6. [WIT Interface Definitions](#wit-interface-definitions)
7. [Plugin Capability Declarations](#plugin-capability-declarations)
8. [Plugin Lifecycle](#plugin-lifecycle)
9. [UI Extension Points](#ui-extension-points)
10. [Tool Plugins for AI Agents](#tool-plugins-for-ai-agents)
11. [Search Provider Plugins](#search-provider-plugins)
12. [Distribution, Signing & Marketplace](#distribution-signing--marketplace)

---

## Plugin Model Overview

Plugins are the primary extension mechanism for CognyxOS. They allow third-party developers to extend the OS without compromising the security model.

**Why WebAssembly (Wasm)?**
1. Memory-safe sandbox. No undefined behavior escapes; no RCE risk.
2. Deterministic execution. Identical bytecode = identical output across platforms.
3. Near-native performance. Wasmtime + Wizer pre-initialization = microsecond startup.
4. Language-agnostic. Compile to Wasm from Rust, C/C++, Zig, Go, TypeScript, Swift, Kotlin, Python, C#, Grain, etc.
5. Fine-grained host functions. Plugins only get the host calls their capabilities grant.

### Plugin Sandbox Model

```
┌──────────────────────────────────────────────────────────────┐
│                      Wasm Instance                            │
│  Linear Memory: Bounded size                                  │
│  Stack: Isolated                                              │
│  WASI: Files only in approved paths + socket only if granted │
├──────────────────────────WIT Boundary─────────────────────────┤
│  Host Functions (imports granted per declared capability):    │
│    cognyx:bus/send_command                                    │
│    cognyx:bus/subscribe_events                                │
│    cognyx:filesystem/open (if capability filesystem.read)     │
│    cognyx:ai/generate_text (if capability ai.generate_text)   │
│    cognyx:notification/send                                   │
│    cognyx:ui/* (if UI extension declared)                     │
│    wasi:clocks/*, wasi:random/* (always)                      │
├──────────────────────────────────────────────────────────────┤
│  CognyxOS Plugin Host (Wasmtime, trusted)                     │
└──────────────────────────────────────────────────────────────┘
```

---

## Extension SDK Architecture

### SDK Packages Per Language

| SDK Language | Package Name | Capability Support | Maturity |
|--------------|--------------|--------------------|----------|
| Rust | `cognyx-plugin-sdk = "0.1"` | Full | Stable |
| TypeScript | `@cognyxos/plugin-sdk` | Full | Stable |
| Python | `cognyx_plugin_sdk` | Basic | Beta |
| C++ | `libcognyx-plugin-sdk` | Basic | Beta |
| Go | `github.com/cognyxos/plugin-sdk-go` | Basic | Alpha |

### SDK Dependencies (Rust)

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
cognyx-plugin-sdk = { version = "0.1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[profile.release]
opt-level = "z"           # Optimize for size (Wasm code size)
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

---

## Plugin Kinds

| Kind | Trait/Interface | What it does | Use Case |
|------|-----------------|--------------|----------|
| **UI Extension** | `UiExtension` | Adds components to shell UI | Widgets, settings panels, sidebar entries |
| **Tool** | `AiTool` | Exposes callable functions to AI agents | GitHub API client, image processor, calculator |
| **Search Provider** | `SearchProvider` | Extends search with external sources | Intranet, SaaS provider, legacy DB search |
| **Command Handler** | `CommandHandler` | Responds to custom user commands | Custom shell `:command` |
| **Event Listener** | `EventListener` | Reacts to system events (no UI) | Workflow automation, backup triggers, sync |
| **Indexer** | `FileIndexer` | Custom file-type extraction for search | CAD file, proprietary format indexing |
| **Authenticator** | `Authenticator` | SSO / custom auth method | SAML login, smart card, hardware dongle |

A single plugin `.wasm` can implement multiple kinds simultaneously.

---

## WebAssembly Plugin Development (Rust)

### Minimal Example: AI Tool Plugin

```rust
//! A sample plugin that provides an AI-callable "markdown_to_pdf" tool.

use cognyx_plugin_sdk::prelude::*;

// --- Declare manifest (capabilities, metadata) ---
declare_plugin_manifest! {
    id: "com.example.markdownpdf",
    display_name: "Markdown to PDF Converter",
    version: "1.0.0",
    min_os_version: "0.2.0",
    author: "Example Inc <support@example.com>",
    description: "Converts Markdown files to beautiful PDFs via AI agent tools.",
    tags: ["pdf", "markdown", "document"],
    capabilities: [
        required [
            "ai.tool_register",
            "filesystem.read:/workspaces/**/*.md",
            "filesystem.write:/workspaces/**/*.pdf"
        ],
        optional [
            "notification.send"
        ]
    ],
    // Declares this plugin implements an AI Tool
    tools: [MarkdownToPdf]
}

// --- Define the tool with typed input/output schema ---
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
struct MarkdownToPdfInput {
    /// Absolute path to the input markdown file within the workspace
    #[schemars(example = "example_path")]
    input_path: String,

    /// Where to write the output PDF; overwritten if exists
    output_path: String,

    /// Paper size for output
    #[serde(default)]
    paper_size: PaperSize,
}

fn example_path() -> &'static str { "/workspaces/123/docs/notes.md" }

#[derive(Serialize, Deserialize, JsonSchema, Debug, Default)]
#[serde(rename_all = "lowercase")]
enum PaperSize { A4, Letter, Legal, #[default] A4 }

#[derive(Serialize, Deserialize, Debug)]
struct MarkdownToPdfOutput {
    output_path: String,
    size_bytes: u64,
    pages: u32,
}

// --- Implement the tool trait ---
#[tool(
    name = "markdown_to_pdf",
    description = "Convert a Markdown document to a beautifully typeset PDF document."
)]
async fn markdown_to_pdf(ctx: ToolContext, input: MarkdownToPdfInput) -> Result<MarkdownToPdfOutput, ToolError> {
    // Use SDK to read file (subject to declared filesystem caps)
    let md_content = ctx.fs.read_to_string(&input.input_path).await?;

    // (Plugin does its conversion logic here - local to Wasm)
    let pdf_bytes = convert_markdown(&md_content, input.paper_size)
        .map_err(|e| ToolError::Internal(e.to_string()))?;

    // Write result
    ctx.fs.write_all(&input.output_path, &pdf_bytes).await?;

    // If user granted optional notification cap, tell them
    if let Ok(notif) = ctx.notification.as_ref() {
        let _ = notif.send(format!(
            "✅ PDF generated: {} ({} bytes, {} pages)",
            input.output_path, pdf_bytes.len(), count_pages(&pdf_bytes)
        )).await;
    }

    Ok(MarkdownToPdfOutput {
        output_path: input.output_path,
        size_bytes: pdf_bytes.len() as u64,
        pages: count_pages(&pdf_bytes),
    })
}

// Boilerplate: register main
cognyx_plugin_entrypoint!();
```

### Build It

```bash
# Add wasm target
rustup target add wasm32-wasip2

# Build .wasm (optimized for size)
cargo build --release --target wasm32-wasip2

# Optimize with wasm-opt (shave ~40% off typical size)
wasm-opt -Os -o target/wasm32-wasip2/release/my-plugin.wasm \
    target/wasm32-wasip2/release/my-plugin.wasm

# Verify with cognyx CLI
cognyx plugin inspect target/wasm32-wasip2/release/my-plugin.wasm
# → Manifest, declared capabilities, declared host imports

# Sign with your Ed25519 developer key
cognyx plugin sign --key ./my-dev-key.ed25519 my-plugin.wasm
```

---

## WebAssembly Plugin Development (TypeScript)

### Using Jco + Component Model

```ts
// src/lib.ts
import { plugin, ToolContext, registerTool, JsonSchema } from "@cognyxos/plugin-sdk";

// Define manifest via decorator or manifest.json next to the package
@plugin({
  id: "com.example.weather",
  displayName: "Weather Lookup",
  version: "0.2.0",
  capabilities: {
    required: [
      "ai.tool_register",
      "network.outbound:https://api.weather.example/**"
    ]
  }
})
export default class WeatherPlugin {
  @registerTool({
    name: "get_current_temperature",
    description: "Gets current temperature at a location using the user's preferred units."
  })
  async getTemperature(
    ctx: ToolContext,
    input: { city: string, countryCode: string, units?: "celsius" | "fahrenheit" }
  ): Promise<{ temperature: number; units: string; description: string }> {
    const resp = await ctx.fetch(
      `https://api.weather.example/weather?q=${encodeURIComponent(input.city)},${input.countryCode}`
    );
    const data = await resp.json();
    return {
      temperature: input.units === "fahrenheit" ? data.main.temp_f : data.main.temp_c,
      units: input.units || "celsius",
      description: data.weather[0].description
    };
  }
}
```

Build with `cognyx-plugin build` (uses Jco to componentize TS to Wasm).

---

## WIT Interface Definitions

The canonical host↔plugin ABI is defined in **WIT (WebAssembly Interface Types)** format, versioned.

File: `interfaces/wit/cognyx/plugin.wit`

```wit
package cognyx:plugin@0.1.0;

// ------------------ Common types ------------------
world plugin-world {
  // Every plugin MUST export: entry-point and metadata
  export cognyx:plugin/metadata;
  export cognyx:plugin/entry-point;

  // Host imports are granted per-declared-capability:
  import cognyx:bus/message-sender;         // If plugin declares bus.* capability
  import cognyx:filesystem/read;             // If filesystem.read declared
  import cognyx:filesystem/write;            // If filesystem.write declared
  import cognyx:notification/sender;         // If notification.send declared
  import cognyx:ai/tool-registrar;           // If ai.tool_register declared
  import cognyx:search/provider;             // If search.register declared
  import cognyx:ui/extension-host;           // If ui.extension declared

  // Standard WASI (always imported, fundamental runtime env)
  import wasi:clocks/monotonic-clock@0.2.0;
  import wasi:clocks/wall-clock@0.2.0;
  import wasi:random/random@0.2.0;
}

interface metadata {
  get-manifest: func() -> manifest;
}

interface entry-point {
  start: func(init-params: init-parameters) -> result<_, error>;
  stop: func() -> result<_, error>;
}

record manifest {
  id: string,
  display-name: string,
  version: string,
  min-os-version: string,
  description: string,
  tags: list<string>,
  declared-capabilities: list<capability-declaration>,
  entry-points: list<entry-point-info>,
}

record capability-declaration {
  capability-name: string,
  required: bool,
  resource-pattern: option<string>,
  justification: option<string>,
}

// ------------------ AI Tool interface ------------------
interface tool-registrar {
  register-tool: func(schema: tool-schema) -> result<tool-id, error>;
  call-tool-response: func(call-id: string, result: tool-call-result);
}

record tool-schema {
  name: string,
  description: string,
  input-json-schema: string,
  output-json-schema: string,
}

// ------------------ Filesystem ------------------
interface read {
  read-text: func(path: string) -> result<string, error>;
  read-bytes: func(path: string) -> result<list<u8>, error>;
  list-dir: func(path: string) -> result<list<dir-entry>, error>;
  stat: func(path: string) -> result<file-stat, error>;
  exists: func(path: string) -> bool;
}

interface write {
  write-text: func(path: string, data: string, create: bool, overwrite: bool) -> result<u64, error>;
  write-bytes: func(path: string, data: list<u8>, create: bool, overwrite: bool) -> result<u64, error>;
  mkdir: func(path: string, parents: bool) -> result<_, error>;
  remove: func(path: string) -> result<_, error>;
}

// (... and so on for every host interface)
```

---

## Plugin Capability Declarations

### Manifest Format

```toml
# plugin.toml (inside bundle, or compiled into wasm custom section)
[plugin]
id = "com.example.foo"
display_name = "Foo Plugin"
version = "1.2.3"
min_os_version = "0.2.0"

[[capability]]
name = "filesystem.read"
required = true
resource_pattern = "/workspaces/**/*.md"
justification = "To read markdown files for PDF conversion"

[[capability]]
name = "filesystem.write"
required = true
resource_pattern = "/workspaces/**/*.pdf"
justification = "To write converted PDF output"

[[capability]]
name = "network.outbound"
required = false
resource_pattern = "https://api.example.com/**"
justification = "Optional: upload generated PDFs to example.com cloud storage"
```

### Capability Matching at Install

The user is shown:
- ✅ Required capabilities (must all be accepted to install)
- ⚙️ Optional capabilities (user toggles per capability)

At runtime: attempting to use a host function without the matching declared → trap in Wasm (deterministic failure; plugin can catch and recover).

---

## Plugin Lifecycle

```
INSTALL → VERIFY (signature + cap audit) → USER CONSENT (capabilities shown)
    → REGISTER (manifest stored) → ENABLE →
        LOAD Wasm MODULE → INSTANTIATE (with granted host functions)
            → START() → RUNNING ⇄ PAUSED → STOP() → UNLOAD
```

---

## UI Extension Points

Extend the AI Shell UI with React components bundled as pre-compiled component WASM components or via iframe + MessageBus bridge.

```ts
// ui-extension.tsx
import { UiExtension, ShellSidebarItem, ShellWidget } from "@cognyxos/plugin-sdk/ui";

export default class MyUIExt extends UiExtension {
  getSidebarItems(): ShellSidebarItem[] {
    return [
      {
        id: "my-foo-view",
        label: "Foo Dashboard",
        icon: "📊",
        route: "/plugins/com.example.foo/dashboard",
        component: import("./Dashboard.jsx")
      }
    ];
  }

  getDashboardWidgets(): ShellWidget[] {
    return [{ id: "foo-stats", gridSize: [2, 1], component: <StatsWidget /> }];
  }
}
```

---

## Tool Plugins for AI Agents

AI tools expose structured functions the planning engine can call. Key rules:
- Input/output MUST have JSON Schema; LLM validates parameters before calling
- Execution time limit default: 30 seconds; configurable
- Outputs are returned to the planning engine which verifies them via step-verifier

---

## Search Provider Plugins

Expose external search systems:

```rust
#[search_provider(name = "corporate-sharepoint")]
async fn search_sharepoint(ctx: SearchContext, query: SearchQuery) -> Result<SearchResponse, SearchError> {
    // Use ctx.http (capability-scoped fetch) to hit SharePoint REST API
    // Transform results into CognyxOS SearchResult schema
    Ok(SearchResponse { results: ... })
}
```

---

## Distribution, Signing & Marketplace

### Package Format

```
com.example.foo-1.2.3.cogplug
├── manifest.toml
├── plugin.wasm
├── signature.sig               # Ed25519 of SHA-256(manifest.toml + plugin.wasm)
├── icons/                      # 32, 64, 128, 512 px PNG/SVG
├── schemas/
├── ui/                         # (optional) pre-compiled UI components
└── README.md
```

### Signing & Verification

```bash
# Developer signs on publish
cognyx plugin sign --key ./dev-key.ed25519 ./com.example.foo-1.2.3.cogplug

# Marketplace verifies signature + provenance + reproducible build
# CognyxOS client verifies signature + cert chain before install
```

### Marketplace Levels

| Level | Requirements | Audience |
|-------|--------------|----------|
| **Community** | Signed, manifest, passes static analysis tests | Open to all |
| **Verified** | + Source available, reproducible build, SLSA Level 3 provenance | Enterprise users |
| **Enterprise** | + Security audit, 99.9% uptime SLA on support, indemnification | Large orgs |
