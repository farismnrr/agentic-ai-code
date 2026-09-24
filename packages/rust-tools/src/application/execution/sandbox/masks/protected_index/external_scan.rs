use super::index::{ProtectedPathIndex, ProtectedPathInventory};
use super::IndexScanBudget;
use std::collections::{BTreeSet, HashMap};
use std::ffi::OsString;
use std::io::{self, Read};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::FileTypeExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::Duration;

struct ExternalOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn resolve_external_tool(name: &str) -> Option<PathBuf> {
    ["/usr/local/bin", "/usr/bin", "/bin"]
        .into_iter()
        .map(Path::new)
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
}

fn run_external_scan(
    executable: &Path,
    args: &[OsString],
    root: &Path,
    budget: &IndexScanBudget,
) -> io::Result<ExternalOutput> {
    budget.check()?;
    let mut child = Command::new(executable)
        .args(args)
        .current_dir(root)
        .env_remove("RIPGREP_CONFIG_PATH")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("protected-path scanner stdout unavailable"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("protected-path scanner stderr unavailable"))?;
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).map(|_| bytes)
    });
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).map(|_| bytes)
    });

    let status = loop {
        if budget.remaining().is_zero() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "protected-path external scan deadline elapsed",
            ));
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        thread::sleep(budget.remaining().min(Duration::from_millis(2)));
    };

    let stdout = stdout_reader
        .join()
        .map_err(|_| io::Error::other("protected-path scanner stdout reader panicked"))??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| io::Error::other("protected-path scanner stderr reader panicked"))??;
    Ok(ExternalOutput {
        status,
        stdout,
        stderr,
    })
}

fn normalized_relative(raw: &[u8]) -> io::Result<PathBuf> {
    let relative = PathBuf::from(OsString::from_vec(raw.to_vec()));
    let relative = relative
        .strip_prefix(".")
        .unwrap_or(relative.as_path())
        .to_path_buf();
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "protected-path scanner returned an unsafe path",
        ));
    }
    Ok(relative)
}

fn outermost_protected_relative(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .filter(|ancestor| !ancestor.as_os_str().is_empty())
        .filter(|ancestor| crate::core::protected_paths::is_protected_relative(ancestor))
        .last()
        .map(Path::to_path_buf)
}

pub(super) fn build_external_inventory(
    root: &Path,
    budget: &mut IndexScanBudget,
) -> io::Result<Option<ProtectedPathIndex>> {
    #[cfg(feature = "test-protected-index")]
    if budget.has_entry_limit() {
        return Ok(None);
    }

    let Some(rg) = resolve_external_tool("rg") else {
        return Ok(None);
    };
    let Some(find) = resolve_external_tool("find") else {
        return Ok(None);
    };

    let mut rg_args = vec![
        OsString::from("--files"),
        OsString::from("--hidden"),
        OsString::from("--no-ignore"),
        OsString::from("--no-config"),
        OsString::from("--null"),
    ];
    for directory in crate::core::protected_paths::PROTECTED_DIRECTORIES {
        rg_args.push(OsString::from("--glob"));
        rg_args.push(OsString::from(format!("**/{directory}/**")));
    }
    for file in crate::core::protected_paths::PROTECTED_FILES {
        rg_args.push(OsString::from("--glob"));
        rg_args.push(OsString::from(format!("**/{file}")));
    }
    rg_args.push(OsString::from("--glob"));
    rg_args.push(OsString::from("**/.env.*"));
    rg_args.push(OsString::from("--glob"));
    rg_args.push(OsString::from("!**/.env.example"));
    for directory in crate::application::workspace::DEPENDENCY_OR_GENERATED_DIRECTORIES {
        rg_args.push(OsString::from("--glob"));
        rg_args.push(OsString::from(format!("!**/{directory}/**")));
    }
    rg_args.push(OsString::from("--"));
    rg_args.push(OsString::from("."));

    let rg_output = run_external_scan(&rg, &rg_args, root, budget)?;
    let rg_code = rg_output.status.code();
    if !matches!(rg_code, Some(0) | Some(1)) {
        let detail = String::from_utf8_lossy(&rg_output.stderr);
        return Err(io::Error::other(format!(
            "ripgrep protected-path scan failed with status {:?}: {}",
            rg_code,
            detail.trim()
        )));
    }

    let mut protected_paths = BTreeSet::new();
    let mut reported_entries = 0usize;
    for raw in rg_output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|raw| !raw.is_empty())
    {
        budget.check()?;
        reported_entries = reported_entries.saturating_add(1);
        let relative = normalized_relative(raw)?;
        let protected = outermost_protected_relative(&relative).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "ripgrep protected-path scan returned a non-protected match",
            )
        })?;
        protected_paths.insert(root.join(protected));
    }

    let mut find_args = vec![OsString::from(".")];
    find_args.extend([
        OsString::from("("),
        OsString::from("-type"),
        OsString::from("d"),
        OsString::from("("),
    ]);
    for (index, directory) in crate::application::workspace::DEPENDENCY_OR_GENERATED_DIRECTORIES
        .iter()
        .enumerate()
    {
        if index > 0 {
            find_args.push(OsString::from("-o"));
        }
        find_args.extend([OsString::from("-name"), OsString::from(directory)]);
    }
    find_args.extend([
        OsString::from(")"),
        OsString::from("-prune"),
        OsString::from(")"),
        OsString::from("-o"),
        OsString::from("("),
        OsString::from("-type"),
        OsString::from("s"),
        OsString::from("-o"),
        OsString::from("-type"),
        OsString::from("l"),
        OsString::from(")"),
        OsString::from("-print0"),
    ]);
    let find_output = run_external_scan(&find, &find_args, root, budget)?;
    if !find_output.status.success() {
        let detail = String::from_utf8_lossy(&find_output.stderr);
        return Err(io::Error::other(format!(
            "find protected-path special-file scan failed with status {:?}: {}",
            find_output.status.code(),
            detail.trim()
        )));
    }

    for raw in find_output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|raw| !raw.is_empty())
    {
        budget.check()?;
        reported_entries = reported_entries.saturating_add(1);
        let relative = normalized_relative(raw)?;
        let path = root.join(&relative);
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        let protected_ancestor = outermost_protected_relative(&relative);
        if metadata.file_type().is_symlink() {
            if let Some(protected) = protected_ancestor {
                protected_paths.insert(root.join(protected));
            }
            continue;
        }
        if metadata.file_type().is_socket() {
            protected_paths.insert(
                protected_ancestor
                    .map(|protected| root.join(protected))
                    .unwrap_or(path),
            );
        }
    }

    Ok(Some(ProtectedPathIndex {
        inventory: std::sync::Mutex::new(ProtectedPathInventory {
            directories: HashMap::new(),
            protected_paths,
            scanned_entries: reported_entries,
            watcher: None,
        }),
    }))
}
