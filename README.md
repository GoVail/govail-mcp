# GoVail MCP

GoVail MCP는 GoVail 플랫폼을 MCP(Model Context Protocol) 도구로 노출하는 SSE 서버다. 중앙 서버는 맥미니에서 구동하고, M1 Max는 코드 편집 전용으로 사용한다.

## 핵심 역할

- GoVail Memory 문맥 검색
- GoVail Runtime 비동기 작업 트리거와 상태 조회
- Sentinel 기반 DLP 정책 검증
- GRC evidence bundle 수신, 검증, 통제 매핑, 리스크 브리프 생성

## GRC Evidence 흐름

`repo-snapshot`과 `repo-guard-sentinel`은 원본 코드가 있는 클라이언트 또는 CI에서 실행한다. 중앙 MCP는 원본 소스코드를 직접 보관하지 않고, 마스킹된 evidence bundle만 수신한다.

```text
Client / Agent
  -> repo-snapshot
  -> repo-guard-sentinel
  -> evidence bundle
  -> GoVail Gateway
  -> govail-mcp
  -> local LLM / Postgres / NAS / Loki / Langfuse
```

## 주요 도구

| 도구 | 설명 |
| --- | --- |
| `search_govail_context` | GoVail Memory에서 설계 문서와 정책 문맥을 검색한다. |
| `trigger_govail_job` | GoVail Runtime 작업을 생성한다. |
| `get_govail_job_status` | Runtime 작업 상태와 결과 리포트를 조회한다. |
| `verify_dlp_policy` | Sentinel API로 코드 또는 diff의 DLP 위반을 검사한다. |
| `validate_evidence_bundle` | evidence bundle의 최소 스키마와 원본 코드 포함 위험을 검사한다. |
| `submit_evidence_bundle` | 검증된 evidence bundle을 중앙 저장소에 기록한다. |
| `map_findings_to_controls` | finding을 NIST CSF, ISO 27001, CIS 통제 후보로 매핑한다. |
| `generate_risk_brief` | 로컬 LLM으로 GRC 리스크 브리프를 생성한다. |

## 환경 변수

실제 서비스 주소는 배포 환경에서 주입한다. 공용 템플릿에는 사설 IP를 기록하지 않는다.

```env
GOVAIL_GATEWAY_URL=http://<gateway_api_url>
GOVAIL_RUNTIME_URL=http://<runtime_api_url>
GOVAIL_MEMORY_URL=http://<memory_api_url>
SENTINEL_API_URL=http://<sentinel_api_url>
LLM_API_URL=http://<llm_api_url>/v1
LLM_API_KEY=none
LLM_MODEL=auto
GRC_DATABASE_URL=postgres://<user>:<password>@<postgres_host>:5432/<database>
GRC_EVIDENCE_HOST_DIR=/srv/nas/shared/govail/grc-evidence
GRC_EVIDENCE_STORE_DIR=/data/evidence
GOVAIL_HTTP_TIMEOUT=90.0
MCP_PORT=8096
LOG_LEVEL=info
```

## 초기화

Postgres 스키마를 사용할 경우 맥미니에서 다음을 실행한다.

```bash
export GRC_DATABASE_URL="postgres://<user>:<password>@<postgres_host>:5432/<database>"
./scripts/init-grc-db.sh
```

초기 MVP는 evidence bundle을 컨테이너 내부 `GRC_EVIDENCE_STORE_DIR` 아래 JSON 파일로 저장한다. 호스트 경로는 `GRC_EVIDENCE_HOST_DIR`로 마운트한다. Postgres 테이블은 검색, 대시보드, 감사 쿼리가 필요해지는 시점에 같은 스키마로 연결한다.

## 배포

서비스는 맥미니에서 Docker Compose로 구동한다.

```bash
docker compose up -d --build govail-mcp
```

헬스체크:

```bash
curl http://<mcp_api_url>/health
```

## 설계 문서

상세 구조는 `docs/architecture.md`를 기준으로 한다.
