use super::super::*;
use super::common::*;
use relay_core::redaction::redact_credentials;
use serde_json::Value;
mod model;
use model::*;

const MAX_ALERTS: usize = 50;
const MAX_LOCATIONS: usize = 50;
const MAX_TEXT: usize = 512;

fn text(v: String) -> Result<String, McpError> {
    if v.len() > MAX_TEXT || v.chars().any(|c| c == '\0' || c == '\r' || c == '\n') {
        Err(McpError::InvalidRequest(
            "security alert field is invalid".into(),
        ))
    } else {
        Ok(redact_credentials(&v))
    }
}
fn opt_text(v: Option<String>) -> Result<Option<String>, McpError> {
    v.map(text).transpose()
}
fn path(v: String) -> Result<String, McpError> {
    if v.starts_with('/') || v.split('/').any(|p| p == "..") || v.contains('\0') {
        Err(McpError::InvalidRequest(
            "security alert path is invalid".into(),
        ))
    } else {
        text(v)
    }
}
fn sha(v: String) -> Result<String, McpError> {
    if v.len() == 40 && v.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(v)
    } else {
        Err(McpError::InvalidRequest(
            "security alert sha is invalid".into(),
        ))
    }
}
fn url(v: String, remote: &remote::GitRemoteIdentity) -> Result<String, McpError> {
    let prefix = format!("https://github.com/{}/{}/", remote.owner, remote.repository);
    if v.starts_with(&prefix) && v.len() <= 2048 {
        Ok(v)
    } else {
        Err(McpError::InvalidRequest(
            "security alert url is invalid".into(),
        ))
    }
}
fn alert_number(a: &Value) -> Result<u64, McpError> {
    a.get("alert_number")
        .and_then(Value::as_u64)
        .filter(|n| *n > 0)
        .ok_or_else(|| McpError::InvalidRequest("alert_number is invalid".into()))
}
fn filter(a: &Value, key: &str, allowed: &[&str]) -> Result<Option<String>, McpError> {
    let Some(v) = a.get(key) else { return Ok(None) };
    let s = v
        .as_str()
        .ok_or_else(|| McpError::InvalidRequest(format!("{key} is invalid")))?;
    if allowed.contains(&s) {
        Ok(Some(s.into()))
    } else {
        Err(McpError::InvalidRequest(format!("{key} is invalid")))
    }
}
fn api_args(endpoint: String, query: &[(String, String)]) -> Vec<String> {
    let mut a = vec![
        "api".into(),
        "--method".into(),
        "GET".into(),
        "-H".into(),
        "Accept: application/vnd.github+json".into(),
        "-H".into(),
        "X-GitHub-Api-Version: 2022-11-28".into(),
        endpoint,
    ];
    for (k, v) in query {
        a.push("-f".into());
        a.push(format!("{k}={v}"));
    }
    a
}
async fn api<T: serde::de::DeserializeOwned>(
    root: &std::path::Path,
    args: Vec<String>,
) -> Result<T, McpError> {
    parse_json(&forge_process::run_gh(root, &args, &[]).await?)
}
fn endpoint(remote: &remote::GitRemoteIdentity, suffix: &str) -> String {
    format!("repos/{}/{}/{}", remote.owner, remote.repository, suffix)
}

