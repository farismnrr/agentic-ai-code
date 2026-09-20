use super::index::ProtectedPathIndex;
use super::watcher::IndexChange;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub(super) struct ProtectedMaskSnapshot {
    pub(super) generation: u64,
    pub(super) protected_paths: Arc<[PathBuf]>,
    pub(super) scanned_entries: usize,
}

pub(super) enum RootState {
    Cold {
        generation: u64,
    },
    Initializing {
        generation: u64,
    },
    Ready {
        generation: u64,
        index: Arc<ProtectedPathIndex>,
        snapshot: Arc<ProtectedMaskSnapshot>,
    },
    Reconciling {
        generation: u64,
        index: Arc<ProtectedPathIndex>,
    },
    NeedsFullScan {
        generation: u64,
        retry_after: Instant,
        reason: &'static str,
    },
    Failed {
        generation: u64,
        error_kind: io::ErrorKind,
        reason: &'static str,
    },
}

impl RootState {
    pub(super) fn generation(&self) -> u64 {
        match self {
            Self::Cold { generation }
            | Self::Initializing { generation }
            | Self::Ready { generation, .. }
            | Self::Reconciling { generation, .. }
            | Self::NeedsFullScan { generation, .. }
            | Self::Failed { generation, .. } => *generation,
        }
    }
}

pub(super) struct RootIndexController {
    pub(super) root: PathBuf,
    state: Mutex<RootState>,
    // Acquired before state transitions, event draining, and Command::spawn.
    pre_spawn_gate: Mutex<()>,
}

impl RootIndexController {
    pub(super) fn new(root: PathBuf) -> Self {
        Self {
            root,
            state: Mutex::new(RootState::Cold { generation: 0 }),
            pre_spawn_gate: Mutex::new(()),
        }
    }

    pub(super) fn state(&self) -> MutexGuard<'_, RootState> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }

    pub(super) fn gate(&self) -> MutexGuard<'_, ()> {
        self.pre_spawn_gate
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }
}

pub(in crate::application::execution::sandbox) struct ProtectedPathFreshness {
    pub(super) controller: Arc<RootIndexController>,
    pub(super) snapshot: Arc<ProtectedMaskSnapshot>,
    pub(super) deadline: Option<Instant>,
}

impl ProtectedPathFreshness {
    pub(in crate::application::execution::sandbox) fn protected_paths(&self) -> &[PathBuf] {
        &self.snapshot.protected_paths
    }

    pub(in crate::application::execution::sandbox) fn scanned_entries(&self) -> usize {
        self.snapshot.scanned_entries
    }

    pub(in crate::application::execution::sandbox) fn watcher_enabled(&self) -> bool {
        let state = self.controller.state();
        match &*state {
            RootState::Ready { index, .. } => index.watcher_enabled(),
            _ => false,
        }
    }
}

pub(in crate::application::execution::sandbox) struct PreSpawnFreshnessGuard<'a> {
    pub(super) _guards: Vec<MutexGuard<'a, ()>>,
}

pub(super) enum InitializationWork {
    FullScan,
    Reconcile {
        index: Arc<ProtectedPathIndex>,
        changes: Vec<IndexChange>,
    },
}

pub(super) struct InitializationRequest {
    pub(super) controller: Arc<RootIndexController>,
    pub(super) generation: u64,
    pub(super) budget: Duration,
    pub(super) work: InitializationWork,
}
