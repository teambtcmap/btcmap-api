use std::sync::atomic::{AtomicBool, Ordering};

/// Process-wide guard that allows only one run of a given background sync at
/// a time.
///
/// The systemd timers fire on a fixed schedule and don't wait for a previous
/// `oneshot` run to finish. A slow upstream used to let runs pile up, each
/// holding DB connections and actix worker time. The flag is released on drop,
/// including on early return.
pub struct SyncLock {
    in_progress: &'static AtomicBool,
}

impl SyncLock {
    /// Returns `Some` if the lock was acquired, `None` if another run holds it.
    pub fn try_acquire(in_progress: &'static AtomicBool) -> Option<Self> {
        in_progress
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Self { in_progress })
    }
}

impl Drop for SyncLock {
    fn drop(&mut self) {
        self.in_progress.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod test {
    use super::SyncLock;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn rejects_concurrent_acquire_and_releases_on_drop() {
        static LOCK: AtomicBool = AtomicBool::new(false);
        let first = SyncLock::try_acquire(&LOCK).expect("first acquire should succeed");
        assert!(
            SyncLock::try_acquire(&LOCK).is_none(),
            "a second acquire must be rejected while the first is held"
        );
        drop(first);
        assert!(
            SyncLock::try_acquire(&LOCK).is_some(),
            "acquire must succeed again once the previous guard is dropped"
        );
    }
}
