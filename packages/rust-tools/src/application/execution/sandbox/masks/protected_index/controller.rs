use super::index::ProtectedPathIndex;
use super::state::{InitializationRequest, InitializationWork, RootIndexController, RootState};
use super::traversal;
use super::watcher::{IndexChange, WatchChanges};
use super::{
    IndexScanBudget, CHURN_RETRY_DELAY, FAILED_INDEX_RETRY, MAX_CACHED_INDEX_ENTRIES,
    MAX_CACHE_ROOTS, MAX_QUEUED_INITIALIZATIONS, WORKSPACE_INDEX_BUDGET,
};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

static ROOT_INDEXES: OnceLock<Mutex<HashMap<PathBuf, Arc<RootIndexController>>>> = OnceLock::new();
static INITIALIZATION_QUEUE: OnceLock<Result<mpsc::SyncSender<InitializationRequest>, String>> =
    OnceLock::new();

fn root_indexes() -> &'static Mutex<HashMap<PathBuf, Arc<RootIndexController>>> {
    ROOT_INDEXES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cached_entry_count(controller: &RootIndexController) -> usize {
    match &*controller.state() {
        RootState::Ready { index, .. } | RootState::Reconciling { index, .. } => {
            index.cache_entry_count()
        }
        RootState::Cold { .. }
        | RootState::Initializing { .. }
        | RootState::NeedsFullScan { .. } => 0,
    }
}

fn make_cache_room(
    entries: &mut HashMap<PathBuf, Arc<RootIndexController>>,
    current_root: &Path,
    new_entry_count: usize,
) -> io::Result<()> {
    loop {
        let cached_entry_count = entries
            .values()
            .map(|controller| cached_entry_count(controller))
            .sum::<usize>();
        if entries.len() < MAX_CACHE_ROOTS
            && cached_entry_count.saturating_add(new_entry_count) <= MAX_CACHED_INDEX_ENTRIES
        {
            return Ok(());
        }
        let evict = entries.iter().find_map(|(root, controller)| {
            (root.as_path() != current_root && Arc::strong_count(controller) == 1)
                .then(|| root.clone())
        });
        let Some(evict) = evict else {
            // Preserve active controllers and fail closed rather than evicting
            // a generation a running job still owns.
            if entries.contains_key(current_root)
                && entries.len() <= MAX_CACHE_ROOTS
                && cached_entry_count.saturating_add(new_entry_count) <= MAX_CACHED_INDEX_ENTRIES
            {
                return Ok(());
            }
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "protected-path index cache is at capacity",
            ));
        };
        entries.remove(&evict);
    }
}

pub(super) fn controller_for_root(root: &Path) -> io::Result<Arc<RootIndexController>> {
    let mut entries = root_indexes()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if let Some(controller) = entries.get(root) {
        return Ok(controller.clone());
    }
    make_cache_room(&mut entries, root, 0)?;
    let controller = Arc::new(RootIndexController::new(root.to_path_buf()));
    entries.insert(root.to_path_buf(), controller.clone());
    Ok(controller)
}

fn initialization_sender() -> io::Result<&'static mpsc::SyncSender<InitializationRequest>> {
    let initialized = INITIALIZATION_QUEUE.get_or_init(|| {
        let (sender, receiver) = mpsc::sync_channel(MAX_QUEUED_INITIALIZATIONS);
        thread::Builder::new()
            .name("protected-path-indexer".into())
            .spawn(move || {
                while let Ok(request) = receiver.recv() {
                    run_initialization(request);
                }
            })
            .map_err(|error| format!("failed to start protected-path index worker: {error}"))?;
        Ok(sender)
    });
    initialized
        .as_ref()
        .map_err(|message| io::Error::other(message.clone()))
}

pub(super) fn start_initialization_locked(
    controller: &Arc<RootIndexController>,
    state: &mut RootState,
    budget: Duration,
) -> io::Result<()> {
    let generation = state.generation().saturating_add(1);
    *state = RootState::Initializing { generation };
    let request = InitializationRequest {
        controller: controller.clone(),
        generation,
        budget,
        work: InitializationWork::FullScan,
    };
    let result = initialization_sender().and_then(|sender| {
        sender.try_send(request).map_err(|error| {
            let message = match error {
                mpsc::TrySendError::Full(_) => "protected-path index queue is full",
                mpsc::TrySendError::Disconnected(_) => "protected-path index worker stopped",
            };
            io::Error::new(io::ErrorKind::WouldBlock, message)
        })
    });
    if let Err(error) = result {
        *state = RootState::NeedsFullScan {
            generation: generation.saturating_add(1),
            retry_after: Instant::now() + FAILED_INDEX_RETRY,
            reason: "initialization could not be queued",
        };
        return Err(error);
    }
    Ok(())
}

