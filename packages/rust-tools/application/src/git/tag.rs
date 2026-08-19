//! Structured Git tag operations.

use super::context::resolve_repo;
use super::process::{run_git, validate_ref};
use super::MAX_GIT_OUTPUT_BYTES;
use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Serialize, Clone)]
pub struct TagEntry {
    pub name: String,
    pub commit_sha: String,
    pub subject: String,
    pub tagger: Option<String>,
    pub date: Option<String>,
}

pub fn git_tag_list(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let out = run_git(
        &repo.root,
        &[
            "for-each-ref",
            "--sort=-creatordate",
            "--format=%(refname:short)%09%(objectname)%09%(subject)%09%(taggername)%09%(taggerdate:iso)",
            "refs/tags",
        ],
        MAX_GIT_OUTPUT_BYTES,
    )?;
    let text = std::str::from_utf8(&out).unwrap_or("");
    let mut tags = Vec::new();

    for line in text.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 && !parts[0].is_empty() {
            let name = parts[0];
            let commit_sha = parts[1];
            let subject = parts.get(2).copied().unwrap_or("").to_string();
            let tagger = parts
                .get(3)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            let date = parts
                .get(4)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());

            tags.push(TagEntry {
                name: name.to_string(),
                commit_sha: commit_sha.to_string(),
                subject,
                tagger,
                date,
            });
        }
    }

    Ok(json!({
        "tags": tags,
        "total": tags.len()
    }))
}

pub fn git_tag_create(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    super::mutation::validate_mutation_config(&repo.root)?;
    let name = arguments
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("tag name is required".into()))?;
    validate_ref(name)?;

    let target = arguments.get("target").and_then(Value::as_str);
    if let Some(tgt) = target {
        validate_ref(tgt)?;
    }

    let message = arguments.get("message").and_then(Value::as_str);
    if message.is_some_and(|msg| msg.len() > 4096 || msg.contains('\0')) {
        return Err(McpError::InvalidRequest("tag message is invalid".into()));
    }

    let mut args: Vec<&str> = vec!["tag"];
    if let Some(msg) = message {
        args.extend(["-a", name, "-m", msg]);
    } else {
        args.push(name);
    }
    if let Some(tgt) = target {
        args.push(tgt);
    }

    run_git(&repo.root, &args, 8192)?;

    Ok(json!({
        "name": name,
        "created": true,
    }))
}

pub fn git_tag_delete(arguments: &Value, config: &ServerConfig) -> Result<Value, McpError> {
    let repo = resolve_repo(arguments, config)?;
    super::mutation::validate_mutation_config(&repo.root)?;
    let name = arguments
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| McpError::InvalidRequest("tag name is required".into()))?;
    validate_ref(name)?;

    run_git(&repo.root, &["tag", "-d", name], 8192)?;

    Ok(json!({
        "name": name,
        "deleted": true,
    }))
}
