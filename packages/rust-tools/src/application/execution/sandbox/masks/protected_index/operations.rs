#[cfg(feature = "test-protected-index")]
use super::controller::{
    controller_for_root, fail_initialization, fail_initialization_permanent, publish_candidate,
    start_initialization_locked, start_reconciliation_locked,
};
use super::controller::{invalidate_and_queue, is_permanent_index_error, mark_failed};
use super::state::{
    PreSpawnFreshnessGuard, ProtectedMaskSnapshot, ProtectedPathFreshness, RootIndexController,
    RootState,
};
#[cfg(feature = "test-protected-index")]
use super::traversal::scan;
use super::watcher::WatchChanges;
use super::{IndexScanBudget, WORKSPACE_INDEX_BUDGET};
#[cfg(feature = "test-protected-index")]
use super::{CHURN_RETRY_DELAY, FAILED_INDEX_RETRY};
use std::io;
#[cfg(feature = "test-protected-index")]
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

const INLINE_RECONCILIATION_BUDGET: Duration = Duration::from_secs(2);
const INLINE_INITIALIZATION_RESERVE: Duration = Duration::from_millis(250);

#[cfg(feature = "test-protected-index")]
pub(in crate::application::execution::sandbox) fn schedule_initialization(
    root: &Path,
    budget: Duration,
) -> io::Result<()> {
    let root = std::fs::canonicalize(root)?;
    let controller = controller_for_root(&root)?;
    let _gate = controller.gate();
    let mut state = controller.state();

    if matches!(&*state, RootState::Failed { .. }) {
        return Ok(());
    }

    if matches!(
        &*state,
        RootState::Initializing { .. } | RootState::Reconciling { .. }
    ) {
        return Ok(());
    }
    if let RootState::NeedsFullScan { retry_after, .. } = &*state {
        if Instant::now() < *retry_after {
            return Ok(());
        }
    }
    if let RootState::Ready { index, .. } = &*state {
        let index = index.clone();
        match index.take_changes(&root) {
            Ok(WatchChanges::Clean) => return Ok(()),
            Ok(WatchChanges::Changes(changes)) => {
                return start_reconciliation_locked(&controller, &mut state, index, changes)
            }
            Ok(WatchChanges::FullScan(reason)) => {
                invalidate_and_queue(&controller, &mut state, reason);
                return Ok(());
            }
            Err(error) => {
                if is_permanent_index_error(error.kind()) {
                    mark_failed(
                        &mut state,
                        error.kind(),
                        "watcher became permanently unavailable while scheduling",
                    );
                } else {
                    invalidate_and_queue(
                        &controller,
                        &mut state,
                        "watcher became unavailable while scheduling",
                    );
                }
                return Err(error);
            }
        }
    }
    start_initialization_locked(&controller, &mut state, budget)
}

#[cfg(feature = "test-protected-index")]
pub(super) fn prime_with_entry_limit(
    root: &Path,
    budget: Duration,
    max_entries: usize,
) -> io::Result<usize> {
    prime_with_budget(root, IndexScanBudget::with_max_entries(budget, max_entries))
}

#[cfg(feature = "test-protected-index")]
fn prime_with_budget(root: &Path, mut scan_budget: IndexScanBudget) -> io::Result<usize> {
    let root = std::fs::canonicalize(root)?;
    let controller = controller_for_root(&root)?;
    let generation = {
        let _gate = controller.gate();
        let mut state = controller.state();

        if let RootState::Failed {
            error_kind, reason, ..
        } = &*state
        {
            return Err(io::Error::new(*error_kind, *reason));
        }

        if let RootState::Ready { index, .. } = &*state {
            let index = index.clone();
            match index.take_changes(&root) {
                Ok(WatchChanges::Clean) => return Ok(index.scanned_entries()),
                Ok(WatchChanges::Changes(_)) | Ok(WatchChanges::FullScan(_)) => {}
                Err(error) => {
                    if is_permanent_index_error(error.kind()) {
                        mark_failed(
                            &mut state,
                            error.kind(),
                            "watcher became permanently unavailable during priming",
                        );
                    } else {
                        invalidate_and_queue(
                            &controller,
                            &mut state,
                            "watcher became unavailable during priming",
                        );
                    }
                    return Err(error);
                }
            }
        }
        if matches!(
            &*state,
            RootState::Initializing { .. } | RootState::Reconciling { .. }
        ) {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "protected-path index initialization is already in progress",
            ));
        }
        if let RootState::NeedsFullScan { retry_after, .. } = &*state {
            if Instant::now() < *retry_after {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "protected-path index retry is waiting for its backoff",
                ));
            }
        }
        let generation = state.generation().saturating_add(1);
        *state = RootState::Initializing { generation };
        generation
    };

    match scan(&root, &mut scan_budget) {
        Ok(index) => {
            if let Err(error) = publish_candidate(&controller, generation, index) {
                fail_initialization(
                    &controller,
                    generation,
                    "synchronous publication failed",
                    CHURN_RETRY_DELAY,
                );
                return Err(error);
            }
            let state = controller.state();
            match &*state {
                RootState::Ready { index, .. } => Ok(index.scanned_entries()),
                _ => Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "protected-path index was invalidated during initialization",
                )),
            }
        }
        Err(error) => {
            if is_permanent_index_error(error.kind()) {
                fail_initialization_permanent(
                    &controller,
                    generation,
                    error.kind(),
                    "bounded synchronous scan failed permanently",
                );
            } else {
                let retry_delay = if error.kind() == io::ErrorKind::Interrupted {
                    CHURN_RETRY_DELAY
                } else {
                    FAILED_INDEX_RETRY
                };
                fail_initialization(
                    &controller,
                    generation,
                    "bounded synchronous scan failed",
                    retry_delay,
                );
            }
            Err(error)
        }
    }
}