pub(super) fn start_reconciliation_locked(
    controller: &Arc<RootIndexController>,
    state: &mut RootState,
    index: Arc<ProtectedPathIndex>,
    changes: Vec<IndexChange>,
) -> io::Result<()> {
    let generation = state.generation().saturating_add(1);
    *state = RootState::Reconciling {
        generation,
        index: index.clone(),
    };
    let request = InitializationRequest {
        controller: controller.clone(),
        generation,
        budget: WORKSPACE_INDEX_BUDGET,
        work: InitializationWork::Reconcile { index, changes },
    };
    let result = initialization_sender().and_then(|sender| {
        sender.try_send(request).map_err(|error| {
            let message = match error {
                mpsc::TrySendError::Full(_) => "protected-path index queue is full",
                mpsc::TrySendError::Disconnected(_) => "protected-path index worker stopped",
            };
            io::Error::new(io::ErrorKind::WouldBlock, message)
        })
    });
    if let Err(error) = result {
        *state = RootState::NeedsFullScan {
            generation: generation.saturating_add(1),
            retry_after: Instant::now() + FAILED_INDEX_RETRY,
            reason: "subtree reconciliation could not be queued",
        };
        return Err(error);
    }
    Ok(())
}

pub(super) fn mark_needs_full_scan(
    state: &mut RootState,
    reason: &'static str,
    retry_after: Instant,
) {
    let generation = state.generation().saturating_add(1);
    *state = RootState::NeedsFullScan {
        generation,
        retry_after,
        reason,
    };
}

pub(super) fn invalidate_and_queue(
    controller: &Arc<RootIndexController>,
    state: &mut RootState,
    reason: &'static str,
) {
    mark_needs_full_scan(state, reason, Instant::now());
    if let Err(error) = start_initialization_locked(controller, state, WORKSPACE_INDEX_BUDGET) {
        tracing::warn!(
            event = "relay.sandbox.stage",
            stage = "protected_path_cache_prime",
            outcome = "queue_failed",
            error_kind = ?error.kind(),
        );
    }
}

fn run_initialization(request: InitializationRequest) {
    let started = Instant::now();
    let mut budget = IndexScanBudget::new(request.budget);
    let result = match request.work {
        InitializationWork::FullScan => traversal::scan(&request.controller.root, &mut budget)
            .and_then(|index| publish_candidate(&request.controller, request.generation, index)),
        InitializationWork::Reconcile { index, changes } => {
            match traversal::reconcile(&request.controller.root, &index, changes, &mut budget) {
                Ok(()) => publish_reconciled(&request.controller, request.generation, index),
                Err(error) => {
                    fail_reconciliation(
                        &request.controller,
                        request.generation,
                        "subtree reconciliation failed",
                    );
                    Err(error)
                }
            }
        }
    };
    match result {
        Ok(()) => tracing::info!(
            event = "relay.sandbox.stage",
            stage = "protected_path_cache_prime",
            outcome = "completed",
            scope = "workspace_background",
            duration_ms = started.elapsed().as_millis() as u64,
        ),
        Err(error) => {
            let retry_delay = if error.kind() == io::ErrorKind::Interrupted {
                CHURN_RETRY_DELAY
            } else {
                FAILED_INDEX_RETRY
            };
            fail_initialization(
                &request.controller,
                request.generation,
                "bounded index work failed",
                retry_delay,
            );
            tracing::warn!(
                event = "relay.sandbox.stage",
                stage = "protected_path_cache_prime",
                outcome = "failed",
                scope = "workspace_background",
                error_kind = ?error.kind(),
                duration_ms = started.elapsed().as_millis() as u64,
            );
        }
    }
}

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
            mark_needs_full_scan(
                &mut state,
                "watcher failed before publication",
                Instant::now() + FAILED_INDEX_RETRY,
            );
            Err(error)
        }
    }
}

fn publish_reconciled(
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
