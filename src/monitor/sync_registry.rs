use std::collections::HashSet;

use libspa::utils::result::AsyncSeq;
use pipewire::core::Core;

/// Track pending syncs in order to determine when the monitor has all initial
/// information and is waiting for new events.
#[derive(Default)]
pub struct SyncRegistry {
    pending: HashSet<i32>,
    done: bool,
}

impl SyncRegistry {
    /// Register a pending sync.
    pub fn global(&mut self, core: &Core) {
        if !self.done {
            if let Ok(seq) = core.sync(0) {
                self.pending.insert(seq.seq());
            }
        }
    }

    /// Mark a sync as done, return true when all are done for the first time.
    pub fn done(&mut self, seq: AsyncSeq) -> bool {
        if self.done {
            return false;
        }

        self.pending.remove(&seq.seq());
        self.done |= self.pending.is_empty();
        self.pending.is_empty()
    }
}
