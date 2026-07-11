# govail-mcp 작업 규칙

## 1. 도메인 원칙

- 이 저장소는 맥미니에서 구동되는 중앙 MCP 서버다.
- `repo-snapshot`과 `repo-guard-sentinel`은 원본 코드가 있는 클라이언트 또는 CI에서 evidence bundle을 생성한다.
- `govail-mcp`는 원본 소스코드를 저장하지 않고, 마스킹된 evidence bundle만 수신한다.
- GRC Evidence 구조는 `docs/architecture.md`를 기준으로 한다.

## 2. 개발 및 검증

- 기능 구현 전 `docs/architecture.md` 또는 README에 구조를 먼저 반영한다.
- Rust 코드, 문서, 설정 주석은 한국어를 기본으로 작성한다.
- M1 Max에서는 코드를 편집하고, `cargo check`, Docker Compose, 서비스 헬스체크는 맥미니에서 수행한다.
- 환경별 서비스 주소는 `GOVAIL_GATEWAY_URL`, `SENTINEL_API_URL`, `LLM_API_URL`, `GRC_DATABASE_URL`처럼 환경 변수로 주입한다.
- 공개 문서와 템플릿에는 사설 IP, 실제 토큰, 비밀번호, 내부 전용 호스트명을 기록하지 않는다.

## 3. GRC Evidence MCP

- 중앙 MCP 도구는 evidence bundle의 검증, 제출, 통제 매핑, 리스크 브리프 생성을 담당한다.
- `validate_evidence_bundle`은 원본 소스코드로 오해될 수 있는 필드를 차단해야 한다.
- `submit_evidence_bundle`은 검증된 bundle만 저장해야 한다.
- `generate_risk_brief`는 제공된 evidence 안의 근거만 사용하고, 원본 소스코드를 추측하지 않아야 한다.

## 4. 초기화

- DB 스키마는 `init/001_grc_evidence_schema.sql`을 기준으로 한다.
- 맥미니 초기화는 `scripts/init-grc-db.sh`와 `GRC_DATABASE_URL`을 사용한다.
