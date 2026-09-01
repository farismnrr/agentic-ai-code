use ai_tools::application::git::dispatch_git_tool;
use ai_tools::core::config::ServerConfig;
use serde_json::{json, Value};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

fn check_cmd(act: &[String], exp: &str) {
    let e: Vec<&str> = exp.split(';').collect();
    let a: Vec<&str> = act.iter().map(String::as_str).collect();
    assert_eq!(a, e);
}

struct Fixture<'a> {
    base: &'a Path,
    repo: &'a Path,
    cfg: &'a ServerConfig,
}

impl<'a> Fixture<'a> {
    fn file(&self, name: &str, val: &str) {
        fs::write(self.base.join(name), val).unwrap();
    }
    fn clear(&self) {
        for f in [
            "argv.log",
            "all_argv.log",
            "exit_code",
            "override_response.json",
        ] {
            let _ = fs::remove_file(self.base.join(f));
        }
    }
    fn last_argv(&self) -> Vec<String> {
        fs::read_to_string(self.base.join("argv.log"))
            .unwrap_or_default()
            .lines()
            .filter_map(|l| l.strip_prefix("ARG:").map(str::to_owned))
            .collect()
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
    async fn err(&self, tool: &str, args: Value, exp: &str) {
        let e = dispatch_git_tool(tool, &args, self.cfg).await.unwrap_err();
        assert!(
            format!("{e:?}").contains(exp),
            "expected {exp:?}, got {e:?}"
        );
    }
    async fn err_field(&self, tool: &str, key: &str, val: Value, exp: &str) {
        self.err(tool, json!({"cwd": self.repo, key: val}), exp)
            .await;
    }
    async fn err_up_field(&self, key: &str, val: Value, exp: &str) {
        self.err(
            "issue_update",
            json!({"cwd": self.repo, "number": 42, key: val}),
            exp,
        )
        .await;
    }
    async fn call(&self, tool: &str, args: Value) -> Value {
        let res = dispatch_git_tool(tool, &args, self.cfg)
            .await
            .unwrap()
            .unwrap();
        serde_json::from_str(&res.content[0].text).unwrap()
    }
    async fn verify_inject(&self, tool: &str, mut args: Value) {
        let atk = json!({"owner": "atk-org", "repository": "atk-repo", "repo": "atk-org/atk-repo", "url": "https://github.com/atk-org/atk-repo"});
        args.as_object_mut()
            .unwrap()
            .extend(atk.as_object().unwrap().clone());
        self.clear();
        let data = self.call(tool, args).await;
        assert_eq!(data["forge"]["owner"], "farismnrr");
        assert_eq!(data["forge"]["repository"], "ai-code");
        for b in self.blocks() {
            let idx = b
                .iter()
                .position(|x| x == "--repo")
                .expect("must have --repo");
            assert_eq!(b[idx + 1], "farismnrr/ai-code");
            assert!(!b.iter().any(|arg| arg.contains("atk")));
        }
    }
    async fn test_create(&self) {
        self.err(
            "issue_create",
            json!({"cwd": self.repo}),
            "title is required",
        )
        .await;
        for t in ["", "   ", "a\0b", &"a".repeat(257)] {
            self.err_field("issue_create", "title", json!(t), "title is invalid")
                .await;
        }
        for b in ["a\0b", &"a".repeat(64 * 1024 + 1)] {
            self.err(
                "issue_create",
                json!({"cwd": self.repo, "title": "t", "body": b}),
                "body is invalid",
            )
            .await;
        }
        let too_many: Vec<String> = (0..51).map(|i| format!("l{i}")).collect();
        self.err(
            "issue_create",
            json!({"cwd": self.repo, "title": "t", "labels": too_many}),
            "issue labels exceed maximum",
        )
        .await;
        for l in ["", "   ", "a\nb", "a\0b", &"a".repeat(129)] {
            self.err(
                "issue_create",
                json!({"cwd": self.repo, "title": "t", "labels": [l]}),
                "issue label is invalid",
            )
            .await;
        }
        self.err(
            "issue_create",
            json!({"cwd": self.repo, "title": "t", "labels": "bad"}),
            "issue labels are invalid",
        )
        .await;

        self.clear();
        let view = json!({"number": 42, "title": "Bug in parser", "url": "https://github.com/farismnrr/ai-code/issues/42", "state": "OPEN", "stateReason": "", "author": {"login": "alice"}, "labels": [{"name": "bug"}, {"name": "forge"}], "createdAt": "2026-08-19T12:00:00Z", "updatedAt": "2026-08-19T12:00:00Z", "closedAt": null, "body": "Detailed steps."});
        self.file("view_response.json", &view.to_string());
        let data = self.call("issue_create", json!({"cwd": self.repo, "title": "Bug in parser", "body": "Detailed steps.", "labels": ["bug", "forge"]})).await;
        assert_eq!(data["issue"]["number"], 42);
        assert_eq!(data["issue"]["title"], "Bug in parser");
        assert_eq!(data["issue"]["labels"], json!(["bug", "forge"]));
        assert_eq!(data["forge"]["owner"], "farismnrr");

        let b = self.blocks();
        assert_eq!(b.len(), 2);
        check_cmd(&b[0], "issue;create;--repo;farismnrr/ai-code;--title;Bug in parser;--body;Detailed steps.;--label;bug;--label;forge");
        check_cmd(&b[1], "issue;view;42;--repo;farismnrr/ai-code;--json;number,title,url,state,stateReason,author,labels,createdAt,updatedAt,closedAt,body");

        self.clear();
        let _ = self
            .call("issue_create", json!({"cwd": self.repo, "title": "Min"}))
            .await;
        check_cmd(
            &self.blocks()[0],
            "issue;create;--repo;farismnrr/ai-code;--title;Min;--body;",
        );

        self.file(
            "override_response.json",
            "https://github.com/evil-org/ai-code/issues/42\n",
        );
        self.err_field(
            "issue_create",
            "title",
            json!("Evil"),
            "created issue identity is invalid",
        )
        .await;
        self.file("override_response.json", "garbage output\n");
        self.err_field(
            "issue_create",
            "title",
            json!("Garbage"),
            "created issue identity is invalid",
        )
        .await;
        self.file("exit_code", "1");
        self.err_field(
            "issue_create",
            "title",
            json!("Fail"),
            "forge operation failed",
        )
        .await;
    }
    async fn test_update(&self) {
        self.err(
            "issue_update",
            json!({"cwd": self.repo}),
            "issue number is required",
        )
        .await;
        self.err_field(
            "issue_update",
            "number",
            json!(0),
            "issue number is required",
        )
        .await;
        self.err_field(
            "issue_update",
            "number",
            json!(42),
            "no issue update was supplied",
        )
        .await;
        self.err(
            "issue_update",
            json!({"cwd": self.repo, "number": 42, "add_labels": [], "remove_labels": []}),
            "no issue update was supplied",
        )
        .await;
        for t in ["", "   ", "a\0b", &"a".repeat(257)] {
            self.err_up_field("title", json!(t), "title is invalid")
                .await;
        }
        for b in ["a\0b", &"a".repeat(64 * 1024 + 1)] {
            self.err_up_field("body", json!(b), "body is invalid").await;
        }
        self.err_up_field("add_labels", json!("bad"), "issue labels are invalid")
            .await;
        self.err_up_field("add_labels", json!([""]), "issue label is invalid")
            .await;
        self.err_up_field("remove_labels", json!(["a\0b"]), "issue label is invalid")
            .await;

        self.clear();
        let view = json!({"number": 42, "title": "Updated", "url": "https://github.com/farismnrr/ai-code/issues/42", "state": "OPEN", "stateReason": "", "author": {"login": "alice"}, "labels": [{"name": "enhancement"}], "createdAt": "2026-08-19T12:00:00Z", "updatedAt": "2026-08-19T13:00:00Z", "closedAt": null, "body": "New body."});
        self.file("view_response.json", &view.to_string());
        let data = self.call("issue_update", json!({"cwd": self.repo, "number": 42, "title": "Updated", "body": "New body.", "add_labels": ["enhancement"], "remove_labels": ["bug"]})).await;
        assert_eq!(data["issue"]["title"], "Updated");
        assert_eq!(data["issue"]["labels"], json!(["enhancement"]));

        let b = self.blocks();
        assert_eq!(b.len(), 2);
        check_cmd(&b[0], "issue;edit;42;--repo;farismnrr/ai-code;--title;Updated;--body;New body.;--add-label;enhancement;--remove-label;bug");
        check_cmd(&b[1], "issue;view;42;--repo;farismnrr/ai-code;--json;number,title,url,state,stateReason,author,labels,createdAt,updatedAt,closedAt,body");

        self.clear();
        let _ = self
            .call(
                "issue_update",
                json!({"cwd": self.repo, "number": 42, "title": "Only Title"}),
            )
            .await;
        check_cmd(
            &self.blocks()[0],
            "issue;edit;42;--repo;farismnrr/ai-code;--title;Only Title",
        );

        self.clear();
        let _ = self
            .call(
                "issue_update",
                json!({"cwd": self.repo, "number": 42, "add_labels": ["doc"]}),
            )
            .await;
        check_cmd(
            &self.blocks()[0],
            "issue;edit;42;--repo;farismnrr/ai-code;--add-label;doc",
        );

        self.file("exit_code", "1");
        self.err_up_field("title", json!("New"), "forge operation failed")
            .await;
    }
    async fn test_comment(&self) {
        self.err(
            "issue_comment",
            json!({"cwd": self.repo}),
            "issue number is required",
        )
        .await;
        self.err(
            "issue_comment",
            json!({"cwd": self.repo, "number": 0, "body": "c"}),
            "issue number is required",
        )
        .await;
        self.err_field("issue_comment", "number", json!(42), "body is required")
            .await;
        for b in ["", "   ", "a\0b", &"a".repeat(64 * 1024 + 1)] {
            self.err(
                "issue_comment",
                json!({"cwd": self.repo, "number": 42, "body": b}),
                "body is invalid",
            )
            .await;
        }

        self.clear();
        let data = self
            .call(
                "issue_comment",
                json!({"cwd": self.repo, "number": 42, "body": "Fixed in 123"}),
            )
            .await;
        assert_eq!(data["issueNumber"], 42);
        assert_eq!(
            data["commentUrl"],
            "https://github.com/farismnrr/ai-code/issues/42#issuecomment-987654321"
        );
        assert_eq!(data["forge"]["owner"], "farismnrr");

        check_cmd(
            &self.last_argv(),
            "issue;comment;42;--repo;farismnrr/ai-code;--body;Fixed in 123",
        );
        assert_eq!(
            self.blocks().len(),
            1,
            "issue_comment must not fetch threads"
        );

        let err_cases = [
            (
                "https://github.com/farismnrr/ai-code/issues/99#issuecomment-123\n",
                "comment identity is invalid",
            ),
            (
                "https://github.com/evil-org/ai-code/issues/42#issuecomment-123\n",
                "comment identity is invalid",
            ),
            (
                "https://github.com/farismnrr/ai-code/issues/42#issuecomment-nonnum\n",
                "comment identity is invalid",
            ),
            (
                "https://github.com/farismnrr/ai-code/issues/42\n",
                "comment identity is invalid",
            ),
            ("not a url\n", "comment identity is invalid"),
        ];
        for (payload, exp) in err_cases {
            self.file("override_response.json", payload);
            self.err(
                "issue_comment",
                json!({"cwd": self.repo, "number": 42, "body": "c"}),
                exp,
            )
            .await;
        }
        self.file("exit_code", "1");
        self.err(
            "issue_comment",
            json!({"cwd": self.repo, "number": 42, "body": "c"}),
            "forge operation failed",
        )
        .await;
    }
}

fn setup_repo(repo: &Path) {
    fs::create_dir_all(repo).unwrap();
    let git = |args: &[&str]| {
        assert!(Command::new("git")
            .current_dir(repo)
            .args(args)
            .status()
            .unwrap()
            .success())
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "user.email", "f@ex.test"]);
    git(&["config", "user.name", "f"]);
    fs::write(repo.join("README.md"), "t\n").unwrap();
    git(&["add", "README.md"]);
    git(&["commit", "-qm", "init"]);
    git(&[
        "remote",
        "add",
        "origin",
        "https://github.com/farismnrr/ai-code.git",
    ]);
}

