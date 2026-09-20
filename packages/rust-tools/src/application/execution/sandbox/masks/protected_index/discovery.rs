use super::controller::{
    controller_for_root, initialize_inline, invalidate_and_queue, is_permanent_index_error,
    mark_failed, mark_needs_full_scan, start_initialization_locked,
};
use super::state::{ProtectedPathFreshness, RootState};
use super::watcher::WatchChanges;
use super::{IndexScanBudget, WORKSPACE_INDEX_BUDGET};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const INLINE_RECONCILIATION_BUDGET: Duration = Duration::from_secs(2);
const INLINE_INITIALIZATION_RESERVE: Duration = Duration::from_millis(250);

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
    let freshness_deadline = control
        .and_then(|control| control.remaining())
        .map(|remaining| Instant::now() + remaining);
    let root = std::fs::canonicalize(root)?;
    let controller = controller_for_root(&root)?;
    let cold = {
        let state = controller.state();
        matches!(&*state, RootState::Cold { .. })
    };
    let mut initialized_inline = false;
    if cold {
        let inline_budget = control
            .and_then(|control| control.remaining())
            .map(|remaining| remaining.saturating_sub(INLINE_INITIALIZATION_RESERVE))
            .unwrap_or(WORKSPACE_INDEX_BUDGET)
            .min(WORKSPACE_INDEX_BUDGET);
        if inline_budget.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "protected-path index has no remaining initialization budget",
            ));
        }
        match initialize_inline(&controller, inline_budget) {
            Ok(()) => initialized_inline = true,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error),
        }
        if let Some(control) = control {
            control.check()?;
        }
    }
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
        | RootState::NeedsFullScan { .. }
        | RootState::Failed { .. } => None,
    };
    if let Some((index, snapshot)) = ready {
        if !index.watcher_enabled() {
            if initialized_inline {
                return Ok((
                    root,
                    ProtectedPathFreshness {
                        controller: controller.clone(),
                        snapshot,
                        deadline: freshness_deadline,
                    },
                    false,
                ));
            }
            let refresh_budget = control
                .and_then(|control| control.remaining())
                .map(|remaining| remaining.saturating_sub(INLINE_INITIALIZATION_RESERVE))
                .unwrap_or(WORKSPACE_INDEX_BUDGET)
                .min(WORKSPACE_INDEX_BUDGET);
            if refresh_budget.is_zero() {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "protected-path index has no remaining refresh budget",
                ));
            }
            let mut budget = IndexScanBudget::new(refresh_budget);
            let refreshed_index = Arc::new(super::traversal::scan(&root, &mut budget)?);
            let generation = state.generation().saturating_add(1);
            let refreshed = refreshed_index.snapshot(generation);
            let freshness = ProtectedPathFreshness {
                controller: controller.clone(),
                snapshot: refreshed.clone(),
                deadline: freshness_deadline,
            };
            *state = RootState::Ready {
                generation,
                index: refreshed_index,
                snapshot: refreshed,
            };
            if let Some(control) = control {
                control.check()?;
            }
            return Ok((root, freshness, false));
        }
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
                        deadline: freshness_deadline,
                    },
                    true,
                ));
            }
            Ok(WatchChanges::Changes(changes)) => {
                let generation = state.generation();
                let mut budget = IndexScanBudget::new(INLINE_RECONCILIATION_BUDGET);
                match super::traversal::reconcile(&root, &index, changes, &mut budget) {
                    Ok(()) => {
                        let refreshed = index.snapshot(generation);
                        let freshness = ProtectedPathFreshness {
                            controller: controller.clone(),
                            snapshot: refreshed.clone(),
                            deadline: freshness_deadline,
                        };
                        *state = RootState::Ready {
                            generation,
                            index,
                            snapshot: refreshed,
                        };
                        return Ok((root, freshness, true));
                    }
                    Err(error) => {
                        if is_permanent_index_error(error.kind()) {
                            mark_failed(
                                &mut state,
                                error.kind(),
                                "inline protected-path reconciliation failed permanently",
                            );
                        } else {
                            invalidate_and_queue(
                                &controller,
                                &mut state,
                                "inline protected-path reconciliation failed",
                            );
                        }
                        return Err(error);
                    }
                }
            }
            Ok(WatchChanges::FullScan(reason)) => {
                invalidate_and_queue(&controller, &mut state, reason);
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "protected-path watch state requires a full rebuild",
                ));
            }
            Err(error) => {
                if is_permanent_index_error(error.kind()) {
                    mark_failed(
                        &mut state,
                        error.kind(),
                        "watcher became permanently unavailable",
                    );
                } else {
                    invalidate_and_queue(&controller, &mut state, "watcher became unavailable");
                }
                return Err(error);
            }
        }
    }

    if matches!(&*state, RootState::Ready { .. }) {
        mark_needs_full_scan(&mut state, "index generation mismatch", Instant::now());
    }

    if let RootState::Failed {
        error_kind, reason, ..
    } = &*state
    {
        return Err(io::Error::new(*error_kind, *reason));
    }

    let should_queue = match &*state {
        RootState::Cold { .. } => true,
        RootState::NeedsFullScan { retry_after, .. } => Instant::now() >= *retry_after,
        RootState::Initializing { .. } | RootState::Reconciling { .. } => false,
        RootState::Ready { .. } | RootState::Failed { .. } => false,
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
        RootState::Failed { reason, .. } => reason,
    };
    Err(io::Error::new(
        io::ErrorKind::WouldBlock,
        format!("protected-path index is not ready: {reason}"),
    ))
}
