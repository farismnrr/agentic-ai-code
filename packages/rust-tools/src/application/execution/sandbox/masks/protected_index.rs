//! Bounded, fail-closed protected-path indexing for mounted workspace trees.
use std::io;
use std::time::{Duration, Instant};

mod controller;
mod index;
mod operations;
mod state;
mod traversal;
mod watcher;

pub(super) use operations::{
    discover, lock_and_validate_freshness, prime, schedule_initialization,
};
pub(in crate::application::execution::sandbox) use state::{
    PreSpawnFreshnessGuard, ProtectedPathFreshness,
};

pub(super) const MAX_PROTECTED_SCAN_ENTRIES: usize = 500_000;
const CONTROL_CHECK_INTERVAL: usize = 64;
const MAX_CACHE_ROOTS: usize = 64;
const MAX_CACHED_INDEX_ENTRIES: usize = 1_000_000;
const MAX_QUEUED_INITIALIZATIONS: usize = MAX_CACHE_ROOTS;
// Keep failures fail-closed while retrying soon enough to recover after a
// transient permission or mount issue is repaired.
const FAILED_INDEX_RETRY: Duration = Duration::from_secs(1);
const CHURN_RETRY_DELAY: Duration = Duration::from_millis(250);
const WORKSPACE_INDEX_BUDGET: Duration = Duration::from_secs(60);

/// An index lifecycle budget is independent of any one terminal command's deadline.
pub(super) struct IndexScanBudget {
    deadline: Instant,
    max_entries: usize,
    scanned_entries: usize,
}

impl IndexScanBudget {
    pub(super) fn new(timeout: Duration) -> Self {
        Self {
            deadline: Instant::now() + timeout,
            max_entries: MAX_PROTECTED_SCAN_ENTRIES,
            scanned_entries: 0,
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
        if self.scanned_entries > self.max_entries {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "protected-path scan exceeds bounded workspace maximum",
            ));
        }
        if self.scanned_entries.is_multiple_of(CONTROL_CHECK_INTERVAL) {
            self.check()?;
        }
        Ok(self.scanned_entries)
    }

    pub(super) fn scanned_entries(&self) -> usize {
        self.scanned_entries
    }
}
