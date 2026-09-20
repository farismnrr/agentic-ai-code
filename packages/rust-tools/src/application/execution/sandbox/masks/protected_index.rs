//! Bounded, fail-closed protected-path indexing for mounted workspace trees.
use std::io;
use std::time::{Duration, Instant};

mod controller;
mod index;
mod operations;
mod state;
mod traversal;
mod watcher;

#[cfg(feature = "test-protected-index")]
pub(super) use operations::schedule_initialization;
pub(super) use operations::{discover, lock_and_validate_freshness};
pub(in crate::application::execution::sandbox) use state::{
    PreSpawnFreshnessGuard, ProtectedPathFreshness,
};

#[cfg(feature = "test-protected-index")]
pub(super) fn prime_with_entry_limit(
    root: &std::path::Path,
    budget: Duration,
    max_entries: usize,
) -> io::Result<usize> {
    operations::prime_with_entry_limit(root, budget, max_entries)
}

#[cfg(feature = "test-protected-index")]
pub(super) fn is_permanent_index_error(kind: io::ErrorKind) -> bool {
    controller::is_permanent_index_error(kind)
}

// Protected names such as `.env` may occur at arbitrary depth in source trees.
// Dependency/generated directories use the canonical workspace skip policy and
// are intentionally outside protected-path discovery. The independent scan
// deadline remains the CPU/time bound and all incomplete scans fail closed.
pub(super) const MAX_PROTECTED_SCAN_DIRECTORIES: usize = 100_000;
const CONTROL_CHECK_INTERVAL: usize = 64;
const MAX_CACHE_ROOTS: usize = 64;
const MAX_CACHED_INDEX_ENTRIES: usize = 1_000_000;
const MAX_QUEUED_INITIALIZATIONS: usize = MAX_CACHE_ROOTS;
// Keep failures fail-closed while retrying soon enough to recover after a
// transient permission or mount issue is repaired.
const FAILED_INDEX_RETRY: Duration = Duration::from_secs(30);
const CHURN_RETRY_DELAY: Duration = Duration::from_millis(250);
const WORKSPACE_INDEX_BUDGET: Duration = Duration::from_secs(60);

/// An index lifecycle budget is independent of any one terminal command's deadline.
pub(super) struct IndexScanBudget {
    deadline: Instant,
    max_directories: usize,
    #[cfg(feature = "test-protected-index")]
    max_entries: Option<usize>,
    scanned_entries: usize,
    scanned_directories: usize,
}

impl IndexScanBudget {
    pub(super) fn new(timeout: Duration) -> Self {
        Self {
            deadline: Instant::now() + timeout,
            max_directories: MAX_PROTECTED_SCAN_DIRECTORIES,
            #[cfg(feature = "test-protected-index")]
            max_entries: None,
            scanned_entries: 0,
            scanned_directories: 0,
        }
    }

    #[cfg(feature = "test-protected-index")]
    pub(super) fn with_max_entries(timeout: Duration, max_entries: usize) -> Self {
        Self {
            deadline: Instant::now() + timeout,
            max_directories: MAX_PROTECTED_SCAN_DIRECTORIES,
            max_entries: Some(max_entries),
            scanned_entries: 0,
            scanned_directories: 0,
        }
    }

    pub(super) fn check(&self) -> io::Result<()> {
        if Instant::now() >= self.deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "protected-path index initialization deadline elapsed",
            ));
        }
        Ok(())
    }

    pub(super) fn consume_entry(&mut self) -> io::Result<usize> {
        self.check()?;
        self.scanned_entries = self.scanned_entries.saturating_add(1);
        #[cfg(feature = "test-protected-index")]
        if self
            .max_entries
            .is_some_and(|max_entries| self.scanned_entries > max_entries)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "protected-path scan exceeds test entry maximum",
            ));
        }
        if self.scanned_entries.is_multiple_of(CONTROL_CHECK_INTERVAL) {
            self.check()?;
        }
        Ok(self.scanned_entries)
    }

    pub(super) fn consume_directory(&mut self) -> io::Result<usize> {
        self.check()?;
        self.scanned_directories = self.scanned_directories.saturating_add(1);
        if self.scanned_directories > self.max_directories {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "protected-path scan exceeds bounded directory/watch maximum",
            ));
        }
        Ok(self.scanned_directories)
    }

    pub(super) fn scanned_entries(&self) -> usize {
        self.scanned_entries
    }

    pub(super) fn remaining(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }

    #[cfg(feature = "test-protected-index")]
    pub(super) fn has_entry_limit(&self) -> bool {
        self.max_entries.is_some()
    }
}
