# GoVail — AI Collaboration Guide

> [!IMPORTANT]
> **작업 시작 전 반드시 `agents/rules/L1-00-bootstream.md`를 먼저 읽을 것.**

이후의 `L2`, `rules.md` whitelist, `L3`, skill, workflow 로딩 순서는 L1 부트스트림을 따른다.

## GRC Evidence MCP 작업 메모

- 이 저장소는 맥미니에서 구동되는 중앙 MCP 서버다.
- M1 Max는 코드 편집 전용이며, 빌드·서비스 구동·통합 검증은 맥미니에서 수행한다.
- `repo-snapshot`과 `repo-guard-sentinel`은 원본 코드가 있는 클라이언트 또는 CI에서 evidence bundle을 생성한다.
- `govail-mcp`는 원본 소스코드를 저장하지 않고, 마스킹된 evidence bundle만 수신한다.
- 외부 서비스 주소는 `GOVAIL_GATEWAY_URL`, `SENTINEL_API_URL`, `LLM_API_URL`처럼 환경 변수로 주입한다.
- GRC 스키마 초기화는 `init/001_grc_evidence_schema.sql`과 `scripts/init-grc-db.sh`를 사용한다.
- 상세 설계는 `docs/architecture.md`를 먼저 확인한다.
