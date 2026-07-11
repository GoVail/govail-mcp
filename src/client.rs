use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::fs;
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::AppConfig;
use crate::mcp::protocol::{BusinessError, McpError, SystemError};

/// GoVail 플랫폼의 각 컴포넌트(Memory, Runtime, Sentinel)와 통신하는 통합 HTTP 클라이언트
#[derive(Debug, Clone)]
pub struct GoVailClient {
    client: Client,
    config: AppConfig,
    db_pool: Option<sqlx::PgPool>,
}

impl GoVailClient {
    /// AppConfig를 기반으로 HTTP 클라이언트를 초기화합니다.
    pub fn new(config: AppConfig) -> Self {
        // L2 규칙: 개별 타임아웃 대신 GOVAIL_HTTP_TIMEOUT(기본값 90.0초) 환경변수를 사용합니다.
        let timeout = Duration::from_secs_f64(config.http_timeout_seconds);

        let client = Client::builder()
            .timeout(timeout)
            // L2 규칙: rustls-tls를 default-features false와 조합하여 빌드 속도 및 용량을 최적화합니다.
            .build()
            .unwrap_or_else(|e| {
                warn!("HTTP 클라이언트 빌더 생성 실패: {e}. 기본 클라이언트를 생성합니다.");
                Client::new()
            });

        let db_pool = if !config.grc_database_url.is_empty() {
            match sqlx::postgres::PgPoolOptions::new()
                .max_connections(5)
                .acquire_timeout(Duration::from_secs(5))
                .connect_lazy(&config.grc_database_url)
            {
                Ok(pool) => Some(pool),
                Err(e) => {
                    warn!("Lazy GRC database connection pool 생성 실패: {e}");
                    None
                }
            }
        } else {
            None
        };

        Self {
            client,
            config,
            db_pool,
        }
    }

    // ──────────────────────────────────────────────
    // 1. Memory RAG API 연동 (govail-memory: 8095)
    // ──────────────────────────────────────────────

    /// govail-memory의 프로젝트 RAG 검색 API를 호출합니다.
    ///
    /// - 엔드포인트: `POST {memory_url}/internal/projects/{project_id}/search`
    /// - 파라미터:
    ///   - `project_id`: 대상 프로젝트 ID (기본값 "govail")
    ///   - `query`: RAG 검색어
    ///   - `limit`: 반환할 청크 수
    pub async fn search_memory(
        &self,
        project_id: &str,
        query: &str,
        limit: u32,
    ) -> Result<Value, McpError> {
        let url = format!(
            "{}/internal/projects/{}/search",
            self.config.memory_url.trim_end_matches('/'),
            project_id
        );

        let body = serde_json::json!({
            "query": query,
            "limit": limit
        });

        info!(url = %url, query = %query, "GoVail Memory RAG 검색 요청");

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(SystemError::Http)?;

        let status = response.status();
        let body_text = response.text().await.map_err(SystemError::Http)?;

        if !status.is_success() {
            return Err(McpError::Business(BusinessError::ApiFailure {
                status: status.as_u16(),
                body: body_text,
            }));
        }

        let parsed: Value = serde_json::from_str(&body_text)
            .map_err(|e| SystemError::Internal(format!("Memory 검색 결과 JSON 파싱 실패: {e}")))?;

        Ok(parsed)
    }

    // ──────────────────────────────────────────────
    // 2. Runtime API 연동 (govail-runtime: 8080)
    // ──────────────────────────────────────────────

    /// govail-runtime의 비동기 워크플로우 Job 생성 API를 호출합니다.
    ///
    /// - 엔드포인트: `POST {runtime_url}/api/jobs`
    pub async fn trigger_job(
        &self,
        workflow_type: &str,
        params: Value,
        issue_ref: &str,
    ) -> Result<Value, McpError> {
        let url = format!("{}/api/jobs", self.config.runtime_url.trim_end_matches('/'));

        let body = serde_json::json!({
            "workflow_type": workflow_type,
            "params": params,
            "issue_ref": issue_ref
        });

        info!(url = %url, workflow = %workflow_type, "GoVail Runtime Job 트리거 요청");

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(SystemError::Http)?;

        let status = response.status();
        let body_text = response.text().await.map_err(SystemError::Http)?;

        if !status.is_success() {
            return Err(McpError::Business(BusinessError::ApiFailure {
                status: status.as_u16(),
                body: body_text,
            }));
        }

        let parsed: Value = serde_json::from_str(&body_text).map_err(|e| {
            SystemError::Internal(format!("Runtime Job 생성 결과 JSON 파싱 실패: {e}"))
        })?;

        Ok(parsed)
    }

