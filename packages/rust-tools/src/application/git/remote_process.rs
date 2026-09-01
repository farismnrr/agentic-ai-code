use crate::core::error::McpError;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::time::timeout;

const MAX_REMOTE_OUTPUT_BYTES: usize = 64 * 1024;
const REMOTE_OPERATION_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) async fn run_remote_git(root: &Path, args: &[&str]) -> Result<Vec<u8>, McpError> {
    let home = runtime_home()?;
    let mut command = Command::new("git");
    command
        .current_dir(root)
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/usr/sbin:/bin")
        .env("HOME", &home)
        .env("GH_CONFIG_DIR", home.join(".config/gh"))
        .env("LC_ALL", "C")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "")
        .env("GIT_PAGER", "cat")
        .env("PAGER", "cat")
        .arg("--no-pager");
    for kv in [
        "core.pager=cat",
        "core.fsmonitor=false",
        "core.hooksPath=/dev/null",
        "credential.helper=",
        "credential.helper=!gh auth git-credential",
        "diff.external=",
        "diff.trustExitCode=false",
        "color.ui=false",
    ] {
        command.arg("-c").arg(kv);
    }
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    forward_secret_env(&mut command, "GH_TOKEN");
    forward_secret_env(&mut command, "GITHUB_TOKEN");
    let mut child = command
        .spawn()
        .map_err(|_| McpError::Internal("failed to start remote git operation".into()))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| McpError::Internal("failed to capture remote git output".into()))?;
    let mut output = Vec::with_capacity(8192);
    let result =
        timeout(REMOTE_OPERATION_TIMEOUT, async {
            let mut limited = (&mut stdout).take((MAX_REMOTE_OUTPUT_BYTES + 1) as u64);
            limited.read_to_end(&mut output).await.map_err(|_| {
                McpError::InvalidRequest("remote git output could not be read".into())
            })?;
            if output.len() > MAX_REMOTE_OUTPUT_BYTES {
                crate::application::execution::kill_process_group(&mut child).await;
                let _ = child.wait().await;
                return Err(McpError::InvalidRequest(
                    "remote git output exceeds maximum".into(),
                ));
            }
            let status = child.wait().await.map_err(|_| {
                McpError::Internal("failed to wait for remote git operation".into())
            })?;
            if !status.success() {
                return Err(McpError::InvalidRequest(
                    "remote git operation failed".into(),
                ));
            }
            Ok::<(), McpError>(())
        })
        .await;
    match result {
        Ok(inner) => inner?,
        Err(_) => {
            crate::application::execution::kill_process_group(&mut child).await;
            let _ = child.wait().await;
            return Err(McpError::InvalidRequest(
                "remote git operation timed out".into(),
            ));
        }
    }
    Ok(output)
}

fn runtime_home() -> Result<PathBuf, McpError> {
    let value = std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| McpError::InvalidRequest("remote credential home is unavailable".into()))?;
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(McpError::InvalidRequest(
            "remote credential home is invalid".into(),
        ));
    }
    std::fs::canonicalize(path)
        .map_err(|_| McpError::InvalidRequest("remote credential home is unavailable".into()))
}

fn forward_secret_env(command: &mut Command, key: &str) {
    if let Some(value) = std::env::var_os(key).filter(|value| !value.is_empty()) {
        command.env(key, value);
    }
}
