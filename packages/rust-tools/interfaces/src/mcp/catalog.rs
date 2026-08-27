mod file_edit;
mod forge;

use relay_core::error::McpError;
use serde::Serialize;
use serde_json::{json, Value};

/// A single MCP tool definition: stable name, human description, and a
/// JSON Schema 2020-12-compatible `inputSchema`.
#[derive(Debug, Clone, Serialize)]
pub struct Tool {
    pub name: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<&'static str>,
    pub description: &'static str,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,
    #[serde(rename = "securitySchemes")]
    pub security_schemes: Vec<ToolSecurityScheme>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolAnnotations {
    pub read_only_hint: bool,
    pub destructive_hint: bool,
    pub idempotent_hint: bool,
    pub open_world_hint: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSecurityScheme {
    #[serde(rename = "type")]
    pub scheme_type: &'static str,
    pub scopes: Vec<&'static str>,
}

/// OAuth scope required by the coding MCP surface.
pub const CODING_SCOPE: &str = "relay.coding";

fn coding_security_scheme() -> Vec<ToolSecurityScheme> {
    vec![ToolSecurityScheme {
        scheme_type: "oauth2",
        scopes: vec![CODING_SCOPE],
    }]
}

/// The canonical MCP tool catalog, mapping 1:1 onto the Plan 027 Rust CLI
/// binaries. Execution is deliberately not wired here (Phase 3) — this only
/// describes the surface a client can discover and validate calls against.
pub fn tool_catalog() -> Vec<Tool> {
    let mut tools = vec![
        Tool {
            name: "terminal_exec",
            title: Some("Sandboxed Coding Terminal"),
            description: "Run a sandboxed executable in the workspace using direct argv semantics. Shell operators such as ;, |, &&, globbing, and redirection are not implicit; use command=sh with args=[\"-lc\", \"...\"] when shell syntax is required. Supports scripts, builds, package managers, Git, and interpreters. Returns stdout, stderr, and exit status.",
            input_schema: json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "command": { "type": "string", "minLength": 1, "maxLength": 65536 },
                    "args": {
                        "type": "array",
                        "items": { "type": "string", "maxLength": 65536 },
                        "maxItems": 100,
                        "default": []
                    },
                    "cwd": { "type": "string" },
                    "timeout_ms": {
                        "type": "integer",
                        "minimum": 0,
                        "default": 30000,
                        "description": "Requested command runtime in milliseconds. Choose a realistic value for the operation; 0 means no command deadline unless the relay operator configured a maximum."
                    },
                    "execution_mode": {
                        "type": "string",
                        "enum": ["sync", "async", "auto"],
                        "default": "auto",
                        "description": "Use sync for short commands whose result is needed immediately, async for long-running work that should survive the initial request, or auto to use task execution when the client supports Tasks and the call is safe to resume."
                    },
                    "idempotency_key": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": 128,
                        "description": "Stable key for one logical async command. Required for async terminal execution so retries or lost responses resolve to the same accepted task instead of running the command twice."
                    }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
            annotations: Some(ToolAnnotations {
                read_only_hint: false,
                destructive_hint: true,
                idempotent_hint: false,
                open_world_hint: true,
            }),
            security_schemes: coding_security_scheme(),
            execution: Some(json!({ "taskSupport": "optional" })),
        },
        Tool {
            name: "http_fetch",
            title: Some("HTTP Fetch"),
            description: "Make an HTTP(S) request and return the response; methods may mutate remote state.",
            input_schema: json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "url": { "type": "string", "format": "uri", "maxLength": 65536 },
                    "method": {
                        "type": "string",
                        "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"],
                        "default": "GET"
                    },
                    "headers": {
                        "type": "object",
                        "maxProperties": 100,
                        "additionalProperties": { "type": "string", "maxLength": 65536 }
                    },
                    "data": { "type": "string", "maxLength": 65536 },
                    "timeout_ms": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 300000,
                        "default": 30000,
                        "description": "Requested HTTP operation timeout in milliseconds. Choose it based on expected latency; the relay still enforces this tool's maximum."
                    },
                    "execution_mode": {
                        "type": "string",
                        "enum": ["sync", "async", "auto"],
                        "default": "auto",
                        "description": "Use sync for short requests, async for eligible long-running safe requests, or auto to use task execution when supported. Mutating HTTP methods remain synchronous until request-level idempotency is available."
                    },
                    "idempotency_key": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": 128,
                        "description": "Stable logical-operation key reserved for task-backed requests that require retry deduplication."
                    }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
            annotations: Some(ToolAnnotations {
                read_only_hint: false,
                destructive_hint: true,
                idempotent_hint: false,
                open_world_hint: true,
            }),
            security_schemes: coding_security_scheme(),
            execution: Some(json!({ "taskSupport": "optional" })),
        },
        Tool {
            name: "web_search",
            title: Some("Web Search"),
            description: "Search the web through the configured SearxNG backend and return matching results.",
            input_schema: json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "query": { "type": "string", "minLength": 1, "maxLength": 65536 },
                    "execution_mode": {
                        "type": "string",
                        "enum": ["sync", "async", "auto"],
                        "default": "auto",
                        "description": "Use sync for an immediate result, async when the search may take longer and the client can poll MCP Tasks, or auto to let the relay select task execution when supported."
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
            annotations: Some(ToolAnnotations {
                read_only_hint: true,
                destructive_hint: false,
                idempotent_hint: true,
                open_world_hint: true,
            }),
            security_schemes: coding_security_scheme(),
            execution: Some(json!({ "taskSupport": "optional" })),
        },
        Tool {
            name: "directory_list",
            title: Some("Directory List"),
            description: "List a workspace directory with deterministic ordering, bounded recursion, entry types, and explicit truncation without following symlink directories.",
            input_schema: json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "path": { "type": "string", "maxLength": 65536, "default": "." },
                    "cwd": { "type": "string", "maxLength": 65536 },
                    "depth": { "type": "integer", "minimum": 0, "maximum": 4, "default": 2 },
                    "max_entries": { "type": "integer", "minimum": 1, "maximum": 100, "default": 100 },
                    "continuation": { "type": "string", "maxLength": 4096 }
                },
                "additionalProperties": false
            }),
            annotations: Some(ToolAnnotations {
                read_only_hint: true,
                destructive_hint: false,
                idempotent_hint: true,
                open_world_hint: false,
            }),
            security_schemes: coding_security_scheme(),
            execution: None,
        },
        Tool {
            name: "file_search",
            title: Some("File Search"),
            description: "Search regular workspace files using a bounded glob subset (*, ?, and ** path segments) with deterministic cwd-relative results. Hidden files are searchable; .git, node_modules, target, .nuxt, and .output directories are skipped; symlinks observed during traversal are not followed recursively. On Linux, descendant traversal uses stable directory descriptors with no-follow opens. Native entries whose names are not valid UTF-8 are omitted from JSON results.",
            input_schema: json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "minLength": 1, "maxLength": 4096 },
                    "cwd": { "type": "string", "maxLength": 4096 },
                    "max_results": { "type": "integer", "minimum": 1, "maximum": 100, "default": 100 },
                    "continuation": { "type": "string", "maxLength": 4096 }
                },
                "required": ["pattern"],
                "additionalProperties": false
            }),
            annotations: Some(ToolAnnotations {
                read_only_hint: true,
                destructive_hint: false,
                idempotent_hint: true,
                open_world_hint: false,
            }),
            security_schemes: coding_security_scheme(),
            execution: None,
        },
        Tool {
            name: "file_write",
            title: Some("File Write"),
            description: "Atomically create or explicitly overwrite a contained UTF-8 text file. overwrite=false and create_parents=false are the defaults. Parent traversal uses no-follow directory descriptors; symlinked parents/final targets and root escapes are rejected. New files use mode 0644; overwrites preserve existing permissions.",
            input_schema: json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "path": { "type": "string", "minLength": 1, "maxLength": 4096 },
                    "content": { "type": "string", "maxLength": 1048576 },
                    "cwd": { "type": "string", "maxLength": 4096 },
                    "create_parents": { "type": "boolean", "default": false },
                    "overwrite": { "type": "boolean", "default": false }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
            annotations: Some(ToolAnnotations {
                read_only_hint: false, destructive_hint: true, idempotent_hint: false, open_world_hint: false,
            }),
            security_schemes: coding_security_scheme(),
            execution: None,
        },
        file_edit::tool(),
        Tool {
            name: "file_read",
            title: Some("File Read"),
            description: "Read a contained UTF-8 text file using 1-based line ranges with hard line/byte bounds and explicit truncation. Directories, invalid UTF-8, external symlink targets, oversized lines, and out-of-root paths are rejected.",
            input_schema: json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "path": { "type": "string", "minLength": 1, "maxLength": 4096 },
                    "cwd": { "type": "string", "maxLength": 4096 },
                    "offset_line": { "type": "integer", "minimum": 1, "default": 1 },
                    "limit_lines": { "type": "integer", "minimum": 1, "maximum": 1000, "default": 200 }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
            annotations: Some(ToolAnnotations {
                read_only_hint: true, destructive_hint: false, idempotent_hint: true, open_world_hint: false,
            }),
            security_schemes: coding_security_scheme(),
            execution: None,
        },
        Tool {
            name: "text_search",
            title: Some("Text Search"),
            description: "Search workspace text with ripgrep using direct argv in a read-only execution-root sandbox. Defaults to literal, case-sensitive matching; regex=true enables regex syntax. Ripgrep's normal hidden/ignore behavior applies, symlinks are not followed, previews and total results are server-bounded.",
            input_schema: json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "query": { "type": "string", "minLength": 1, "maxLength": 4096 },
                    "cwd": { "type": "string", "maxLength": 4096 },
                    "glob": { "type": "string", "minLength": 1, "maxLength": 4096 },
                    "regex": { "type": "boolean", "default": false },
                    "case_sensitive": { "type": "boolean", "default": true },
                    "max_results": { "type": "integer", "minimum": 1, "maximum": 100, "default": 50 },
                    "continuation": { "type": "string", "maxLength": 4096 }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
            annotations: Some(ToolAnnotations {
                read_only_hint: true,
                destructive_hint: false,
                idempotent_hint: true,
                open_world_hint: false,
            }),
            security_schemes: coding_security_scheme(),
            execution: None,
        },
        Tool { name: "git_status", title: Some("Git Status"), description: "Inspect bounded structured repository status without invoking user Git helpers.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"include_untracked":{"type":"boolean","default":true}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_diff", title: Some("Git Diff"), description: "Read a bounded Git diff with external diff and textconv disabled; continuation is signed and bound to the repository and diff query.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"mode":{"type":"string","enum":["working","staged","refs"],"default":"working"},"base_ref":{"type":"string","maxLength":512},"head_ref":{"type":"string","maxLength":512},"path":{"type":"string","maxLength":4096},"context_lines":{"type":"integer","minimum":0,"maximum":20,"default":3},"max_bytes":{"type":"integer","minimum":1,"maximum":262144,"default":65536},"continuation":{"type":"string","maxLength":4096}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_log", title: Some("Git Log"), description: "Read bounded commit metadata from repository history.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"ref":{"type":"string","maxLength":512},"path":{"type":"string","maxLength":4096},"max_results":{"type":"integer","minimum":1,"maximum":100,"default":50},"continuation":{"type":"string","maxLength":4096}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_show", title: Some("Git Show"), description: "Read bounded commit/object presentation with executable diff helpers disabled; continuation is signed and bound to the resolved object.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"ref":{"type":"string","minLength":1,"maxLength":512},"path":{"type":"string","maxLength":4096},"include_patch":{"type":"boolean","default":true},"max_bytes":{"type":"integer","minimum":1,"maximum":262144,"default":65536},"continuation":{"type":"string","maxLength":4096}},"required":["ref"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_blame", title: Some("Git Blame"), description: "Read bounded line-to-commit mappings for one contained repository file.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"path":{"type":"string","minLength":1,"maxLength":4096},"start_line":{"type":"integer","minimum":1,"default":1},"end_line":{"type":"integer","minimum":1}},"required":["path"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_branch_list", title: Some("Git Branch List"), description: "List bounded local branch identities and current/upstream facts without invoking user Git helpers.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_branch_create", title: Some("Git Branch Create"), description: "Create one validated local branch at HEAD or an explicit contained commit start point without switching the worktree.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"name":{"type":"string","minLength":1,"maxLength":512},"start_point":{"type":"string","minLength":1,"maxLength":512}},"required":["name"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:false, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_branch_switch", title: Some("Git Branch Switch"), description: "Switch to one existing validated local branch without force, detach, remote guessing, or carrying a dirty worktree.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"name":{"type":"string","minLength":1,"maxLength":512}},"required":["name"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_stage", title: Some("Git Stage"), description: "Stage an explicit bounded set of contained repository file paths; protected paths and unsafe symlinks are denied.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"paths":{"type":"array","minItems":1,"maxItems":64,"items":{"type":"string","minLength":1,"maxLength":4096}}},"required":["paths"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_unstage", title: Some("Git Unstage"), description: "Unstage an explicit bounded set of contained repository file paths without changing worktree content.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"paths":{"type":"array","minItems":1,"maxItems":64,"items":{"type":"string","minLength":1,"maxLength":4096}}},"required":["paths"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_commit", title: Some("Git Commit"), description: "Create one local commit from the already-staged index using bounded message input and repository-local identity only; user hooks and global Git config remain disabled.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"message":{"type":"string","minLength":1,"maxLength":4096}},"required":["message"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:false, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_commit_amend", title: Some("Git Commit Amend"), description: "Amend HEAD with staged changes and/or a bounded replacement message; user hooks, signing, editors, and global Git config remain disabled.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"message":{"type":"string","minLength":1,"maxLength":4096}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_operation_status", title: Some("Git Operation Status"), description: "Inspect bounded structured merge/rebase state and conflicted paths for the contained repository.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_merge_start", title: Some("Git Merge Start"), description: "Start a bounded local no-ff merge from a validated commit ref; conflicts are returned through structured operation state.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"ref":{"type":"string","minLength":1,"maxLength":512}},"required":["ref"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:false, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_merge_continue", title: Some("Git Merge Continue"), description: "Continue an active merge only after all conflicts are resolved and staged.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:false, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_merge_abort", title: Some("Git Merge Abort"), description: "Abort the active contained-repository merge operation.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_rebase_start", title: Some("Git Rebase Start"), description: "Start a bounded local rebase onto a validated commit ref; conflicts are returned through structured operation state.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"ref":{"type":"string","minLength":1,"maxLength":512}},"required":["ref"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_rebase_continue", title: Some("Git Rebase Continue"), description: "Continue an active rebase only after all conflicts are resolved and staged.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_rebase_abort", title: Some("Git Rebase Abort"), description: "Abort the active contained-repository rebase operation.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_branch_delete", title: Some("Git Branch Delete"), description: "Delete one non-current local branch using Git's merged-state safety check; force deletion is unavailable.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"name":{"type":"string","minLength":1,"maxLength":512}},"required":["name"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_remote_list", title: Some("Git Remote List"), description: "Inspect validated repository remotes and canonical GitHub repository identity without exposing credentials or user Git config.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_remote_branch_get", title: Some("Git Remote Branch Get"), description: "Read one validated remote branch head through the narrow authenticated Git transport and return bounded identity facts.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"remote":{"type":"string","minLength":1,"maxLength":64,"default":"origin"},"branch":{"type":"string","minLength":1,"maxLength":512}},"required":["branch"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:true }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_fetch", title: Some("Git Fetch"), description: "Fetch one validated GitHub branch into its bounded remote-tracking ref using the credential-isolated native transport.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"remote":{"type":"string","minLength":1,"maxLength":64,"default":"origin"},"branch":{"type":"string","minLength":1,"maxLength":512}},"required":["branch"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:false, idempotent_hint:true, open_world_hint:true }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_push", title: Some("Git Push"), description: "Push one validated local branch to the same-name branch of the validated GitHub remote without force, hooks, arbitrary refspecs, or credential exposure; verify the resulting remote head.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"remote":{"type":"string","minLength":1,"maxLength":64,"default":"origin"},"branch":{"type":"string","minLength":1,"maxLength":512},"set_upstream":{"type":"boolean","default":false}},"required":["branch"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:true, open_world_hint:true }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "workspace_add", title: Some("Workspace Add"), description: "Explicitly authorize an additional existing directory as a workspace root for the current session. Rejects root/system directories and credential paths.", input_schema: json!({"type":"object","properties":{"path":{"type":"string","minLength":1,"maxLength":4096}},"required":["path"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "workspace_list", title: Some("Workspace List"), description: "List all currently authorized workspace roots, including primary and dynamically authorized roots.", input_schema: json!({"type":"object","properties":{},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "workspace_get", title: Some("Workspace Get"), description: "Inspect an authorized workspace root by path.", input_schema: json!({"type":"object","properties":{"path":{"type":"string","minLength":1,"maxLength":4096}},"required":["path"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "workspace_remove", title: Some("Workspace Remove"), description: "Remove a dynamically authorized workspace root from the allowlist. Primary workspace root cannot be removed.", input_schema: json!({"type":"object","properties":{"path":{"type":"string","minLength":1,"maxLength":4096}},"required":["path"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_worktree_list", title: Some("Git Worktree List"), description: "List bounded worktrees for the current Git repository with containment and branch status.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_worktree_get", title: Some("Git Worktree Get"), description: "Inspect a specific Git worktree by path.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"path":{"type":"string","minLength":1,"maxLength":4096}},"required":["path"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_worktree_add", title: Some("Git Worktree Add"), description: "Create and register a new Git worktree at a validated destination within an authorized workspace root.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"path":{"type":"string","minLength":1,"maxLength":4096},"branch":{"type":"string","maxLength":512},"commit":{"type":"string","maxLength":512},"create_branch":{"type":"string","maxLength":512},"force":{"type":"boolean","default":false}},"required":["path"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:false, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_worktree_remove", title: Some("Git Worktree Remove"), description: "Remove a linked Git worktree.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"path":{"type":"string","minLength":1,"maxLength":4096},"force":{"type":"boolean","default":false}},"required":["path"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_worktree_prune", title: Some("Git Worktree Prune"), description: "Prune stale worktree metadata.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"dry_run":{"type":"boolean","default":false},"expire":{"type":"string","maxLength":64}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_stash_list", title: Some("Git Stash List"), description: "List bounded stashes in the repository.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_stash_push", title: Some("Git Stash Push"), description: "Stash modified and untracked changes with optional message and path filters.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"message":{"type":"string","maxLength":4096},"include_untracked":{"type":"boolean","default":false},"keep_index":{"type":"boolean","default":false},"paths":{"type":"array","items":{"type":"string","maxLength":4096},"maxItems":64}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:false, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_stash_pop", title: Some("Git Stash Pop"), description: "Apply and drop a stash by index.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"index":{"type":"integer","minimum":0,"default":0}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_stash_apply", title: Some("Git Stash Apply"), description: "Apply a stash by index without dropping it.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"index":{"type":"integer","minimum":0,"default":0}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:false, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_stash_drop", title: Some("Git Stash Drop"), description: "Drop a stash by index.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"index":{"type":"integer","minimum":0,"default":0}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_tag_list", title: Some("Git Tag List"), description: "List bounded repository tags with commit SHA and subject.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_tag_create", title: Some("Git Tag Create"), description: "Create a lightweight or annotated Git tag at a target ref.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"name":{"type":"string","minLength":1,"maxLength":512},"target":{"type":"string","maxLength":512},"message":{"type":"string","maxLength":4096}},"required":["name"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:false, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_tag_delete", title: Some("Git Tag Delete"), description: "Delete a Git tag by name.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"name":{"type":"string","minLength":1,"maxLength":512}},"required":["name"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_branch_rename", title: Some("Git Branch Rename"), description: "Rename a local Git branch.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"old_name":{"type":"string","maxLength":512},"new_name":{"type":"string","minLength":1,"maxLength":512},"force":{"type":"boolean","default":false}},"required":["new_name"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_restore", title: Some("Git Restore"), description: "Restore working tree or staged files from HEAD or a specified source ref.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"paths":{"type":"array","items":{"type":"string","minLength":1,"maxLength":4096},"minItems":1,"maxItems":64},"staged":{"type":"boolean","default":false},"source":{"type":"string","maxLength":512}},"required":["paths"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_clean", title: Some("Git Clean"), description: "Remove untracked files from the working tree with dry_run safety by default.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"dry_run":{"type":"boolean","default":true},"directories":{"type":"boolean","default":false},"paths":{"type":"array","items":{"type":"string","maxLength":4096},"maxItems":64}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_cherry_pick", title: Some("Git Cherry Pick"), description: "Apply changes from an existing commit onto the current branch.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"commit":{"type":"string","minLength":1,"maxLength":512},"no_commit":{"type":"boolean","default":false}},"required":["commit"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_revert", title: Some("Git Revert"), description: "Revert an existing commit by creating a new inverse commit.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"commit":{"type":"string","minLength":1,"maxLength":512},"no_commit":{"type":"boolean","default":false}},"required":["commit"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_reset", title: Some("Git Reset"), description: "Reset HEAD or unstage files using soft or mixed mode. Hard reset is blocked.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"target":{"type":"string","maxLength":512,"default":"HEAD"},"mode":{"type":"string","enum":["soft","mixed"],"default":"mixed"},"paths":{"type":"array","items":{"type":"string","maxLength":4096},"maxItems":64}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_remote_add", title: Some("Git Remote Add"), description: "Add a named remote to the Git repository with URL validation.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"name":{"type":"string","minLength":1,"maxLength":64},"url":{"type":"string","minLength":1,"maxLength":2048}},"required":["name","url"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:false, idempotent_hint:false, open_world_hint:true }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_remote_remove", title: Some("Git Remote Remove"), description: "Remove a named remote from the repository.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"name":{"type":"string","minLength":1,"maxLength":64}},"required":["name"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:false, open_world_hint:true }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_remote_set_url", title: Some("Git Remote Set URL"), description: "Update the URL of an existing Git remote.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"name":{"type":"string","minLength":1,"maxLength":64},"url":{"type":"string","minLength":1,"maxLength":2048}},"required":["name","url"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:false, open_world_hint:true }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "git_remote_branch_delete", title: Some("Git Remote Branch Delete"), description: "Delete one non-default validated GitHub remote branch only when its current head matches expected_sha; verify absence after mutation.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"remote":{"type":"string","minLength":1,"maxLength":64,"default":"origin"},"branch":{"type":"string","minLength":1,"maxLength":512},"expected_sha":{"type":"string","pattern":"^[0-9a-fA-F]{40}$"}},"required":["branch","expected_sha"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:false, open_world_hint:true }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "change_request_list", title: Some("Change Request List"), description: "List bounded change-request summaries for the validated forge repository using a provider-neutral result contract.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"remote":{"type":"string","minLength":1,"maxLength":64,"default":"origin"},"state":{"type":"string","enum":["open","closed","merged","all"],"default":"open"}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:true }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "change_request_get", title: Some("Change Request Get"), description: "Read one bounded change-request summary including base/head identity, mergeability, and review-decision classification.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"remote":{"type":"string","minLength":1,"maxLength":64,"default":"origin"},"number":{"type":"integer","minimum":1}},"required":["number"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:true }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "change_request_create", title: Some("Change Request Create"), description: "Create a change request for an already-pushed validated head branch without implicit push/fork behavior or arbitrary provider API access.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"remote":{"type":"string","minLength":1,"maxLength":64,"default":"origin"},"head_branch":{"type":"string","minLength":1,"maxLength":512},"base_branch":{"type":"string","minLength":1,"maxLength":512},"title":{"type":"string","minLength":1,"maxLength":256},"body":{"type":"string","maxLength":65536,"default":""},"draft":{"type":"boolean","default":false}},"required":["head_branch","base_branch","title","body"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:false, open_world_hint:true }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "change_request_update", title: Some("Change Request Update"), description: "Update only bounded title/body/base fields on one validated change request; reviewer/admin/provider passthrough flags are unavailable.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"remote":{"type":"string","minLength":1,"maxLength":64,"default":"origin"},"number":{"type":"integer","minimum":1},"title":{"type":"string","minLength":1,"maxLength":256},"body":{"type":"string","maxLength":65536},"base_branch":{"type":"string","minLength":1,"maxLength":512}},"required":["number"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:false, open_world_hint:true }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "change_request_checks", title: Some("Change Request Checks"), description: "Read bounded check classifications for one validated change request without returning raw provider logs.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"remote":{"type":"string","minLength":1,"maxLength":64,"default":"origin"},"number":{"type":"integer","minimum":1}},"required":["number"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:true }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "change_request_merge", title: Some("Change Request Merge"), description: "Merge one eligible change request using an explicit strategy only when the observed head SHA still matches and current checks/review state permit it; admin/auto/bypass flags are unavailable.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"remote":{"type":"string","minLength":1,"maxLength":64,"default":"origin"},"number":{"type":"integer","minimum":1},"expected_head_sha":{"type":"string","pattern":"^[0-9a-fA-F]{40}$"},"strategy":{"type":"string","enum":["merge","squash","rebase"],"default":"squash"}},"required":["number","expected_head_sha"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:false, open_world_hint:true }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "apply_patch", title: Some("Apply Patch"), description: "Apply a constrained unified text patch to existing contained regular files. All targets and hunks are preflighted; symlinks, protected paths, stale context, adds/deletes/renames, binary content, and root escapes fail closed. dry_run validates without mutation; commit uses atomic per-file replacement with best-effort rollback if a later file fails.", input_schema: json!({"type":"object","properties":{"patch":{"type":"string","minLength":1,"maxLength":524288},"cwd":{"type":"string","maxLength":4096},"dry_run":{"type":"boolean","default":false}},"required":["patch"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:false, destructive_hint:true, idempotent_hint:false, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "code_symbols", title: Some("Code Symbols"), description: "Bounded document symbols for one contained source file, or workspace symbol search when a query is given (only if the language server advertises workspace-symbol support). Language is inferred from the file extension / project markers; never both path and query ambiguity.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"path":{"type":"string","maxLength":4096},"query":{"type":"string","maxLength":256},"max_results":{"type":"integer","minimum":1,"maximum":128,"default":50},"continuation":{"type":"string","maxLength":64}},"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "code_definition", title: Some("Code Definition"), description: "Bounded, contained definition locations for a symbol at a UTF-8 file position, using real language-server semantics.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"path":{"type":"string","minLength":1,"maxLength":4096},"line":{"type":"integer","minimum":0},"column":{"type":"integer","minimum":0}},"required":["path","line","column"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "code_references", title: Some("Code References"), description: "Bounded, contained reference locations for a symbol at a UTF-8 file position.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"path":{"type":"string","minLength":1,"maxLength":4096},"line":{"type":"integer","minimum":0},"column":{"type":"integer","minimum":0},"include_declaration":{"type":"boolean","default":true},"max_results":{"type":"integer","minimum":1,"maximum":128,"default":50},"continuation":{"type":"string","maxLength":64}},"required":["path","line","column"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "code_implementations", title: Some("Code Implementations"), description: "Bounded, contained implementation locations for a symbol at a UTF-8 file position. Capability-gated: returns a distinct unsupported-capability error when the language server does not advertise implementation search.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"path":{"type":"string","minLength":1,"maxLength":4096},"line":{"type":"integer","minimum":0},"column":{"type":"integer","minimum":0},"max_results":{"type":"integer","minimum":1,"maximum":128,"default":50},"continuation":{"type":"string","maxLength":64}},"required":["path","line","column"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "code_hover", title: Some("Code Hover"), description: "Bounded plain/markdown hover (type/docs) text for a symbol at a UTF-8 file position. Server-provided markdown is treated as inert text, never rendered as executable HTML.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"path":{"type":"string","minLength":1,"maxLength":4096},"line":{"type":"integer","minimum":0},"column":{"type":"integer","minimum":0}},"required":["path","line","column"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "code_diagnostics", title: Some("Code Diagnostics"), description: "Bounded normalized diagnostics for one contained source file, including severity, stable diagnostic code when available, source, and document version (when the server reports one), so a stale result can be detected.", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"path":{"type":"string","minLength":1,"maxLength":4096},"severity":{"type":"integer","minimum":1,"maximum":4},"max_results":{"type":"integer","minimum":1,"maximum":128,"default":50},"continuation":{"type":"string","maxLength":64}},"required":["path"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool { name: "code_rename_preview", title: Some("Code Rename Preview"), description: "Preview-only bounded rename: normalizes the language server's WorkspaceEdit into per-file text replacements without applying anything. Apply the result yourself through apply_patch/file_edit after review. Rejects edits outside the contained workspace, protected paths, unsafe symlinks, and any unsupported resource operation (file create/rename/delete).", input_schema: json!({"type":"object","properties":{"cwd":{"type":"string","maxLength":4096},"path":{"type":"string","minLength":1,"maxLength":4096},"line":{"type":"integer","minimum":0},"column":{"type":"integer","minimum":0},"new_name":{"type":"string","minLength":1,"maxLength":4096}},"required":["path","line","column","new_name"],"additionalProperties":false}), annotations: Some(ToolAnnotations { read_only_hint:true, destructive_hint:false, idempotent_hint:true, open_world_hint:false }), security_schemes:coding_security_scheme(), execution:None },
        Tool {
            name: "terminal_job_start",
            title: Some("Start Terminal Job"),
            description: "Start a bounded sandboxed terminal job and return its task ID for polling.",
            input_schema: json!({ "type": "object", "properties": { "command": { "type": "string", "minLength": 1, "maxLength": 65536 }, "args": { "type": "array", "items": { "type": "string" }, "maxItems": 100 }, "cwd": { "type": "string" }, "timeout_ms": { "type": "integer", "minimum": 0 } }, "required": ["command"], "additionalProperties": false }),
            annotations: Some(ToolAnnotations {
                read_only_hint: false,
                destructive_hint: true,
                idempotent_hint: false,
                open_world_hint: true,
            }),
            security_schemes: coding_security_scheme(),
            execution: None,
        },
        Tool {
            name: "terminal_job_get",
            title: Some("Get Terminal Job"),
            description: "Get bounded state and retained output for a terminal job.",
            input_schema: json!({ "type": "object", "properties": { "taskId": { "type": "string", "minLength": 1 } }, "required": ["taskId"], "additionalProperties": false }),
            annotations: Some(ToolAnnotations {
                read_only_hint: true,
                destructive_hint: false,
                idempotent_hint: true,
                open_world_hint: false,
            }),
            security_schemes: coding_security_scheme(),
            execution: None,
        },
        Tool {
            name: "terminal_job_cancel",
            title: Some("Cancel Terminal Job"),
            description: "Cancel a running terminal job and its process group.",
            input_schema: json!({ "type": "object", "properties": { "taskId": { "type": "string", "minLength": 1 } }, "required": ["taskId"], "additionalProperties": false }),
            annotations: Some(ToolAnnotations {
                read_only_hint: false,
                destructive_hint: true,
                idempotent_hint: true,
                open_world_hint: true,
            }),
            security_schemes: coding_security_scheme(),
            execution: None,
        },
    ];
    tools.extend(forge::issue_tools());
    tools.extend(forge::action_tools());
    tools.extend(forge::security_tools());
    tools.extend(forge::action_mutation_tools());
    tools
}

pub const PRIMARY_TOOL_NAMES: &[&str] = &[
    "terminal_exec",
    "terminal_job_start",
    "terminal_job_get",
    "terminal_job_cancel",
    "directory_list",
    "file_search",
    "text_search",
    "file_read",
    "file_edit",
    "file_write",
    "apply_patch",
    "git_status",
    "git_diff",
    "git_log",
    "git_show",
    "git_blame",
    "git_branch_list",
    "git_stage",
    "git_unstage",
    "git_commit",
    "git_fetch",
    "git_push",
    "git_remote_list",
    "change_request_list",
    "change_request_get",
    "change_request_checks",
    "code_symbols",
    "code_definition",
    "code_references",
    "code_hover",
    "code_diagnostics",
];

pub fn tool_catalog_for_profile(profile: relay_core::config::ToolProfile) -> Vec<Tool> {
    match profile {
        relay_core::config::ToolProfile::Full => tool_catalog(),
        relay_core::config::ToolProfile::Primary => {
            let all = tool_catalog();
            PRIMARY_TOOL_NAMES
                .iter()
                .filter_map(|name| all.iter().find(|t| t.name == *name).cloned())
                .collect()
        }
    }
}

pub fn find_tool_for_profile(name: &str, profile: relay_core::config::ToolProfile) -> Option<Tool> {
    tool_catalog_for_profile(profile)
        .into_iter()
        .find(|t| t.name == name)
}

pub fn find_tool(name: &str) -> Option<Tool> {
    tool_catalog().into_iter().find(|t| t.name == name)
}

/// Validate `arguments` against a tool's declared `inputSchema` (JSON
/// Schema 2020-12) — the enforcement boundary required before execution:
/// `tools/call` validates argument *shape*, not just that a tool with this
/// name exists. Validation diagnostics are deliberately not returned: schema
/// errors can echo request values, property names, or other attacker-
/// controlled text through `Display`.
pub fn validate_tool_arguments(tool: &Tool, arguments: &Value) -> Result<(), McpError> {
    let validator = jsonschema::validator_for(&tool.input_schema)
        .map_err(|_| McpError::Internal("invalid tool schema".to_string()))?;

    if validator.iter_errors(arguments).next().is_some() {
        return Err(McpError::InvalidParams(
            "tool arguments do not match the required schema".to_string(),
        ));
    }

    Ok(())
}
