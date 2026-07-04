// CORE-ATOM-04: Interrupt Sequencer — priority-ordered signal queue.
//
// Critical signals are pulled off the bus and placed here for immediate
// dispatch before lower-priority signals are processed. The queue is a
// max-heap ordered by SignalPriority — Critical signals always surface first.

use crate::signal::BusSignal;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
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
    /// Push a signal into the interrupt queue.
    pub fn push(&self, signal: BusSignal) {
        self.queue.lock().unwrap().push(PrioritizedSignal(signal));
    }

    /// Pop the highest-priority signal.
    pub fn pop(&self) -> Option<BusSignal> {
        self.queue.lock().unwrap().pop().map(|p| p.0)
    }

    pub fn len(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.lock().unwrap().is_empty()
    }
}
