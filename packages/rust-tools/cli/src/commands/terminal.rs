use std::env;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(clap::Args, Debug)]
pub struct Args {
    /// Working directory
    #[arg(long = "cwd")]
    cwd: Option<String>,

    /// Bypass exec guard protection
    #[arg(long = "no-guard")]
    no_guard: bool,

    /// Allowed command for guarded execution (can be specified multiple times)
    #[arg(long = "allow-command")]
    allow_command: Vec<String>,

    /// Timeout in milliseconds; zero means no deadline.
    #[arg(long = "timeout")]
    timeout: Option<u64>,

    /// Command and arguments
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    positionals: Vec<String>,
}

struct TerminalOutcome {
    message: Option<String>,
    exit_code: i32,
}

impl TerminalOutcome {
    fn error(message: impl Into<String>) -> Self {
        Self {
            message: Some(message.into()),
            exit_code: 1,
        }
    }
}

async fn run_terminal(
    command_str: &str,
    passed_args: &[String],
    cwd: &str,
    no_guard: bool,
    allow_command: &[String],
    timeout_ms: Option<u64>,
) -> TerminalOutcome {
    let parsed_parts = match shell_words::split(command_str) {
        Ok(parts) => parts,
        Err(_) => return TerminalOutcome::error("Error: failed to parse command string"),
    };

    if parsed_parts.is_empty() {
        return TerminalOutcome::error("Error: Empty command");
    }

    let binary = &parsed_parts[0];

    if !no_guard {
        if allow_command.is_empty() {
            eprintln!(
                "WARN: Exec guard is enabled but no external validation is provided in CLI. Pass --allow-command <cmd> or --no-guard if you want to bypass exec protection."
            );
            return TerminalOutcome::error(
                "Error: Exec guard blocked request. Use --allow-command to whitelist.",
            );
        }
        if !allow_command.iter().any(|c| c == binary) {
            eprintln!(
                "WARN: Exec guard blocked command '{}'. It is not in the --allow-command list.",
                binary
            );
            return TerminalOutcome::error(format!(
                "Error: Exec guard blocked request. Command '{}' is not approved.",
                binary
            ));
        }
    }

    let glued_args = &parsed_parts[1..];
    let mut final_args: Vec<String> = glued_args.to_vec();
    final_args.extend_from_slice(passed_args);

    let mut cmd = Command::new(binary);
    cmd.args(&final_args);
    cmd.current_dir(cwd);
    cmd.env_clear();

    if let Ok(path) = env::var("PATH") {
        cmd.env("PATH", path);
    }
    if let Ok(home) = env::var("HOME") {
        cmd.env("HOME", home);
    }
    if let Ok(lang) = env::var("LANG") {
        cmd.env("LANG", lang);
    }

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    #[cfg(unix)]
    {
        cmd.process_group(0);
    }
    #[cfg(not(unix))]
    {
        cmd.kill_on_drop(true);
    }

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(_) => return TerminalOutcome::error("Error: failed to start command"),
    };
    let Some(mut stdout) = child.stdout.take() else {
        return TerminalOutcome::error("Error: stdout unavailable");
    };
    let Some(mut stderr) = child.stderr.take() else {
        return TerminalOutcome::error("Error: stderr unavailable");
    };
    let stdout_task =
        tokio::spawn(async move { tokio::io::copy(&mut stdout, &mut tokio::io::stdout()).await });
    let stderr_task =
        tokio::spawn(async move { tokio::io::copy(&mut stderr, &mut tokio::io::stderr()).await });

    let (status, timed_out) = if let Some(ms) = timeout_ms.filter(|ms| *ms > 0) {
        match timeout(Duration::from_millis(ms), child.wait()).await {
            Ok(result) => (result, false),
            Err(_) => {
                if let Some(pid) = child.id() {
                    #[cfg(unix)]
                    unsafe {
                        libc::kill(-(pid as i32), libc::SIGKILL);
                    }
                }
                (child.wait().await, true)
            }
        }
    } else {
        (child.wait().await, false)
    };

    let _ = stdout_task.await;
    let _ = stderr_task.await;

    if timed_out {
        return TerminalOutcome {
            message: Some("Error: command timed out".to_string()),
            exit_code: 124,
        };
    }

    let status = match status {
        Ok(status) => status,
        Err(_) => return TerminalOutcome::error("Error: failed to collect command output"),
    };
    let exit_code = status.code().unwrap_or(1);
    TerminalOutcome {
        message: Some(format!("Exit: {exit_code}")),
        exit_code,
    }
}

pub async fn run(args: Args) {
    if args.positionals.is_empty() {
        eprintln!(
            "Usage: ai-tools terminal <command> [args...] [--cwd <path>] [--no-guard] [--timeout <ms>]"
        );
        std::process::exit(1);
    }

    let command = &args.positionals[0];
    let cmd_args = &args.positionals[1..];
    let cwd = args
        .cwd
        .unwrap_or_else(|| env::current_dir().unwrap().display().to_string());

    let outcome = run_terminal(
        command,
        cmd_args,
        &cwd,
        args.no_guard,
        &args.allow_command,
        args.timeout,
    )
    .await;

    if let Some(message) = &outcome.message {
        if message.starts_with("Error:") {
            eprintln!("{message}");
        } else {
            println!("{message}");
        }
    }

    if outcome.exit_code != 0 {
        std::process::exit(outcome.exit_code.clamp(1, 255));
    }
}
