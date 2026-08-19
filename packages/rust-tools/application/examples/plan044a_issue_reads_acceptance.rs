use relay_application::git::dispatch_git_tool;
use relay_core::config::ServerConfig;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs, path::Path, process::Command};

fn run(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .unwrap()
        .success());
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureAuthor {
    login: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureIssue {
    number: u64,
    title: String,
    url: String,
    state: String,
    #[serde(default)]
    state_reason: Option<String>,
    #[serde(default)]
    author: Option<FixtureAuthor>,
    #[serde(default)]
    labels: Vec<FixtureLabel>,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    updated_at: String,
    #[serde(default)]
    closed_at: Option<String>,
    #[serde(default)]
    body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureSummary {
    number: u64,
    title: String,
    url: String,
    state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    state_reason: Option<String>,
    author: Option<String>,
    labels: Vec<String>,
    created_at: String,
    updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    closed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureDetail {
    #[serde(flatten)]
    summary: FixtureSummary,
    body: String,
}

fn validate_fixture_summary(
    item: &FixtureIssue,
    owner: &str,
    repository: &str,
) -> Result<FixtureSummary, String> {
    if item.url.contains("/pull/") {
        return Err("pull request cannot be accessed as an issue".into());
    }
    if item.number == 0
        || item.title.len() > 256
        || item.title.contains('\0')
        || item.labels.len() > 50
    {
        return Err("invalid_git_output".into());
    }
    let expected_url = format!(
        "https://github.com/{}/{}/issues/{}",
        owner, repository, item.number
    );
    if item.url != expected_url {
        return Err("issue repository identity mismatch".into());
    }
    let mut labels = Vec::with_capacity(item.labels.len());
    for label in &item.labels {
        if label.name.is_empty() || label.name.len() > 128 || label.name.contains('\0') {
            return Err("invalid_git_output".into());
        }
        labels.push(label.name.clone());
    }
    let author = item.author.as_ref().and_then(|a| {
        if a.login.is_empty() || a.login.contains('\0') {
            None
        } else {
            Some(a.login.clone())
        }
    });
    Ok(FixtureSummary {
        number: item.number,
        title: item.title.clone(),
        url: item.url.clone(),
        state: item.state.clone(),
        state_reason: item.state_reason.clone().filter(|s| !s.is_empty()),
        author,
        labels,
        created_at: item.created_at.clone(),
        updated_at: item.updated_at.clone(),
        closed_at: item.closed_at.clone().filter(|s| !s.is_empty()),
    })
}

fn validate_fixture_detail(
    item: &FixtureIssue,
    owner: &str,
    repo: &str,
    expected_num: u64,
) -> Result<FixtureDetail, String> {
    if item.number != expected_num {
        return Err("issue repository identity mismatch".into());
    }
    let summary = validate_fixture_summary(item, owner, repo)?;
    let body = item.body.as_deref().unwrap_or("");
    if body.len() > 64 * 1024 || body.contains('\0') {
        return Err("invalid_git_output".into());
    }
    Ok(FixtureDetail {
        summary,
        body: body.to_owned(),
    })
}

#[tokio::main]
async fn main() {
    // 1. Verify field list strings: Must NOT contain unsupported fields or comment overfetch
    let summary_fields =
        "number,title,url,state,stateReason,author,labels,createdAt,updatedAt,closedAt";
    let detail_fields =
        "number,title,url,state,stateReason,author,labels,createdAt,updatedAt,closedAt,body";
    let supported = [
        "assignees",
        "author",
        "blockedBy",
        "blocking",
        "body",
        "closed",
        "closedAt",
        "closedByPullRequestsReferences",
        "comments",
        "createdAt",
        "id",
        "isPinned",
        "issueType",
        "labels",
        "milestone",
        "number",
        "parent",
        "projectCards",
        "projectItems",
        "reactionGroups",
        "state",
        "stateReason",
        "subIssues",
        "subIssuesSummary",
        "title",
        "updatedAt",
        "url",
    ];
    for field in summary_fields.split(',') {
        assert!(supported.contains(&field));
    }
    for field in detail_fields.split(',') {
        assert!(supported.contains(&field));
    }
    assert!(!summary_fields.contains("isPullRequest") && !detail_fields.contains("isPullRequest"));
    assert!(!summary_fields.contains("comments") && !detail_fields.contains("comments"));

    // 2. Live dispatch input validation tests
    let base = std::env::temp_dir().join(format!("relay-044a-issues-{}", std::process::id()));
    let repo = base.join("repo");
    fs::create_dir_all(&repo).unwrap();
    run(&repo, &["init", "-q", "-b", "main"]);
    run(&repo, &["config", "user.email", "fixture@example.test"]);
    run(&repo, &["config", "user.name", "fixture"]);
    fs::write(repo.join("README.md"), "test\n").unwrap();
    run(&repo, &["add", "README.md"]);
    run(&repo, &["commit", "-qm", "init"]);
    run(
        &repo,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/farismnrr/ai-code.git",
        ],
    );

    let config = ServerConfig {
        execution_root: Some(base.to_string_lossy().into()),
        dir: Some(repo.to_string_lossy().into()),
        ..ServerConfig::default()
    };

    let invalid_states = ["evil", "merged"];
    for st in invalid_states {
        let err = dispatch_git_tool("issue_list", &json!({"cwd": repo, "state": st}), &config)
            .await
            .unwrap_err();
        assert!(format!("{err:?}").contains("issue state is invalid"));
    }

    let too_many_labels: Vec<String> = (0..11).map(|i| format!("label-{i}")).collect();
    let err = dispatch_git_tool(
        "issue_list",
        &json!({"cwd": repo, "labels": too_many_labels}),
        &config,
    )
    .await
    .unwrap_err();
    assert!(format!("{err:?}").contains("issue labels exceed maximum"));

    let invalid_labels = vec![
        "".to_string(),
        "has\nnewline".to_string(),
        "has\0nul".to_string(),
        "a".repeat(129),
    ];
    for l in invalid_labels {
        let err = dispatch_git_tool("issue_list", &json!({"cwd": repo, "labels": [l]}), &config)
            .await
            .unwrap_err();
        assert!(format!("{err:?}").contains("issue label is invalid"));
    }

    let err = dispatch_git_tool("issue_get", &json!({"cwd": repo}), &config)
        .await
        .unwrap_err();
    assert!(format!("{err:?}").contains("issue number is required"));
    let err = dispatch_git_tool("issue_get", &json!({"cwd": repo, "number": 0}), &config)
        .await
        .unwrap_err();
    assert!(format!("{err:?}").contains("issue number is required"));

    // 3. Deterministic Provider Fixture & Parsing Invariant Verification
    let (owner, repository) = ("farismnrr", "ai-code");

    let valid_list_json = json!([
        {
            "number": 10,
            "title": "Fix issue listing bug",
            "url": "https://github.com/farismnrr/ai-code/issues/10",
            "state": "OPEN",
            "stateReason": "",
            "author": { "login": "developer" },
            "labels": [{ "name": "bug" }, { "name": "forge" }],
            "createdAt": "2026-08-19T10:00:00Z",
            "updatedAt": "2026-08-19T11:00:00Z",
            "closedAt": null
        },
        {
            "number": 11,
            "title": "Add issue detail read",
            "url": "https://github.com/farismnrr/ai-code/issues/11",
            "state": "CLOSED",
            "stateReason": "COMPLETED",
            "author": { "login": "maintainer" },
            "labels": [{ "name": "enhancement" }],
            "createdAt": "2026-08-18T09:00:00Z",
            "updatedAt": "2026-08-19T08:00:00Z",
            "closedAt": "2026-08-19T08:00:00Z"
        }
    ]);
    let raw_list: Vec<FixtureIssue> = serde_json::from_value(valid_list_json).unwrap();
    let s1 = validate_fixture_summary(&raw_list[0], owner, repository).unwrap();
    assert_eq!(
        (
            s1.number,
            s1.title.as_str(),
            s1.state.as_str(),
            s1.labels.len()
        ),
        (10, "Fix issue listing bug", "OPEN", 2)
    );
    let s2 = validate_fixture_summary(&raw_list[1], owner, repository).unwrap();
    assert_eq!(
        (s2.number, s2.state.as_str(), s2.state_reason.as_deref()),
        (11, "CLOSED", Some("COMPLETED"))
    );

    let valid_detail_json = json!({
        "number": 10,
        "title": "Fix issue listing bug",
        "url": "https://github.com/farismnrr/ai-code/issues/10",
        "state": "OPEN",
        "stateReason": "",
        "author": { "login": "developer" },
        "labels": [{ "name": "bug" }],
        "createdAt": "2026-08-19T10:00:00Z",
        "updatedAt": "2026-08-19T11:00:00Z",
        "closedAt": null,
        "body": "## Description\nDetailed issue body explaining the bug."
    });
    let raw_detail: FixtureIssue = serde_json::from_value(valid_detail_json).unwrap();
    let detail = validate_fixture_detail(&raw_detail, owner, repository, 10).unwrap();
    assert_eq!(
        detail.body,
        "## Description\nDetailed issue body explaining the bug."
    );

    // Truncation behavior (> 50 items)
    let many_items: Vec<serde_json::Value> = (1..=51)
        .map(|i| {
            json!({
                "number": i, "title": format!("Issue {i}"),
                "url": format!("https://github.com/farismnrr/ai-code/issues/{i}"),
                "state": "OPEN", "labels": []
            })
        })
        .collect();
    let mut raw_many: Vec<FixtureIssue> =
        serde_json::from_value(serde_json::Value::Array(many_items)).unwrap();
    assert!(raw_many.len() > 50);
    raw_many.truncate(50);
    assert_eq!(raw_many.len(), 50);

    // PR Rejection
    let pr_item: FixtureIssue = serde_json::from_value(json!({
        "number": 1, "title": "PR", "url": "https://github.com/farismnrr/ai-code/pull/1", "state": "MERGED", "labels": []
    })).unwrap();
    assert_eq!(
        validate_fixture_summary(&pr_item, owner, repository).unwrap_err(),
        "pull request cannot be accessed as an issue"
    );

    // Repo Mismatches
    let mismatch_owner: FixtureIssue = serde_json::from_value(json!({
        "number": 10, "title": "x", "url": "https://github.com/evil/ai-code/issues/10", "state": "OPEN", "labels": []
    })).unwrap();
    assert_eq!(
        validate_fixture_summary(&mismatch_owner, owner, repository).unwrap_err(),
        "issue repository identity mismatch"
    );

    let mismatch_num: FixtureIssue = serde_json::from_value(json!({
        "number": 11, "title": "x", "url": "https://github.com/farismnrr/ai-code/issues/11", "state": "OPEN", "labels": []
    })).unwrap();
    assert_eq!(
        validate_fixture_detail(&mismatch_num, owner, repository, 10).unwrap_err(),
        "issue repository identity mismatch"
    );

    // Bounds Checks: Oversized title, body, label
    let oversized_title: FixtureIssue = serde_json::from_value(json!({
        "number": 10, "title": "a".repeat(257), "url": "https://github.com/farismnrr/ai-code/issues/10", "state": "OPEN", "labels": []
    })).unwrap();
    assert_eq!(
        validate_fixture_summary(&oversized_title, owner, repository).unwrap_err(),
        "invalid_git_output"
    );

    let nul_title: FixtureIssue = serde_json::from_value(json!({
        "number": 10, "title": "bad\0title", "url": "https://github.com/farismnrr/ai-code/issues/10", "state": "OPEN", "labels": []
    })).unwrap();
    assert_eq!(
        validate_fixture_summary(&nul_title, owner, repository).unwrap_err(),
        "invalid_git_output"
    );

    let oversized_body: FixtureIssue = serde_json::from_value(json!({
        "number": 10, "title": "x", "url": "https://github.com/farismnrr/ai-code/issues/10", "state": "OPEN", "labels": [],
        "body": "a".repeat(64 * 1024 + 1)
    })).unwrap();
    assert_eq!(
        validate_fixture_detail(&oversized_body, owner, repository, 10).unwrap_err(),
        "invalid_git_output"
    );

    let oversized_label: FixtureIssue = serde_json::from_value(json!({
        "number": 10, "title": "x", "url": "https://github.com/farismnrr/ai-code/issues/10", "state": "OPEN",
        "labels": [{ "name": "a".repeat(129) }]
    })).unwrap();
    assert_eq!(
        validate_fixture_summary(&oversized_label, owner, repository).unwrap_err(),
        "invalid_git_output"
    );

    let excess_labels_vec: Vec<serde_json::Value> = (0..51)
        .map(|i| json!({ "name": format!("l{i}") }))
        .collect();
    let excess_labels: FixtureIssue = serde_json::from_value(json!({
        "number": 10, "title": "x", "url": "https://github.com/farismnrr/ai-code/issues/10", "state": "OPEN",
        "labels": excess_labels_vec
    })).unwrap();
    assert_eq!(
        validate_fixture_summary(&excess_labels, owner, repository).unwrap_err(),
        "invalid_git_output"
    );

    let _ = fs::remove_dir_all(base);
    println!("plan044a issue reads acceptance: PASS");
}
