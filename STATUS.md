# GoVail MCP & Promptia Integration — 상태 및 인수인계 현황

> **사용자가 "어디까지 됐니?"라고 질문하면 이 문서를 최우선으로 참조하여 바로 요약 답변할 수 있습니다.**

---

## 1. 프로젝트 목표 (목표는 뭐니?)

- **`govail-mcp`의 정체성**:
  - 특정 애플리케이션의 상태나 워크플로우를 소유하지 않는 **순수 Rust 계약(Contracts) 및 Server SDK 라이브러리**.
  - 와이어 프로토콜, 라이프사이클, Stdio 전송 처리는 **공식 Rust SDK(`rmcp 3.1.2`)에 100% 위임**.
  - `ContextEnvelope<T>`, `Provenance`, `Freshness`, `CapabilityMetadata`, `ToolError` 등 고수준 도메인 인터페이스 제공.
- **`Promptia MCP Integration`의 목적**:
  - `govail-mcp`를 외부 의존성(Crate)으로 소비하는 **최초의 실제 프로덕션 컨슈머(First Production Consumer)** 실증.
  - Promptia의 기존 도메인 경계(PostgreSQL SSOT, FastAPI Application Service)를 완벽히 유지한 채, Read Tool 2개(`get_story_context`, `get_character_state`)를 안전하게 노출하고 SDK Ergonomics를 검증.

---

## 2. 완료 현황 (어디까지 됐니?)

```text
================================================================================
                           Milestone Status Summary
================================================================================
[V1.0] Foundation Architecture & Contracts      [CLOSED] (Envelope, Error, Metadata)
[V1.1] Official SDK Delegation (rmcp)           [CLOSED] (G1~G7 Gates 100% Passed)
[V1.2] Promptia MCP Readiness Assessment        [CLOSED] (Python/FastAPI 진단 완료)
[V1.3] Promptia MCP Integration V1 (Consumer)   [CLOSED / PASS] (Rust Adapter 구축)
================================================================================
```

### 세부 완료 내역:
1. **`govail-mcp` 레포지토리 (`~/srv/govail-mcp`)**:
   - `crates/govail-mcp-contracts`: 공유 계약 정의 완료
   - `crates/govail-mcp-sdk`: `GovailMcpServer` 빌더 및 `rmcp ServerHandler` 어댑터 구현 완료
   - `crates/govail-mcp-core`: 프로토콜 타입 및 스키마 유틸리티 완료
   - `examples/promptia-context`: 참조 픽스처 기반 Conformance Gate (G1~G7) 100% PASS
   - Git 커밋 완료 (`b7405bc`)
2. **`promptia` 레포지토리 (`~/srv/promptia`)**:
   - **Python API (`apps/api`)**:
     - `character_state()` Use-case 및 `CharacterStateResponse` DTO 구현
     - `GET /v1/novels/{novel_id}/characters/{character_id}` 라우트 등록
     - `pytest` 18/18 100% PASS
   - **Rust MCP Adapter (`services/mcp/promptia-mcp`)**:
     - `govail-mcp-sdk` 및 `govail-mcp-contracts` Crate 의존
     - Direct DB Access = 0 (PostgreSQL 직접 쿼리 없음, REST API 소비)
     - Business Logic in Adapter = 0 (순수 Protocol Adapter & ContextEnvelope 래핑)
     - `get_story_context`, `get_character_state` 2개 Read 도구 등록
     - 7대 Conformance & rmcp Interop 테스트 100% PASS (`cargo test` 7/7 PASS, `clippy -D warnings` 통과)
   - **SDK Ergonomics 평가**: Server Bootstrap 8줄, Tool 등록 4줄, Envelope 래핑 1줄로 사용성 우수(GOOD) 판정.

---

## 3. 남은 작업 및 로드맵 (어떤 게 남았니?)

1. **GoVail 메인 모노레포(`~/srv/govail`) Track 재개 (최우선)**:
   - Evidence Hardening, Audit Trail & Live Regression 테스트 세션 진행.
2. **OpenCode / GoVail MCP Config 실사용 연동**:
   - 개발 환경의 AI 에이전트(OpenCode, GoVail) 설정에 `promptia-mcp` Stdio 바이너리를 등록하여 실제 소설 생성/검토 파이프라인에서 컨텍스트 조회 실사용 루프 검증.
3. **`govail-mcp` V1.2 Minor Ergonomics Patch (권장)**:
   - `async-trait` SDK re-export 제공
   - `ToolError::unauthorized(...)`, `ToolError::forbidden(...)` 편의 팩토리 함수 추가
4. **Modern Protocol (MCP 2026-07-28 per-request metadata) Migration (추후)**:
   - 에코시스템 요구사항에 맞춰 필요한 시점에 진행.

---

## 4. 핵심 아키텍처 불변식 (Invariants)

- **Repository Separation**: `govail-mcp`와 `govail`, `promptia`는 완전히 독립된 별개의 리포지토리입니다.
- **Data & Meaning Ownership**: 데이터의 의미와 접근 권한은 `Promptia`가 소유하며, `govail-mcp`는 단지 이를 MCP 표준으로 노출하는 generic SDK입니다.
