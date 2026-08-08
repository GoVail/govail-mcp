use std::path::{Component, Path};
use std::process::Command;

use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use super::git::GitRepository;
use super::model::{
    ApprovalClaims, ApprovalReceipt, PushIntent, PushProposal, RepositoryInspection,
    VerificationAttestation,
};
use super::store::DeliveryStore;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Error)]
pub enum DeliveryError {
    #[error("git operation failed: {0}")]
    Git(String),
    #[error("policy denied operation: {0}")]
    Policy(String),
    #[error("approval denied operation: {0}")]
    Approval(String),
    #[error("invalid workflow state: {0}")]
    State(String),
    #[error("I/O failure: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid UTF-8 output: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("state serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct DeliveryService {
    repository: GitRepository,
    store: DeliveryStore,
}

impl DeliveryService {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DeliveryError> {
        let repository = GitRepository::open(path)?;
        let store = DeliveryStore::new(repository.git_dir());
        Ok(Self { repository, store })
    }

    pub fn inspect(&self) -> Result<RepositoryInspection, DeliveryError> {
        Ok(RepositoryInspection {
            repository_fingerprint: repository_fingerprint(&self.repository),
            branch: self.repository.current_branch()?,
            head: self.repository.head().ok(),
            changed_paths: self.repository.changed_paths()?,
        })
    }

    pub fn commit_allowlisted(
        &self,
        files: &[String],
        message: &str,
    ) -> Result<String, DeliveryError> {
        if files.is_empty() {
            return Err(DeliveryError::Policy(
                "at least one explicit file is required".into(),
            ));
        }
        if message.trim().is_empty() {
            return Err(DeliveryError::Policy("commit message is required".into()));
        }
        if !self.repository.staged_paths()?.is_empty() {
            return Err(DeliveryError::Policy(
                "repository already contains staged changes; refusing to mix changesets".into(),
            ));
        }
        for file in files {
            validate_relative_path(file)?;
            if self.repository.root().join(file).is_dir() {
                return Err(DeliveryError::Policy(format!(
                    "directory staging is forbidden; list files explicitly: {file}"
                )));
            }
        }
        self.repository.stage_paths(files)?;
        if !self.repository.has_staged_changes()? {
            return Err(DeliveryError::State(
                "allowlisted files produced no staged changes".into(),
            ));
        }
        let mut staged = self.repository.staged_paths()?;
        let mut allowed = files.to_vec();
        staged.sort();
        staged.dedup();
        allowed.sort();
        allowed.dedup();
        if staged != allowed {
            return Err(DeliveryError::Policy(format!(
                "staged paths do not exactly match allowlist: staged={staged:?}"
            )));
        }
        self.repository.commit(message.trim())
    }

    pub fn verify(
        &self,
        check_name: &str,
        program: &str,
        args: &[String],
    ) -> Result<VerificationAttestation, DeliveryError> {
        if check_name.trim().is_empty() || program.trim().is_empty() {
            return Err(DeliveryError::Policy(
                "verification name and program are required".into(),
            ));
        }
        let commit = self.repository.head()?;
        let status = Command::new(program)
            .args(args)
            .current_dir(self.repository.root())
            .status()?;
        if !status.success() {
            return Err(DeliveryError::State(format!(
                "verification '{check_name}' failed with status {status}"
            )));
        }
        let command_fingerprint = hash_json(&(check_name, program, args))?;
        let attestation = VerificationAttestation {
            id: Uuid::new_v4().simple().to_string(),
            repository_fingerprint: repository_fingerprint(&self.repository),
            commit,
            check_name: check_name.trim().to_owned(),
            command_fingerprint,
            completed_at: Utc::now(),
            passed: true,
        };
        self.store.save_attestation(&attestation)?;
        Ok(attestation)
    }

