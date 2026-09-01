use super::super::common::ForgeRepository;
use serde::{Deserialize, Serialize};

pub(super) const MAX_ISSUES: usize = 50;
pub(super) const MAX_LABEL_FILTER_COUNT: usize = 10;
pub(super) const MAX_LABELS_PER_ISSUE: usize = 50;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProviderAuthor {
    pub(super) login: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProviderLabel {
    pub(super) name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProviderIssue {
    pub(super) number: u64,
    pub(super) title: String,
    pub(super) url: String,
    pub(super) state: String,
    #[serde(default)]
    pub(super) state_reason: Option<String>,
    #[serde(default)]
    pub(super) author: Option<ProviderAuthor>,
    #[serde(default)]
    pub(super) labels: Vec<ProviderLabel>,
    #[serde(default)]
    pub(super) created_at: String,
    #[serde(default)]
    pub(super) updated_at: String,
    #[serde(default)]
    pub(super) closed_at: Option<String>,
    #[serde(default)]
    pub(super) body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::application::git) struct IssueSummary {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_reason: Option<String>,
    pub author: Option<String>,
    pub labels: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::application::git) struct IssueDetail {
    #[serde(flatten)]
    pub summary: IssueSummary,
    pub body: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::application::git) struct IssueListResult {
    pub repository_root: String,
    pub forge: ForgeRepository,
    pub issues: Vec<IssueSummary>,
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::application::git) struct IssueResult {
    pub repository_root: String,
    pub forge: ForgeRepository,
    pub issue: IssueDetail,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::application::git) struct IssueCommentResult {
    pub repository_root: String,
    pub forge: ForgeRepository,
    pub issue_number: u64,
    pub comment_url: String,
}

pub(super) fn summary_fields() -> &'static str {
    "number,title,url,state,stateReason,author,labels,createdAt,updatedAt,closedAt"
}

pub(super) fn detail_fields() -> &'static str {
    "number,title,url,state,stateReason,author,labels,createdAt,updatedAt,closedAt,body"
}
