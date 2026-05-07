//! Per-user concurrent SSE connection limiter. In-memory, per-process state.
//!
//! Each calendar `EventSource` holds three broadcast receivers and a tokio
//! task slot. Without a cap, a single authenticated user can open unlimited
//! connections and exhaust receiver/task budgets — a trivial `DoS`.
//!
//! `try_acquire(user_id)` returns `Some(guard)` when below the cap and
//! `None` when at the cap. The guard decrements the counter on `Drop`, so
//! callers just need to keep it alive for the connection lifetime.
//!
//! The state is process-local. With N actix workers each holding their own
//! `Arc<TrackerInner>` (cloned via `Clone`), the effective system cap is
//! `cap × N`. That bound is acceptable for `DoS` prevention; the audit (H-6)
//! called for "Redis or in-memory map" and we chose in-memory for simplicity.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SseConnectionTracker {
    inner: Arc<TrackerInner>,
}

struct TrackerInner {
    cap: usize,
    counts: Mutex<HashMap<i32, usize>>,
}

impl SseConnectionTracker {
    #[must_use]
    pub fn new(cap: usize) -> Self {
        Self {
            inner: Arc::new(TrackerInner {
                cap,
                counts: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// Try to register a new SSE connection for `user_id`. Returns a guard
    /// that decrements the counter on drop, or `None` if the user is at
    /// the cap.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned (i.e. another thread panicked
    /// while holding it). This is a programmer error in the tracker code; it
    /// cannot be triggered through normal API usage.
    #[must_use]
    pub fn try_acquire(&self, user_id: i32) -> Option<SseConnectionGuard> {
        let mut map = self
            .inner
            .counts
            .lock()
            .expect("sse tracker mutex poisoned");
        let count = map.entry(user_id).or_insert(0);
        if *count >= self.inner.cap {
            return None;
        }
        *count += 1;
        drop(map);
        Some(SseConnectionGuard {
            inner: Arc::clone(&self.inner),
            user_id,
        })
    }

    /// Test-only observability hook.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    #[cfg(test)]
    #[must_use]
    pub fn count_for(&self, user_id: i32) -> usize {
        self.inner
            .counts
            .lock()
            .expect("sse tracker mutex poisoned")
            .get(&user_id)
            .copied()
            .unwrap_or(0)
    }
}

pub struct SseConnectionGuard {
    inner: Arc<TrackerInner>,
    user_id: i32,
}

impl Drop for SseConnectionGuard {
    fn drop(&mut self) {
        let mut map = match self.inner.counts.lock() {
            Ok(g) => g,
            Err(e) => {
                // Mutex poisoned — best-effort; we can't recover, just leak the slot.
                log::error!("sse tracker mutex poisoned during release: {e}");
                return;
            }
        };
        if let Some(count) = map.get_mut(&self.user_id) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                map.remove(&self.user_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_succeeds_below_cap() {
        let tracker = SseConnectionTracker::new(2);
        let g1 = tracker.try_acquire(42);
        let g2 = tracker.try_acquire(42);
        assert!(g1.is_some());
        assert!(g2.is_some());
        assert_eq!(tracker.count_for(42), 2);
    }

    #[test]
    fn acquire_returns_none_at_cap() {
        let tracker = SseConnectionTracker::new(1);
        let _g1 = tracker.try_acquire(7).expect("first acquire");
        assert!(tracker.try_acquire(7).is_none());
        assert_eq!(tracker.count_for(7), 1);
    }

    #[test]
    fn drop_decrements_count() {
        let tracker = SseConnectionTracker::new(2);
        let g1 = tracker.try_acquire(99).expect("first");
        let g2 = tracker.try_acquire(99).expect("second");
        assert_eq!(tracker.count_for(99), 2);
        drop(g1);
        assert_eq!(tracker.count_for(99), 1);
        drop(g2);
        assert_eq!(tracker.count_for(99), 0);
    }

    #[test]
    fn cap_is_per_user() {
        let tracker = SseConnectionTracker::new(1);
        let _g1 = tracker.try_acquire(1).expect("user 1 first");
        let g2 = tracker.try_acquire(2);
        assert!(g2.is_some(), "user 2 should not be capped by user 1");
    }

    #[test]
    fn re_acquire_after_drop_succeeds() {
        let tracker = SseConnectionTracker::new(1);
        {
            let _g = tracker.try_acquire(5).expect("first");
        }
        assert_eq!(tracker.count_for(5), 0);
        assert!(tracker.try_acquire(5).is_some());
    }
}