    /// govail-runtime의 특정 Job 상세 정보를 조회합니다.
    ///
    /// - 엔드포인트: `GET {runtime_url}/api/jobs/{job_id}`
    pub async fn get_job_status(&self, job_id: &str) -> Result<Value, McpError> {
        let url = format!(
            "{}/api/jobs/{}",
            self.config.runtime_url.trim_end_matches('/'),
            job_id
        );

        info!(url = %url, job_id = %job_id, "GoVail Runtime Job 상세 조회 요청");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(SystemError::Http)?;

        let status = response.status();
        let body_text = response.text().await.map_err(SystemError::Http)?;

        if !status.is_success() {
            return Err(McpError::Business(BusinessError::ApiFailure {
                status: status.as_u16(),
                body: body_text,
            }));
        }

        let parsed: Value = serde_json::from_str(&body_text).map_err(|e| {
            SystemError::Internal(format!("Runtime Job 조회 결과 JSON 파싱 실패: {e}"))
        })?;

        Ok(parsed)
    }

    /// govail-runtime에 저장된 Job 결과 리포트(report.md) 내용을 조회합니다.
    ///
    /// - 엔드포인트: `GET {runtime_url}/api/jobs/{job_id}/report`
    pub async fn get_job_report(&self, job_id: &str) -> Result<String, McpError> {
        let url = format!(
            "{}/api/jobs/{}/report",
            self.config.runtime_url.trim_end_matches('/'),
            job_id
        );

        info!(url = %url, job_id = %job_id, "GoVail Runtime Job 리포트 다운로드 요청");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(SystemError::Http)?;

        let status = response.status();
        let body_text = response.text().await.map_err(SystemError::Http)?;

        if !status.is_success() {
            return Err(McpError::Business(BusinessError::ApiFailure {
                status: status.as_u16(),
                body: body_text,
            }));
        }

        Ok(body_text)
    }

    // ──────────────────────────────────────────────
    // 3. Sentinel API 연동 (sentinel-api: 8300)
    // ──────────────────────────────────────────────

    /// sentinel-api의 코드 DLP 및 규칙 위반 스캔 API를 호출합니다.
    ///
    /// - 엔드포인트: `POST {sentinel_url}/api/v1/scan`
    pub async fn scan_code(
        &self,
        diff: &str,
        mode: &str,
        auto_fix: bool,
    ) -> Result<Value, McpError> {
        let url = format!(
            "{}/api/v1/scan",
            self.config.sentinel_url.trim_end_matches('/')
        );

        let body = serde_json::json!({
            "diff": diff,
            "mode": mode,
            "auto_fix": auto_fix
        });

        info!(url = %url, mode = %mode, "Sentinel 코드 보안 스캔 요청");

        let mut request = self.client.post(&url).json(&body);
        if !self.config.sentinel_api_token.is_empty() {
            request = request.bearer_auth(&self.config.sentinel_api_token);
        }

        let response = request.send().await.map_err(SystemError::Http)?;

        let status = response.status();
        let body_text = response.text().await.map_err(SystemError::Http)?;

        if !status.is_success() {
            return Err(McpError::Business(BusinessError::ApiFailure {
                status: status.as_u16(),
                body: body_text,
            }));
        }

        let parsed: Value = serde_json::from_str(&body_text).map_err(|e| {
            SystemError::Internal(format!("Sentinel 스캔 결과 JSON 파싱 실패: {e}"))
        })?;

        Ok(parsed)
    }

    // ──────────────────────────────────────────────
    // 4. GRC Evidence 저장 및 로컬 LLM 브리프 생성
    // ──────────────────────────────────────────────

