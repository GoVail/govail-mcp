use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

use super::protocol::{McpError, ToolResult};

/// MCP 도구 추가 메타데이터 정보 (Spec 2025.3 준수)
///
/// AI 클라이언트(예: Cursor)가 특정 도구의 보안 특성을 감지하고
/// 자동 실행 여부 또는 사용자 확인 팝업 노출 여부를 결정하는 힌트로 사용됩니다.
#[derive(Debug, Clone, Default)]
pub struct ToolAnnotations {
    /// 단순 읽기 전용 도구 (동의 없이 자동 실행 가능)
    pub read_only_hint: Option<bool>,
    /// 리소스를 변경하거나 위험성 있는 도구 (사용자 확인 팝업 필요)
    pub destructive_hint: Option<bool>,
    /// 멱등성 보장 여부 (여러 번 실행해도 영향이 동일하여 재시도 안전)
    pub idempotent_hint: Option<bool>,
    /// 외부 인터넷망 접근 여부
    pub open_world_hint: Option<bool>,
}

/// GoVail MCP 상에서 동작할 도구의 공통 추상화 Trait
///
/// 모든 GoVail MCP 도구는 이 트레이트를 상속받아 구현되어야 합니다.
pub trait ToolDefinition: Send + Sync {
    /// 도구 식별자 이름 (예: `search_govail_context`)
    fn name(&self) -> &str;

    /// 도구 역할 설명 (AI 모델이 툴 쓰기 결정을 내리는 기준이 됨)
    fn description(&self) -> &str;

    /// JSON Schema 규격의 인풋 파라미터 구조 정의
    fn input_schema(&self) -> Value;

    /// 도구의 실행 어노테이션 힌트 (기본값 제공)
    fn annotations(&self) -> ToolAnnotations {
        ToolAnnotations::default()
    }

    /// 비동기 도구 실행 핸들러
    ///
    /// `dyn ToolDefinition` 다형성 처리를 위해 `Pin<Box<dyn Future>>` 형태로 반환합니다.
    fn execute(
        &self,
        params: Value,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, McpError>> + Send + '_>>;
}
