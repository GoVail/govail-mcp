use std::env;

/// GoVail MCP 서버 설정을 담는 구조체
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// GoVail Gateway 주소 (DLP 검증 및 라우팅 관문)
    pub gateway_url: String,

    /// GoVail Runtime 주소 (비동기 Job 실행 및 상태 조회)
    pub runtime_url: String,

    /// GoVail Memory 주소 (RAG 지식 검색 데이터베이스)
    pub memory_url: String,

    /// Sentinel API 주소 (코드 DLP 가드레일 검증)
    pub sentinel_url: String,

    /// OpenAI 호환 로컬 LLM API 주소 (리스크 브리프 생성)
    pub llm_api_url: String,

    /// OpenAI 호환 로컬 LLM API 키
    pub llm_api_key: String,

    /// 리스크 브리프 생성에 사용할 모델명
    pub llm_model: String,

    /// GRC evidence 영속화를 위한 Postgres DSN
    pub grc_database_url: String,

    /// evidence bundle 파일 저장 경로
    pub grc_evidence_store_dir: String,

    /// HTTP 클라이언트 요청 타임아웃 (초 단위, 기본값 90.0초)
    pub http_timeout_seconds: f64,

    /// SSE 서버가 리스닝할 포트 번호 (기본값 8096)
    pub port: u16,

    /// 로깅 필터 레벨 (기본값 info)
    pub log_level: String,
}

impl AppConfig {
    /// 환경 변수 및 시스템 설정으로부터 설정을 로드하고 유효성을 검증합니다.
    pub fn load() -> Self {
        // HTTP 타임아웃 파싱 (기본값 90.0초, L2 규칙 준수)
        let http_timeout_seconds = env::var("GOVAIL_HTTP_TIMEOUT")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(90.0);

        // SSE 리스닝 포트 파싱
        let port = env::var("MCP_PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(8096);

        Self {
            gateway_url: env::var("GOVAIL_GATEWAY_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            runtime_url: env::var("GOVAIL_RUNTIME_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            memory_url: env::var("GOVAIL_MEMORY_URL")
                .unwrap_or_else(|_| "http://localhost:8095".to_string()),
            sentinel_url: env::var("SENTINEL_API_URL")
                .unwrap_or_else(|_| "http://localhost:8300".to_string()),
            llm_api_url: env::var("LLM_API_URL")
                .unwrap_or_else(|_| "http://localhost:8080/v1".to_string()),
            llm_api_key: env::var("LLM_API_KEY").unwrap_or_else(|_| "none".to_string()),
            llm_model: env::var("LLM_MODEL").unwrap_or_else(|_| "auto".to_string()),
            grc_database_url: env::var("GRC_DATABASE_URL").unwrap_or_else(|_| "".to_string()),
            grc_evidence_store_dir: env::var("GRC_EVIDENCE_STORE_DIR")
                .unwrap_or_else(|_| "/tmp/govail-mcp/evidence".to_string()),
            http_timeout_seconds,
            port,
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        }
    }
}