    /// 검증된 evidence bundle을 파일 저장소에 기록합니다.
    ///
    /// 초기 MVP에서는 원본 소스코드 비보관 원칙을 지키기 위해 JSON bundle만 파일로 저장합니다.
    /// Postgres 영속화는 `init/001_grc_evidence_schema.sql` 기준으로 후속 연결합니다.
    pub async fn store_evidence_bundle(
        &self,
        project_id: &str,
        bundle_id: &str,
        bundle: &Value,
    ) -> Result<String, McpError> {
        // 1. PostgreSQL DB 적재 시도 (db_pool이 존재할 때만)
        if let Some(ref pool) = self.db_pool {
            let db_insert_result = async {
                let bundle_id_uuid = Uuid::parse_str(bundle_id)
                    .unwrap_or_else(|_| Uuid::new_v4());

                let source = bundle.get("source")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                let inner_bundle = bundle.get("bundle")
                    .unwrap_or(bundle);

                let asset_type = inner_bundle.get("asset_type").and_then(|v| v.as_str());
                let asset_name = inner_bundle.get("asset_name").and_then(|v| v.as_str());

                // validation이 없으면 빈 JSON 객체 사용
                let default_validation = serde_json::json!({});
                let validation = bundle.get("validation").unwrap_or(&default_validation);

                let mut tx = pool.begin().await?;

                // (1) evidence_bundles 테이블에 저장
                sqlx::query(
                    "INSERT INTO grc.evidence_bundles (id, project_id, source, asset_type, asset_name, bundle, validation) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7)"
                )
                .bind(bundle_id_uuid)
                .bind(project_id)
                .bind(source)
                .bind(asset_type)
                .bind(asset_name)
                .bind(inner_bundle)
                .bind(validation)
                .execute(&mut *tx)
                .await?;

                // (2) findings 테이블에 저장
                if let Some(findings) = inner_bundle.get("findings").and_then(|v| v.as_array()) {
                    for finding in findings.iter() {
                        let finding_id_uuid = Uuid::new_v4();
                        let severity = finding.get("severity").and_then(|v| v.as_str()).unwrap_or("unknown").to_lowercase();
                        let category = finding.get("category").and_then(|v| v.as_str()).unwrap_or("unknown").to_lowercase();
                        let title = finding.get("title").and_then(|v| v.as_str()).unwrap_or("제목 없음");

                        sqlx::query(
                            "INSERT INTO grc.findings (id, bundle_id, project_id, severity, category, title, finding) \
                             VALUES ($1, $2, $3, $4, $5, $6, $7)"
                        )
                        .bind(finding_id_uuid)
                        .bind(bundle_id_uuid)
                        .bind(project_id)
                        .bind(&severity)
                        .bind(&category)
                        .bind(title)
                        .bind(finding)
                        .execute(&mut *tx)
                        .await?;

                        // (3) control_mappings 테이블에 저장 (map_single_finding 헬퍼의 매핑 로직과 동일하게 생성)
                        let mappings = if category.contains("secret") {
                            vec![
                                ("NIST-CSF", "PR.DS-5", "시크릿 노출은 데이터 보호 및 유출 방지 통제와 연결됩니다.", 0.82),
                                ("ISO27001", "A.8.12", "민감정보 유출 방지 및 DLP 통제 후보입니다.", 0.78),
                                ("CIS", "CIS-3.3", "민감 데이터 접근 및 보관 통제와 관련됩니다.", 0.65),
                            ]
                        } else if category.contains("network") || category.contains("ip") {
                            vec![
                                ("NIST-CSF", "ID.AM-3", "내부망 주소와 서비스 노출은 자산 식별 통제와 연결됩니다.", 0.74),
                                ("ISO27001", "A.8.20", "네트워크 보안 및 분리 통제 후보입니다.", 0.70),
                                ("CIS", "CIS-12.2", "네트워크 인프라 보안 관리와 관련됩니다.", 0.62),
                            ]
                        } else if category.contains("dependency") || category.contains("cve") || category.contains("vulnerability") {
                            vec![
                                ("NIST-CSF", "ID.RA-1", "취약점 식별 및 위험 평가 통제와 연결됩니다.", 0.84),
                                ("ISO27001", "A.8.8", "기술적 취약점 관리 통제 후보입니다.", 0.86),
                                ("CIS", "CIS-7.1", "지속적인 취약점 관리와 관련됩니다.", 0.82),
                            ]
                        } else {
                            vec![
                                ("NIST-CSF", "ID.RA-5", "일반 보안 finding은 리스크 판단 및 우선순위화 통제와 연결됩니다.", 0.55),
                                ("ISO27001", "A.5.7", "위협 인텔리전스 및 보안 이벤트 판단 후보입니다.", 0.48),
                                ("CIS", "CIS-17.1", "보안 사고 대응 준비와 관련될 수 있습니다.", 0.42),
                            ]
                        };

                        for (framework, control_id, rationale, confidence) in mappings {
                            let mapping_id_uuid = Uuid::new_v4();
                            sqlx::query(
                                "INSERT INTO grc.control_mappings (id, finding_id, framework, control_id, rationale, confidence) \
                                 VALUES ($1, $2, $3, $4, $5, $6)"
                            )
                            .bind(mapping_id_uuid)
                            .bind(finding_id_uuid)
                            .bind(framework)
                            .bind(control_id)
                            .bind(rationale)
                            .bind(confidence as f64)
                            .execute(&mut *tx)
                            .await?;
                        }
                    }
                }

                tx.commit().await?;
                Ok::<(), sqlx::Error>(())
            }
            .await;

            match db_insert_result {
                Ok(_) => {
                    info!(bundle_id = %bundle_id, "GRC DB에 evidence bundle 및 하위 데이터 적재 완료")
                }
                Err(e) => warn!("GRC DB 적재 중 에러 발생 (파일 Fallback 적용): {e}"),
            }
        }

        // 2. 로컬 파일 저장 (Fallback 및 기본 영속화)
        let safe_project_id = sanitize_path_segment(project_id);
        let safe_bundle_id = sanitize_path_segment(bundle_id);
        let dir = format!(
            "{}/{}",
            self.config.grc_evidence_store_dir.trim_end_matches('/'),
            safe_project_id
        );
        let path = format!("{dir}/{safe_bundle_id}.json");

        fs::create_dir_all(&dir).await.map_err(SystemError::Io)?;

        let body = serde_json::to_string_pretty(bundle)
            .map_err(|e| SystemError::Internal(format!("Evidence bundle 직렬화 실패: {e}")))?;

        fs::write(&path, body).await.map_err(SystemError::Io)?;
        info!(path = %path, "GRC evidence bundle 파일 저장 완료");

        Ok(path)
    }

