use relay_application::git::dispatch_git_tool;
use relay_core::config::ServerConfig;
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};
fn run(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .unwrap()
        .success())
}
fn response(base: &Path, payload: &str) {
    fs::write(base.join("response.json"), payload).unwrap()
}
fn argv(base: &Path) -> Vec<String> {
    fs::read_to_string(base.join("argv.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}
#[tokio::main]
async fn main() {
    let base = std::env::temp_dir().join(format!("relay-044b-actions-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let repo = base.join("repo");
    fs::create_dir_all(&repo).unwrap();
    run(&repo, &["init", "-q", "-b", "main"]);
    run(
        &repo,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/farismnrr/ai-code.git",
        ],
    );
    let fake = base.join("fake-gh");
    fs::write(
        &fake,
        format!(
            "#!/usr/bin/env bash\nprintf '%s\\n' \"$@\" > '{}/argv.log'\ncat '{}/response.json'\n",
            base.display(),
            base.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    std::env::set_var("RELAY_TEST_GH_PATH", &fake);
    let config = ServerConfig {
        execution_root: Some(base.to_string_lossy().into()),
        dir: Some(repo.to_string_lossy().into()),
        ..ServerConfig::default()
    };
    response(
        &base,
        r#"[{"id":1,"name":"CI","path":".github/workflows/ci.yml","state":"active"}]"#,
    );
    let out = dispatch_git_tool("workflow_list", &json!({"cwd":repo}), &config)
        .await
        .unwrap()
        .unwrap();
    assert!(out.content[0].text.contains("CI"));
    response(
        &base,
        r#"{"id":1,"name":"CI","path":".github/workflows/ci.yml","state":"active","html_url":"https://github.com/farismnrr/ai-code/actions/workflows/ci.yml"}"#,
    );
    let out = dispatch_git_tool(
        "workflow_get",
        &json!({"cwd":repo,"workflow_id":1}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    assert!(out.content[0].text.contains("CI"));
    assert_eq!(argv(&base)[0], "api");
    assert!(argv(&base)[1].ends_with("/actions/workflows/1"));
    let run_json = r#"{"databaseId":42,"name":"CI","workflowName":"CI","workflowDatabaseId":1,"displayTitle":"test","event":"push","headBranch":"main","headSha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","status":"completed","conclusion":"failure","createdAt":"x","startedAt":"x","updatedAt":"x","url":"https://github.com/farismnrr/ai-code/actions/runs/42","attempt":1,"number":7}"#;
    response(&base, &format!("[{run_json}]"));
    let out=dispatch_git_tool("workflow_run_list",&json!({"cwd":repo,"workflow_id":1,"branch":"main","commit_sha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","status":"failure"}),&config).await.unwrap().unwrap();
    assert!(out.content[0].text.contains("42"));
    let a = argv(&base);
    assert!(a.contains(&"--workflow".into()) && a.contains(&"--commit".into()));
    response(&base, run_json);
    let out = dispatch_git_tool(
        "workflow_run_get",
        &json!({"cwd":repo,"run_id":42}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    assert!(out.content[0].text.contains("42"));
    assert!(!out.content[0].text.contains("steps"));
    let jobs_json = run_json.trim_end_matches('}').to_owned()
        + r#", "jobs":[{"databaseId":99,"name":"test","status":"completed","conclusion":"failure","startedAt":"x","completedAt":"x","url":"https://github.com/farismnrr/ai-code/actions/runs/42/job/99","steps":[{"number":1,"name":"compile","status":"completed","conclusion":"failure","startedAt":"x","completedAt":"x"}]}]}"#;
    response(&base, &jobs_json);
    let out = dispatch_git_tool(
        "workflow_run_jobs",
        &json!({"cwd":repo,"run_id":42}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    assert!(out.content[0].text.contains("compile"));
    response(&base,"failure\tstep\tAuthorization: Bearer ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ123456\nnormal compiler failure\n");
    let out = dispatch_git_tool(
        "workflow_job_log_preview",
        &json!({"cwd":repo,"job_id":99,"max_lines":10}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let text = &out.content[0].text;
    assert!(!text.contains("ghp_"));
    assert!(text.contains("[REDACTED]"));
    let a = argv(&base);
    assert!(a.contains(&"--job".into()) && a.contains(&"--log-failed".into()));
    assert!(dispatch_git_tool(
        "workflow_run_list",
        &json!({"cwd":repo,"commit_sha":"bad"}),
        &config
    )
    .await
    .is_err());
    assert!(dispatch_git_tool(
        "workflow_get",
        &json!({"cwd":repo,"workflow_id":0}),
        &config
    )
    .await
    .is_err());
    let _ = fs::remove_dir_all(&base);
    println!("044B actions acceptance: PASS");
}