    pub fn propose_push(
        &self,
        remote: &str,
        target_branch: &str,
        verification_id: &str,
    ) -> Result<PushProposal, DeliveryError> {
        validate_remote(remote)?;
        self.repository.validate_branch(target_branch)?;
        deny_protected_branch(target_branch)?;

        let source_commit = self.repository.head()?;
        let attestation = self.store.load_attestation(verification_id)?;
        if !attestation.passed
            || attestation.commit != source_commit
            || attestation.repository_fingerprint != repository_fingerprint(&self.repository)
        {
            return Err(DeliveryError::State(
                "verification attestation does not match current commit".into(),
            ));
        }
        let remote_url = self.repository.remote_url(remote)?;
        let intent = PushIntent {
            schema_version: "developer-delivery/v1".into(),
            repository_fingerprint: repository_fingerprint(&self.repository),
            remote: remote.to_owned(),
            remote_url_fingerprint: sha256_hex(remote_url.as_bytes()),
            source_commit,
            parent_commit: self.repository.parent()?,
            target_branch: target_branch.to_owned(),
            expected_remote_commit: self.repository.remote_head(remote, target_branch)?,
            verification_attestation_id: verification_id.to_owned(),
            force: false,
        };
        let proposal = PushProposal {
            id: Uuid::new_v4().simple().to_string(),
            intent_hash: hash_json(&intent)?,
            intent,
            created_at: Utc::now(),
        };
        self.store.save_proposal(&proposal)?;
        Ok(proposal)
    }

    pub fn approve(
        &self,
        proposal_id: &str,
        approver: &str,
        ttl_seconds: i64,
    ) -> Result<ApprovalReceipt, DeliveryError> {
        if approver.trim().is_empty() {
            return Err(DeliveryError::Approval(
                "approver identity is required".into(),
            ));
        }
        if !(30..=3600).contains(&ttl_seconds) {
            return Err(DeliveryError::Approval(
                "approval TTL must be between 30 and 3600 seconds".into(),
            ));
        }
        let proposal = self.store.load_proposal(proposal_id)?;
        self.validate_proposal(&proposal)?;
        let approved_at = Utc::now();
        let claims = ApprovalClaims {
            approval_id: Uuid::new_v4().simple().to_string(),
            proposal_id: proposal.id.clone(),
            intent_hash: proposal.intent_hash.clone(),
            approver: approver.trim().to_owned(),
            approved_at,
            expires_at: approved_at + Duration::seconds(ttl_seconds),
        };
        let key = self.store.approval_key()?;
        let receipt = ApprovalReceipt {
            signature: sign_claims(&key, &claims)?,
            claims,
            consumed_at: None,
        };
        self.store.save_approval(&receipt)?;
        Ok(receipt)
    }

    pub fn push(&self, approval_id: &str) -> Result<PushProposal, DeliveryError> {
        let _lock = self.store.acquire_approval_lock(approval_id)?;
        let mut receipt = self.store.load_approval(approval_id)?;
        if receipt.consumed_at.is_some() {
            return Err(DeliveryError::Approval(
                "approval receipt has already been consumed".into(),
            ));
        }
        if Utc::now() >= receipt.claims.expires_at {
            return Err(DeliveryError::Approval("approval receipt expired".into()));
        }
        let key = self.store.approval_key()?;
        verify_claims(&key, &receipt)?;

        let proposal = self.store.load_proposal(&receipt.claims.proposal_id)?;
        if proposal.intent_hash != receipt.claims.intent_hash {
            return Err(DeliveryError::Approval(
                "approval intent hash mismatch".into(),
            ));
        }
        self.validate_proposal(&proposal)?;

        receipt.consumed_at = Some(Utc::now());
        self.store.save_approval(&receipt)?;
        self.repository.push_exact(
            &proposal.intent.remote,
            &proposal.intent.target_branch,
            &proposal.intent.source_commit,
            proposal.intent.expected_remote_commit.as_deref(),
        )?;
        Ok(proposal)
    }

