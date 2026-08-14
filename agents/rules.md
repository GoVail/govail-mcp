# GoVail MCP Project Rules

## Boot order

1. Read this file.
2. Read `docs/architecture.md`.
3. Check target sub-crate specs (`crates/govail-mcp-contracts`, `crates/govail-mcp-sdk`, `crates/govail-mcp-core`).

## Core Invariants & Boundaries

- **Repository Separation**: `govail-mcp` is an independent Rust library crate repository. It is NOT the `govail` orchestration platform monorepo (`~/srv/govail`).
- **GoVail MCP standardizes how applications expose capabilities and context; it does not own application state or application workflows.**
- **GoVail MCP MUST NOT become an independent implementation of the MCP wire protocol when an official SDK can provide that responsibility.**
- The application owns the data, DB schema, domain state, and domain workflows.
- `govail-mcp` provides shared contracts, capability metadata, tool schema conventions, error standards, and server SDKs.
- `govail-mcp` MUST NOT contain central application databases or central domain logic.
- Reference implementations (e.g. `examples/promptia-context/`) act as **Reference Contract Examples** with mock/fixture data. Real database integrations belong to the application repositories.

## Protocol Version Pinning & Delegation

- **Baseline**: MCP Spec `2025-11-25` (legacy `initialize` handshake protocol).
- **Delegation Status (V1.1 CLOSED)**: Wire protocol, JSON-RPC serialization, and stdio transport are delegated to official `rmcp` Rust SDK (3.1.2).

## Security & Trust Model

- Models propose plans and actions; deterministic application policy authorizes and executes.
- Read capabilities and Action capabilities are strictly separated.
- Action capabilities require principal, correlation ID, risk rating, and governance metadata bindings.

## Development Standards

- Keep Rust workspace code clean and warning-free.
- Run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` before completing tasks.
- Keep tests hermetic. Reference contract examples must use isolated in-memory fixtures.
