use super::dirs_home;
use crate::error::RelayError;
use std::path::Path;

impl super::ServerConfig {
    pub fn agent_environment_for(&self, provider: &str) -> Vec<(String, String)> {
        self.agent_env_vars
            .iter()
            .filter_map(|entry| {
                let (configured_provider, variable) = entry.split_once('=')?;
                (configured_provider == provider)
                    .then(|| std::env::var(variable).ok())
                    .flatten()
                    .map(|value| (variable.to_owned(), value))
            })
            .collect()
    }

    pub fn agent_auth_root_for(&self, provider: &str) -> Option<std::path::PathBuf> {
        self.agent_auth_roots_for(provider).into_iter().next()
    }

    pub fn has_explicit_agent_auth_root(&self, provider: &str) -> bool {
        self.agent_auth_roots.iter().any(|entry| {
            entry
                .split_once('=')
                .is_some_and(|(configured_provider, _)| configured_provider == provider)
        })
    }

    /// Return the provider's explicitly configured auth root, or the small
    /// set of well-known local session directories that the sandbox may mount
    /// automatically. The fallback is deliberately provider-specific and
    /// never broadens to the whole runtime HOME.
    pub fn agent_auth_roots_for(&self, provider: &str) -> Vec<std::path::PathBuf> {
        let explicit = self
            .agent_auth_roots
            .iter()
            .filter_map(|entry| {
                let (configured_provider, path) = entry.split_once('=')?;
                (configured_provider == provider)
                    .then(|| std::fs::canonicalize(path).ok())
                    .flatten()
            })
            .collect::<Vec<_>>();
        if !explicit.is_empty() {
            return explicit;
        }

        let Some(home) = dirs_home().and_then(|path| std::fs::canonicalize(path).ok()) else {
            return Vec::new();
        };
        let candidates = match provider {
            // The CLI's OAuth session is stored below this directory. Only
            // mount it when the session file is present.
            "codex" if home.join(".codex/auth.json").is_file() => {
                vec![home.join(".codex")]
            }
            // The CLI may use either a credentials file or the platform
            // keychain; mounting this narrow directory covers the file-backed
            // session without exposing unrelated home data.
            "claude" if home.join(".claude").is_dir() => vec![home.join(".claude")],
            // Antigravity has two documented local state locations. Presence
            // is only an auth-root hint; capability discovery still requires
            // an explicit non-request auth source because the CLI has no
            // side-effect-free auth-status command.
            "agy" => [
                home.join(".gemini/antigravity-cli"),
                home.join(".gemini/antigravity"),
            ]
            .into_iter()
            .filter(|path| path.is_dir())
            .collect(),
            _ => Vec::new(),
        };

        candidates
            .into_iter()
            .filter_map(|path| std::fs::canonicalize(path).ok())
            .filter(|path| {
                path.is_dir()
                    && path != &home
                    && path.starts_with(&home)
                    && !crate::protected_paths::is_protected_path(&home, path)
                    && !self
                        .resolved_execution_root()
                        .is_ok_and(|root| crate::protected_paths::is_protected_path(&root, path))
            })
            .collect()
    }
}

pub(super) fn validate_env_entries(entries: &[String]) -> Result<(), RelayError> {
    if entries.len() > 16 {
        return Err(RelayError::InvalidConfig(
            "at most 16 agent-env mappings may be configured".into(),
        ));
    }
    let mut seen = std::collections::HashSet::new();
    for entry in entries {
        let Some((provider, variable)) = entry.split_once('=') else {
            return Err(RelayError::InvalidConfig(
                "agent-env must use provider=ENV_NAME syntax".into(),
            ));
        };
        if !valid_provider(provider)
            || !valid_env_name(variable)
            || matches!(
                variable,
                "PATH" | "HOME" | "TMPDIR" | "LD_PRELOAD" | "BASH_ENV"
            )
            || !seen.insert((provider.to_owned(), variable.to_owned()))
        {
            return Err(RelayError::InvalidConfig(
                "agent-env contains an invalid, duplicate, or sandbox-control environment name"
                    .into(),
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_auth_root_entries(
    entries: &[String],
    execution_root: &Path,
) -> Result<(), RelayError> {
    if entries.len() > 8 {
        return Err(RelayError::InvalidConfig(
            "at most 8 agent-auth-root mappings may be configured".into(),
        ));
    }
    let home = dirs_home()
        .and_then(|path| std::fs::canonicalize(path).ok())
        .ok_or_else(|| RelayError::InvalidConfig("runtime HOME cannot be resolved".into()))?;
    let mut seen = std::collections::HashSet::new();
    for entry in entries {
        let Some((provider, path)) = entry.split_once('=') else {
            return Err(RelayError::InvalidConfig(
                "agent-auth-root must use provider=/absolute/path syntax".into(),
            ));
        };
        let candidate = Path::new(path);
        let canonical = std::fs::canonicalize(candidate).map_err(|_| {
            RelayError::InvalidConfig(
                "agent-auth-root must resolve to an existing directory".into(),
            )
        })?;
        if !valid_provider(provider)
            || !candidate.is_absolute()
            || !canonical.is_dir()
            || !canonical.starts_with(&home)
            || canonical == home
            || crate::protected_paths::is_protected_path(&home, &canonical)
            || crate::protected_paths::is_protected_path(execution_root, &canonical)
            || !seen.insert(provider.to_owned())
        {
            return Err(RelayError::InvalidConfig(
                "agent-auth-root must be a unique provider directory beneath runtime HOME and outside protected paths".into(),
            ));
        }
    }
    Ok(())
}

fn valid_provider(value: &str) -> bool {
    matches!(value, "codex" | "agy" | "claude")
}

fn valid_env_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().enumerate().all(|(index, byte)| {
            (index == 0 && (byte.is_ascii_alphabetic() || byte == b'_'))
                || (index > 0 && (byte.is_ascii_alphanumeric() || byte == b'_'))
        })
}

pub(super) fn validate_lsp_entries(entries: &[String]) -> Result<(), RelayError> {
    if entries.len() > 16 {
        return Err(RelayError::InvalidConfig(
            "at most 16 lsp-server mappings may be configured".into(),
        ));
    }
    let mut languages = std::collections::HashSet::new();
    for entry in entries {
        let Some((language, executable)) = entry.split_once('=') else {
            return Err(RelayError::InvalidConfig(
                "lsp-server must use language=executable syntax".into(),
            ));
        };
        let valid_language = !language.is_empty()
            && language.len() <= 64
            && language.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'+')
            });
        let valid_executable = !executable.is_empty()
            && executable.len() <= 128
            && !executable.contains('/')
            && !executable.contains('\\')
            && executable.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'+')
            });
        if !valid_language || !valid_executable {
            return Err(RelayError::InvalidConfig(
                "lsp-server language/executable contains unsupported characters".into(),
            ));
        }
        if !languages.insert(language.to_ascii_lowercase()) {
            return Err(RelayError::InvalidConfig(
                "lsp-server language is configured more than once".into(),
            ));
        }
    }
    Ok(())
}
