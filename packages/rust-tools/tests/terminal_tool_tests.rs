use std::env;
use std::fs;
use std::process::Command;

fn get_bin(name: &str) -> String {
    let mut path = env::current_exe().unwrap();
    path.pop(); // remove test exe name
    path.pop(); // remove deps dir
    path.push(name);
    path.to_str().unwrap().to_string()
}

#[test]
#[cfg(unix)]
fn test_terminal_timeout_descendants_killed() {
    let temp_dir = env::temp_dir();
    let pid_file = temp_dir.join(format!("test_pids_{}.txt", std::process::id()));
    // Clean up any old file
    let _ = fs::remove_file(&pid_file);

    let script = format!(
        r#"#!/usr/bin/env bash
sleep 9999 &
echo $! >> {pid_file}
sleep 9999 &
echo $! >> {pid_file}
wait
"#,
        pid_file = pid_file.display()
    );
    let script_path = temp_dir.join(format!("test_descendants_{}.sh", std::process::id()));
    fs::write(&script_path, &script).unwrap();
    Command::new("chmod")
        .arg("+x")
        .arg(&script_path)
        .status()
        .unwrap();

    let output = Command::new(get_bin("terminal-tool"))
        .arg("--no-guard")
        .arg("--timeout")
        .arg("500")
        .arg(script_path.to_str().unwrap())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("timed out"));

    // read PIDs from file
    let pids_str = fs::read_to_string(&pid_file).unwrap_or_default();
    let pids: Vec<&str> = pids_str
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    assert!(!pids.is_empty(), "Script did not spawn any children");

    // verify each PID is dead
    for pid in pids {
        // kill -0 returns success if process exists
        let kill_status = Command::new("kill").arg("-0").arg(pid).status();

        if let Ok(status) = kill_status {
            assert!(
                !status.success(),
                "Found stray descendant process with PID {}",
                pid
            );
        }
    }

    let _ = fs::remove_file(&pid_file);
    let _ = fs::remove_file(&script_path);
}

#[test]
fn test_invalid_timeout() {
    let output = Command::new(get_bin("terminal-tool"))
        .arg("--no-guard")
        .arg("--timeout")
        .arg("-5")
        .arg("echo")
        .arg("hello")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error:"));

    let output = Command::new(get_bin("terminal-tool"))
        .arg("--no-guard")
        .arg("--timeout")
        .arg("notanumber")
        .arg("echo")
        .arg("hello")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error:"));
}

#[test]
fn test_argument_boundaries() {
    let args_to_test = vec!["hello world", "", "-n", "& * |", "\"quoted\"", "'single'"];

    for arg in args_to_test {
        let output = Command::new(get_bin("terminal-tool"))
            .arg("--no-guard")
            .arg("printf")
            .arg("[%s]")
            .arg(arg)
            .output()
            .unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout);
        let expected = format!("[{arg}]");
        assert!(stdout.contains(&expected), "Failed on arg: {arg}");
    }
}

#[test]
fn test_process_cwd() {
    let temp_dir = env::temp_dir();
    let temp_dir_str = temp_dir.to_str().unwrap();

    let output = Command::new(get_bin("terminal-tool"))
        .arg("--no-guard")
        .arg("--cwd")
        .arg(temp_dir_str)
        .arg("pwd")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("Exit: 0"));
}

#[test]
fn test_environment_filtering() {
    let output = Command::new(get_bin("terminal-tool"))
        .env("DUMMY_VAR", "12345")
        .arg("--no-guard")
        .arg("env")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("DUMMY_VAR=12345"));
}

#[test]
fn test_terminal_exit_code_contract_invalid_usage() {
    let output = std::process::Command::new(get_bin("terminal-tool"))
        .arg("--timeout")
        .arg("notanumber")
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit code 2 (Invalid CLI Usage)"
    );
}
