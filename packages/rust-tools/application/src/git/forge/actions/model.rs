use super::super::common::ForgeRepository;
use serde::{Deserialize, Serialize};

pub(super) const MAX_WORKFLOWS: usize = 50;
pub(super) const MAX_RUNS: usize = 50;
pub(super) const MAX_JOBS: usize = 100;
pub(super) const MAX_STEPS: usize = 100;
pub(super) const MAX_LOG_LINES: usize = 200;
pub(super) const MAX_LOG_BYTES: usize = 64 * 1024;
pub(super) const MAX_LOG_LINE_BYTES: usize = 4096;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProviderWorkflow {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub state: String,
    #[serde(default, alias = "html_url")]
    pub html_url: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct WorkflowSummary {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub state: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub url: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct WorkflowListResult {
    pub repository_root: String,
    pub forge: ForgeRepository,
    pub workflows: Vec<WorkflowSummary>,
    pub truncated: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct WorkflowResult {
    pub repository_root: String,
    pub forge: ForgeRepository,
    pub workflow: WorkflowSummary,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProviderRun {
    pub database_id: u64,
    pub name: String,
    #[serde(default)]
    pub workflow_name: String,
    #[serde(default)]
    pub workflow_database_id: u64,
    #[serde(default)]
    pub display_title: String,
    #[serde(default)]
    pub event: String,
    #[serde(default)]
    pub head_branch: String,
    #[serde(default)]
    pub head_sha: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub conclusion: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub started_at: String,
    #[serde(default)]
    pub updated_at: String,
    pub url: String,
    #[serde(default)]
    pub attempt: u64,
    #[serde(default)]
    pub number: u64,
    #[serde(default)]
    pub jobs: Vec<ProviderJob>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProviderJob {
    pub database_id: u64,
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub conclusion: String,
    #[serde(default)]
    pub started_at: String,
    #[serde(default)]
    pub completed_at: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub steps: Vec<ProviderStep>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProviderStep {
    pub number: u64,
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub conclusion: String,
    #[serde(default)]
    pub started_at: String,
    #[serde(default)]
    pub completed_at: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct RunSummary {
    pub id: u64,
    pub number: u64,
    pub attempt: u64,
    pub workflow_id: u64,
    pub workflow_name: String,
    pub name: String,
    pub display_title: String,
    pub event: String,
    pub head_branch: String,
    pub head_sha: String,
    pub status: String,
    pub conclusion: String,
    pub created_at: String,
    pub started_at: String,
    pub updated_at: String,
    pub url: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct RunListResult {
    pub repository_root: String,
    pub forge: ForgeRepository,
    pub runs: Vec<RunSummary>,
    pub truncated: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct RunResult {
    pub repository_root: String,
    pub forge: ForgeRepository,
    pub run: RunSummary,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct StepSummary {
    pub number: u64,
    pub name: String,
    pub status: String,
    pub conclusion: String,
    pub started_at: String,
    pub completed_at: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct JobSummary {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub conclusion: String,
    pub started_at: String,
    pub completed_at: String,
    pub url: String,
    pub steps: Vec<StepSummary>,
    pub steps_truncated: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct RunJobsResult {
    pub repository_root: String,
    pub forge: ForgeRepository,
    pub run_id: u64,
    pub jobs: Vec<JobSummary>,
    pub truncated: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::git) struct JobLogPreviewResult {
    pub repository_root: String,
    pub forge: ForgeRepository,
    pub job_id: u64,
    pub failed_only: bool,
    pub returned_lines: usize,
    pub truncated: bool,
    pub redacted: bool,
    pub lines: Vec<String>,
}
