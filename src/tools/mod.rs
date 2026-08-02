pub mod grc;
pub mod guardrails;
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

pub fn register_all(registry: &mut ToolRegistry, client: Arc<GoVailClient>) {
    registry.register(SearchGovailContextTool::new(Arc::clone(&client)));
    registry.register(TriggerGovailJobTool::new(Arc::clone(&client)));
    registry.register(GetGovailJobStatusTool::new(Arc::clone(&client)));
    registry.register(VerifyDlpPolicyTool::new(Arc::clone(&client)));
    registry.register(ValidateEvidenceBundleTool::new());
    registry.register(SubmitEvidenceBundleTool::new(Arc::clone(&client)));
    registry.register(MapFindingsToControlsTool::new());
    registry.register(GenerateRiskBriefTool::new(client));
}
