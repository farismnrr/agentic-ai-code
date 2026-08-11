use clap::Parser;
use std::env;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Parser, Debug)]
#[command(name = "terminal-tool")]
#[command(
    version,
    about = "Run an executable command within the workspace directory",
    long_about = None
)]
struct Args {
    /// Working directory
    #[arg(long = "cwd")]
    cwd: Option<String>,

    /// Bypass exec guard protection
    #[arg(long = "no-guard")]
    no_guard: bool,

    /// Timeout in milliseconds (default: 30000)
    #[arg(long = "timeout")]
    timeout: Option<u64>,

    /// Command and arguments
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    positionals: Vec<String>,
}

fn truncate(s: &str, limit: usize) -> String {
    if s.len() > limit {
        format!("{}... (truncated)", &s[..limit])
    } else {
        s.to_string()
    }
}

async fn run_terminal(
    command_str: &str,
    passed_args: &[String],
    cwd: &str,
    no_guard: bool,
    timeout_ms: u64,
) -> String {
    if !no_guard {
        eprintln!(
            "WARN: Exec guard is enabled but no external validation is provided in CLI. Pass --no-guard if you want to bypass exec protection."
        );
        return "Error: Exec guard blocked request. Use --no-guard to bypass.".to_string();
    }

    let parsed_parts = match shell_words::split(command_str) {
        Ok(parts) => parts,
        Err(e) => return format!("Error: failed to parse command string: {e}"),
    };

    if parsed_parts.is_empty() {
        return "Error: Empty command".to_string();
    }

    let binary = &parsed_parts[0];
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

    let timeout_duration = Duration::from_millis(timeout_ms);

    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return format!("Error: {e}"),
    };

    let pid = child.id().unwrap_or(0);

    // We use timeout to limit execution time. wait_with_output consumes the child.
    // If a timeout occurs, the Future is dropped, which drops the Child handle.
    // Under Unix, we placed the child in its own process group (process_group(0)).
    // When timeout occurs, we explicitly send SIGKILL to the entire process group
    // and reap the child to ensure no zombies or orphaned descendants are left behind.
    let output_result = timeout(timeout_duration, child.wait_with_output()).await;

    match output_result {
        Err(_) => {
            #[cfg(unix)]
            {
                if pid > 0 {
                    let _ = tokio::task::spawn_blocking(move || unsafe {
                        libc::kill(-(pid as i32), libc::SIGKILL);
                        let mut status = 0;
                        // wait/reap the process group leader explicitly
                        libc::waitpid(pid as i32, &mut status, 0);
                    })
                    .await;
                }
            }
            format!("Error: command timed out after {timeout_ms}ms and was killed.")
        }
        Ok(Err(e)) => format!("Error: {e}"),
        Ok(Ok(out)) => {
            let stdout_str = String::from_utf8_lossy(&out.stdout);
            let stderr_str = String::from_utf8_lossy(&out.stderr);
            let code = out.status.code().unwrap_or(-1);

            format!(
                "Exit: {}\nStdout: {}\nStderr: {}",
                code,
                truncate(&stdout_str, 20000),
                truncate(&stderr_str, 20000)
            )
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if args.positionals.is_empty() {
        eprintln!(
            "Usage: terminal-tool <command> [args...] [--cwd <path>] [--no-guard] [--timeout <ms>]"
        );
        std::process::exit(1);
    }

    let command = &args.positionals[0];
    let cmd_args = &args.positionals[1..];

    let cwd = args
        .cwd
        .unwrap_or_else(|| env::current_dir().unwrap().display().to_string());

    let timeout_ms = args.timeout.unwrap_or(30000);

    let output = run_terminal(command, cmd_args, &cwd, args.no_guard, timeout_ms).await;

    let is_error = output.starts_with("Error:");
    println!("{output}");

    if is_error {
        std::process::exit(1);
    }
}
