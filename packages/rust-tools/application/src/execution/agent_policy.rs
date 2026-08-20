use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentProvider {
    Codex,
    Antigravity,
    Claude,
}

impl AgentProvider {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "codex" => Some(Self::Codex),
            "agy" | "antigravity" => Some(Self::Antigravity),
            "claude" => Some(Self::Claude),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Antigravity => "agy",
            Self::Claude => "claude",
        }
    }

    pub(crate) fn binary(self) -> &'static str {
        self.name()
    }

    pub fn auth_probe_argv(self) -> Option<&'static [&'static str]> {
        match self {
            Self::Codex => Some(&["login", "status"]),
            Self::Claude => Some(&["auth", "status"]),
            // The CLI does not currently expose a documented local auth
            // status command that can be checked without starting a model
            // request. Capability discovery therefore uses explicit auth
            // configuration for this provider.
            Self::Antigravity => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
    Quota,
    Auth,
    Unavailable,
    Failed,
}

pub fn provider_argv(provider: AgentProvider, prompt: &str, max_turns: u64) -> Vec<String> {
    match provider {
        AgentProvider::Codex => vec!["exec".into(), "--full-auto".into(), prompt.into()],
        AgentProvider::Antigravity => vec![
            "--print".into(),
            "--mode=accept-edits".into(),
            prompt.into(),
        ],
        AgentProvider::Claude => vec![
            "-p".into(),
            prompt.into(),
            "--permission-mode".into(),
            "acceptEdits".into(),
            "--allowedTools".into(),
            "Read,Edit,Write,Bash".into(),
            "--max-turns".into(),
            max_turns.to_string(),
            "--no-session-persistence".into(),
        ],
    }
}

pub fn classify_failure(exit_code: i32, stdout: &str, stderr: &str) -> Option<FailureClass> {
    if exit_code == 0 {
        return None;
    }
    let text = format!("{stdout}\n{stderr}").to_ascii_lowercase();
    if [
        "insufficient_quota",
        "quota exceeded",
        "rate limit",
        "ratelimit",
        "too many requests",
        "429",
        "billing",
        "credit limit",
        "usage limit",
        "limit reached",
        "capacity",
        "overloaded",
        "service unavailable",
    ]
    .iter()
    .any(|marker| text.contains(marker))
    {
        return Some(FailureClass::Quota);
    }
    if [
        "login required",
        "unauthorized",
        "forbidden",
        "invalid api key",
        "invalid token",
        "authentication failed",
        "not authenticated",
    ]
    .iter()
    .any(|marker| text.contains(marker))
    {
        return Some(FailureClass::Auth);
    }
    if [
        "command not found",
        "command is not available",
        "provider unavailable",
        "service unavailable",
        "connection refused",
        "network is unreachable",
        "enoent",
    ]
    .iter()
    .any(|marker| text.contains(marker))
    {
        return Some(FailureClass::Unavailable);
    }
    Some(FailureClass::Failed)
}

pub fn fallback_allowed(class: FailureClass, workspace_changed: bool) -> bool {
    !workspace_changed
        && matches!(
            class,
            FailureClass::Quota | FailureClass::Auth | FailureClass::Unavailable
        )
}
