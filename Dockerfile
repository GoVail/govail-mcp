# ─── 빌드 스테이지 ──────────────────────────────────────────
FROM rust:slim-bookworm AS builder

WORKDIR /usr/src/govail-mcp

# 소스코드 전체 복사 및 릴리즈 빌드
COPY . .
RUN cargo build --release

# ─── 런타임 스테이지 (Zero-Trust 경량화) ─────────────────────
FROM gcr.io/distroless/cc-debian12

WORKDIR /app

# 빌드된 바이너리 복사
COPY --from=builder /usr/src/govail-mcp/target/release/govail-mcp /app/govail-mcp

# 기본 환경변수 정의
ENV LOG_LEVEL=info
ENV MCP_PORT=8096

# 포트 노출
EXPOSE 8096

# 실행 진입점
ENTRYPOINT ["/app/govail-mcp"]
