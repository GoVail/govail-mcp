use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use crate::client::GoVailClient;
use crate::mcp::protocol::{McpError, ParamError, ToolResult};
use crate::mcp::types::{ToolAnnotations, ToolDefinition};

/// evidence bundle의 기본 안전성을 검사하는 도구
pub struct ValidateEvidenceBundleTool;

impl ValidateEvidenceBundleTool {
    pub fn new() -> Self {
        Self
    }
}

impl ToolDefinition for ValidateEvidenceBundleTool {
    fn name(&self) -> &str {
        "validate_evidence_bundle"
    }

    fn description(&self) -> &str {
        "클라이언트가 생성한 GRC evidence bundle의 필수 필드, finding 구조, 원본 소스코드 포함 위험을 검사합니다."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "bundle": {
                    "type": "object",
                    "description": "repo-snapshot 또는 repo-guard-sentinel이 생성한 evidence bundle"
                }
            },
            "required": ["bundle"]
        })
    }

    fn annotations(&self) -> ToolAnnotations {
        ToolAnnotations {
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
            let bundle = required_object(&params, "bundle")?;
            let report = validate_bundle(bundle);
            ToolResult::json(&report).map_err(|e| McpError::Param(ParamError::JsonParse(e)))
        })
    }
}

/// evidence bundle을 중앙 저장소에 제출하는 도구
pub struct SubmitEvidenceBundleTool {
    client: Arc<GoVailClient>,
}

impl SubmitEvidenceBundleTool {
    pub fn new(client: Arc<GoVailClient>) -> Self {
        Self { client }
    }
}

impl ToolDefinition for SubmitEvidenceBundleTool {
    fn name(&self) -> &str {
        "submit_evidence_bundle"
    }

    fn description(&self) -> &str {
        "검증된 GRC evidence bundle을 중앙 저장소에 제출합니다. 원본 소스코드는 저장하지 않고 마스킹된 증거와 finding만 수신합니다."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "bundle": {
                    "type": "object",
                    "description": "제출할 evidence bundle"
                },
                "project_id": {
                    "type": "string",
                    "description": "중앙 저장소에서 사용할 프로젝트 ID. bundle.project_id가 있으면 생략 가능합니다."
                },
                "source": {
                    "type": "string",
                    "description": "evidence 생성 도구명. 예: repo-snapshot, repo-guard-sentinel"
                }
            },
            "required": ["bundle"]
        })
    }

    fn annotations(&self) -> ToolAnnotations {
        ToolAnnotations {
            read_only_hint: Some(false),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            ..Default::default()
        }
    }

    fn execute(
        &self,
        params: Value,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, McpError>> + Send + '_>> {
        Box::pin(async move {
            let bundle = required_object(&params, "bundle")?;
            let validation = validate_bundle(bundle);
            if !validation["valid"].as_bool().unwrap_or(false) {
                return ToolResult::json(&validation)
                    .map_err(|e| McpError::Param(ParamError::JsonParse(e)));
            }

            let project_id = params
                .get("project_id")
                .and_then(|v| v.as_str())
                .or_else(|| bundle.get("project_id").and_then(|v| v.as_str()))
                .unwrap_or("govail");

            let source = params
                .get("source")
                .and_then(|v| v.as_str())
                .or_else(|| bundle.get("source").and_then(|v| v.as_str()))
                .unwrap_or("unknown");

            let bundle_id = bundle
                .get("bundle_id")
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
                .unwrap_or_else(|| Uuid::new_v4().to_string());

            let stored_bundle = serde_json::json!({
                "bundle_id": bundle_id,
                "project_id": project_id,
                "source": source,
                "received_at": Utc::now(),
                "validation": validation,
                "bundle": bundle
            });

            let stored_path = self
                .client
                .store_evidence_bundle(project_id, &bundle_id, &stored_bundle)
                .await?;

            let response = serde_json::json!({
                "status": "accepted",
                "bundle_id": bundle_id,
                "project_id": project_id,
                "source": source,
                "stored_path": stored_path,
                "next_tools": [
                    "map_findings_to_controls",
                    "generate_risk_brief"
                ]
            });

            ToolResult::json(&response).map_err(|e| McpError::Param(ParamError::JsonParse(e)))
        })
    }
}

