pub mod grc;
pub mod memory;
pub mod runtime;
pub mod sentinel;

use std::sync::Arc;

use crate::client::GoVailClient;
use crate::mcp::ToolRegistry;

use self::grc::{
    GenerateRiskBriefTool, MapFindingsToControlsTool, SubmitEvidenceBundleTool,
    ValidateEvidenceBundleTool,
};
use self::memory::SearchGovailContextTool;
use self::runtime::{GetGovailJobStatusTool, TriggerGovailJobTool};
use self::sentinel::VerifyDlpPolicyTool;

/// 등록된 모든 고베일(GoVail) 전용 MCP 도구들을 레지스트리에 일괄 바인딩합니다.
///
/// # 파라미터
/// * `registry` - 도구를 바인딩할 대상 MCP 도구 레지스트리
/// * `client` - 각 도구에서 공유하여 통신할 GoVail 공통 HTTP 클라이언트 인스턴스 (Arc 래핑)
pub fn register_all(registry: &mut ToolRegistry, client: Arc<GoVailClient>) {
    // 1. Memory RAG 검색 도구 등록
    registry.register(SearchGovailContextTool::new(Arc::clone(&client)));

    // 2. Runtime 비동기 Job 기동 도구 등록
    registry.register(TriggerGovailJobTool::new(Arc::clone(&client)));

    // 3. Runtime Job 상태 및 마크다운 리포트 조회 도구 등록
    registry.register(GetGovailJobStatusTool::new(Arc::clone(&client)));

    // 4. Sentinel API 기반 DLP 코드 보안 스캐너 도구 등록
    registry.register(VerifyDlpPolicyTool::new(Arc::clone(&client)));

    // 5. GRC Evidence bundle 수신 및 컴플라이언스 매핑 도구 등록
    registry.register(ValidateEvidenceBundleTool::new());
    registry.register(SubmitEvidenceBundleTool::new(Arc::clone(&client)));
    registry.register(MapFindingsToControlsTool::new());
    registry.register(GenerateRiskBriefTool::new(client));
}
