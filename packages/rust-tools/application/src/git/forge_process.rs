use relay_core::error::McpError;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::time::timeout;

const MAX_FORGE_OUTPUT_BYTES: usize = 256 * 1024;
const FORGE_OPERATION_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) async fn run_gh(
    root: &Path,
    args: &[String],
    accepted_exit_codes: &[i32],
) -> Result<Vec<u8>, McpError> {
    let home = runtime_home()?;
    let mut command = Command::new("gh");
    command
        .current_dir(root)
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/usr/sbin:/bin")
        .env("HOME", &home)
        .env("GH_CONFIG_DIR", home.join(".config/gh"))
        .env("GH_PAGER", "cat")
        .env("PAGER", "cat")
        .env("NO_COLOR", "1")
        .env("LC_ALL", "C")
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
        .map_err(|_| McpError::Internal("failed to start forge operation".into()))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| McpError::Internal("failed to capture forge output".into()))?;
    let mut output = Vec::with_capacity(8192);
    let result = timeout(FORGE_OPERATION_TIMEOUT, async {
        let mut limited = (&mut stdout).take((MAX_FORGE_OUTPUT_BYTES + 1) as u64);
        limited
            .read_to_end(&mut output)
            .await
            .map_err(|_| McpError::InvalidRequest("forge output could not be read".into()))?;
        if output.len() > MAX_FORGE_OUTPUT_BYTES {
            crate::execution::kill_process_group(&mut child).await;
            let _ = child.wait().await;
            return Err(McpError::InvalidRequest(
                "forge output exceeds maximum".into(),
            ));
        }
        let status = child
            .wait()
            .await
            .map_err(|_| McpError::Internal("failed to wait for forge operation".into()))?;
        let code = status.code().unwrap_or(-1);
        if !status.success() && !accepted_exit_codes.contains(&code) {
            return Err(McpError::InvalidRequest("forge operation failed".into()));
        }
        Ok::<(), McpError>(())
    })
    .await;
    match result {
        Ok(inner) => inner?,
        Err(_) => {
            crate::execution::kill_process_group(&mut child).await;
            let _ = child.wait().await;
            return Err(McpError::InvalidRequest("forge operation timed out".into()));
        }
    }
    Ok(output)
}

fn runtime_home() -> Result<PathBuf, McpError> {
    let value = std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| McpError::InvalidRequest("forge credential home is unavailable".into()))?;
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(McpError::InvalidRequest(
            "forge credential home is invalid".into(),
        ));
    }
    std::fs::canonicalize(path)
        .map_err(|_| McpError::InvalidRequest("forge credential home is unavailable".into()))
}

fn forward_secret_env(command: &mut Command, key: &str) {
    if let Some(value) = std::env::var_os(key).filter(|value| !value.is_empty()) {
        command.env(key, value);
    }
}
