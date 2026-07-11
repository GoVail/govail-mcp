mod client;
mod config;
mod mcp;
mod tools;

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    routing::{get, post},
    Json, Router,
};
use futures_core::Stream;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{debug, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use crate::client::GoVailClient;
use crate::config::AppConfig;
use crate::mcp::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::mcp::ToolRegistry;

// ═══════════════════════════════════════════════════════════
// GoVail MCP SSE 서버 - main.rs (상세 한국어 주석 포함)
// ═══════════════════════════════════════════════════════════

/// 세션별 응답 채널 맵 타입 정의 (session_id -> Sender)
type SessionMap = Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>;

/// Axum 핸들러들이 공유하는 글로벌 서버 상태 구조체
#[derive(Clone)]
struct SseState {
    /// 도구들이 등록되는 레지스트리
    registry: Arc<ToolRegistry>,
    /// 세션별 독립 채널 맵 (클라이언트 응답 격리 보장)
    sessions: SessionMap,
}

/// POST /message 요청의 쿼리 파라미터 파싱용 구조체
#[derive(Debug, Deserialize)]
struct MessageQuery {
    session_id: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 환경 변수 및 설정 로드
    let config = AppConfig::load();

    // 2. 구조화된 Fmt 로깅 셋업 (기본값 설정값 준수)
    let log_level = match config.log_level.to_lowercase().as_str() {
        "debug" => Level::DEBUG,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder().with_max_level(log_level).finish();
    tracing::subscriber::set_global_default(subscriber).expect("로깅 서브스크라이버 설정 실패");

    info!("GoVail MCP (Rust Edition) 서버 초기화 시작");

    // 3. GoVail 통합 HTTP API 클라이언트 생성 (Arc 래핑하여 스레드 안전하게 공유)
    let govail_client = Arc::new(GoVailClient::new(config.clone()));

    // 4. MCP 도구 레지스트리 초기화 및 일괄 도구 등록
    let mut registry = ToolRegistry::new();
    tools::register_all(&mut registry, govail_client);
    let registry = Arc::new(registry);

    // 5. SSE 세션 맵 생성
    let sessions = Arc::new(RwLock::new(HashMap::new()));

    let state = SseState { registry, sessions };

    // 6. Axum HTTP 라우터 구성
    // - GET /sse : 클라이언트가 이 스트림을 열어 비동기 푸시 이벤트를 수신합니다.
    // - POST /message : JSON-RPC 요청을 전송하는 단방향 POST 엔드포인트입니다.
    // - GET /health : 인프라 통합 검사용 헬스체크 엔드포인트입니다.
    let app = Router::new()
        .route("/sse", get(sse_handler))
        .route("/message", post(message_handler))
        .route("/health", get(health_handler))
        .layer(CorsLayer::permissive()) // 개발 및 원격 연결을 위한 허용적 CORS 레이어
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // 7. 지정된 포트로 서버 바인딩 및 구동
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(address = %addr, "GoVail MCP SSE 서버 구동 완료");

    axum::serve(listener, app).await?;
    Ok(())
}

// ──────────────────────────────────────────────
// API 라우트 핸들러 함수군
// ──────────────────────────────────────────────

/// GET /sse 핸들러
///
/// 클라이언트가 이 주소로 들어오면 고유한 UUID 세션 ID를 부여하고,
/// 해당 세션과 1:1로 매핑되는 broadcast 채널을 개설하여 지속적인 Event Stream을 수송합니다.
async fn sse_handler(
    State(state): State<SseState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let (tx, mut rx) = broadcast::channel::<String>(256);

    // 세션 맵에 등록 (스레드 세이프 쓰기 잠금 확보)
    {
        let mut sessions = state.sessions.write().await;
        sessions.insert(session_id.clone(), tx);
    }

    info!(session = %session_id, "새로운 SSE 연결 채널 및 세션 생성 완료");

    let sessions_ref = Arc::clone(&state.sessions);
    let sid_for_cleanup = session_id.clone();

    // async-stream 매크로를 사용하여 SSE 스트림 루프를 작성합니다.
    let stream = async_stream::stream! {
        // 첫 연결 확립 시, 클라이언트에게 자신의 세션 메시지를 보낼 POST 주소(endpoint)를 먼저 알려줍니다.
        yield Ok(Event::default()
            .event("endpoint")
            .data(format!("/message?session_id={session_id}")));

        // 세션 전용 채널에 메시지가 들어올 때까지 대기하며, 들어오는 즉시 SSE message 이벤트로 발송합니다.
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    yield Ok(Event::default()
                        .event("message")
                        .data(msg));
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(skipped = n, session = %session_id, "SSE 채널 전송 지연 - 일부 이벤트 스킵됨");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }

        // 스트림 연결이 해제되면 세션 맵에서 해당 세션 정보를 제거하여 메모리 누수를 방지합니다.
        let mut sessions = sessions_ref.write().await;
        sessions.remove(&sid_for_cleanup);
        info!(session = %sid_for_cleanup, "SSE 연결 해제 감지: 세션 맵 정리 성공");
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// POST /message 핸들러
///
/// 클라이언트가 Stdio 대신 HTTP를 통해 보낸 JSON-RPC 요청을 처리합니다.
/// `initialize`, `tools/list`, `tools/call` 등의 규격 메소드를 분류하고 처리합니다.
async fn message_handler(
    State(state): State<SseState>,
    Query(query): Query<MessageQuery>,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let id = request.id.clone().unwrap_or(Value::Null);

    let response = match request.method.as_str() {
        // 1. 프로토콜 초기화 핸들러
        "initialize" => {
            info!("MCP 초기화 요청 감지");
            JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": { "listChanged": false }
                    },
                    "serverInfo": {
                        "name": "govail-mcp",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }),
            )
        }
        // 2. 초기화 완료 시그널 알림
        "notifications/initialized" => {
            info!("MCP 클라이언트 초기화 핸드셰이크 완료");
            return Json(JsonRpcResponse::success(Value::Null, Value::Null));
        }
        // 3. 도구 목록 요청 핸들러
        "tools/list" => {
            debug!("MCP 도구 목록 조회 호출");
            JsonRpcResponse::success(id, state.registry.list_tools())
        }
        // 4. 도구 실제 호출 실행 핸들러
        "tools/call" => {
            let params = request.params.unwrap_or(Value::Object(Default::default()));
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or(Value::Object(Default::default()));

            debug!(tool = %tool_name, "MCP 도구 실행 위임 디스패치");
            let result = state.registry.dispatch(tool_name, arguments).await;

            match serde_json::to_value(&result) {
                Ok(value) => JsonRpcResponse::success(id, value),
                Err(e) => JsonRpcResponse::error(id, -32603, format!("결과 JSON 직렬화 실패: {e}")),
            }
        }
        // 5. 기타 지원되지 않는 메소드 예외 처리
        method => JsonRpcResponse::method_not_found(id, method),
    };

    // ─── 세션 격리 라우팅 ───
    // POST /message?session_id=xxx 형태로 전송된 경우,
    // 해당 session_id의 SSE 채널로 응답 전문을 문자열로 직렬화하여 송신합니다.
    if let Some(sid) = &query.session_id {
        if let Ok(json) = serde_json::to_string(&response) {
            let sessions = state.sessions.read().await;
            if let Some(tx) = sessions.get(sid) {
                if tx.send(json).is_err() {
                    warn!(session = %sid, "SSE 전송 실패: 수신 지연 또는 이미 닫힌 채널입니다.");
                }
            } else {
                warn!(session = %sid, "유효하지 않거나 만료된 session_id가 요청되었습니다.");
            }
        }
    } else {
        warn!(
            "요청에 session_id 쿼리 스트링이 생략되었습니다. SSE 채널로 이벤트를 쏘지 못했습니다."
        );
    }

    Json(response)
}

/// GET /health 핸들러
///
/// L2-00-common 및 Uptime Kuma 모니터링을 위한 경량 헬스체크 정보 반환
async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "govail-mcp",
        "version": env!("CARGO_PKG_VERSION"),
        "transport": "sse"
    }))
}
