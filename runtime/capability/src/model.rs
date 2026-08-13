use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CapabilityVersion {
    pub major: u16,
    pub minor: u16,
}
impl CapabilityVersion {
    pub const fn v1() -> Self {
        Self { major: 1, minor: 0 }
    }
    pub fn compatible_with(&self, requested: &Self) -> bool {
        self.major == requested.major && self.minor >= requested.minor
    }
}
impl std::fmt::Display for CapabilityVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}.{}", self.major, self.minor)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CapabilityRuntime {
    Linux,
    Windows,
    MacOS,
    Container,
    Remote,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SecurityLevel {
    Low,
    Sensitive,
    Privileged,
    Critical,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Idempotency {
    ReadOnly,
    Idempotent,
    NonIdempotent,
    Destructive,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditPolicy {
    None,
    Metadata,
    Full,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityMetadata {
    pub required_permissions: Vec<String>,
    pub required_resources: Vec<String>,
    pub supported_runtimes: Vec<CapabilityRuntime>,
    pub security_level: SecurityLevel,
    pub risk_level: RiskLevel,
    pub idempotency: Idempotency,
    pub timeout_ms: u64,
    pub audit_policy: AuditPolicy,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityDefinition {
    pub capability_id: String,
    pub name: String,
    pub version: CapabilityVersion,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub metadata: CapabilityMetadata,
    pub deprecated: bool,
}
impl CapabilityDefinition {
    pub fn basic(
        id: impl Into<String>,
        description: impl Into<String>,
        runtimes: Vec<CapabilityRuntime>,
        idempotency: Idempotency,
    ) -> Self {
        let capability_id = id.into();
        Self {
            name: capability_id.clone(),
            capability_id,
            version: CapabilityVersion::v1(),
            description: description.into(),
            input_schema: Value::Object(Default::default()),
            output_schema: Value::Object(Default::default()),
            deprecated: false,
            metadata: CapabilityMetadata {
                required_permissions: vec![],
                required_resources: vec![],
                supported_runtimes: runtimes,
                security_level: SecurityLevel::Low,
                risk_level: RiskLevel::Low,
                idempotency,
                timeout_ms: 30_000,
                audit_policy: AuditPolicy::Metadata,
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CapabilityStatus {
    Completed,
    Failed,
    Denied,
    Timeout,
    Unavailable,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CapabilityErrorCode {
    FileNotFound,
    PermissionDenied,
    ApplicationNotFound,
    RuntimeUnavailable,
    CapabilityUnavailable,
    Timeout,
    ResourceExhausted,
    UserApprovalRequired,
    Unsupported,
    ProviderUnavailable,
    InvalidInput,
    Internal,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityError {
    pub code: CapabilityErrorCode,
    pub message: String,
    pub retryable: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityRequest {
    pub request_id: String,
    pub task_id: String,
    pub agent_id: String,
    pub capability_id: String,
    pub requested_version: Option<CapabilityVersion>,
    pub runtime_hint: Option<String>,
    pub input: Value,
    pub timeout_ms: Option<u64>,
    pub trace_id: String,
    pub span_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityResult {
    pub request_id: String,
    pub capability_id: String,
    pub runtime_id: String,
    pub status: CapabilityStatus,
    pub output: Value,
    pub error: Option<CapabilityError>,
    pub metadata: Value,
    pub execution_time_ms: u64,
    pub artifacts: Vec<String>,
    pub side_effects: Vec<String>,
    pub provider_id: Option<String>,
}
impl CapabilityResult {
    pub fn failed(
        request: &CapabilityRequest,
        runtime_id: impl Into<String>,
        code: CapabilityErrorCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            request_id: request.request_id.clone(),
            capability_id: request.capability_id.clone(),
            runtime_id: runtime_id.into(),
            status: CapabilityStatus::Failed,
            output: Value::Null,
            error: Some(CapabilityError {
                code,
                message: message.into(),
                retryable: false,
            }),
            metadata: Value::Null,
            execution_time_ms: 0,
            artifacts: vec![],
            side_effects: vec![],
            provider_id: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderAvailability {
    Available,
    Degraded,
    Unavailable,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityProviderHealth {
    pub availability: ProviderAvailability,
    pub latency_ms: u64,
    pub failure_rate: f64,
    pub last_success_ms: Option<u64>,
    pub last_failure_ms: Option<u64>,
}
impl Default for CapabilityProviderHealth {
    fn default() -> Self {
        Self {
            availability: ProviderAvailability::Available,
            latency_ms: 0,
            failure_rate: 0.0,
            last_success_ms: None,
            last_failure_ms: None,
        }
    }
}
pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Application {
    pub application_id: String,
    pub name: String,
    pub display_name: String,
    pub version: Option<String>,
    pub runtime_id: String,
    pub executable: Option<String>,
    pub capabilities: Vec<String>,
    pub permissions: Vec<String>,
    pub status: String,
    pub metadata: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProcessState {
    Running,
    Stopped,
    Exited,
    Unknown,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub process_id: String,
    pub runtime_id: String,
    pub name: String,
    pub state: ProcessState,
    pub command: Vec<String>,
    pub metadata: Value,
}
