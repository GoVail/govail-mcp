use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepositoryInspection {
    pub repository_fingerprint: String,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationAttestation {
    pub id: String,
    pub repository_fingerprint: String,
    pub commit: String,
    pub check_name: String,
    pub command_fingerprint: String,
    pub completed_at: DateTime<Utc>,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PushIntent {
    pub schema_version: String,
    pub repository_fingerprint: String,
    pub remote: String,
    pub remote_url_fingerprint: String,
    pub source_commit: String,
    pub parent_commit: Option<String>,
    pub target_branch: String,
    pub expected_remote_commit: Option<String>,
    pub verification_attestation_id: String,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PushProposal {
    pub id: String,
    pub intent: PushIntent,
    pub intent_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovalClaims {
    pub approval_id: String,
    pub proposal_id: String,
    pub intent_hash: String,
    pub approver: String,
    pub approved_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovalReceipt {
    pub claims: ApprovalClaims,
    pub signature: String,
    pub consumed_at: Option<DateTime<Utc>>,
}
