//! Execution-id-keyed response retention (PLAN.md): full downloaded bytes are
//! kept so "save response to file" works after the (truncated) IPC copy has
//! been displayed. FIFO budget: max 50 MB total AND max 10 entries — insertion
//! and eviction happen under one lock so concurrent completions can never
//! exceed either budget. No tab ids here — the frontend owns tab cleanup.

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, PoisonError};

pub const MAX_TOTAL_BYTES: usize = 50 * 1024 * 1024;
pub const MAX_ENTRIES: usize = 10;

#[derive(Default)]
struct Inner {
    entries: HashMap<String, Vec<u8>>,
    /// Insertion order, oldest first.
    order: VecDeque<String>,
    total_bytes: usize,
}

#[derive(Default)]
pub struct Retention {
    inner: Mutex<Inner>,
}

impl Retention {
    pub fn new() -> Self {
        Self::default()
    }

    /// Retains `bytes` for `execution_id`, evicting oldest entries as needed
    /// to stay within both budgets. Re-inserting an existing id replaces it.
    pub fn insert(&self, execution_id: &str, bytes: Vec<u8>) {
        let mut inner = self.lock();
        if let Some(old) = inner.entries.remove(execution_id) {
            inner.total_bytes -= old.len();
            inner.order.retain(|id| id.as_str() != execution_id);
        }
        inner.total_bytes += bytes.len();
        inner.entries.insert(execution_id.to_string(), bytes);
        inner.order.push_back(execution_id.to_string());
        // Evict oldest-first until within budget. A single entry larger than
        // the byte budget is allowed to remain (it is the newest response and
        // the whole point of retention).
        while (inner.order.len() > MAX_ENTRIES || inner.total_bytes > MAX_TOTAL_BYTES)
            && inner.order.len() > 1
        {
            if let Some(oldest) = inner.order.pop_front() {
                if let Some(evicted) = inner.entries.remove(&oldest) {
                    inner.total_bytes -= evicted.len();
                }
            }
        }
    }

    /// Idempotent and silent — stale-completion cleanup, FIFO eviction, and
    /// tab cleanup may all release the same execution more than once.
    pub fn release(&self, execution_id: &str) {
        let mut inner = self.lock();
        if let Some(removed) = inner.entries.remove(execution_id) {
            inner.total_bytes -= removed.len();
            inner.order.retain(|id| id.as_str() != execution_id);
        }
    }

    pub fn get(&self, execution_id: &str) -> Option<Vec<u8>> {
        self.lock().entries.get(execution_id).cloned()
    }

    pub fn contains(&self, execution_id: &str) -> bool {
        self.lock().entries.contains_key(execution_id)
    }

    pub fn len(&self) -> usize {
        self.lock().order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn total_bytes(&self) -> usize {
        self.lock().total_bytes
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        // A panic while holding the lock cannot leave retention in a state
        // worse than the panic itself; recover instead of propagating poison.
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
