use ai_tools::application::git::dispatch_git_tool;
use ai_tools::core::config::ServerConfig;
use serde_json::{json, Value};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

struct Fx {
    base: PathBuf,
    repo: PathBuf,
    cfg: ServerConfig,
}
impl Fx {
    fn clear(&self) {
        for n in ["all_argv.log", "exit_code"] {
            let _ = fs::remove_file(self.base.join(n));
        }
    }
    fn view(&self, state: &str, reason: Option<&str>) {
        let closed_at = if state == "CLOSED" {
            json!("2026-08-19T11:00:00Z")
        } else {
            Value::Null
        };
        let v = json!({"number":42,"title":"Transition","url":"https://github.com/farismnrr/ai-code/issues/42",
            "state":state,"stateReason":reason,"author":{"login":"alice"},"labels":[],
            "createdAt":"2026-08-19T10:00:00Z","updatedAt":"2026-08-19T10:00:00Z",
            "closedAt":closed_at,"body":"body"});
        fs::write(self.base.join("view_response.json"), v.to_string()).unwrap();
    }
    fn blocks(&self) -> Vec<Vec<String>> {
        fs::read_to_string(self.base.join("all_argv.log"))
            .unwrap_or_default()
            .split("---BLOCK---\n")
            .map(|b| {
                b.lines()
                    .filter_map(|l| l.strip_prefix("ARG:").map(str::to_owned))
                    .collect()
            })
            .filter(|v: &Vec<String>| !v.is_empty())
            .collect()
    }
    async fn call(&self, tool: &str, args: Value) -> Value {
        let r = dispatch_git_tool(tool, &args, &self.cfg)
            .await
            .unwrap()
            .unwrap();
        serde_json::from_str(&r.content[0].text).unwrap()
    }
    async fn err(&self, tool: &str, mut args: Value, needle: &str) {
        args.as_object_mut()
            .unwrap()
            .insert("cwd".into(), json!(self.repo));
        let e = dispatch_git_tool(tool, &args, &self.cfg).await.unwrap_err();
        assert!(
            format!("{e:?}").contains(needle),
            "expected {needle:?}, got {e:?}"
        );
    }
    fn assert_argv(&self, expected: &str, index: usize) {
        let blocks = self.blocks();
        let actual = blocks.get(index).unwrap();
        let expected: Vec<String> = expected.split(';').map(str::to_owned).collect();
        assert_eq!(actual, &expected);
    }
}

fn setup_repo(repo: &Path) {
    fs::create_dir_all(repo).unwrap();
    for args in [
        &["init", "-q", "-b", "main"][..],
        &["config", "user.email", "f@ex.test"][..],
        &["config", "user.name", "f"][..],
    ] {
        assert!(Command::new("git")
            .current_dir(repo)
            .args(args)
            .status()
            .unwrap()
            .success());
    }
    fs::write(repo.join("README.md"), "fixture\n").unwrap();
    assert!(Command::new("git")
        .current_dir(repo)
        .args(["add", "README.md"])
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .current_dir(repo)
        .args(["commit", "-qm", "init"])
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .current_dir(repo)
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/farismnrr/ai-code.git"
        ])
        .status()
        .unwrap()
        .success());
}

