//! One Bubblewrap construction path for application-owned subprocesses.

use super::{InvocationProgram, InvocationSecurity, ToolInvocation};
use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use tokio::sync::watch;

mod managed_process;
mod masks;
mod paths;
mod protected_paths;
mod spawn;
mod ssh_material;
mod toolchain_mounts;

pub(super) use managed_process::spawn;
pub(crate) use managed_process::{spawn_hook, spawn_lsp};
pub(crate) use protected_paths::{
    prime_protected_path_indexes, schedule_workspace_protected_path_indexes,
};
use spawn::spawn_with_profile;

#[derive(Clone, Copy)]
pub(crate) enum WorkspaceAccess {
    ReadOnly,
    Writable,
}

pub(crate) struct SpawnControl<'a> {
    deadline: Option<Instant>,
    cancel: &'a watch::Receiver<bool>,
}

impl SpawnControl<'_> {
    pub(crate) fn check(&self) -> Result<(), io::Error> {
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "terminal execution deadline elapsed",
            ));
        }
        if *self.cancel.borrow() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "terminal execution was cancelled",
            ));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct SandboxError {
    pub(crate) stage: &'static str,
    source: io::Error,
}

impl SandboxError {
    fn at(stage: &'static str, source: io::Error) -> Self {
        Self { stage, source }
    }

    pub(crate) fn kind(&self) -> io::ErrorKind {
        self.source.kind()
    }

    fn into_io_error(self) -> io::Error {
        self.source
    }
}

impl From<io::Error> for SandboxError {
    fn from(source: io::Error) -> Self {
        Self::at("sandbox_profile", source)
    }
}

#[derive(Clone, Copy)]
enum NetworkAccess {
    Host,
    Isolated,
}

#[derive(Clone, Copy)]
enum WorkspaceMount<'a> {
    Authorized,
    Fixed(&'a Path),
    Hidden,
}

struct SandboxProfile<'a> {
    workspace_access: WorkspaceAccess,
    network_access: NetworkAccess,
    expose_optional_sockets: bool,
    expose_runtime_extras: bool,
    workspace_mount: WorkspaceMount<'a>,
}

pub(crate) use super::toolchain::{resolve_safe_executable, safe_path_entries};

pub(crate) fn runtime_home() -> Result<PathBuf, std::io::Error> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| std::io::Error::other("HOME is unavailable"))?;
    if !home.is_absolute() {
        return Err(std::io::Error::other("HOME must be an absolute path"));
    }
    std::fs::canonicalize(home).map_err(|_| std::io::Error::other("HOME is unavailable"))
}