    fn validate_proposal(&self, proposal: &PushProposal) -> Result<(), DeliveryError> {
        if proposal.intent.force {
            return Err(DeliveryError::Policy("force push is forbidden".into()));
        }
        deny_protected_branch(&proposal.intent.target_branch)?;
        if hash_json(&proposal.intent)? != proposal.intent_hash {
            return Err(DeliveryError::Approval(
                "proposal contents were modified".into(),
            ));
        }
        if repository_fingerprint(&self.repository) != proposal.intent.repository_fingerprint {
            return Err(DeliveryError::Approval(
                "repository identity changed".into(),
            ));
        }
        if self.repository.head()? != proposal.intent.source_commit {
            return Err(DeliveryError::Approval(
                "local HEAD changed after proposal".into(),
            ));
        }
        let remote_url = self.repository.remote_url(&proposal.intent.remote)?;
        if sha256_hex(remote_url.as_bytes()) != proposal.intent.remote_url_fingerprint {
            return Err(DeliveryError::Approval(
                "remote URL changed after proposal".into(),
            ));
        }
        let remote_head = self
            .repository
            .remote_head(&proposal.intent.remote, &proposal.intent.target_branch)?;
        if remote_head != proposal.intent.expected_remote_commit {
            return Err(DeliveryError::Approval(
                "remote HEAD changed after proposal".into(),
            ));
        }
        if let Some(expected) = proposal.intent.expected_remote_commit.as_deref() {
            if !self
                .repository
                .is_ancestor(expected, &proposal.intent.source_commit)?
            {
                return Err(DeliveryError::Policy(
                    "push would not be a fast-forward update".into(),
                ));
            }
        }
        let attestation = self
            .store
            .load_attestation(&proposal.intent.verification_attestation_id)?;
        if !attestation.passed || attestation.commit != proposal.intent.source_commit {
            return Err(DeliveryError::State(
                "verification attestation is invalid for push commit".into(),
            ));
        }
        Ok(())
    }
}

fn repository_fingerprint(repository: &GitRepository) -> String {
    sha256_hex(repository.root().as_os_str().as_encoded_bytes())
}

fn validate_relative_path(value: &str) -> Result<(), DeliveryError> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(DeliveryError::Policy(format!(
            "file path must be repository-relative without traversal: {value}"
        )));
    }
    Ok(())
}

fn validate_remote(remote: &str) -> Result<(), DeliveryError> {
    if remote.is_empty()
        || !remote
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character))
    {
        return Err(DeliveryError::Policy("invalid remote name".into()));
    }
    Ok(())
}

fn deny_protected_branch(branch: &str) -> Result<(), DeliveryError> {
    if branch == "main" || branch == "master" || branch.starts_with("release/") {
        return Err(DeliveryError::Policy(format!(
            "direct push to protected branch '{branch}' is forbidden"
        )));
    }
    Ok(())
}

fn sign_claims(key: &[u8], claims: &ApprovalClaims) -> Result<String, DeliveryError> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|_| DeliveryError::State("invalid approval signing key".into()))?;
    mac.update(&serde_json::to_vec(claims)?);
    Ok(hex(mac.finalize().into_bytes().as_slice()))
}

fn verify_claims(key: &[u8], receipt: &ApprovalReceipt) -> Result<(), DeliveryError> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|_| DeliveryError::State("invalid approval signing key".into()))?;
    mac.update(&serde_json::to_vec(&receipt.claims)?);
    let signature = decode_hex(&receipt.signature)?;
    mac.verify_slice(&signature)
        .map_err(|_| DeliveryError::Approval("approval signature is invalid".into()))
}

fn hash_json<T: Serialize>(value: &T) -> Result<String, DeliveryError> {
    Ok(sha256_hex(&serde_json::to_vec(value)?))
}

pub(crate) fn sha256_bytes(value: &[u8]) -> Vec<u8> {
    Sha256::digest(value).to_vec()
}

fn sha256_hex(value: &[u8]) -> String {
    hex(&sha256_bytes(value))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_hex(value: &str) -> Result<Vec<u8>, DeliveryError> {
    if !value.len().is_multiple_of(2) {
        return Err(DeliveryError::Approval("invalid approval signature".into()));
    }
    (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16)
                .map_err(|_| DeliveryError::Approval("invalid approval signature".into()))
        })
        .collect()
}
