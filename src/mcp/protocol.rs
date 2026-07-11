use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

// ═══════════════════════════════════════════════════════════
// JSON-RPC 2.0 및 MCP 프로토콜 규격 정의 (한국어 주석 포함)
// ═══════════════════════════════════════════════════════════

/// JSON-RPC 2.0 요청 구조체
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 응답 구조체
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 에러 객체 구조체
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    /// 성공 응답을 생성하는 팩토리 메서드
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// 에러 응답을 생성하는 팩토리 메서드
    pub fn error(id: Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }

    /// JSON 파싱 실패 (-32700) 에러 응답
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::error(Value::Null, -32700, message)
    }

    /// 존재하지 않는 메소드 호출 (-32601) 에러 응답
    pub fn method_not_found(id: Value, method: &str) -> Self {
        Self::error(id, -32601, format!("존재하지 않는 메소드입니다: {method}"))
    }
}

// ─── MCP Tool 실행 결과 구조체 ─────────────────────────────

/// MCP Tool 실행 결과
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// 실행 결과 리스트 (텍스트 콘텐츠 블록 등)
    pub content: Vec<ContentBlock>,
    /// 에러 발생 여부 플래그
    #[serde(rename = "isError")]
    pub is_error: bool,
}

/// MCP 콘텐츠 블록 (현재 텍스트 블록만 표준 지원)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
}

impl ToolResult {
    /// 성공 시 텍스트 콘텐츠를 담은 응답 생성
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: vec![ContentBlock::Text {
                text: content.into(),
            }],
            is_error: false,
        }
    }

    /// 실패 시 에러 메시지를 담은 응답 생성
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![ContentBlock::Text {
                text: message.into(),
            }],
            is_error: true,
        }
    }

    /// 임의의 객체를 JSON 포맷의 예쁜 문자열로 직렬화하여 응답 생성
    pub fn json<T: Serialize>(data: &T) -> Result<Self, serde_json::Error> {
        let text = serde_json::to_string_pretty(data)?;
        Ok(Self::text(text))
    }
}

// ─── L1/L2/L3 3계층 에러 경계 정의 ───────────────────────

/// L1: 파라미터 유효성 검증 에러
#[derive(Debug, Error)]
pub enum ParamError {
    #[error("필수 파라미터 누락: {field}")]
    Missing { field: String },

    #[error("파라미터 타입 불일치: {field} (기대: {expected}, 실제: {actual})")]
    TypeMismatch {
        field: String,
        expected: String,
        actual: String,
    },

    #[error("JSON 파싱 실패: {0}")]
    JsonParse(#[from] serde_json::Error),
}

/// L2: 비즈니스 로직 에러
#[derive(Debug, Error)]
pub enum BusinessError {
    #[error("API 호출 실패 (상태 코드: {status}, 바디: {body})")]
    ApiFailure { status: u16, body: String },

    #[error("원하는 대상을 찾을 수 없습니다: {detail}")]
    NotFound { detail: String },
}

/// L3: 시스템 에러 (예상치 못한 인프라 및 IO 에러)
#[derive(Debug, Error)]
pub enum SystemError {
    #[error("IO 오류: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP 클라이언트 통신 오류: {0}")]
    Http(#[from] reqwest::Error),

    #[error("기타 내부 시스템 오류: {0}")]
    Internal(String),
}

/// GoVail MCP 통합 에러 (모든 도구 실행의 공통 에러 경계)
#[derive(Debug, Error)]
pub enum McpError {
    #[error(transparent)]
    Param(#[from] ParamError),

    #[error(transparent)]
    Business(#[from] BusinessError),

    #[error(transparent)]
    System(#[from] SystemError),
}
