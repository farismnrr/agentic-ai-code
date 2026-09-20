use super::index::ProtectedPathIndex;
pub(super) use super::publication::{
    fail_initialization, fail_initialization_permanent, fail_reconciliation,
    fail_reconciliation_permanent, is_permanent_index_error, mark_failed, publish_candidate,
    publish_reconciled,
};
use super::state::{InitializationRequest, InitializationWork, RootIndexController, RootState};
use super::traversal;
use super::watcher::IndexChange;
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

pub(super) fn root_indexes() -> &'static Mutex<HashMap<PathBuf, Arc<RootIndexController>>> {
    ROOT_INDEXES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cached_entry_count(controller: &RootIndexController) -> usize {
    match &*controller.state() {
        RootState::Ready { index, .. } | RootState::Reconciling { index, .. } => {
            index.cache_entry_count()
        }
        RootState::Cold { .. }
        | RootState::Initializing { .. }
        | RootState::NeedsFullScan { .. }
        | RootState::Failed { .. } => 0,
    }
}

pub(super) fn make_cache_room(
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

pub(super) fn initialize_inline(
    controller: &Arc<RootIndexController>,
    budget: Duration,
) -> io::Result<()> {
    let generation = {
        let _gate = controller.gate();
        let mut state = controller.state();
        match &*state {
            RootState::Ready { .. } => return Ok(()),
            RootState::Failed {
                error_kind, reason, ..
            } => return Err(io::Error::new(*error_kind, *reason)),
            RootState::Cold { .. } | RootState::NeedsFullScan { .. } => {}
            RootState::Initializing { .. } | RootState::Reconciling { .. } => {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "protected-path index work is already in progress",
                ));
            }
        }
        let generation = state.generation().saturating_add(1);
        *state = RootState::Initializing { generation };
        generation
    };

    let started = Instant::now();
    let mut scan_budget = IndexScanBudget::new(budget);
    let result = traversal::scan(&controller.root, &mut scan_budget)
        .and_then(|index| publish_candidate(controller, generation, index));
    if let Err(error) = &result {
        let permanent = is_permanent_index_error(error.kind());
        if permanent {
            fail_initialization_permanent(
                controller,
                generation,
                error.kind(),
                "bounded inline index initialization failed permanently",
            );
        } else {
            let retry_delay = if error.kind() == io::ErrorKind::Interrupted {
                CHURN_RETRY_DELAY
            } else {
                FAILED_INDEX_RETRY
            };
            fail_initialization(
                controller,
                generation,
                "bounded inline index initialization failed",
                retry_delay,
            );
        }
        tracing::warn!(
            event = "relay.sandbox.stage",
            stage = "protected_path_cache_prime",
            outcome = "failed",
            scope = "workspace_inline",
            error_kind = ?error.kind(),
            retryable = !permanent,
            duration_ms = started.elapsed().as_millis() as u64,
        );
    } else {
        tracing::info!(
            event = "relay.sandbox.stage",
            stage = "protected_path_cache_prime",
            outcome = "completed",
            scope = "workspace_inline",
            duration_ms = started.elapsed().as_millis() as u64,
        );
    }
    result
}

pub(super) fn start_initialization_locked(
    controller: &Arc<RootIndexController>,
    state: &mut RootState,
    budget: Duration,
) -> io::Result<()> {
    if let RootState::Failed {
        error_kind, reason, ..
    } = state
    {
        return Err(io::Error::new(*error_kind, *reason));
    }

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
    if let RootState::Failed {
        error_kind, reason, ..
    } = state
    {
        return Err(io::Error::new(*error_kind, *reason));
    }

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
    if matches!(state, RootState::Failed { .. }) {
        return;
    }

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
    if matches!(state, RootState::Failed { .. }) {
        return;
    }

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
                    if is_permanent_index_error(error.kind()) {
                        fail_reconciliation_permanent(
                            &request.controller,
                            request.generation,
                            error.kind(),
                            "subtree reconciliation failed permanently",
                        );
                    } else {
                        fail_reconciliation(
                            &request.controller,
                            request.generation,
                            "subtree reconciliation failed",
                        );
                    }
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
            let permanent = is_permanent_index_error(error.kind());
            if permanent {
                fail_initialization_permanent(
                    &request.controller,
                    request.generation,
                    error.kind(),
                    "bounded index work failed permanently",
                );
            } else {
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
            }
            tracing::warn!(
                event = "relay.sandbox.stage",
                stage = "protected_path_cache_prime",
                outcome = "failed",
                scope = "workspace_background",
                error_kind = ?error.kind(),
                retryable = !permanent,
                duration_ms = started.elapsed().as_millis() as u64,
            );
        }
    }
}