fn dep(p: ProviderDependabot, r: &remote::GitRemoteIdentity) -> Result<DependabotAlert, McpError> {
    Ok(DependabotAlert {
        number: p.number,
        state: text(p.state)?,
        package: text(p.dependency.package.name)?,
        ecosystem: text(p.dependency.package.ecosystem)?,
        manifest_path: p.dependency.manifest_path.map(path).transpose()?,
        scope: opt_text(p.dependency.scope)?,
        ghsa_id: text(p.security_advisory.ghsa_id)?,
        cve_id: opt_text(p.security_advisory.cve_id)?,
        severity: text(p.security_advisory.severity)?,
        vulnerable_range: text(p.security_vulnerability.vulnerable_version_range)?,
        patched_version: p
            .security_vulnerability
            .first_patched_version
            .map(|v| text(v.identifier))
            .transpose()?,
        created_at: text(p.created_at)?,
        updated_at: text(p.updated_at)?,
        dismissed_at: opt_text(p.dismissed_at)?,
        fixed_at: opt_text(p.fixed_at)?,
        url: url(p.html_url, r)?,
    })
}
fn code(p: ProviderCode, r: &remote::GitRemoteIdentity) -> Result<CodeAlert, McpError> {
    let i = p.most_recent_instance;
    let (rf, sh, loc) = if let Some(i) = i {
        (
            opt_text(i.git_ref)?,
            i.commit_sha.map(sha).transpose()?,
            i.location
                .map(|l| {
                    Ok::<_, McpError>(CodeLocation {
                        path: path(l.path)?,
                        start_line: l.start_line,
                        end_line: l.end_line,
                        start_column: l.start_column,
                        end_column: l.end_column,
                    })
                })
                .transpose()?,
        )
    } else {
        (None, None, None)
    };
    Ok(CodeAlert {
        number: p.number,
        state: text(p.state)?,
        dismissed_reason: opt_text(p.dismissed_reason)?,
        rule_id: text(p.rule.id)?,
        rule_name: opt_text(p.rule.name)?,
        security_severity: opt_text(p.rule.security_severity_level)?,
        description: opt_text(p.rule.description)?,
        tool_name: text(p.tool.name)?,
        tool_version: opt_text(p.tool.version)?,
        instance_ref: rf,
        commit_sha: sh,
        location: loc,
        url: url(p.html_url, r)?,
    })
}
fn secret(p: ProviderSecret, r: &remote::GitRemoteIdentity) -> Result<SecretAlert, McpError> {
    Ok(SecretAlert {
        number: p.number,
        state: text(p.state)?,
        resolution: opt_text(p.resolution)?,
        secret_type: text(p.secret_type)?,
        secret_type_display_name: opt_text(p.secret_type_display_name)?,
        validity: opt_text(p.validity)?,
        publicly_leaked: p.publicly_leaked,
        multi_repo: p.multi_repo,
        push_protection_bypassed: p.push_protection_bypassed,
        push_protection_bypassed_at: opt_text(p.push_protection_bypassed_at)?,
        created_at: text(p.created_at)?,
        resolved_at: opt_text(p.resolved_at)?,
        url: url(p.html_url, r)?,
    })
}

macro_rules! list_fn {
    ($name:ident,$provider:ty,$dto:ty,$suffix:expr,$convert:ident,$arg:ident,$query:block) => {
        pub(in crate::git) async fn $name(
            arguments: &Value,
            config: &ServerConfig,
        ) -> Result<SecurityListResult<$dto>, McpError> {
            let repo = resolve_repo(arguments, config)?;
            let remote = remote::requested_remote(&repo.root, arguments)?;
            let $arg = arguments;
            let mut q: Vec<(String, String)> = $query;
            q.push(("per_page".into(), (MAX_ALERTS + 1).to_string()));
            let raw: Vec<$provider> =
                api(&repo.root, api_args(endpoint(&remote, $suffix), &q)).await?;
            let truncated = raw.len() > MAX_ALERTS;
            let alerts = raw
                .into_iter()
                .take(MAX_ALERTS)
                .map(|p| $convert(p, &remote))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(SecurityListResult {
                repository_root: repo.relative_root,
                forge: forge_identity(&remote),
                alerts,
                truncated,
            })
        }
    };
}
list_fn!(
    dependabot_alert_list,
    ProviderDependabot,
    DependabotAlert,
    "dependabot/alerts",
    dep,
    arguments,
    {
        let mut q = Vec::new();
        if let Some(v) = filter(
            arguments,
            "state",
            &["auto_dismissed", "dismissed", "fixed", "open"],
        )? {
            q.push(("state".into(), v))
        }
        if let Some(v) = filter(
            arguments,
            "severity",
            &["low", "medium", "high", "critical"],
        )? {
            q.push(("severity".into(), v))
        }
        q
    }
);
list_fn!(
    code_scanning_alert_list,
    ProviderCode,
    CodeAlert,
    "code-scanning/alerts",
    code,
    arguments,
    {
        let mut q = Vec::new();
        if let Some(v) = filter(
            arguments,
            "state",
            &["open", "closed", "dismissed", "fixed"],
        )? {
            q.push(("state".into(), v))
        }
        if let Some(v) = filter(arguments, "severity", &["none", "note", "warning", "error"])? {
            q.push(("severity".into(), v))
        }
        q
    }
);
list_fn!(
    secret_scanning_alert_list,
    ProviderSecret,
    SecretAlert,
    "secret-scanning/alerts",
    secret,
    arguments,
    {
        let mut q = vec![("hide_secret".into(), "true".into())];
        if let Some(v) = filter(arguments, "state", &["open", "resolved"])? {
            q.push(("state".into(), v))
        }
        if let Some(v) = filter(arguments, "validity", &["active", "inactive", "unknown"])? {
            q.push(("validity".into(), v))
        }
        q
    }
);