#[tokio::main]
async fn main() {
    let base = std::env::temp_dir().join(format!("relay-044a-transition-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let repo = base.join("repo");
    setup_repo(&repo);
    let fake = base.join("fake-gh");
    let script = format!(
        r#"#!/usr/bin/env bash
base="{}"
for arg in "$@"; do printf 'ARG:%s\n' "$arg" >> "$base/all_argv.log"; done
echo '---BLOCK---' >> "$base/all_argv.log"
if [ -f "$base/exit_code" ]; then code=$(cat "$base/exit_code"); rm -f "$base/exit_code"; exit "$code"; fi
if [ "$1" = issue ] && [ "$2" = view ]; then cat "$base/view_response.json"; exit 0; fi
exit 0
"#,
        base.display()
    );
    fs::write(&fake, script).unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    std::env::set_var("RELAY_TEST_GH_PATH", &fake);
    let cfg = ServerConfig {
        execution_root: Some(base.to_string_lossy().into()),
        dir: Some(repo.to_string_lossy().into()),
        ..ServerConfig::default()
    };
    let fx = Fx { base, repo, cfg };

    // Required issue number/reason, normalized reason, bounded comment.
    fx.err("issue_close", json!({}), "issue number is required")
        .await;
    fx.err(
        "issue_close",
        json!({"number":1}),
        "close reason is required",
    )
    .await;
    for r in ["", "COMPLETED", "not-planned", "completed\n"] {
        fx.err(
            "issue_close",
            json!({"number":42,"reason":r}),
            "close reason is invalid",
        )
        .await;
    }
    for c in ["", "   ", "a\0b", &"x".repeat(64 * 1024 + 1)] {
        fx.err(
            "issue_close",
            json!({"number":42,"reason":"completed","comment":c}),
            "comment is invalid",
        )
        .await;
    }

    // Duplicate validation: required, positive, non-self, and forbidden for other reasons.
    fx.err(
        "issue_close",
        json!({"number":42,"reason":"duplicate"}),
        "duplicate_of is required",
    )
    .await;
    for d in [json!(0), json!(-1), json!("42")] {
        fx.err(
            "issue_close",
            json!({"number":42,"reason":"duplicate","duplicate_of":d}),
            "duplicate_of is invalid",
        )
        .await;
    }
    fx.err(
        "issue_close",
        json!({"number":42,"reason":"duplicate","duplicate_of":42}),
        "cannot reference",
    )
    .await;
    fx.err(
        "issue_close",
        json!({"number":42,"reason":"completed","duplicate_of":7}),
        "only valid",
    )
    .await;

    // completed close with atomic comment and verified closed reason.
    fx.clear();
    fx.view("CLOSED", Some("COMPLETED"));
    let d = fx
        .call(
            "issue_close",
            json!({"number":42,"reason":"completed","comment":"done"}),
        )
        .await;
    assert_eq!(d["issue"]["state"], "CLOSED");
    fx.assert_argv(
        "issue;close;42;--repo;farismnrr/ai-code;--reason;completed;--comment;done",
        0,
    );
    fx.assert_argv("issue;view;42;--repo;farismnrr/ai-code;--json;number,title,url,state,stateReason,author,labels,createdAt,updatedAt,closedAt,body", 1);

    // Model-supplied repository selectors cannot alter validated identity.
    fx.clear();
    fx.view("CLOSED", Some("COMPLETED"));
    fx.call("issue_close", json!({"number":42,"reason":"completed","owner":"evil","repository":"repo","repo":"evil/repo","url":"https://github.com/evil/repo/issues/42"})).await;
    fx.assert_argv(
        "issue;close;42;--repo;farismnrr/ai-code;--reason;completed",
        0,
    );

    // not_planned maps to provider's spaced reason.
    fx.clear();
    fx.view("CLOSED", Some("NOT_PLANNED"));
    fx.call("issue_close", json!({"number":42,"reason":"not_planned"}))
        .await;
    fx.assert_argv(
        "issue;close;42;--repo;farismnrr/ai-code;--reason;not planned",
        0,
    );

    // duplicate uses the narrow provider mechanism and exact positive target.
    fx.clear();
    fx.view("CLOSED", Some("DUPLICATE"));
    fx.call(
        "issue_close",
        json!({"number":42,"reason":"duplicate","duplicate_of":7}),
    )
    .await;
    fx.assert_argv(
        "issue;close;42;--repo;farismnrr/ai-code;--duplicate-of;7",
        0,
    );

    // Provider failure is surfaced and never followed by a speculative retry/view.
    fx.clear();
    fx.view("CLOSED", Some("COMPLETED"));
    fs::write(fx.base.join("exit_code"), "1").unwrap();
    fx.err(
        "issue_close",
        json!({"number":42,"reason":"completed"}),
        "forge operation failed",
    )
    .await;
    assert_eq!(fx.blocks().len(), 1);

    // Successful provider with OPEN post-state or wrong reason fails closed.
    fx.clear();
    fx.view("OPEN", None);
    fx.err(
        "issue_close",
        json!({"number":42,"reason":"completed"}),
        "post-state is not closed",
    )
    .await;
    assert_eq!(fx.blocks().len(), 2);
    fx.clear();
    fx.view("CLOSED", Some("NOT_PLANNED"));
    fx.err(
        "issue_close",
        json!({"number":42,"reason":"completed"}),
        "reason does not match",
    )
    .await;
    assert_eq!(fx.blocks().len(), 2);

    // Already-closed close is accepted only when provider observable state is correct.
    fx.clear();
    fx.view("CLOSED", Some("COMPLETED"));
    fx.call("issue_close", json!({"number":42,"reason":"completed"}))
        .await;
    assert_eq!(fx.blocks().len(), 2);

    // Reopen requires a positive number and supports one atomic bounded comment.
    fx.err("issue_reopen", json!({}), "issue number is required")
        .await;
    fx.err(
        "issue_reopen",
        json!({"number":0}),
        "issue number is required",
    )
    .await;
    fx.err(
        "issue_reopen",
        json!({"number":42,"comment":""}),
        "comment is invalid",
    )
    .await;
    fx.clear();
    fx.view("OPEN", None);
    fx.call("issue_reopen", json!({"number":42,"comment":"reopening"}))
        .await;
    fx.assert_argv(
        "issue;reopen;42;--repo;farismnrr/ai-code;--comment;reopening",
        0,
    );
    fx.assert_argv("issue;view;42;--repo;farismnrr/ai-code;--json;number,title,url,state,stateReason,author,labels,createdAt,updatedAt,closedAt,body", 1);

    // Provider failure and CLOSED post-state both fail closed; no retry.
    fx.clear();
    fx.view("OPEN", None);
    fs::write(fx.base.join("exit_code"), "1").unwrap();
    fx.err(
        "issue_reopen",
        json!({"number":42}),
        "forge operation failed",
    )
    .await;
    assert_eq!(fx.blocks().len(), 1);
    fx.clear();
    fx.view("CLOSED", Some("COMPLETED"));
    fx.err(
        "issue_reopen",
        json!({"number":42}),
        "post-state is not open",
    )
    .await;
    assert_eq!(fx.blocks().len(), 2);

    // Already-open reopen is accepted only with observable OPEN state.
    fx.clear();
    fx.view("OPEN", None);
    fx.call("issue_reopen", json!({"number":42})).await;
    fx.assert_argv("issue;reopen;42;--repo;farismnrr/ai-code", 0);
    assert_eq!(fx.blocks().len(), 2);

    let _ = fs::remove_dir_all(&fx.base);
    println!("plan044a issue transitions acceptance: PASS");
}
