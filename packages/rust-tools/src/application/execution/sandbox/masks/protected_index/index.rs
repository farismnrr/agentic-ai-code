use super::state::ProtectedMaskSnapshot;
use super::watcher::{DirectoryRecord, DirectoryWatcher, WatchChanges};
use std::collections::{BTreeSet, HashMap};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub(super) struct ProtectedPathIndex {
    pub(super) inventory: Mutex<ProtectedPathInventory>,
}

pub(super) struct ProtectedPathInventory {
    pub(super) directories: HashMap<PathBuf, DirectoryRecord>,
    pub(super) protected_paths: BTreeSet<PathBuf>,
    pub(super) scanned_entries: usize,
    pub(super) watcher: DirectoryWatcher,
}

impl ProtectedPathIndex {
    pub(super) fn take_changes(&self, root: &Path) -> io::Result<WatchChanges> {
        let mut inventory = self
            .inventory
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        inventory.watcher.drain_changes(root)
    }

    pub(super) fn snapshot(&self, generation: u64) -> Arc<ProtectedMaskSnapshot> {
        let inventory = self
            .inventory
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        Arc::new(ProtectedMaskSnapshot {
            generation,
            protected_paths: Arc::from(
                inventory
                    .protected_paths
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>(),
            ),
            scanned_entries: inventory.scanned_entries,
        })
    }

    pub(super) fn scanned_entries(&self) -> usize {
        self.inventory
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .scanned_entries
    }

    pub(super) fn cache_entry_count(&self) -> usize {
        let inventory = self
            .inventory
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        inventory
            .directories
            .len()
            .saturating_add(inventory.protected_paths.len())
    }
}
