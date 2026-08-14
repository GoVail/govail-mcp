# GoVail MCP — Application SDK & Contract Kit

> **GoVail-MCP is a Rust SDK and contract layer for exposing application-owned context and capabilities through MCP without transferring ownership of application state or workflows.**

---

## 1. Core Invariants & Non-Goals

### SSOT Principle
* **The application owns the data, domain logic, workflow, and state.**
* **GoVail MCP owns the MCP exposure conventions, shared contracts, capability metadata, provenance/freshness envelopes, and SDK ergonomics.**
* **GoVail consumes capabilities; it does not absorb application domain responsibility.**

### Non-Goals
GoVail MCP is **NOT**:
* An Agent Runtime or Workflow Engine
* An Application Database Layer or Central Memory Server
* A Search Engine or Generic RAG Platform
* A Custom MCP Wire Protocol Implementation (Wire protocol is 100% delegated to official `rmcp 3.1.2`)

---

## 2. When to Use & When NOT to Use

| 상황 | GoVail-MCP 사용 여부 | 이유 |
| :--- | :---: | :--- |
| 내 애플리케이션의 데이터를 AI 에이전트에게 정규화된 컨텍스트로 제공할 때 | ✅ **사용** | `ContextEnvelope`, `Provenance`, `Freshness` 보장 |
| 내 서비스의 관리/조회 기능을 안전한 권한 메타데이터와 함께 노출할 때 | ✅ **사용** | `CapabilityMetadata`, `RiskLevel`, `ToolError` 표준화 |
| 단순 JSON-RPC 와이어 서버만 독립적으로 빠르게 띄우고 싶을 때 | ⚠️ `rmcp` 직접 사용 | 도메인 메타데이터/엔벨로프 계약이 필요 없다면 직접 사용 권장 |
| 중앙 데이터베이스나 멀티에이전트 오케스트레이션을 구현할 때 | ❌ **사용 금지** | 애플리케이션 또는 GoVail Core 플랫폼 영역임 |

---

## 3. Quick Start: 10-Line MCP Server

```rust
use govail_mcp_sdk::{async_trait, GovailMcpServer, ToolHandler, ToolError, ContextEnvelope};
use serde_json::{json, Value};

struct GetStatusHandler;

#[async_trait]
impl ToolHandler for GetStatusHandler {
    async fn call(&self, _params: Option<Value>) -> Result<Value, ToolError> {
        let envelope = ContextEnvelope::new(
            json!({ "status": "healthy", "uptime_secs": 3600 }),
            "my_service",
            "service:status",
            "get_status",
        );
        Ok(serde_json::to_value(envelope).unwrap())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = GovailMcpServer::builder("my-service-mcp", "0.1.0")
        .tool("get_status", "Get current service health", json!({"type": "object"}), GetStatusHandler)
        .build();

    server.run_stdio().await?;
    Ok(())
}
```

---

## 4. Usage Patterns

### A. Returning Structured `ContextEnvelope<T>` (READ Tool)

```rust
let envelope = ContextEnvelope::new(
    data_payload,
    "promptia",                      // Source application
    format!("story:{}", story_id),   // Resource identifier
    "get_story_context",             // Capability name
);
```

### B. Returning Standardized `ToolError`

```rust
// Resource not found (404)
return Err(ToolError::not_found("Story 101 does not exist"));

// Invalid user arguments (422)
return Err(ToolError::invalid_args("Parameter 'story_id' is required"));

// Unauthorized or Forbidden (401 / 403)
return Err(ToolError::unauthorized("Bearer token is missing or expired"));
return Err(ToolError::forbidden("Insufficient permissions for this story"));

// Recoverable internal error (500, retryable)
return Err(ToolError::internal("Upstream database connection timed out"));
```

---

## 5. Repository Layout

```text
govail-mcp/
├── crates/
│   ├── govail-mcp-contracts/    # ContextEnvelope, Provenance, CapabilityMetadata, ToolError
│   ├── govail-mcp-sdk/          # GovailMcpServer builder, ToolHandler & rmcp ServerHandler adapter
│   └── govail-mcp-core/         # MCP schema utilities & protocol baseline types
│
├── examples/
│   └── promptia-context/        # Reference Contract Example (in-memory mock fixture)
│
└── docs/
    ├── architecture.md          # Architecture invariants & delegation rules
    └── contracts.md             # Standard MCP JSON contract specification
```

---

## 6. Verification & Testing

```bash
# Code formatting & strict lint
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings

# All unit & protocol conformance tests (G1 ~ G7)
cargo test --all

# Release build
cargo build --release
```