/// finding을 주요 보안 통제 프레임워크 후보로 매핑하는 도구
pub struct MapFindingsToControlsTool;

impl MapFindingsToControlsTool {
    pub fn new() -> Self {
        Self
    }
}

impl ToolDefinition for MapFindingsToControlsTool {
    fn name(&self) -> &str {
        "map_findings_to_controls"
    }

    fn description(&self) -> &str {
        "GRC evidence finding을 NIST CSF, ISO 27001, CIS 관점의 통제 후보로 매핑합니다."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "findings": {
                    "type": "array",
                    "description": "매핑할 finding 배열"
                },
                "frameworks": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "대상 프레임워크 목록. 기본값: nist_csf, iso27001, cis"
                }
            },
            "required": ["findings"]
        })
    }

    fn annotations(&self) -> ToolAnnotations {
        ToolAnnotations {
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
            let findings = params
                .get("findings")
                .and_then(|v| v.as_array())
                .ok_or_else(|| ParamError::Missing {
                    field: "findings".to_string(),
                })?;

            let mappings: Vec<Value> = findings
                .iter()
                .enumerate()
                .map(|(index, finding)| map_single_finding(index, finding))
                .collect();

            let response = serde_json::json!({
                "mapping_count": mappings.len(),
                "mappings": mappings
            });

            ToolResult::json(&response).map_err(|e| McpError::Param(ParamError::JsonParse(e)))
        })
    }
}

/// 로컬 LLM으로 GRC 리스크 브리프를 생성하는 도구
pub struct GenerateRiskBriefTool {
    client: Arc<GoVailClient>,
}

impl GenerateRiskBriefTool {
    pub fn new(client: Arc<GoVailClient>) -> Self {
        Self { client }
    }
}

impl ToolDefinition for GenerateRiskBriefTool {
    fn name(&self) -> &str {
        "generate_risk_brief"
    }

    fn description(&self) -> &str {
        "evidence bundle을 기반으로 로컬 LLM을 호출해 거버넌스, 리스크, 컴플라이언스 관점의 한국어 리스크 브리프를 생성합니다."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "bundle": {
                    "type": "object",
                    "description": "리스크 브리프 생성에 사용할 evidence bundle"
                },
                "language": {
                    "type": "string",
                    "description": "출력 언어. 기본값: ko"
                }
            },
            "required": ["bundle"]
        })
    }

    fn annotations(&self) -> ToolAnnotations {
        ToolAnnotations {
            read_only_hint: Some(true),
            idempotent_hint: Some(false),
            ..Default::default()
        }
    }

    fn execute(
        &self,
        params: Value,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, McpError>> + Send + '_>> {
        Box::pin(async move {
            let bundle = required_object(&params, "bundle")?;
            let validation = validate_bundle(bundle);
            if !validation["valid"].as_bool().unwrap_or(false) {
                return ToolResult::json(&validation)
                    .map_err(|e| McpError::Param(ParamError::JsonParse(e)));
            }

            let language = params
                .get("language")
                .and_then(|v| v.as_str())
                .unwrap_or("ko");

            let brief = self.client.generate_risk_brief(bundle, language).await?;
            Ok(ToolResult::text(brief))
        })
    }
}

fn required_object<'a>(params: &'a Value, field: &str) -> Result<&'a Value, McpError> {
    let value = params.get(field).ok_or_else(|| ParamError::Missing {
        field: field.to_string(),
    })?;

    if !value.is_object() {
        return Err(ParamError::TypeMismatch {
            field: field.to_string(),
            expected: "object".to_string(),
            actual: value_type(value).to_string(),
        }
        .into());
    }

    Ok(value)
}

