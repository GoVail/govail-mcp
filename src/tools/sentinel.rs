use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::client::GoVailClient;
use crate::mcp::protocol::{McpError, ParamError, ToolResult};
use crate::mcp::types::{ToolAnnotations, ToolDefinition};

/// sentinel-api 기반 코드 DLP 정책 검증 도구
pub struct VerifyDlpPolicyTool {
    client: Arc<GoVailClient>,
}

impl VerifyDlpPolicyTool {
    /// 도구 구조체를 생성하고 의존 클라이언트를 연결합니다.
    pub fn new(client: Arc<GoVailClient>) -> Self {
        Self { client }
    }
}

impl ToolDefinition for VerifyDlpPolicyTool {
    fn name(&self) -> &str {
        "verify_dlp_policy"
    }

    fn description(&self) -> &str {
        "소스코드 또는 Git Diff 텍스트를 검증하여 GoVail DLP(기밀 문자열 누출 방지) 및 보안 설계 원칙 위반 여부를 검사합니다."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "code": {
                    "type": "string",
                    "description": "검증할 소스코드 원문 또는 Git Diff 텍스트"
                },
                "mode": {
                    "type": "string",
                    "description": "스캔 모드: static (정적 검사만), llm (보안 모델 검사만), full (정적+LLM)",
                    "enum": ["static", "llm", "full"]
                },
                "auto_fix": {
                    "type": "boolean",
                    "description": "규칙 위반 발견 시 해결하기 위한 AI 자동 수정본 생성 여부 (기본값: true)"
                }
            },
            "required": ["code"]
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
            let code =
                params
                    .get("code")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ParamError::Missing {
                        field: "code".to_string(),
                    })?;

            let mode = params
                .get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("full");

            let auto_fix = params
                .get("auto_fix")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            // API 호출
            let response = self.client.scan_code(code, mode, auto_fix).await?;

            // 스캔 응답 파싱 및 마크다운 포맷팅
            let verdict = response
                .get("verdict")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            // 판정에 따른 이모지 매핑
            let verdict_emoji = match verdict.to_lowercase().as_str() {
                "safe" => "✅ SAFE (보안 통과)",
                "warn" => "⚠️ WARN (주의/수정 권고)",
                "block" => "🚫 BLOCK (정책 위반으로 제출 차단)",
                _ => "❓ UNKNOWN (판정 알 수 없음)",
            };

            let mut report = format!(
                "# 🛡️ GoVail Sentinel DLP 검증 결과\n\n- **최종 보안 판정**: `{}`\n",
                verdict_emoji
            );

            // 통계 정보
            if let Some(stats) = response.get("stats") {
                let violations = stats
                    .get("violations_found")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let critical = stats
                    .get("critical_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let warning = stats
                    .get("warning_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let duration = stats
                    .get("scan_duration_ms")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);

                report.push_str(&format!(
                    "- **발견된 위반 건수**: `{}`건 (Critical: `{}`, Warning: `{}`)\n- **검사 소요 시간**: `{:.2} ms`\n\n",
                    violations, critical, warning, duration
                ));
            }

            // 위반 내용 상세 목록
            if let Some(violations) = response.get("violations").and_then(|v| v.as_array()) {
                if !violations.is_empty() {
                    report.push_str("## 🚨 검출된 보안/설계 규칙 위반 목록\n\n");
                    for (i, vio) in violations.iter().enumerate() {
                        let rule = vio
                            .get("rule")
                            .and_then(|r| r.as_str())
                            .unwrap_or("unknown-rule");
                        let severity = vio
                            .get("severity")
                            .and_then(|s| s.as_str())
                            .unwrap_or("info");
                        let file = vio.get("file").and_then(|f| f.as_str()).unwrap_or("N/A");
                        let line = vio
                            .get("line")
                            .and_then(|l| l.as_u64())
                            .map(|l| l.to_string())
                            .unwrap_or_else(|| "N/A".to_string());
                        let desc = vio
                            .get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("");
                        let sugg = vio
                            .get("suggestion")
                            .and_then(|s| s.as_str())
                            .unwrap_or("-");

                        report.push_str(&format!(
                            "### {}. `{}` [{}]\n",
                            i + 1,
                            rule,
                            severity.to_uppercase()
                        ));
                        report
                            .push_str(&format!("- **대상**: 파일 `{}` (라인 `{}`)\n", file, line));
                        report.push_str(&format!("- **내용**: {}\n", desc));
                        report.push_str(&format!("- **해결 방안 가이드**: {}\n\n", sugg));
                    }
                }
            }

            // 자동 수정 제안 목록
            if let Some(fixes) = response.get("fixes").and_then(|f| f.as_array()) {
                if !fixes.is_empty() {
                    report.push_str("## 💡 권장 수정 제안 (Auto-Fix Suggestions)\n\n");
                    for fix in fixes {
                        let file = fix
                            .get("file")
                            .and_then(|f| f.as_str())
                            .unwrap_or("unknown");
                        let line = fix
                            .get("line")
                            .and_then(|l| l.as_u64())
                            .map(|l| l.to_string())
                            .unwrap_or_else(|| "N/A".to_string());
                        let orig = fix.get("original").and_then(|o| o.as_str()).unwrap_or("");
                        let repl = fix
                            .get("replacement")
                            .and_then(|r| r.as_str())
                            .unwrap_or("");
                        let reason = fix.get("reason").and_then(|r| r.as_str()).unwrap_or("");

                        report.push_str(&format!("### 📂 파일: `{}` (라인: `{}`)\n", file, line));
                        report.push_str(&format!("- **원인**: {}\n", reason));
                        if !orig.is_empty() {
                            report.push_str(&format!(
                                "```diff\n- {}\n+ {}\n```\n\n",
                                orig.trim(),
                                repl.trim()
                            ));
                        } else {
                            report.push_str(&format!(
                                "```text\n(가이드라인 및 추가 제안):\n{}\n```\n\n",
                                repl
                            ));
                        }
                    }
                }
            }

            Ok(ToolResult::text(report))
        })
    }
}
