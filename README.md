# GoVail MCP — Foundation V1.1 (Official SDK Delegation)

> **GoVail MCP standardizes how applications expose capabilities and context; it does not own application state or application workflows.**
> **GoVail MCP MUST NOT become an independent implementation of the MCP wire protocol when an official SDK can provide that responsibility.**

GoVail MCP is an application-owned MCP interoperability foundation providing standard Rust contracts, server SDK, protocol binding, and reference contract examples.

---

## 1. Repository Independence

- **`govail-mcp` is an independent Rust library crate workspace.** It is NOT the `govail` orchestration platform monorepo (`~/srv/govail`).
- GoVail core orchestration platform and individual applications (Promptia, Quant, etc.) consume `govail-mcp` as an external dependency (Crate).
- `govail-mcp` does not contain application databases, business engines, or orchestration logic.

---

## 2. Protocol Baseline & Official SDK Delegation

- **Primary Compatibility Baseline**: `MCP 2025-11-25` (legacy `initialize` handshake protocol).
- **Official SDK Delegation**: MCP Wire protocol, JSON-RPC 2.0 serialization/framing, lifecycle state machine, and stdio transport are delegated to the official Rust SDK (`rmcp 3.1.2`).
- **Interoperability Tested**: Verified end-to-end via `rmcp::service::serve_client` <-> `GovailMcpServer` (`rmcp::service::serve_server`).

---

## 3. Repository Layout

```text
govail-mcp/
├── crates/
│   ├── govail-mcp-contracts/    # Standard ContextEnvelope, Provenance, Capability metadata contracts
│   ├── govail-mcp-sdk/          # High-level GovailMcpServer builder, ToolHandler & rmcp ServerHandler adapter
│   └── govail-mcp-core/         # MCP schema utilities & protocol baseline types
│
├── examples/
│   └── promptia-context/        # Reference Contract Example (in-memory mock fixture)
│
├── docs/
│   ├── architecture.md          # Architecture invariants, topology, wire delegation & boundary rules
│   └── contracts.md             # Standard MCP JSON contract specification
│
└── agents/                      # AI collaboration guides & rules
```

---

## 4. Quick Start

### Build all workspace crates

```bash
cargo build --workspace
```

### Run comprehensive conformance tests (G1 ~ G7)

```bash
cargo test --workspace
```

### Run Promptia Reference MCP Server (stdio mode)

```bash
cargo run --example promptia-context
```

---

## 5. Milestone Status

- [x] **V1.0 Foundation**: Standard contracts, envelope definitions, and boundary invariants.
- [x] **V1.1 Official SDK Delegation**: `rmcp` delegation, G1~G7 Conformance & Interop gates 100% passed.
- [ ] **NEXT (V1.2 / Integration V1)**: `Promptia Production MCP Integration` (Real database / storage binding in Promptia application repo).
