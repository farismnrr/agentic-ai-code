use super::{bounded_display, MAX_PATH, MAX_TEXT};
use serde_json::Value;
use std::path::Path;

pub(super) fn target_for_tool(
    tool_id: &str,
    arguments: &Value,
    root: Option<&Path>,
) -> Option<String> {
    if tool_id == "terminal_exec" {
        return arguments
            .get("cwd")
            .and_then(Value::as_str)
            .map(|cwd| relative_display_path(cwd, root))
            .and_then(|cwd| bounded_display(&cwd, MAX_PATH));
    }
    if tool_id == "ssh_readonly_exec" {
        return arguments
            .get("alias")
            .and_then(Value::as_str)
            .map(|alias| sanitize_action_text(alias, None))
            .and_then(|alias| bounded_display(&alias, MAX_PATH));
    }
    let raw = arguments.get("path").and_then(Value::as_str).or_else(|| {
        (tool_id == "http_fetch")
            .then(|| arguments.get("url").and_then(Value::as_str))
            .flatten()
    });
    let raw = raw.or_else(|| (tool_id.starts_with("git_")).then_some("repository"))?;
    let target = if let Some(root) = root.filter(|_| Path::new(raw).is_absolute()) {
        Path::new(raw)
            .strip_prefix(root)
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "workspace target".into())
    } else if tool_id == "http_fetch" {
        url::Url::parse(raw)
            .ok()
            .map(|url| format!("{}{}", url.host_str().unwrap_or("remote"), url.path()))
            .unwrap_or_else(|| "remote endpoint".into())
    } else {
        raw.into()
    };
    bounded_display(&target, MAX_PATH)
}

pub(super) fn action_for_tool(
    tool_id: &str,
    arguments: &Value,
    root: Option<&Path>,
) -> Option<String> {
    let action = match tool_id {
        "terminal_exec" | "terminal_job_start" => terminal_action(arguments, root),
        "ssh_readonly_exec" => ssh_readonly_action(arguments),
        "terminal_job_get" => identifier_action("Get terminal job", arguments, "taskId", root),
        "terminal_job_cancel" => {
            identifier_action("Cancel terminal job", arguments, "taskId", root)
        }
        "file_read" => {
            let target = target_for_tool(tool_id, arguments, root)
                .unwrap_or_else(|| "workspace file".into());
            let offset = arguments
                .get("offset_line")
                .and_then(Value::as_u64)
                .unwrap_or(1);
            let limit = arguments.get("limit_lines").and_then(Value::as_u64);
            match limit {
                Some(limit) => format!(
                    "Read {target} · lines {offset}-{}",
                    offset.saturating_add(limit).saturating_sub(1)
                ),
                None => format!("Read {target}"),
            }
        }
        "file_write" => {
            let target = target_for_tool(tool_id, arguments, root)
                .unwrap_or_else(|| "workspace file".into());
            if arguments
                .get("overwrite")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                format!("Write {target} · overwrite")
            } else {
                format!("Create {target}")
            }
        }
        "file_edit" => {
            let target = target_for_tool(tool_id, arguments, root)
                .unwrap_or_else(|| "workspace file".into());
            let count = arguments
                .get("edits")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(1);
            format!(
                "Edit {target} · {count} replacement{}",
                if count == 1 { "" } else { "s" }
            )
        }
        "apply_patch" => patch_action(arguments, root),
        "directory_list" => {
            let target = target_for_tool(tool_id, arguments, root).unwrap_or_else(|| ".".into());
            format!("List {target}")
        }
        "text_search" => {
            let query = arguments
                .get("query")
                .and_then(Value::as_str)
                .map(|value| sanitize_action_text(value, root));
            let scope = arguments
                .get("glob")
                .and_then(Value::as_str)
                .map(|value| sanitize_action_text(value, root));
            match (query, scope) {
                (Some(query), Some(scope)) => format!("Search text · {query} · {scope}"),
                (Some(query), None) => format!("Search text · {query}"),
                (None, Some(scope)) => format!("Search text · {scope}"),
                (None, None) => "Search text".into(),
            }
        }
        "file_search" => {
            let pattern = arguments
                .get("pattern")
                .and_then(Value::as_str)
                .map(|value| sanitize_action_text(value, root));
            pattern
                .map(|pattern| format!("Search files · {pattern}"))
                .unwrap_or_else(|| "Search files".into())
        }
        "http_fetch" => {
            let method = arguments
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or("GET");
            let target = target_for_tool(tool_id, arguments, root)
                .unwrap_or_else(|| "remote endpoint".into());
            format!("{method} {target}")
        }
        name if name.starts_with("git_") => git_action(name, arguments, root),
        name => generic_action(name, arguments, root),
    };
    bounded_display(&action, MAX_TEXT)
}

