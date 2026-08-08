use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Serialize};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use uuid::Uuid;

use super::model::{ApprovalReceipt, PushProposal, VerificationAttestation};
use super::service::DeliveryError;

pub struct DeliveryStore {
    root: PathBuf,
}

impl DeliveryStore {
    pub fn new(git_dir: &Path) -> Self {
        Self {
            root: git_dir.join("govail").join("developer-delivery"),
        }
    }

    pub fn save_attestation(&self, value: &VerificationAttestation) -> Result<(), DeliveryError> {
        self.write_json("attestations", &value.id, value)
    }

    pub fn load_attestation(&self, id: &str) -> Result<VerificationAttestation, DeliveryError> {
        self.read_json("attestations", id)
    }

    pub fn save_proposal(&self, value: &PushProposal) -> Result<(), DeliveryError> {
        self.write_json("proposals", &value.id, value)
    }

    pub fn load_proposal(&self, id: &str) -> Result<PushProposal, DeliveryError> {
        self.read_json("proposals", id)
    }

    pub fn save_approval(&self, value: &ApprovalReceipt) -> Result<(), DeliveryError> {
        self.write_json("approvals", &value.claims.approval_id, value)
    }

    pub fn load_approval(&self, id: &str) -> Result<ApprovalReceipt, DeliveryError> {
        self.read_json("approvals", id)
    }

    pub fn approval_key(&self) -> Result<Vec<u8>, DeliveryError> {
        fs::create_dir_all(&self.root)?;
        let path = self.root.join("approval.key");
        if path.exists() {
            let mut key = Vec::new();
            File::open(path)?.read_to_end(&mut key)?;
            if key.len() != 32 {
                return Err(DeliveryError::State("invalid approval signing key".into()));
            }
            return Ok(key);
        }

        let seed = format!("{}{}", Uuid::new_v4(), Uuid::new_v4());
        let key = crate::developer::service::sha256_bytes(seed.as_bytes());
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        match options.open(&path) {
            Ok(mut file) => {
                file.write_all(&key)?;
                file.sync_all()?;
                Ok(key)
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let mut existing = Vec::new();
                File::open(path)?.read_to_end(&mut existing)?;
                Ok(existing)
            }
            Err(error) => Err(error.into()),
        }
    }

    pub fn acquire_approval_lock(&self, approval_id: &str) -> Result<ApprovalLock, DeliveryError> {
        validate_id(approval_id)?;
        let directory = self.root.join("locks");
        fs::create_dir_all(&directory)?;
        let path = directory.join(approval_id);
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|_| DeliveryError::Approval("approval is already executing".into()))?;
        Ok(ApprovalLock { path })
    }

    fn write_json<T: Serialize>(
        &self,
        collection: &str,
        id: &str,
        value: &T,
    ) -> Result<(), DeliveryError> {
        validate_id(id)?;
        let directory = self.root.join(collection);
        fs::create_dir_all(&directory)?;
        let destination = directory.join(format!("{id}.json"));
        let temporary = directory.join(format!(".{id}.{}.tmp", Uuid::new_v4()));
        let bytes = serde_json::to_vec_pretty(value)?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options.open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(temporary, destination)?;
        Ok(())
    }

    fn read_json<T: DeserializeOwned>(
        &self,
        collection: &str,
        id: &str,
    ) -> Result<T, DeliveryError> {
        validate_id(id)?;
        let bytes = fs::read(self.root.join(collection).join(format!("{id}.json")))?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}

pub struct ApprovalLock {
    path: PathBuf,
}

impl Drop for ApprovalLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn validate_id(value: &str) -> Result<(), DeliveryError> {
    if value.is_empty()
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err(DeliveryError::State("invalid state identifier".into()));
    }
    Ok(())
}