fn validate_bundle(bundle: &Value) -> Value {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if bundle.get("project_id").and_then(|v| v.as_str()).is_none() {
        warnings.push("project_id가 없어 제출 시 기본 프로젝트로 분류됩니다.".to_string());
    }

    if bundle.get("findings").and_then(|v| v.as_array()).is_none() {
        errors.push("findings 배열이 필요합니다.".to_string());
    }

    let risky_paths = find_raw_source_fields(bundle, "$");
    if !risky_paths.is_empty() {
        errors.push(format!(
            "원본 소스코드로 오해될 수 있는 필드가 포함되어 있습니다: {}",
            risky_paths.join(", ")
        ));
    }

    serde_json::json!({
        "valid": errors.is_empty(),
        "errors": errors,
        "warnings": warnings,
        "checked_at": Utc::now()
    })
}

fn find_raw_source_fields(value: &Value, path: &str) -> Vec<String> {
    let mut hits = Vec::new();
    let blocked_keys = [
        "source_code",
        "raw_source",
        "raw_code",
        "file_contents",
        "unmasked_secret",
        "secret_value",
        "private_key",
    ];

    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let child_path = format!("{path}.{key}");
                if blocked_keys.contains(&key.as_str()) {
                    hits.push(child_path.clone());
                }
                hits.extend(find_raw_source_fields(child, &child_path));
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                hits.extend(find_raw_source_fields(child, &format!("{path}[{index}]")));
            }
        }
        _ => {}
    }

    hits
}

fn map_single_finding(index: usize, finding: &Value) -> Value {
    let category = finding
        .get("category")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_lowercase();
    let severity = finding
        .get("severity")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_lowercase();
    let title = finding
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("제목 없음");
    let finding_id = finding
        .get("finding_id")
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("finding-{index}"));

    let controls = if category.contains("secret") {
        vec![
            control(
                "NIST-CSF",
                "PR.DS-5",
                "시크릿 노출은 데이터 보호 및 유출 방지 통제와 연결됩니다.",
                0.82,
            ),
            control(
                "ISO27001",
                "A.8.12",
                "민감정보 유출 방지 및 DLP 통제 후보입니다.",
                0.78,
            ),
            control(
                "CIS",
                "CIS-3.3",
                "민감 데이터 접근 및 보관 통제와 관련됩니다.",
                0.65,
            ),
        ]
    } else if category.contains("network") || category.contains("ip") {
        vec![
            control(
                "NIST-CSF",
                "ID.AM-3",
                "내부망 주소와 서비스 노출은 자산 식별 통제와 연결됩니다.",
                0.74,
            ),
            control(
                "ISO27001",
                "A.8.20",
                "네트워크 보안 및 분리 통제 후보입니다.",
                0.7,
            ),
            control(
                "CIS",
                "CIS-12.2",
                "네트워크 인프라 보안 관리와 관련됩니다.",
                0.62,
            ),
        ]
    } else if category.contains("dependency")
        || category.contains("cve")
        || category.contains("vulnerability")
    {
        vec![
            control(
                "NIST-CSF",
                "ID.RA-1",
                "취약점 식별 및 위험 평가 통제와 연결됩니다.",
                0.84,
            ),
            control(
                "ISO27001",
                "A.8.8",
                "기술적 취약점 관리 통제 후보입니다.",
                0.86,
            ),
            control("CIS", "CIS-7.1", "지속적인 취약점 관리와 관련됩니다.", 0.82),
        ]
    } else {
        vec![
            control(
                "NIST-CSF",
                "ID.RA-5",
                "일반 보안 finding은 리스크 판단 및 우선순위화 통제와 연결됩니다.",
                0.55,
            ),
            control(
                "ISO27001",
                "A.5.7",
                "위협 인텔리전스 및 보안 이벤트 판단 후보입니다.",
                0.48,
            ),
            control(
                "CIS",
                "CIS-17.1",
                "보안 사고 대응 준비와 관련될 수 있습니다.",
                0.42,
            ),
        ]
    };

    serde_json::json!({
        "finding_id": finding_id,
        "title": title,
        "severity": severity,
        "category": category,
        "controls": controls
    })
}

fn control(framework: &str, control_id: &str, rationale: &str, confidence: f64) -> Value {
    serde_json::json!({
        "framework": framework,
        "control_id": control_id,
        "rationale": rationale,
        "confidence": confidence
    })
}

fn value_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
