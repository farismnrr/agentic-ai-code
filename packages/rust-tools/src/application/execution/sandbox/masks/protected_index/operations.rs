use super::controller::{
    controller_for_root, fail_initialization, invalidate_and_queue, mark_needs_full_scan,
    publish_candidate, start_initialization_locked, start_reconciliation_locked,
};
use super::state::{
    PreSpawnFreshnessGuard, ProtectedMaskSnapshot, ProtectedPathFreshness, RootIndexController,
    RootState,
};
use super::traversal::scan;
use super::watcher::WatchChanges;
use super::{IndexScanBudget, CHURN_RETRY_DELAY, FAILED_INDEX_RETRY, WORKSPACE_INDEX_BUDGET};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(in crate::application::execution::sandbox) fn discover(
    root: &Path,
    control: Option<&super::super::super::SpawnControl<'_>>,
) -> io::Result<(PathBuf, ProtectedPathFreshness, bool)> {
    loop {
        match discover_once(root, control) {
            Err(error) if error.kind() == io::ErrorKind::WouldBlock && control.is_some() => {
                if let Some(control) = control {
                    control.check()?;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            result => return result,
        }
    }
}

fn discover_once(
    root: &Path,
    control: Option<&super::super::super::SpawnControl<'_>>,
) -> io::Result<(PathBuf, ProtectedPathFreshness, bool)> {
    if let Some(control) = control {
        control.check()?;
    }
    let root = std::fs::canonicalize(root)?;
    let controller = controller_for_root(&root)?;
    let _gate = controller.gate();
    let mut state = controller.state();

    let ready = match &*state {
        RootState::Ready {
            generation,
            index,
            snapshot,
        } if snapshot.generation == *generation => Some((index.clone(), snapshot.clone())),
        RootState::Ready { .. } => None,
        RootState::Cold { .. }
        | RootState::Initializing { .. }
        | RootState::Reconciling { .. }
        | RootState::NeedsFullScan { .. } => None,
    };
    if let Some((index, snapshot)) = ready {
        match index.take_changes(&root) {
            Ok(WatchChanges::Clean) => {
                if let Some(control) = control {
                    control.check()?;
                }
                return Ok((
                    root,
                    ProtectedPathFreshness {
                        controller: controller.clone(),
                        snapshot,
                    },
                    true,
                ));
            }
            Ok(WatchChanges::Changes(changes)) => {
                if let Err(error) =
                    start_reconciliation_locked(&controller, &mut state, index, changes)
                {
                    tracing::warn!(
                        event = "relay.sandbox.stage",
                        stage = "protected_path_reconciliation",
                        outcome = "queue_failed",
                        error_kind = ?error.kind(),
                    );
                }
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "protected-path index changed and is being reconciled",
                ));
            }
            Ok(WatchChanges::FullScan(reason)) => {
                invalidate_and_queue(&controller, &mut state, reason);
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "protected-path watch state requires a full rebuild",
                ));
            }
            Err(error) => {
                invalidate_and_queue(&controller, &mut state, "watcher became unavailable");
                return Err(error);
            }
        }
    }

    if matches!(&*state, RootState::Ready { .. }) {
        mark_needs_full_scan(&mut state, "index generation mismatch", Instant::now());
    }
    let should_queue = match &*state {
        RootState::Cold { .. } => true,
        RootState::NeedsFullScan { retry_after, .. } => Instant::now() >= *retry_after,
        RootState::Initializing { .. } | RootState::Reconciling { .. } => false,
        RootState::Ready { .. } => false,
    };
    if should_queue {
        if let Err(error) =
            start_initialization_locked(&controller, &mut state, WORKSPACE_INDEX_BUDGET)
        {
            tracing::warn!(
                event = "relay.sandbox.stage",
                stage = "protected_path_cache_prime",
                outcome = "queue_failed",
                error_kind = ?error.kind(),
            );
        }
    }
    let reason = match &*state {
        RootState::NeedsFullScan { reason, .. } => *reason,
        RootState::Ready { .. } => "index generation mismatch",
        RootState::Cold { .. } => "cold index",
        RootState::Initializing { .. } => "initialization in progress",
        RootState::Reconciling { .. } => "subtree reconciliation in progress",
    };
    Err(io::Error::new(
        io::ErrorKind::WouldBlock,
        format!("protected-path index is not ready: {reason}"),
    ))
}

pub(in crate::application::execution::sandbox) fn schedule_initialization(
    root: &Path,
    budget: Duration,
) -> io::Result<()> {
    let root = std::fs::canonicalize(root)?;
    let controller = controller_for_root(&root)?;
    let _gate = controller.gate();
    let mut state = controller.state();
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
        match index.take_changes(&root)? {
            WatchChanges::Clean => return Ok(()),
            WatchChanges::Changes(changes) => {
                return start_reconciliation_locked(&controller, &mut state, index, changes)
            }
            WatchChanges::FullScan(reason) => {
                invalidate_and_queue(&controller, &mut state, reason);
                return Ok(());
            }
        }
    }
    start_initialization_locked(&controller, &mut state, budget)
}

pub(in crate::application::execution::sandbox) fn prime(
    root: &Path,
    budget: Duration,
) -> io::Result<usize> {
    let root = std::fs::canonicalize(root)?;
    let controller = controller_for_root(&root)?;
    let generation = {
        let _gate = controller.gate();
        let mut state = controller.state();
        if let RootState::Ready { index, .. } = &*state {
            let index = index.clone();
            if matches!(index.take_changes(&root)?, WatchChanges::Clean) {
                return Ok(index.scanned_entries());
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
        let generation = state.generation().saturating_add(1);
        *state = RootState::Initializing { generation };
        generation
    };

    let mut scan_budget = IndexScanBudget::new(budget);
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
        check.controller.ensure_snapshot_fresh(&check.snapshot)?;
    }
    Ok(PreSpawnFreshnessGuard { _guards: guards })
}

impl RootIndexController {
    fn ensure_snapshot_fresh(
        self: &Arc<Self>,
        snapshot: &Arc<ProtectedMaskSnapshot>,
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
        match index.take_changes(&self.root) {
            Ok(WatchChanges::Clean) => Ok(()),
            Ok(WatchChanges::Changes(changes)) => {
                if let Err(error) = start_reconciliation_locked(self, &mut state, index, changes) {
                    tracing::warn!(
                        event = "relay.sandbox.stage",
                        stage = "protected_path_reconciliation",
                        outcome = "queue_failed",
                        error_kind = ?error.kind(),
                    );
                }
                Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "workspace changed before sandbox spawn",
                ))
            }
            Ok(WatchChanges::FullScan(reason)) => {
                invalidate_and_queue(self, &mut state, reason);
                Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "workspace watch state became ambiguous before sandbox spawn",
                ))
            }
            Err(error) => {
                invalidate_and_queue(self, &mut state, "watcher failed before spawn");
                Err(error)
            }
        }
    }
}
