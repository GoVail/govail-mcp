use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Resource identifier contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceIdentity {
    pub source: String,
    pub resource_type: String,
    pub id: String,
}

impl ResourceIdentity {
    pub fn new(
        source: impl Into<String>,
        resource_type: impl Into<String>,
        id: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            resource_type: resource_type.into(),
            id: id.into(),
        }
    }
}

/// Provenance metadata tracking the origin of retrieved context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Provenance {
    pub source: String,
    pub resource: String,
    pub retrieved_at: DateTime<Utc>,
}

impl Provenance {
    pub fn new(source: impl Into<String>, resource: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            resource: resource.into(),
            retrieved_at: Utc::now(),
        }
    }
}

/// Freshness indicators for cache control and observational staleness.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Freshness {
    pub observed_at: DateTime<Utc>,
    pub stale: bool,
}

impl Freshness {
    pub fn fresh() -> Self {
        Self {
            observed_at: Utc::now(),
            stale: false,
        }
    }
}

/// Classification of tool capability type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CapabilityType {
    Read,
    Action,
}

/// Risk evaluation level for governance checks.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Capability metadata describing safety and governance limits of a tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityMetadata {
    pub name: String,
    pub capability_type: CapabilityType,
    pub risk_level: RiskLevel,
    pub requires_approval: bool,
}

impl CapabilityMetadata {
    pub fn read(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            capability_type: CapabilityType::Read,
            risk_level: RiskLevel::Low,
            requires_approval: false,
        }
    }

    pub fn action(name: impl Into<String>, risk: RiskLevel, requires_approval: bool) -> Self {
        Self {
            name: name.into(),
            capability_type: CapabilityType::Action,
            risk_level: risk,
            requires_approval,
        }
    }
}

/// Context envelope wrapping domain data payload with provenance & metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextEnvelope<T> {
    pub data: T,
    pub provenance: Provenance,
    pub freshness: Freshness,
    pub metadata: CapabilityMetadata,
}

impl<T> ContextEnvelope<T> {
    pub fn new(
        data: T,
        source: impl Into<String>,
        resource: impl Into<String>,
        capability_name: impl Into<String>,
    ) -> Self {
        Self {
            data,
            provenance: Provenance::new(source, resource),
            freshness: Freshness::fresh(),
            metadata: CapabilityMetadata::read(capability_name),
        }
    }
}

/// Standardized error codes for MCP tools.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Error)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    #[error("Resource not found")]
    ResourceNotFound,
    #[error("Invalid arguments")]
    InvalidArguments,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Internal application error")]
    InternalError,
}

/// Standardized error envelope returned by MCP tools upon failure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub retryable: bool,
}

impl ToolError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::ResourceNotFound,
            message: msg.into(),
            details: None,
            retryable: false,
        }
    }

    pub fn invalid_args(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InvalidArguments,
            message: msg.into(),
            details: None,
            retryable: false,
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InternalError,
            message: msg.into(),
            details: None,
            retryable: true,
        }
    }
}