macro_rules! get_fn {
    ($name:ident,$provider:ty,$dto:ty,$base:expr,$convert:ident,$secret:expr) => {
        pub(in crate::git) async fn $name(
            arguments: &Value,
            config: &ServerConfig,
        ) -> Result<SecurityGetResult<$dto>, McpError> {
            let repo = resolve_repo(arguments, config)?;
            let remote = remote::requested_remote(&repo.root, arguments)?;
            let n = alert_number(arguments)?;
            let mut q = Vec::new();
            if $secret {
                q.push(("hide_secret".into(), "true".into()));
            }
            let raw: $provider = api(
                &repo.root,
                api_args(endpoint(&remote, &format!("{}/{}", $base, n)), &q),
            )
            .await?;
            if raw.number != n {
                return Err(McpError::InvalidRequest(
                    "security alert identity mismatch".into(),
                ));
            }
            Ok(SecurityGetResult {
                repository_root: repo.relative_root,
                forge: forge_identity(&remote),
                alert: $convert(raw, &remote)?,
            })
        }
    };
}
get_fn!(
    dependabot_alert_get,
    ProviderDependabot,
    DependabotAlert,
    "dependabot/alerts",
    dep,
    false
);
get_fn!(
    code_scanning_alert_get,
    ProviderCode,
    CodeAlert,
    "code-scanning/alerts",
    code,
    false
);
get_fn!(
    secret_scanning_alert_get,
    ProviderSecret,
    SecretAlert,
    "secret-scanning/alerts",
    secret,
    true
);

pub(in crate::git) async fn secret_scanning_alert_locations(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<SecretLocationsResult, McpError> {
    let repo = resolve_repo(arguments, config)?;
    let remote = remote::requested_remote(&repo.root, arguments)?;
    let n = alert_number(arguments)?;
    let q = vec![("per_page".into(), (MAX_LOCATIONS + 1).to_string())];
    let raw: Vec<ProviderSecretLocation> = api(
        &repo.root,
        api_args(
            endpoint(&remote, &format!("secret-scanning/alerts/{n}/locations")),
            &q,
        ),
    )
    .await?;
    let truncated = raw.len() > MAX_LOCATIONS;
    let mut locations = Vec::new();
    for p in raw.into_iter().take(MAX_LOCATIONS) {
        let d = &p.details;
        let kind = text(p.kind)?;
        let location = SecretLocation {
            kind,
            commit_sha: d
                .get("commit_sha")
                .and_then(Value::as_str)
                .map(|s| sha(s.into()))
                .transpose()?,
            blob_sha: d
                .get("blob_sha")
                .and_then(Value::as_str)
                .map(|s| sha(s.into()))
                .transpose()?,
            path: d
                .get("path")
                .and_then(Value::as_str)
                .map(|s| path(s.into()))
                .transpose()?,
            start_line: d.get("start_line").and_then(Value::as_u64),
            end_line: d.get("end_line").and_then(Value::as_u64),
            start_column: d.get("start_column").and_then(Value::as_u64),
            end_column: d.get("end_column").and_then(Value::as_u64),
        };
        locations.push(location)
    }
    Ok(SecretLocationsResult {
        repository_root: repo.relative_root,
        forge: forge_identity(&remote),
        alert_number: n,
        locations,
        truncated,
    })
}
