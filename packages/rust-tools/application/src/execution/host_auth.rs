//! Narrow opt-in host authentication mounts for ordinary terminal sandboxes.

use std::path::Path;

pub(crate) fn add_host_github_auth(
    args: &mut Vec<String>,
    host_home: &Path,
) -> Result<(), std::io::Error> {
    // Keep this deliberately narrower than a whole-home bind. `gh` stores its
    // login token beneath ~/.config/gh by default, while Git may need the
    // owner's global config for the `gh auth git-credential` helper. Both are
    // mounted read-only; generic credential stores such as ~/.git-credentials
    // and ~/.ssh remain protected by the existing policy.
    let gh_config = host_home.join(".config/gh");
    if gh_config.exists() {
        add_read_only_home_path(args, host_home, &gh_config, true)?;
    }

    for candidate in [host_home.join(".gitconfig"), host_home.join(".config/git")] {
        if candidate.exists() {
            add_read_only_home_path(args, host_home, &candidate, false)?;
        }
    }
    Ok(())
}

pub(crate) fn skip_protected_mask(root: &Path, path: &Path, enabled: bool) -> bool {
    enabled && path == root.join(".config/gh")
}

fn add_read_only_home_path(
    args: &mut Vec<String>,
    host_home: &Path,
    candidate: &Path,
    require_directory: bool,
) -> Result<(), std::io::Error> {
    if !candidate.is_absolute()
        || !candidate.starts_with(host_home)
        || (require_directory && !candidate.is_dir())
    {
        return Err(std::io::Error::other(
            "invalid host GitHub/Git configuration path",
        ));
    }
    let canonical = std::fs::canonicalize(candidate)?;
    if !canonical.starts_with(host_home) {
        return Err(std::io::Error::other(
            "host GitHub/Git configuration escaped HOME",
        ));
    }
    let value = canonical.to_string_lossy().into_owned();
    args.extend(["--ro-bind".into(), value.clone(), value]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn github_auth_opt_in_skips_only_exact_host_gh_mask() {
        let root = Path::new("/home/example");
        assert!(skip_protected_mask(
            root,
            Path::new("/home/example/.config/gh"),
            true,
        ));
        assert!(!skip_protected_mask(
            root,
            Path::new("/home/example/.ssh"),
            true,
        ));
        assert!(!skip_protected_mask(
            root,
            Path::new("/home/example/.config/gh"),
            false,
        ));
    }

    #[test]
    fn mounts_only_reviewed_git_config_paths() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let home =
            std::env::temp_dir().join(format!("relay-gh-auth-{}-{nonce}", std::process::id()));
        let gh = home.join(".config/gh");
        let git_dir = home.join(".config/git");
        std::fs::create_dir_all(&gh).expect("gh dir");
        std::fs::create_dir_all(&git_dir).expect("git dir");
        std::fs::write(
            home.join(".gitconfig"),
            "[credential]\n\thelper = gh auth git-credential\n",
        )
        .expect("gitconfig");
        std::fs::write(home.join(".git-credentials"), "must-not-be-mounted\n")
            .expect("generic credentials");

        let mut args = Vec::new();
        add_host_github_auth(&mut args, &home).expect("github auth mounts");
        let joined = args.join("\n");
        assert!(joined.contains(gh.to_string_lossy().as_ref()));
        assert!(joined.contains(home.join(".gitconfig").to_string_lossy().as_ref()));
        assert!(joined.contains(git_dir.to_string_lossy().as_ref()));
        assert!(!joined.contains(".git-credentials"));
        assert_eq!(
            args.iter()
                .filter(|arg| arg.as_str() == "--ro-bind")
                .count(),
            3
        );

        std::fs::remove_dir_all(home).expect("cleanup");
    }
}
