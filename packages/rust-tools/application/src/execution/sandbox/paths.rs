use std::path::Path;

pub(super) fn base_bwrap_args(home: &str, root_bind: &str, root: &str) -> Vec<String> {
    let mut args = Vec::with_capacity(32);
    for p in ["/usr", "/lib"] {
        args.extend(["--ro-bind".into(), p.into(), p.into()]);
    }
    for p in ["/lib64", "/etc", "/bin", "/sbin"] {
        args.extend(["--ro-bind-try".into(), p.into(), p.into()]);
    }
    for (flag, p) in [("--dev", "/dev"), ("--proc", "/proc"), ("--tmpfs", "/tmp")] {
        args.extend([flag.into(), p.into()]);
    }
    args.extend([
        "--dir".into(),
        home.into(),
        root_bind.into(),
        root.into(),
        root.into(),
        "--unshare-pid".into(),
        "--die-with-parent".into(),
    ]);
    args
}

pub(super) fn add_bwrap_parent_dirs(args: &mut Vec<String>, parent: Option<&Path>, home: &Path) {
    let Some(parent) = parent else {
        return;
    };
    let mut directories = Vec::new();
    let mut current = parent;
    while current != home && current.starts_with(home) {
        directories.push(current);
        let Some(next) = current.parent() else {
            break;
        };
        current = next;
    }
    directories.reverse();
    for directory in directories {
        args.extend(["--dir".into(), directory.to_string_lossy().into_owned()]);
    }
}