    /// OpenAI 호환 로컬 LLM API를 호출해 리스크 브리프를 생성합니다.
    pub async fn generate_risk_brief(
        &self,
        bundle: &Value,
        language: &str,
    ) -> Result<String, McpError> {
        let url = format!(
            "{}/chat/completions",
            self.config.llm_api_url.trim_end_matches('/')
        );

        let system_prompt = "당신은 GRC 보안 리스크 분석가입니다. 원본 소스코드를 요구하거나 추측하지 말고, 제공된 evidence bundle 안의 근거만 사용하세요. 한국어로 8개 bullet 이하의 간결한 executive brief를 작성하고, 이모지와 사고 과정은 출력하지 마세요.";
        let user_prompt = format!(
            "언어: {language}\n\n다음 evidence bundle을 기반으로 리스크 브리프를 작성하세요.\n\n{}",
            serde_json::to_string_pretty(bundle)
                .map_err(|e| SystemError::Internal(format!("Evidence bundle 직렬화 실패: {e}")))?
        );

        let body = serde_json::json!({
            "model": self.config.llm_model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "temperature": 0.1,
            "max_tokens": self.config.llm_max_tokens,
            "chat_template_kwargs": {
                "enable_thinking": self.config.llm_enable_thinking
            },
            "extra_body": {
                "chat_template_kwargs": {
                    "enable_thinking": self.config.llm_enable_thinking
                }
            }
        });

        let mut request = self.client.post(&url).json(&body);
        if !self.config.llm_api_key.is_empty() && self.config.llm_api_key != "none" {
            request = request.bearer_auth(&self.config.llm_api_key);
        }

        info!(url = %url, model = %self.config.llm_model, "로컬 LLM 리스크 브리프 생성 요청");

        let started_at = Instant::now();
        let response = request.send().await.map_err(SystemError::Http)?;
        let elapsed_ms = started_at.elapsed().as_millis();
        let status = response.status();
        let body_text = response.text().await.map_err(SystemError::Http)?;

        if !status.is_success() {
            return Err(McpError::Business(BusinessError::ApiFailure {
                status: status.as_u16(),
                body: body_text,
            }));
        }

        let parsed: Value = serde_json::from_str(&body_text)
            .map_err(|e| SystemError::Internal(format!("LLM 응답 JSON 파싱 실패: {e}")))?;

        let prompt_tokens = parsed
            .pointer("/usage/prompt_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let completion_tokens = parsed
            .pointer("/usage/completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let total_tokens = parsed
            .pointer("/usage/total_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        info!(
            elapsed_ms,
            prompt_tokens, completion_tokens, total_tokens, "로컬 LLM 리스크 브리프 생성 완료"
        );

        let content = parsed
            .get("choices")
            .and_then(|v| v.as_array())
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .ok_or_else(|| {
                SystemError::Internal("LLM 응답에서 message.content를 찾지 못했습니다.".to_string())
            })?;

        Ok(content.to_string())
    }
}

/// 파일 경로 세그먼트로 사용할 수 있도록 보수적으로 정규화합니다.
fn sanitize_path_segment(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }

    if out.is_empty() {
        "unknown".to_string()
    } else {
        out
    }
}
