use super::paths;
use crate::core::config::ServerConfig;
use std::path::Path;

pub(super) fn add_material(
    args: &mut Vec<String>,
    config: &ServerConfig,
    host_home: &Path,
    identity_file: &Path,
    known_hosts_file: &Path,
) -> Result<(), std::io::Error> {
    if !config.allow_ssh {
        return Err(std::io::Error::other(
            "SSH invocation requested while SSH diagnostics are disabled",
        ));
    }
    let ssh_root = config
        .resolved_ssh_root()
        .map_err(|_| std::io::Error::other("invalid SSH credential root"))?;
    for path in [identity_file, known_hosts_file] {
        let canonical = std::fs::canonicalize(path)
            .map_err(|_| std::io::Error::other("SSH credential material is unavailable"))?;
        if !canonical.starts_with(&ssh_root) || !canonical.is_file() {
            return Err(std::io::Error::other(
                "SSH credential material escaped the approved root",
            ));
        }
        paths::add_bwrap_parent_dirs(args, canonical.parent(), host_home);
        let value = canonical.to_string_lossy().into_owned();
        args.extend(["--ro-bind".into(), value.clone(), value]);
    }
    Ok(())
}
