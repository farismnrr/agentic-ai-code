pub use super::agent_capabilities::{detect_agent_capabilities, AgentCapabilities};
pub use super::agent_policy::{
    classify_failure, fallback_allowed, provider_argv, AgentProvider, FailureClass,
};
use super::agent_snapshot::workspace_snapshot;
use super::paths::resolve_authorized_cwd;
use super::process::{run_process, OutputBuffer, ProcessResult};
use super::sandbox;
use super::{InvocationProgram, ToolInvocation};
use relay_core::config::ServerConfig;
use relay_core::error::McpError;
use relay_interfaces::mcp::{ToolCallResult, ToolResultContent};
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{watch, Mutex};

const MAX_PROMPT_BYTES: usize = 65_536;
const MAX_PROVIDERS: usize = 3;
const MAX_TURNS: u64 = 50;
const MAX_TIMEOUT_MS: u64 = 600_000;
#[derive(Debug, Clone)]
pub(super) struct AgentRequest {
    pub prompt: String,
    pub cwd: PathBuf,
    pub providers: Vec<AgentProvider>,
    pub timeout_ms: u64,
    pub max_turns: u64,
    pub fallback: bool,
}

#[derive(Debug, Serialize)]
struct AttemptSummary {
    provider: &'static str,
    outcome: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
}

#[derive(Debug, Serialize)]
struct AgentResult {
    provider: Option<&'static str>,
    fallback_used: bool,
    attempts: Vec<AttemptSummary>,
    workspace_changed: bool,
    message: String,
}

pub(super) fn build_request(
    arguments: &Value,
    config: &ServerConfig,
) -> Result<AgentRequest, McpError> {
    let prompt = arguments
        .get("prompt")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= MAX_PROMPT_BYTES)
        .ok_or_else(|| McpError::InvalidParams("prompt is required and bounded".into()))?;
    let cwd = resolve_authorized_cwd(arguments, config)?;
    let timeout_ms = arguments
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(config.default_terminal_timeout_ms);
    if timeout_ms > MAX_TIMEOUT_MS
        || (config.max_terminal_timeout_ms > 0 && timeout_ms > config.max_terminal_timeout_ms)
    {
        return Err(McpError::InvalidRequest(
            "agent timeout exceeds the configured maximum".into(),
        ));
    }
    let max_turns = arguments
        .get("max_turns")
        .and_then(Value::as_u64)
        .unwrap_or(20);
    if !(1..=MAX_TURNS).contains(&max_turns) {
        return Err(McpError::InvalidParams(
            "max_turns must be between 1 and 50".into(),
        ));
    }
    let capabilities = detect_agent_capabilities(config);
    let providers = match arguments.get("providers").and_then(Value::as_array) {
        Some(values) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .and_then(AgentProvider::parse)
                    .ok_or_else(|| McpError::InvalidParams("provider is not supported".into()))
            })
            .collect::<Result<Vec<_>, _>>()?,
        None => capabilities.providers().to_vec(),
    };
    if providers.is_empty() || providers.len() > MAX_PROVIDERS {
        return Err(McpError::InvalidParams(
            "no authenticated coding CLI is available; log in locally or configure an explicit provider credential mapping".into(),
        ));
    }
    if providers
        .iter()
        .any(|provider| !capabilities.contains(*provider))
    {
        return Err(McpError::InvalidParams(
            "provider is not authenticated or available in the local CLI session".into(),
        ));
    }
    let mut seen = Vec::new();
    if providers.iter().any(|provider| {
        if seen.contains(provider) {
            true
        } else {
            seen.push(*provider);
            false
        }
    }) {
        return Err(McpError::InvalidParams(
            "providers must not contain duplicates".into(),
        ));
    }
    Ok(AgentRequest {
        prompt: prompt.to_owned(),
        cwd,
        providers,
        timeout_ms,
        max_turns,
        fallback: arguments
            .get("fallback")
            .and_then(Value::as_bool)
            .unwrap_or(true),
    })
}

