use super::controller::{
    make_cache_room, mark_needs_full_scan, root_indexes, start_initialization_locked,
    start_reconciliation_locked,
};
use super::index::ProtectedPathIndex;
use super::state::{RootIndexController, RootState};
use super::watcher::WatchChanges;
use super::{CHURN_RETRY_DELAY, FAILED_INDEX_RETRY, WORKSPACE_INDEX_BUDGET};
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(super) fn publish_candidate(
    controller: &Arc<RootIndexController>,
    generation: u64,
    index: ProtectedPathIndex,
) -> io::Result<()> {
    {
        let mut entries = root_indexes()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        make_cache_room(&mut entries, &controller.root, index.cache_entry_count())?;
    }

    let index = Arc::new(index);
    let snapshot = index.snapshot(generation);

    let _gate = controller.gate();
    let mut state = controller.state();
    if !matches!(&*state, RootState::Initializing { generation: current } if *current == generation)
    {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "protected-path index initialization was superseded",
        ));
    }
    if !index.watcher_enabled() {
        *state = RootState::Ready {
            generation,
            index,
            snapshot,
        };
        return Ok(());
    }
    match index.take_changes(&controller.root) {
        Ok(WatchChanges::Clean) => {
            *state = RootState::Ready {
                generation,
                index,
                snapshot,
            };
            Ok(())
        }
        Ok(WatchChanges::Changes(_)) | Ok(WatchChanges::FullScan(_)) => {
            mark_needs_full_scan(
                &mut state,
                "filesystem changed before publication",
                Instant::now() + CHURN_RETRY_DELAY,
            );
            Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "workspace changed before protected-path index publication",
            ))
        }
        Err(error) => {
            if is_permanent_index_error(error.kind()) {
                mark_failed(
                    &mut state,
                    error.kind(),
                    "watcher failed permanently before publication",
                );
            } else {
                mark_needs_full_scan(
                    &mut state,
                    "watcher failed before publication",
                    Instant::now() + FAILED_INDEX_RETRY,
                );
            }
            Err(error)
        }
    }
}

pub(super) fn publish_reconciled(
    controller: &Arc<RootIndexController>,
    generation: u64,
    index: Arc<ProtectedPathIndex>,
) -> io::Result<()> {
    let cache_result = {
        let mut entries = root_indexes()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        make_cache_room(&mut entries, &controller.root, 0)
    };
    if let Err(error) = cache_result {
        fail_reconciliation(
            controller,
            generation,
            "reconciled index exceeds the protected-path cache budget",
        );
        return Err(error);
    }

    let _gate = controller.gate();
    let mut state = controller.state();
    let same_reconciliation = matches!(
        &*state,
        RootState::Reconciling {
            generation: current,
            index: current_index,
        } if *current == generation && Arc::ptr_eq(current_index, &index)
    );
    if !same_reconciliation {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "subtree reconciliation was superseded",
        ));
    }
    let changes = match index.take_changes(&controller.root) {
        Ok(changes) => changes,
        Err(error) => {
            if is_permanent_index_error(error.kind()) {
                mark_failed(
                    &mut state,
                    error.kind(),
                    "watcher failed permanently during subtree publication",
                );
            } else {
                mark_needs_full_scan(
                    &mut state,
                    "watcher failed during subtree publication",
                    Instant::now(),
                );
                if let Err(queue_error) =
                    start_initialization_locked(controller, &mut state, WORKSPACE_INDEX_BUDGET)
                {
                    tracing::warn!(
                        event = "relay.sandbox.stage",
                        stage = "protected_path_reconciliation",
                        outcome = "full_scan_queue_failed",
                        error_kind = ?queue_error.kind(),
                    );
                }
            }
            return Err(error);
        }
    };
    match changes {
        WatchChanges::Clean => {
            let snapshot = index.snapshot(generation);
            *state = RootState::Ready {
                generation,
                index,
                snapshot,
            };
            Ok(())
        }
        WatchChanges::Changes(changes) => {
            if let Err(error) = start_reconciliation_locked(controller, &mut state, index, changes)
            {
                tracing::warn!(
                    event = "relay.sandbox.stage",
                    stage = "protected_path_reconciliation",
                    outcome = "queue_failed",
                    error_kind = ?error.kind(),
                );
                return Err(error);
            }
            Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "workspace changed during subtree reconciliation",
            ))
        }
        WatchChanges::FullScan(reason) => {
            mark_needs_full_scan(&mut state, reason, Instant::now());
            if let Err(error) =
                start_initialization_locked(controller, &mut state, WORKSPACE_INDEX_BUDGET)
            {
                tracing::warn!(
                    event = "relay.sandbox.stage",
                    stage = "protected_path_reconciliation",
                    outcome = "full_scan_queue_failed",
                    error_kind = ?error.kind(),
                );
                return Err(error);
            }
            Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "watch state became ambiguous during subtree reconciliation",
            ))
        }
    }
}

pub(super) fn fail_reconciliation_permanent(
    controller: &Arc<RootIndexController>,
    generation: u64,
    error_kind: io::ErrorKind,
    reason: &'static str,
) {
    let _gate = controller.gate();
    let mut state = controller.state();
    if matches!(&*state, RootState::Reconciling { generation: current, .. } if *current == generation)
    {
        mark_failed(&mut state, error_kind, reason);
    }
}

pub(super) fn fail_reconciliation(
    controller: &Arc<RootIndexController>,
    generation: u64,
    reason: &'static str,
) {
    let _gate = controller.gate();
    let mut state = controller.state();
    if matches!(&*state, RootState::Reconciling { generation: current, .. } if *current == generation)
    {
        mark_needs_full_scan(&mut state, reason, Instant::now());
        if let Err(error) =
            start_initialization_locked(controller, &mut state, WORKSPACE_INDEX_BUDGET)
        {
            tracing::warn!(
                event = "relay.sandbox.stage",
                stage = "protected_path_reconciliation",
                outcome = "full_scan_queue_failed",
                error_kind = ?error.kind(),
            );
        }
    }
}

pub(super) fn is_permanent_index_error(kind: io::ErrorKind) -> bool {
    matches!(
        kind,
        io::ErrorKind::InvalidData | io::ErrorKind::InvalidInput
    )
}

pub(super) fn mark_failed(state: &mut RootState, error_kind: io::ErrorKind, reason: &'static str) {
    if matches!(state, RootState::Failed { .. }) {
        return;
    }

    *state = RootState::Failed {
        generation: state.generation().saturating_add(1),
        error_kind,
        reason,
    };
}

pub(super) fn fail_initialization_permanent(
    controller: &Arc<RootIndexController>,
    generation: u64,
    error_kind: io::ErrorKind,
    reason: &'static str,
) {
    let _gate = controller.gate();
    let mut state = controller.state();
    if matches!(&*state, RootState::Initializing { generation: current } if *current == generation)
    {
        mark_failed(&mut state, error_kind, reason);
    }
}

pub(super) fn fail_initialization(
    controller: &Arc<RootIndexController>,
    generation: u64,
    reason: &'static str,
    retry_delay: Duration,
) {
    let _gate = controller.gate();
    let mut state = controller.state();
    if matches!(&*state, RootState::Initializing { generation: current } if *current == generation)
    {
        mark_needs_full_scan(&mut state, reason, Instant::now() + retry_delay);
    }
}
