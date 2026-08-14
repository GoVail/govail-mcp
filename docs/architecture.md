# GoVail MCP Architecture — Foundation V1.1 (Official SDK Delegation)

## 0. Repository Boundary & Independence

- **`govail-mcp` is an independent Rust library workspace.** It is NOT the `govail` orchestration platform monorepo (`~/srv/govail`).
- GoVail core orchestration platform and individual applications (Promptia, Quant, etc.) consume `govail-mcp` as an external dependency (Crate).
- `govail-mcp` does not contain application databases, business engines, or orchestration logic.

---

## 1. Core Invariants

> **GoVail MCP standardizes how applications expose capabilities and context; it does not own application state or application workflows.**
> **The application owns the data. The application owns the domain logic. MCP only exposes a controlled interface to them.**
> **GoVail consumes capabilities; it does not absorb their domain responsibility.**
> **GoVail MCP MUST NOT become an independent implementation of the MCP wire protocol when an official SDK can provide that responsibility.**

---

## 2. Protocol Version Pinning & Wire Protocol Delegation

### Protocol Version Target
- **Primary Baseline**: `MCP 2025-11-25` (Legacy baseline using `initialize` handshake protocol).
- **Future Target**: `MCP 2026-07-28` (Modern per-request metadata).

### Protocol Ownership Boundary (V1.1 Delegation Complete)
```text
Application Domain Protocol
  → Owned by Application (e.g. Promptia, Quant)

MCP Wire Protocol (JSON-RPC 2.0 / Stdio Transport / Protocol State Machine)
  → Owned by official Rust SDK (`rmcp 3.1.2`)

GoVail MCP Interoperability Layer
  → Owned by `govail-mcp` (ContextEnvelope, Provenance, CapabilityMetadata, Tracing, Error Normalization, Server SDK Façade)
```

---

## 3. Target Topology

```text
                 ┌─────────────────────┐
                 │     GoVail Core     │
                 │ prompt/intelligence │
                 │ governance boundary │
                 └──────────┬──────────┘
                            │
                      tool discovery / execution
                            │
                 ┌──────────▼──────────┐
                 │     govail-mcp      │
                 │                     │
                 │ Contracts / SDK     │
                 │ Interop conventions │
                 │ Reference impl      │
                 └──────────┬──────────┘
                            │
             ┌──────────────┼──────────────┐
             ▼              ▼              ▼
         Promptia          Quant          DART...
          MCP              MCP             MCP
             │              │               │
          SQLite          DB/API         OpenAPI
          Lore DB       Market State
```

---

## 4. Ownership Model & Reference Classification

### `govail-mcp` owns:
- MCP protocol interoperability contracts & schema conventions
- Context response envelope (`ContextEnvelope<T>`)
- Provenance and freshness metadata structures
- Standardized tool error envelope (`ToolError`)
- Server SDK and stdio/transport harness (`GovailMcpServer`)
- Capability classification (READ vs ACTION)
- Reference contract examples (`examples/promptia-context/`)

### Reference Classification:
- `examples/promptia-context/` is classified as a **Reference Contract Example** using isolated in-memory fixtures.
- Real application database adapters (e.g. SQLite / PostgreSQL) in production belong to **Reference Integrations** owned by the target application repository.

---

## 5. Context Integration Flow (Push vs Pull)

`govail-mcp` supports both Push and Pull context modes:

- **Pull Context**: GoVail / Agent dynamically invokes an Application MCP tool to retrieve context on demand.
- **Push Context**: Application pre-assembles its state into a `ContextEnvelope` and passes it directly to GoVail.

Neither mode is strictly preferred; applications choose based on latency and interaction requirements.

---

## 6. Non-Goals for V1

- Centralized application database or domain runtime
- Custom independent wire-protocol engine competing with official `rmcp`
- Universal memory or agent orchestration engine
- Remote HTTP/TLS hosting infrastructure in core crates
- Automatic write operation authorization without application governance