#[tokio::main]
async fn main() {
    let base = std::env::temp_dir().join(format!("relay-044a-mut-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let repo = base.join("repo");
    setup_repo(&repo);

    let fake_gh = base.join("fake-gh");
    let script = format!(
        r#"#!/usr/bin/env bash
base="{}"
rm -f "$base/argv.log"
for arg in "$@"; do
    printf 'ARG:%s\n' "$arg" >> "$base/argv.log"
    printf 'ARG:%s\n' "$arg" >> "$base/all_argv.log"
done
echo '---BLOCK---' >> "$base/all_argv.log"
if [ -f "$base/exit_code" ]; then
    code=$(cat "$base/exit_code"); rm -f "$base/exit_code"; exit "$code"
fi
if [ -f "$base/override_response.json" ]; then
    cat "$base/override_response.json"; rm -f "$base/override_response.json"; exit 0
fi
if [ "$1" = "issue" ] && [ "$2" = "create" ]; then
    echo "https://github.com/farismnrr/ai-code/issues/42"; exit 0
fi
if [ "$1" = "issue" ] && [ "$2" = "edit" ]; then exit 0; fi
if [ "$1" = "issue" ] && [ "$2" = "comment" ]; then
    echo "https://github.com/farismnrr/ai-code/issues/$3#issuecomment-987654321"; exit 0
fi
if [ "$1" = "issue" ] && [ "$2" = "view" ]; then
    if [ -f "$base/view_response.json" ]; then cat "$base/view_response.json"
    else echo '{{"number":42,"title":"Default","url":"https://github.com/farismnrr/ai-code/issues/42","state":"OPEN","stateReason":"","author":{{"login":"alice"}},"labels":[],"createdAt":"2026-08-19T10:00:00Z","updatedAt":"2026-08-19T10:00:00Z","closedAt":null,"body":"Default body"}}'
    fi
    exit 0
fi
exit 0
"#,
        base.display()
    );
    fs::write(&fake_gh, script).unwrap();
    fs::set_permissions(&fake_gh, fs::Permissions::from_mode(0o755)).unwrap();
    std::env::set_var("RELAY_TEST_GH_PATH", &fake_gh);

    let config = ServerConfig {
        execution_root: Some(base.to_string_lossy().into()),
        dir: Some(repo.to_string_lossy().into()),
        ..ServerConfig::default()
    };
    let fx = Fixture {
        base: &base,
        repo: &repo,
        cfg: &config,
    };

    // 1. issue_create
    fx.test_create().await;

    // 2. issue_update
    fx.test_update().await;

    // 3. issue_comment
    fx.test_comment().await;

    // 4. P2-2: Identity Selector Injection Immunity Proofs (Create, Update, Comment)
    fx.verify_inject(
        "issue_create",
        json!({"cwd": repo, "title": "Sec", "body": "B"}),
    )
    .await;
    fx.verify_inject(
        "issue_update",
        json!({"cwd": repo, "number": 42, "title": "Sec"}),
    )
    .await;
    fx.verify_inject(
        "issue_comment",
        json!({"cwd": repo, "number": 42, "body": "Sec"}),
    )
    .await;

    let _ = fs::remove_dir_all(&base);
    println!("plan044a issue mutations acceptance: PASS");
}
