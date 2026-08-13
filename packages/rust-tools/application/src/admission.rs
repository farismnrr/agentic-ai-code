use std::sync::Mutex;
use std::time::Instant;

const REQUEST_ADMISSION_BURST: f64 = 32.0;
const REQUEST_ADMISSION_RATE_PER_SEC: f64 = 8.0;

struct AdmissionState {
    tokens: f64,
    updated_at: Instant,
}

/// Coarse relay-side admission control. It is deliberately independent from
/// HTTP concurrency and the execution semaphore and fails closed if poisoned.
pub struct RequestAdmission {
    capacity: f64,
    refill_per_sec: f64,
    state: Mutex<AdmissionState>,
}

impl RequestAdmission {
    pub fn configured() -> Self {
        Self::new(REQUEST_ADMISSION_BURST, REQUEST_ADMISSION_RATE_PER_SEC)
    }

    fn new(capacity: f64, refill_per_sec: f64) -> Self {
        Self {
            capacity,
            refill_per_sec,
            state: Mutex::new(AdmissionState {
                tokens: capacity,
                updated_at: Instant::now(),
            }),
        }
    }

    pub fn try_acquire(&self, now: Instant) -> bool {
        let Ok(mut state) = self.state.lock() else {
            return false;
        };
        let elapsed = now
            .saturating_duration_since(state.updated_at)
            .as_secs_f64();
        state.tokens = (state.tokens + elapsed * self.refill_per_sec).min(self.capacity);
        state.updated_at = now;
        if state.tokens < 1.0 {
            return false;
        }
        state.tokens -= 1.0;
        true
    }
}
