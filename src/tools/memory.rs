use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::client::GoVailClient;
use crate::mcp::protocol::{McpError, ParamError, ToolResult};
use crate::mcp::types::{ToolAnnotations, ToolDefinition};

/// govail-memory RAG 검색 도구
///
/// AI 에이전트가 로컬 코드베이스 분석 시 관련 기술 문서, 가이드, 사내 아키텍처 규칙 등을 검색하기 위해 호출합니다.
pub struct SearchGovailContextTool {
    client: Arc<GoVailClient>,
}

impl SearchGovailContextTool {
    /// 도구 구조체를 생성하고 의존 클라이언트를 연결합니다.
    pub fn new(client: Arc<GoVailClient>) -> Self {
        Self { client }
    }
}

impl ToolDefinition for SearchGovailContextTool {
    fn name(&self) -> &str {
        "search_govail_context"
    }

    fn description(&self) -> &str {
        "GoVail Memory RAG 엔진에서 프로젝트 설계 문서, 아키텍처 가이드라인, 보안 원칙을 검색하여 반환합니다."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "RAG 지식 검색을 위한 질문 또는 핵심 키워드"
                },
                "project_id": {
                    "type": "string",
                    "description": "검색할 네임스페이스 프로젝트 ID (기본값: 'govail')"
                },
                "limit": {
                    "type": "integer",
                    "description": "조회할 문서 청크 수 (기본값: 5, 최대: 20)",
                    "minimum": 1,
                    "maximum": 20
                }
            },
            "required": ["query"]
        })
    }

    fn annotations(&self) -> ToolAnnotations {
        ToolAnnotations {
            // 이 도구는 상태를 바꾸지 않는 단순 읽기 전용 도구입니다.
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
            // L1 파라미터 유효성 검증
            let query = params
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ParamError::Missing {
                    field: "query".to_string(),
                })?;

            let project_id = params
                .get("project_id")
                .and_then(|v| v.as_str())
                .unwrap_or("govail");

            let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as u32;

            // RAG API 호출
            let response_json = self.client.search_memory(project_id, query, limit).await?;

            // JSON 결과를 인간 친화적이고 에이전트가 구조적으로 잘 이해할 수 있는 마크다운 포맷으로 가공합니다.
            let mut report = format!(
                "# 🔍 GoVail RAG 검색 결과\n\n- **프로젝트 ID**: `{project_id}`\n- **질의어**: \"{query}\"\n"
            );

            if let Some(hit) = response_json.get("memory_hit") {
                report.push_str(&format!("- **메모리 히트**: `{}`\n\n", hit));
            }

            if let Some(chunks) = response_json.get("chunks").and_then(|c| c.as_array()) {
                if chunks.is_empty() {
                    report.push_str("> ⚠️ 검색어와 관련된 문서 조각(Chunk)을 찾지 못했습니다.\n");
                } else {
                    report.push_str("## 📄 관련 문서 조각 목록\n\n");
                    for (i, chunk) in chunks.iter().enumerate() {
                        let text = chunk.get("text").and_then(|t| t.as_str()).unwrap_or("");
                        let score = chunk.get("score").and_then(|s| s.as_f64()).unwrap_or(0.0);
                        let source_id = chunk
                            .get("source_id")
                            .and_then(|s| s.as_str())
                            .unwrap_or("unknown");

                        report.push_str(&format!(
                            "### [{}] 문서 소스 ID: `{}` (유사도 점수: {:.4})\n",
                            i + 1,
                            source_id,
                            score
                        ));
                        report.push_str(&format!("```text\n{}\n```\n\n", text));
                    }
                }
            }

            Ok(ToolResult::text(report))
        })
    }
}
