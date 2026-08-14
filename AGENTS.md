# GoVail MCP — AI Collaboration Guide

Before design, analysis, or code changes, read `agents/rules.md` and `docs/architecture.md`.

## 1. Repository Independence & Boundary Rule (CRITICAL)

- **`govail-mcp`는 `govail` 메인 모노레포와 완전히 분리된 별개의 독립 리포지토리입니다.**
- **`govail-mcp`의 정체성**: 애플리케이션이 자신의 컨텍스트와 기능을 표준화된 방식으로 노출할 수 있도록 돕는 **순수 Rust 계약(Contracts) 및 Server SDK 라이브러리**입니다.
- **`govail` 메인 모노레포(`~/srv/govail`)와의 관계**: `govail` 메인 플랫폼(또는 Promptia, Quant 등의 애플리케이션)은 `govail-mcp`를 단지 외부 의존성(Crate)으로 소비하는 Client/Consumer일 뿐입니다.
- **도메인 침범 금지**: `govail-mcp` 내부에는 GoVail의 도메인 로직, 중앙 데이터베이스, 특정 애플리케이션의 엔티티/워크플로우가 절대 포함되어서는 안 됩니다.

## 2. Core Invariants

> **GoVail MCP standardizes how applications expose capabilities and context; it does not own application state or application workflows.**
> **The application owns the data. The application owns the domain logic. MCP only exposes a controlled interface to them.**
> **GoVail consumes capabilities; it does not absorb their domain responsibility.**
> **GoVail MCP MUST NOT become an independent implementation of the MCP wire protocol when an official SDK can provide that responsibility.**

## 3. Protocol Target & Official SDK Delegation (V1.1)

- **Protocol Baseline**: `MCP 2025-11-25` (Legacy baseline with `initialize` handshake)
- **Wire Layer Delegation**: 와이어 프로토콜, JSON-RPC 직렬화, 라이프사이클, Stdio 전송 처리는 공식 Rust SDK(`rmcp`)에 100% 위임되었습니다.
- **Crate Layout**:
  - `crates/govail-mcp-contracts`: 공유 컨트랙트 (`ContextEnvelope<T>`, `Provenance`, `Freshness`, `CapabilityMetadata`, `ToolError`)
  - `crates/govail-mcp-sdk`: 고수준 서버 빌더 (`GovailMcpServer`, `ToolHandler`) 및 `rmcp::handler::server::ServerHandler` 어댑터
  - `crates/govail-mcp-core`: 프로토콜 어댑터 및 스키마 유틸리티
  - `examples/promptia-context`: 인메모리 픽스처 기반 참조 컨트랙트 예제 (Reference Contract Example)

## 4. Current Milestone & Next Roadmap

- **V1.0 Foundation**: CLOSED
- **V1.1 Official SDK Delegation**: CLOSED (G1~G7 Conformance & Interop Gates All PASSED)
- **NEXT**: `Promptia Production MCP Integration V1` (Promptia 레포지토리에서의 실제 DB/스토리지 연동 실증)
