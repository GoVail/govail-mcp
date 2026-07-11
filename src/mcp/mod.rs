pub mod protocol;
pub mod types;

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, instrument};

use self::protocol::{McpError, ToolResult};
use self::types::ToolDefinition;

/// GoVail MCP 도구들의 등록 및 실행(디스패치)을 관리하는 레지스트리
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn ToolDefinition>>,
}

impl ToolRegistry {
    /// 빈 레지스트리를 생성합니다.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// 레지스트리에 도구를 등록합니다.
    pub fn register(&mut self, tool: impl ToolDefinition + 'static) {
        let name = tool.name().to_string();
        info!(tool = %name, "GoVail MCP 도구 등록 완료");
        self.tools.insert(name, Arc::new(tool));
    }

    /// MCP 프로토콜 스펙의 `tools/list` 요청에 응답할 JSON 리스트를 생성합니다.
    pub fn list_tools(&self) -> Value {
        let tools: Vec<Value> = self
            .tools
            .values()
            .map(|t| {
                let mut tool_json = serde_json::json!({
                    "name": t.name(),
                    "description": t.description(),
                    "inputSchema": t.input_schema(),
                });

                // 어노테이션 힌트가 설정되어 있다면 메타데이터에 추가합니다.
                let ann = t.annotations();
                let mut annotations = serde_json::Map::new();
                if let Some(v) = ann.read_only_hint {
                    annotations.insert("readOnlyHint".into(), Value::Bool(v));
                }
                if let Some(v) = ann.destructive_hint {
                    annotations.insert("destructiveHint".into(), Value::Bool(v));
                }
                if let Some(v) = ann.idempotent_hint {
                    annotations.insert("idempotentHint".into(), Value::Bool(v));
                }
                if let Some(v) = ann.open_world_hint {
                    annotations.insert("openWorldHint".into(), Value::Bool(v));
                }

                if !annotations.is_empty() {
                    tool_json["annotations"] = Value::Object(annotations);
                }

                tool_json
            })
            .collect();

        serde_json::json!({ "tools": tools })
    }

    /// L1/L2/L3 에러 경계를 격리하여 도구를 비동기 실행(디스패치)합니다.
    ///
    /// - L1 ParamError: 클라이언트 파라미터 오류 (직접 노출)
    /// - L2 BusinessError: 타겟 API 오류 및 비즈니스 예외 (직접 노출)
    /// - L3 SystemError: 예상치 못한 시스템 오류 (Zero Retention/Zero-Trust 규격 준수: 상세 오류 은폐)
    #[instrument(skip(self, params), fields(tool = %name))]
    pub async fn dispatch(&self, name: &str, params: Value) -> ToolResult {
        let Some(tool) = self.tools.get(name) else {
            return ToolResult::error(format!("알 수 없는 고베일 MCP 도구입니다: {name}"));
        };

        match tool.execute(params).await {
            Ok(result) => result,
            Err(McpError::Param(e)) => ToolResult::error(format!("[L1 파라미터 에러] {e}")),
            Err(McpError::Business(e)) => ToolResult::error(format!("[L2 비즈니스 에러] {e}")),
            Err(McpError::System(e)) => {
                error!(error = %e, "L3 시스템 예외 발생 - 디테일 은폐");
                // Zero-Trust 원칙에 따라 서버 내부 상세 자격증명/원격정보 누출을 막기 위해 로그로만 기록하고 에러 메시지를 일반화합니다.
                ToolResult::error(
                    "[L3 시스템 에러] 내부 시스템 장애가 발생하여 요청 처리에 실패했습니다. 서버 로그를 참고하세요."
                )
            }
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