async fn run_provider(
    config: &ServerConfig,
    request: &AgentRequest,
    provider: AgentProvider,
    cancel: &mut watch::Receiver<bool>,
) -> Result<ProcessResult, std::io::Error> {
    let program = sandbox::resolve_safe_executable(config, provider.binary())
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let invocation = ToolInvocation {
        program: InvocationProgram::Direct(program),
        args: provider_argv(provider, &request.prompt, request.max_turns),
        cwd: Some(request.cwd.clone()),
        timeout_ms: request.timeout_ms,
        allow_network: config.allow_agent_network,
        environment: config.agent_environment_for(provider.name()),
        auth_roots: config.agent_auth_roots_for(provider.name()),
        expose_optional_sockets: false,
        expose_authorized_siblings: false,
    };
    let stdout = Arc::new(Mutex::new(OutputBuffer::new(super::now_ms())));
    let stderr = Arc::new(Mutex::new(OutputBuffer::new(super::now_ms())));
    run_process(config, &invocation, cancel, stdout, stderr).await
}

async fn append_attempt_output(
    destination: &Arc<Mutex<OutputBuffer>>,
    provider: &str,
    value: &str,
    limit: usize,
) {
    let mut output = destination.lock().await;
    let label = format!("[{provider}]\n");
    output.push(label.as_bytes(), limit);
    output.push(value.as_bytes(), limit);
    output.push(b"\n", limit);
}

pub(super) async fn run_agent_job(
    config: &ServerConfig,
    request: AgentRequest,
    cancel: &mut watch::Receiver<bool>,
    stdout: Arc<Mutex<OutputBuffer>>,
    stderr: Arc<Mutex<OutputBuffer>>,
) -> Result<ProcessResult, std::io::Error> {
    let mut attempts = Vec::new();
    let mut last_class = FailureClass::Failed;
    let mut fallback_used = false;

    for (index, provider) in request.providers.iter().copied().enumerate() {
        if *cancel.borrow() {
            let result = AgentResult {
                provider: None,
                fallback_used,
                attempts,
                workspace_changed: false,
                message: "delegation cancelled; no fallback was attempted".into(),
            };
            return Ok(ProcessResult::cancelled(result, stdout, stderr).await);
        }
        let has_next = index + 1 < request.providers.len();
        let before =
            (request.fallback && has_next).then(|| workspace_snapshot(&request.cwd, config));
        let provider_result = run_provider(config, &request, provider, cancel).await;
        let provider_spawn_failed = provider_result.is_err();
        let (state, exit_code, provider_stdout, provider_stderr) = match provider_result {
            Ok(process) => (
                process.state,
                process.exit_code,
                process.stdout,
                process.stderr,
            ),
            Err(error) => (
                super::JobState::Failed,
                -1,
                String::new(),
                error.to_string(),
            ),
        };
        append_attempt_output(
            &stdout,
            provider.name(),
            &provider_stdout,
            config.max_retained_output_bytes / 2,
        )
        .await;
        append_attempt_output(
            &stderr,
            provider.name(),
            &provider_stderr,
            config.max_retained_output_bytes / 2,
        )
        .await;

        if state == super::JobState::Completed && exit_code == 0 {
            attempts.push(AttemptSummary {
                provider: provider.name(),
                outcome: "completed",
                exit_code: Some(exit_code),
            });
            let result = AgentResult {
                provider: Some(provider.name()),
                fallback_used,
                attempts,
                workspace_changed: false,
                message: "delegation completed".into(),
            };
            return Ok(ProcessResult::completed(result, stdout, stderr, exit_code).await);
        }
        if state == super::JobState::Cancelled {
            let result = AgentResult {
                provider: None,
                fallback_used,
                attempts,
                workspace_changed: false,
                message: "delegation cancelled; no fallback was attempted".into(),
            };
            return Ok(ProcessResult::cancelled(result, stdout, stderr).await);
        }
        if state == super::JobState::TimedOut {
            attempts.push(AttemptSummary {
                provider: provider.name(),
                outcome: "timed_out",
                exit_code: Some(exit_code),
            });
            let result = AgentResult {
                provider: None,
                fallback_used,
                attempts,
                workspace_changed: false,
                message: "delegation timed out; no fallback was attempted".into(),
            };
            return Ok(ProcessResult::timed_out(result, stdout, stderr).await);
        }
        if state != super::JobState::Completed && !provider_spawn_failed {
            attempts.push(AttemptSummary {
                provider: provider.name(),
                outcome: "failed",
                exit_code: Some(exit_code),
            });
            last_class = FailureClass::Failed;
            break;
        }
        let class = classify_failure(exit_code, &provider_stdout, &provider_stderr)
            .unwrap_or(FailureClass::Failed);
        last_class = class;
        attempts.push(AttemptSummary {
            provider: provider.name(),
            outcome: match class {
                FailureClass::Quota => "quota",
                FailureClass::Auth => "auth",
                FailureClass::Unavailable => "unavailable",
                FailureClass::Failed => "failed",
            },
            exit_code: Some(exit_code),
        });
        if !request.fallback || !has_next || !fallback_allowed(class, false) {
            break;
        }
        let after = workspace_snapshot(&request.cwd, config);
        let workspace_changed = if provider_spawn_failed {
            false
        } else {
            match before {
                Some(before) => {
                    !before.safe_to_compare
                        || !after.safe_to_compare
                        || before.fingerprint != after.fingerprint
                }
                None => true,
            }
        };
        if workspace_changed {
            let result = AgentResult {
                provider: None,
                fallback_used,
                attempts,
                workspace_changed: true,
                message: "automatic fallback stopped because the failed provider may have changed the workspace".into(),
            };
            return Ok(ProcessResult::failed(result, stdout, stderr).await);
        }
        fallback_used = true;
    }

    let result = AgentResult {
        provider: None,
        fallback_used,
        attempts,
        workspace_changed: false,
        message: format!("all selected providers failed ({last_class:?})"),
    };
    Ok(ProcessResult::failed(result, stdout, stderr).await)
}

