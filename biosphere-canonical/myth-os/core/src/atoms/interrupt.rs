// CORE-ATOM-04: Interrupt Sequencer — hardware/software signal prioritization
use mythos::signal::{BusSignal, SignalPriority};
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::sync::{Arc, Mutex};

struct PrioritizedSignal(BusSignal);

impl PartialEq for PrioritizedSignal {
    fn eq(&self, other: &Self) -> bool {
        self.0.priority == other.0.priority
    }
}
impl Eq for PrioritizedSignal {}
impl PartialOrd for PrioritizedSignal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for PrioritizedSignal {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.priority.cmp(&other.0.priority)
    }
}

#[derive(Default)]
pub struct InterruptSequencer {
    queue: Arc<Mutex<BinaryHeap<PrioritizedSignal>>>,
}

impl InterruptSequencer {
    pub fn push(&self, signal: BusSignal) {
        self.queue.lock().unwrap().push(PrioritizedSignal(signal));
    }

    pub fn pop(&self) -> Option<BusSignal> {
        self.queue.lock().unwrap().pop().map(|p| p.0)
    }

    pub fn len(&self) -> usize {
        self.queue.lock().unwrap().len()
    }
}