fn terminal_action(arguments: &Value, root: Option<&Path>) -> String {
    let command = arguments
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or("command");
    let mut parts = vec![sanitize_action_text(command, root)];
    let mut redact_next = false;
    if let Some(args) = arguments.get("args").and_then(Value::as_array) {
        for value in args.iter().take(32) {
            let Some(arg) = value.as_str() else {
                continue;
            };
            if redact_next {
                parts.push("[REDACTED]".into());
                redact_next = false;
                continue;
            }
            let lower = arg.to_ascii_lowercase();
            redact_next = matches!(
                lower.as_str(),
                "--password"
                    | "--passwd"
                    | "--token"
                    | "--api-key"
                    | "--apikey"
                    | "--secret"
                    | "--client-secret"
                    | "--access-key"
            );
            parts.push(sanitize_action_text(arg, root));
        }
    }
    parts.join(" ")
}

fn ssh_readonly_action(arguments: &Value) -> String {
    let alias = arguments
        .get("alias")
        .and_then(Value::as_str)
        .map(|value| sanitize_action_text(value, None))
        .unwrap_or_else(|| "remote".into());
    let command = arguments
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or("diagnostic");
    let family = if command == "docker" {
        let args = arguments.get("args").and_then(Value::as_array);
        match args
            .and_then(|values| values.first())
            .and_then(Value::as_str)
        {
            Some("exec") => args
                .and_then(|values| values.get(2))
                .and_then(Value::as_str)
                .map(|nested| format!("docker exec {nested}"))
                .unwrap_or_else(|| "docker exec".into()),
            Some(subcommand) => format!("docker {subcommand}"),
            None => "docker".into(),
        }
    } else {
        command.to_owned()
    };
    format!("SSH read-only · {alias} · {family}")
}

fn patch_action(arguments: &Value, root: Option<&Path>) -> String {
    let Some(patch) = arguments.get("patch").and_then(Value::as_str) else {
        return "Apply patch".into();
    };
    let mut paths = Vec::new();
    for line in patch.lines() {
        let candidate = line
            .strip_prefix("*** Update File: ")
            .or_else(|| line.strip_prefix("--- a/"))
            .or_else(|| line.strip_prefix("+++ b/"));
        if let Some(path) = candidate {
            let path = sanitize_action_text(path, root);
            if !path.is_empty() && !paths.contains(&path) {
                paths.push(path);
            }
        }
        if paths.len() >= 3 {
            break;
        }
    }
    if paths.is_empty() {
        "Apply patch".into()
    } else {
        format!("Patch {}", paths.join(", "))
    }
}

fn identifier_action(label: &str, arguments: &Value, key: &str, root: Option<&Path>) -> String {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(|value| format!("{label} · {}", sanitize_action_text(value, root)))
        .unwrap_or_else(|| label.into())
}

fn generic_action(tool_id: &str, arguments: &Value, root: Option<&Path>) -> String {
    let mut action = friendly_tool_name(tool_id);
    let Some(object) = arguments.as_object() else {
        return action;
    };
    let mut details = Vec::new();
    for (key, value) in object {
        if details.len() >= 4
            || sensitive_argument_key(key)
            || matches!(
                key.as_str(),
                "content" | "data" | "headers" | "edits" | "patch"
            )
        {
            continue;
        }
        let rendered = match value {
            Value::String(value) => Some(sanitize_action_text(value, root)),
            Value::Number(value) => Some(value.to_string()),
            Value::Bool(value) => Some(value.to_string()),
            _ => None,
        };
        if let Some(rendered) = rendered.filter(|value| !value.is_empty()) {
            details.push(format!("{key}={rendered}"));
        }
    }
    if !details.is_empty() {
        action.push_str(" · ");
        action.push_str(&details.join(" · "));
    }
    action
}

fn sensitive_argument_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "password",
        "passwd",
        "token",
        "secret",
        "authorization",
        "cookie",
        "api_key",
        "apikey",
        "access_key",
        "client_secret",
    ]
    .iter()
    .any(|candidate| key.contains(candidate))
}

fn git_action(tool_id: &str, arguments: &Value, root: Option<&Path>) -> String {
    let mut action = friendly_tool_name(tool_id);
    for key in ["branch", "ref", "commit", "name", "path"] {
        if let Some(value) = arguments.get(key).and_then(Value::as_str) {
            action.push_str(" · ");
            action.push_str(&sanitize_action_text(value, root));
            break;
        }
    }
    action
}

fn friendly_tool_name(tool_id: &str) -> String {
    tool_id.replace('_', " ")
}

fn relative_display_path(raw: &str, root: Option<&Path>) -> String {
    let path = Path::new(raw);
    if path.is_absolute() {
        if let Some(root) = root {
            if let Ok(relative) = path.strip_prefix(root) {
                let display = relative.to_string_lossy();
                return if display.is_empty() {
                    ".".into()
                } else {
                    display.into_owned()
                };
            }
        }
        return "workspace path".into();
    }
    raw.into()
}

fn sanitize_action_text(raw: &str, root: Option<&Path>) -> String {
    let mut value = relay_core::redaction::redact_credentials(raw);
    if let Some(root) = root {
        let root = root.to_string_lossy();
        if !root.is_empty() {
            value = value.replace(root.as_ref(), ".");
        }
    }
    value
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