fn result_text(result: AgentResult) -> ToolCallResult {
    let text = serde_json::to_string(&result)
        .unwrap_or_else(|_| "{\"message\":\"delegation result could not be serialized\"}".into());
    ToolCallResult::error(vec![ToolResultContent { kind: "text", text }])
}

impl ProcessResult {
    async fn completed(
        result: AgentResult,
        stdout: Arc<Mutex<OutputBuffer>>,
        stderr: Arc<Mutex<OutputBuffer>>,
        exit_code: i32,
    ) -> Self {
        let (stdout, stderr, omitted) = output_values(&stdout, &stderr).await;
        let mut result = result_text(result);
        result.is_error = false;
        Self {
            state: super::JobState::Completed,
            exit_code,
            stdout,
            stderr,
            omitted,
            result: Some(result),
        }
    }

    async fn failed(
        result: AgentResult,
        stdout: Arc<Mutex<OutputBuffer>>,
        stderr: Arc<Mutex<OutputBuffer>>,
    ) -> Self {
        let (stdout, stderr, omitted) = output_values(&stdout, &stderr).await;
        Self {
            state: super::JobState::Completed,
            exit_code: 1,
            stdout,
            stderr,
            omitted,
            result: Some(result_text(result)),
        }
    }

    async fn timed_out(
        result: AgentResult,
        stdout: Arc<Mutex<OutputBuffer>>,
        stderr: Arc<Mutex<OutputBuffer>>,
    ) -> Self {
        let (stdout, stderr, omitted) = output_values(&stdout, &stderr).await;
        Self {
            state: super::JobState::TimedOut,
            exit_code: -1,
            stdout,
            stderr,
            omitted,
            result: Some(result_text(result)),
        }
    }

    async fn cancelled(
        result: AgentResult,
        stdout: Arc<Mutex<OutputBuffer>>,
        stderr: Arc<Mutex<OutputBuffer>>,
    ) -> Self {
        let (stdout, stderr, omitted) = output_values(&stdout, &stderr).await;
        Self {
            state: super::JobState::Cancelled,
            exit_code: -1,
            stdout,
            stderr,
            omitted,
            result: Some(result_text(result)),
        }
    }
}

async fn output_values(
    stdout: &Arc<Mutex<OutputBuffer>>,
    stderr: &Arc<Mutex<OutputBuffer>>,
) -> (String, String, u64) {
    let out = stdout.lock().await;
    let err = stderr.lock().await;
    (
        String::from_utf8_lossy(&out.bytes).into_owned(),
        String::from_utf8_lossy(&err.bytes).into_owned(),
        out.omitted + err.omitted,
    )
}
