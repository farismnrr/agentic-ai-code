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
fn test_terminal_timeout_descendants_killed() {
    let temp_dir = env::temp_dir();
    let unique_sleep = "sleep 9999";
    let script = format!(
        r#"#!/usr/bin/env bash
{unique_sleep} &
{unique_sleep} &
wait
"#
    );
    let script_path = temp_dir.join("test_descendants_unique.sh");
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

    // verify pgrep finds nothing
    let pgrep = Command::new("pgrep")
        .arg("-f")
        .arg(unique_sleep)
        .output()
        .unwrap();
    assert!(!pgrep.status.success(), "Found stray descendant processes!");
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
