//! Cohesive workspace capabilities behind stable application exports.

mod allowlist;
mod dispatch;
mod list;
mod mutate;
mod patch;
mod protected;
mod read;
mod search;
mod secure;

pub use allowlist::*;
pub use dispatch::dispatch_native_tool;
pub use list::*;
pub use mutate::*;
pub use patch::*;
pub use read::*;
pub use search::*;

pub(crate) fn reject_protected_target(
    root: &std::path::Path,
    target: &std::path::Path,
) -> Result<(), relay_core::error::McpError> {
    protected::reject_protected_path(root, target)
}
