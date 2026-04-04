use std::sync::Mutex;

/// Aggregates agent events and generates briefings with priority ordering:
/// drift first, then upstream dependency changes, then completed before running.
pub struct EventAggregator {
    /// Pending events not yet consumed into a briefing
    pending_count: Mutex<u32>,
}

impl EventAggregator {
    pub fn new() -> Self {
        Self {
            pending_count: Mutex::new(0),
        }
    }

    /// Record that a new event arrived.
    pub fn notify_event(&self) {
        let mut count = self.pending_count.lock().expect("mutex poisoned");
        *count += 1;
    }

    /// Get the number of pending events (for badge display).
    pub fn pending_count(&self) -> u32 {
        *self.pending_count.lock().expect("mutex poisoned")
    }

    /// Reset pending count (after briefing is generated).
    pub fn clear_pending(&self) {
        let mut count = self.pending_count.lock().expect("mutex poisoned");
        *count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_aggregator_counting() {
        let agg = EventAggregator::new();
        assert_eq!(agg.pending_count(), 0);
        agg.notify_event();
        agg.notify_event();
        assert_eq!(agg.pending_count(), 2);
        agg.clear_pending();
        assert_eq!(agg.pending_count(), 0);
    }
}
