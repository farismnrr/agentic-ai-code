use super::{ForgeRepository, Value};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub(in crate::application::git) struct SecurityListResult<T> {
    pub(super) repository_root: String,
    pub(super) forge: ForgeRepository,
    pub(super) alerts: Vec<T>,
    pub(super) truncated: bool,
}
#[derive(Debug, Serialize)]
pub(in crate::application::git) struct SecurityGetResult<T> {
    pub(super) repository_root: String,
    pub(super) forge: ForgeRepository,
    pub(super) alert: T,
}
#[derive(Debug, Serialize)]
pub(in crate::application::git) struct SecretLocationsResult {
    pub(super) repository_root: String,
    pub(super) forge: ForgeRepository,
    pub(super) alert_number: u64,
    pub(super) locations: Vec<SecretLocation>,
    pub(super) truncated: bool,
}

#[derive(Debug, Serialize)]
pub(in crate::application::git) struct DependabotAlert {
    pub(super) number: u64,
    pub(super) state: String,
    pub(super) package: String,
    pub(super) ecosystem: String,
    pub(super) manifest_path: Option<String>,
    pub(super) scope: Option<String>,
    pub(super) ghsa_id: String,
    pub(super) cve_id: Option<String>,
    pub(super) severity: String,
    pub(super) vulnerable_range: String,
    pub(super) patched_version: Option<String>,
    pub(super) created_at: String,
    pub(super) updated_at: String,
    pub(super) dismissed_at: Option<String>,
    pub(super) fixed_at: Option<String>,
    pub(super) url: String,
}
#[derive(Debug, Serialize)]
pub(in crate::application::git) struct CodeAlert {
    pub(super) number: u64,
    pub(super) state: String,
    pub(super) dismissed_reason: Option<String>,
    pub(super) rule_id: String,
    pub(super) rule_name: Option<String>,
    pub(super) security_severity: Option<String>,
    pub(super) description: Option<String>,
    pub(super) tool_name: String,
    pub(super) tool_version: Option<String>,
    pub(super) instance_ref: Option<String>,
    pub(super) commit_sha: Option<String>,
    pub(super) location: Option<CodeLocation>,
    pub(super) url: String,
}
#[derive(Debug, Serialize)]
pub(in crate::application::git) struct CodeLocation {
    pub(super) path: String,
    pub(super) start_line: Option<u64>,
    pub(super) end_line: Option<u64>,
    pub(super) start_column: Option<u64>,
    pub(super) end_column: Option<u64>,
}
#[derive(Debug, Serialize)]
pub(in crate::application::git) struct SecretAlert {
    pub(super) number: u64,
    pub(super) state: String,
    pub(super) resolution: Option<String>,
    pub(super) secret_type: String,
    pub(super) secret_type_display_name: Option<String>,
    pub(super) validity: Option<String>,
    pub(super) publicly_leaked: Option<bool>,
    pub(super) multi_repo: Option<bool>,
    pub(super) push_protection_bypassed: Option<bool>,
    pub(super) push_protection_bypassed_at: Option<String>,
    pub(super) created_at: String,
    pub(super) resolved_at: Option<String>,
    pub(super) url: String,
}
#[derive(Debug, Serialize)]
pub(in crate::application::git) struct SecretLocation {
    pub(super) kind: String,
    pub(super) commit_sha: Option<String>,
    pub(super) blob_sha: Option<String>,
    pub(super) path: Option<String>,
    pub(super) start_line: Option<u64>,
    pub(super) end_line: Option<u64>,
    pub(super) start_column: Option<u64>,
    pub(super) end_column: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ProviderDependabot {
    pub(super) number: u64,
    pub(super) state: String,
    pub(super) dependency: ProviderDependency,
    pub(super) security_advisory: ProviderAdvisory,
    pub(super) security_vulnerability: ProviderVulnerability,
    pub(super) created_at: String,
    pub(super) updated_at: String,
    pub(super) dismissed_at: Option<String>,
    pub(super) fixed_at: Option<String>,
    pub(super) html_url: String,
}
#[derive(Debug, Deserialize)]
pub(super) struct ProviderDependency {
    pub(super) package: ProviderPackage,
    pub(super) manifest_path: Option<String>,
    pub(super) scope: Option<String>,
}
#[derive(Debug, Deserialize)]
pub(super) struct ProviderPackage {
    pub(super) ecosystem: String,
    pub(super) name: String,
}
#[derive(Debug, Deserialize)]
pub(super) struct ProviderAdvisory {
    pub(super) ghsa_id: String,
    pub(super) cve_id: Option<String>,
    pub(super) severity: String,
}
#[derive(Debug, Deserialize)]
pub(super) struct ProviderVulnerability {
    pub(super) vulnerable_version_range: String,
    pub(super) first_patched_version: Option<ProviderVersion>,
}
#[derive(Debug, Deserialize)]
pub(super) struct ProviderVersion {
    pub(super) identifier: String,
}
#[derive(Debug, Deserialize)]
pub(super) struct ProviderCode {
    pub(super) number: u64,
    pub(super) state: String,
    pub(super) dismissed_reason: Option<String>,
    pub(super) rule: ProviderRule,
    pub(super) tool: ProviderTool,
    pub(super) most_recent_instance: Option<ProviderInstance>,
    pub(super) html_url: String,
}
#[derive(Debug, Deserialize)]
pub(super) struct ProviderRule {
    pub(super) id: String,
    pub(super) name: Option<String>,
    pub(super) security_severity_level: Option<String>,
    pub(super) description: Option<String>,
}
#[derive(Debug, Deserialize)]
pub(super) struct ProviderTool {
    pub(super) name: String,
    pub(super) version: Option<String>,
}
#[derive(Debug, Deserialize)]
pub(super) struct ProviderInstance {
    #[serde(rename = "ref")]
    pub(super) git_ref: Option<String>,
    pub(super) commit_sha: Option<String>,
    pub(super) location: Option<ProviderLocation>,
}
#[derive(Debug, Deserialize)]
pub(super) struct ProviderLocation {
    pub(super) path: String,
    pub(super) start_line: Option<u64>,
    pub(super) end_line: Option<u64>,
    pub(super) start_column: Option<u64>,
    pub(super) end_column: Option<u64>,
}
#[derive(Debug, Deserialize)]
pub(super) struct ProviderSecret {
    pub(super) number: u64,
    pub(super) state: String,
    pub(super) resolution: Option<String>,
    pub(super) secret_type: String,
    pub(super) secret_type_display_name: Option<String>,
    pub(super) validity: Option<String>,
    pub(super) publicly_leaked: Option<bool>,
    pub(super) multi_repo: Option<bool>,
    pub(super) push_protection_bypassed: Option<bool>,
    pub(super) push_protection_bypassed_at: Option<String>,
    pub(super) created_at: String,
    pub(super) resolved_at: Option<String>,
    pub(super) html_url: String,
    #[allow(dead_code)]
    pub(super) secret: Option<String>,
}
#[derive(Debug, Deserialize)]
pub(super) struct ProviderSecretLocation {
    #[serde(rename = "type")]
    pub(super) kind: String,
    pub(super) details: Value,
}
