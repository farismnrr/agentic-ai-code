use super::super::toolchain;
use super::{masks, runtime_home, safe_path_entries};
use crate::core::config::ServerConfig;
use std::path::Path;
#[cfg(feature = "test-protected-index")]
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

const PROTECTED_INDEX_INITIALIZATION_BUDGET: Duration = Duration::from_secs(60);

pub(crate) fn prime_protected_path_indexes(config: &ServerConfig) {
    #[cfg(feature = "test-protected-index")]
    wait_for_test_prime_gate();

    let _ = config.ensure_workspaces_initialized();
    let workspace_roots = config
        .workspaces
        .read()
        .map(|workspaces| workspaces.all_roots())
        .unwrap_or_default();
    // Workspace roots can represent a broad Projects tree and are therefore
    // intentionally demand-driven from the exact sandbox root selected at
    // execution time. Toolchain roots stay on the bounded startup path because
    // they are expected to be small.
    let mut roots = std::collections::BTreeSet::new();
    let home = runtime_home().ok();
    let canonical_home_cargo_bin = home
        .as_ref()
        .and_then(|home| std::fs::canonicalize(home.join(".cargo/bin")).ok());
    let mut toolchain_roots = std::collections::BTreeSet::new();

    for configured in &config.toolchain_paths {
        if let Ok(canonical) = std::fs::canonicalize(configured) {
            toolchain_roots.insert(canonical.clone());
            if let Some(root) = toolchain::reviewed_root(&canonical) {
                toolchain_roots.insert(root.to_path_buf());
            }
            if canonical_home_cargo_bin.as_ref() == Some(&canonical) {
                if let Some(home) = &home {
                    for subdirectory in [".cargo", ".rustup"] {
                        let candidate = home.join(subdirectory);
                        if candidate.is_dir() {
                            toolchain_roots.insert(candidate);
                        }
                    }
                }
            }
        }
    }
    for discovered in safe_path_entries(config) {
        let Ok(canonical) = std::fs::canonicalize(&discovered) else {
            continue;
        };
        if canonical.starts_with(Path::new("/usr"))
            || canonical.starts_with(Path::new("/bin"))
            || canonical.starts_with(Path::new("/sbin"))
            || canonical.starts_with(Path::new("/lib"))
            || canonical.starts_with(Path::new("/opt"))
            || workspace_roots
                .iter()
                .any(|workspace| canonical.starts_with(workspace))
        {
            continue;
        }
        toolchain_roots.insert(canonical);
    }
    for root in &toolchain_roots {
        if workspace_roots
            .iter()
            .any(|workspace| root.starts_with(workspace))
        {
            continue;
        }
        if !toolchain_roots
            .iter()
            .any(|other| other != root && root.starts_with(other))
        {
            roots.insert(root.clone());
        }
    }

    tracing::info!(
        event = "relay.sandbox.stage",
        stage = "protected_path_cache_prime",
        workspace_root_count = workspace_roots.len(),
        toolchain_root_count = roots.len(),
        initialization_budget_ms = PROTECTED_INDEX_INITIALIZATION_BUDGET.as_millis() as u64,
    );

    let initialization_deadline = Instant::now() + PROTECTED_INDEX_INITIALIZATION_BUDGET;
    for root in roots {
        let started = Instant::now();
        let budget = initialization_deadline.saturating_duration_since(Instant::now());
        match masks::prime_protected_path_index(&root, budget) {
            Ok(scanned_entries) => tracing::info!(
                event = "relay.sandbox.stage",
                stage = "protected_path_cache_prime",
                outcome = "completed",
                scope = "toolchain",
                scanned_entries,
                duration_ms = started.elapsed().as_millis() as u64,
            ),
            Err(error) => tracing::warn!(
                event = "relay.sandbox.stage",
                stage = "protected_path_cache_prime",
                outcome = "failed",
                scope = "toolchain",
                error_kind = ?error.kind(),
                duration_ms = started.elapsed().as_millis() as u64,
            ),
        }
    }
}

#[cfg(feature = "test-protected-index")]
struct TestPrimeGate {
    state: Mutex<TestPrimeGateState>,
    changed: Condvar,
}

#[cfg(feature = "test-protected-index")]
#[derive(Default)]
struct TestPrimeGateState {
    entered: bool,
    released: bool,
}

#[cfg(feature = "test-protected-index")]
fn test_prime_gate_slot() -> &'static Mutex<Option<Arc<TestPrimeGate>>> {
    static SLOT: OnceLock<Mutex<Option<Arc<TestPrimeGate>>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

#[cfg(feature = "test-protected-index")]
pub(crate) fn install_test_prime_gate() {
    let gate = Arc::new(TestPrimeGate {
        state: Mutex::new(TestPrimeGateState::default()),
        changed: Condvar::new(),
    });
    *test_prime_gate_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(gate);
}

#[cfg(feature = "test-protected-index")]
pub(crate) fn test_prime_gate_entered(timeout: Duration) -> bool {
    let gate = test_prime_gate_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    let Some(gate) = gate else {
        return false;
    };
    let deadline = Instant::now() + timeout;
    let mut state = gate.state.lock().unwrap_or_else(|error| error.into_inner());
    while !state.entered {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return false;
        }
        let (next, result) = gate
            .changed
            .wait_timeout(state, remaining)
            .unwrap_or_else(|error| error.into_inner());
        state = next;
        if result.timed_out() && !state.entered {
            return false;
        }
    }
    true
}

#[cfg(feature = "test-protected-index")]
pub(crate) fn release_test_prime_gate() {
    let gate = test_prime_gate_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .take();
    if let Some(gate) = gate {
        let mut state = gate.state.lock().unwrap_or_else(|error| error.into_inner());
        state.released = true;
        gate.changed.notify_all();
    }
}

#[cfg(feature = "test-protected-index")]
fn wait_for_test_prime_gate() {
    let gate = test_prime_gate_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    let Some(gate) = gate else {
        return;
    };
    let mut state = gate.state.lock().unwrap_or_else(|error| error.into_inner());
    state.entered = true;
    gate.changed.notify_all();
    while !state.released {
        state = gate
            .changed
            .wait(state)
            .unwrap_or_else(|error| error.into_inner());
    }
}
