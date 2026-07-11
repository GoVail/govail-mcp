use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::client::GoVailClient;
use crate::mcp::protocol::{McpError, ParamError, ToolResult};
use crate::mcp::types::{ToolAnnotations, ToolDefinition};

// ──────────────────────────────────────────────
// 1. trigger_govail_job 도구 구현
// ──────────────────────────────────────────────

/// govail-runtime 비동기 워크플로우 기동 도구
pub struct TriggerGovailJobTool {
    client: Arc<GoVailClient>,
}

impl TriggerGovailJobTool {
    /// 도구 구조체를 생성하고 의존 클라이언트를 연결합니다.
    pub fn new(client: Arc<GoVailClient>) -> Self {
        Self { client }
    }
}

impl ToolDefinition for TriggerGovailJobTool {
    fn name(&self) -> &str {
        "trigger_govail_job"
    }

    fn description(&self) -> &str {
        "GoVail Runtime에서 비동기 에이전트 작업(PR 생성, 코드리뷰, 버그 트레이스 등)을 실행시키고 Job ID를 반환받습니다."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "workflow_type": {
                    "type": "string",
                    "description": "기동할 워크플로우 이름",
                    "enum": ["bug_trace", "pr_create", "code_review", "design_reviewer", "action_approval"]
                },
                "params": {
                    "type": "object",
                    "description": "워크플로우 입력 파라미터 (예: {'bug_description': '...'} 등)"
                },
                "issue_ref": {
                    "type": "string",
                    "description": "작업의 연관 이슈 코드/태스크 ID (기본값: 'GOVAIL-MCP')"
                }
            },
            "required": ["workflow_type"]
        })
    }

    fn annotations(&self) -> ToolAnnotations {
        ToolAnnotations {
            // 이 도구는 백그라운드에 Job을 새로 생성하므로 파괴적/상태 변경 힌트를 제공합니다.
            destructive_hint: Some(false),
            read_only_hint: Some(false),
            idempotent_hint: Some(false),
            ..Default::default()
        }
    }

    fn execute(
        &self,
        params: Value,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, McpError>> + Send + '_>> {
        Box::pin(async move {
            // L1 파라미터 검증
            let workflow_type = params
                .get("workflow_type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ParamError::Missing {
                    field: "workflow_type".to_string(),
                })?;

            let sub_params = params
                .get("params")
                .cloned()
                .unwrap_or_else(|| Value::Object(Default::default()));

            let issue_ref = params
                .get("issue_ref")
                .and_then(|v| v.as_str())
                .unwrap_or("GOVAIL-MCP");

            // API 호출
            let response = self
                .client
                .trigger_job(workflow_type, sub_params, issue_ref)
                .await?;

            let job_id = response
                .get("job_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let message = response
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let result_text = format!(
                "## 🚀 GoVail 비동기 Job 기동 성공\n\n- **워크플로우**: `{workflow_type}`\n- **작업 ID (Job ID)**: `{job_id}`\n- **메시지**: {message}\n\n> 💡 이 Job ID를 사용하여 `get_govail_job_status` 도구로 진행 및 최종 결과 리포트를 받아올 수 있습니다."
            );

            Ok(ToolResult::text(result_text))
        })
    }
}

// ──────────────────────────────────────────────
// 2. get_govail_job_status 도구 구현
// ──────────────────────────────────────────────

/// govail-runtime 특정 Job 진행 상세 및 리포트 조회 도구
pub struct GetGovailJobStatusTool {
    client: Arc<GoVailClient>,
}

impl GetGovailJobStatusTool {
    /// 도구 구조체를 생성하고 의존 클라이언트를 연결합니다.
    pub fn new(client: Arc<GoVailClient>) -> Self {
        Self { client }
    }
}

impl ToolDefinition for GetGovailJobStatusTool {
    fn name(&self) -> &str {
        "get_govail_job_status"
    }

    fn description(&self) -> &str {
        "제시된 Job ID를 기반으로 GoVail 비동기 작업의 최신 진행 상태와 완료된 마크다운 결과 보고서(report.md)를 받아옵니다."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "job_id": {
                    "type": "string",
                    "description": "조회할 대상 작업 고유 ID"
                }
            },
            "required": ["job_id"]
        })
    }

    fn annotations(&self) -> ToolAnnotations {
        ToolAnnotations {
            // 읽기 전용 도구입니다.
            read_only_hint: Some(true),
            idempotent_hint: Some(true),
            ..Default::default()
        }
    }

    fn execute(
        &self,
        params: Value,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, McpError>> + Send + '_>> {
        Box::pin(async move {
            // L1 파라미터 검증
            let job_id = params
                .get("job_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ParamError::Missing {
                    field: "job_id".to_string(),
                })?;

            // 1. 잡 상태 상세 조회
            let status_json = self.client.get_job_status(job_id).await?;
            let status = status_json
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN");
            let wf_type = status_json
                .get("workflow_type")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN");
            let updated_at = status_json
                .get("updated_at")
                .and_then(|v| v.as_str())
                .unwrap_or("-");

            // 2. 리포트 본문 다운로드 (실패 시 빈 텍스트로 대체)
            let report_content = self.client.get_job_report(job_id).await
                .unwrap_or_else(|_| "*[리포트 본문을 다운로드하지 못했습니다. 작업이 진행 중이거나 실패했을 수 있습니다]*".to_string());

            let result_text = format!(
                "# 📊 GoVail Job 상세 및 결과 보고서\n\n- **작업 ID (Job ID)**: `{job_id}`\n- **워크플로우**: `{wf_type}`\n- **현재 상태**: `{status}`\n- **마지막 업데이트**: `{updated_at}`\n\n---\n\n## 📝 작업 결과 리포트 (report.md)\n\n{}\n",
                report_content
            );

            Ok(ToolResult::text(result_text))
        })
    }
}
