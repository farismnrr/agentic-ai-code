use super::paths;
use crate::core::config::ServerConfig;
use std::path::Path;

pub(super) fn add_material(
    args: &mut Vec<String>,
    config: &ServerConfig,
    host_home: &Path,
    material_files: &[std::path::PathBuf],
) -> Result<(), std::io::Error> {
    if !config.allow_ssh {
        return Err(std::io::Error::other(
            "SSH invocation requested while SSH diagnostics are disabled",
        ));
    }
    let ssh_root = config
        .resolved_ssh_root()
        .map_err(|_| std::io::Error::other("invalid SSH credential root"))?;
    let mut seen = std::collections::BTreeSet::new();
    for path in material_files {
        let canonical = std::fs::canonicalize(path)
            .map_err(|_| std::io::Error::other("SSH credential material is unavailable"))?;
        if !canonical.starts_with(&ssh_root) || !canonical.is_file() {
            return Err(std::io::Error::other(
                "SSH credential material escaped the approved root",
            ));
        }
        if !seen.insert(canonical.clone()) {
            continue;
        }
        paths::add_bwrap_parent_dirs(args, canonical.parent(), host_home);
        let value = canonical.to_string_lossy().into_owned();
        args.extend(["--ro-bind".into(), value.clone(), value]);
    }
    Ok(())
}