pub(in crate::application::execution::sandbox) fn lock_and_validate_freshness<'a>(
    checks: &'a [ProtectedPathFreshness],
) -> io::Result<PreSpawnFreshnessGuard<'a>> {
    let mut ordered = checks.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.controller.root.cmp(&right.controller.root));
    ordered.dedup_by(|left, right| Arc::ptr_eq(&left.controller, &right.controller));

    let mut guards = Vec::with_capacity(ordered.len());
    for check in &ordered {
        guards.push(check.controller.gate());
    }
    for check in checks {
        check
            .controller
            .ensure_snapshot_fresh(&check.snapshot, check.deadline)?;
    }
    Ok(PreSpawnFreshnessGuard { _guards: guards })
}

impl RootIndexController {
    fn ensure_snapshot_fresh(
        self: &Arc<Self>,
        snapshot: &Arc<ProtectedMaskSnapshot>,
        deadline: Option<Instant>,
    ) -> io::Result<()> {
        let mut state = self.state();
        let current_index = match &*state {
            RootState::Ready {
                generation,
                index,
                snapshot: current,
            } if *generation == snapshot.generation && Arc::ptr_eq(current, snapshot) => {
                Some(index.clone())
            }
            _ => None,
        };
        let Some(index) = current_index else {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "protected-path snapshot is stale",
            ));
        };
        if !index.watcher_enabled() {
            let final_budget = deadline
                .map(|deadline| {
                    deadline
                        .saturating_duration_since(Instant::now())
                        .saturating_sub(INLINE_INITIALIZATION_RESERVE)
                })
                .unwrap_or(WORKSPACE_INDEX_BUDGET)
                .min(WORKSPACE_INDEX_BUDGET);
            if final_budget.is_zero() {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "protected-path final freshness check has no remaining budget",
                ));
            }
            let mut budget = IndexScanBudget::new(final_budget);
            let refreshed_index = Arc::new(super::traversal::scan(&self.root, &mut budget)?);
            let generation = state.generation().saturating_add(1);
            let refreshed = refreshed_index.snapshot(generation);
            let masks_unchanged = refreshed.protected_paths == snapshot.protected_paths;
            *state = RootState::Ready {
                generation,
                index: refreshed_index,
                snapshot: refreshed,
            };
            return if masks_unchanged {
                Ok(())
            } else {
                Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "protected-path masks changed before sandbox spawn",
                ))
            };
        }
        match index.take_changes(&self.root) {
            Ok(WatchChanges::Clean) => Ok(()),
            Ok(WatchChanges::Changes(changes)) => {
                let generation = state.generation();
                let mut budget = IndexScanBudget::new(INLINE_RECONCILIATION_BUDGET);
                match super::traversal::reconcile(&self.root, &index, changes, &mut budget) {
                    Ok(()) => {
                        let refreshed = index.snapshot(generation);
                        let masks_unchanged = refreshed.protected_paths == snapshot.protected_paths;
                        *state = RootState::Ready {
                            generation,
                            index,
                            snapshot: refreshed,
                        };
                        if masks_unchanged {
                            Ok(())
                        } else {
                            Err(io::Error::new(
                                io::ErrorKind::Interrupted,
                                "protected-path masks changed before sandbox spawn",
                            ))
                        }
                    }
                    Err(error) => {
                        if is_permanent_index_error(error.kind()) {
                            mark_failed(
                                &mut state,
                                error.kind(),
                                "inline protected-path reconciliation failed permanently before spawn",
                            );
                        } else {
                            invalidate_and_queue(
                                self,
                                &mut state,
                                "inline protected-path reconciliation failed before spawn",
                            );
                        }
                        Err(error)
                    }
                }
            }
            Ok(WatchChanges::FullScan(reason)) => {
                invalidate_and_queue(self, &mut state, reason);
                Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "workspace watch state became ambiguous before sandbox spawn",
                ))
            }
            Err(error) => {
                if is_permanent_index_error(error.kind()) {
                    mark_failed(
                        &mut state,
                        error.kind(),
                        "watcher failed permanently before spawn",
                    );
                } else {
                    invalidate_and_queue(self, &mut state, "watcher failed before spawn");
                }
                Err(error)
            }
        }
    }
}
