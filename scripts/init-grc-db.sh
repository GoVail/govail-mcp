#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
SCHEMA_FILE="${ROOT_DIR}/init/001_grc_evidence_schema.sql"

if [[ -z "${GRC_DATABASE_URL:-}" ]]; then
  echo "GRC_DATABASE_URL 환경 변수가 필요합니다." >&2
  exit 1
fi

if ! command -v psql >/dev/null 2>&1; then
  echo "psql 명령을 찾을 수 없습니다. 맥미니의 Postgres 클라이언트 환경에서 실행하세요." >&2
  exit 1
fi

psql "${GRC_DATABASE_URL}" -v ON_ERROR_STOP=1 -f "${SCHEMA_FILE}"
echo "GRC evidence 스키마 초기화가 완료되었습니다."
