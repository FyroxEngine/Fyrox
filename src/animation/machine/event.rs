use crate::{animation::machine::State, core::pool::Handle};
use std::collections::VecDeque;

/// Specific machine event.
#[derive(Debug)]
pub enum Event {
    /// Occurs when enter some state. See module docs for example.
    StateEnter(Handle<State>),

    /// Occurs when leaving some state. See module docs for example.
    StateLeave(Handle<State>),

    /// Occurs when transition is done and new active state was set.
    ActiveStateChanged(Handle<State>),
}

#[derive(Debug)]
pub struct LimitedEventQueue {
    queue: VecDeque<Event>,
    limit: u32,
}

impl Default for LimitedEventQueue {
    fn default() -> Self {
        Self {
            queue: Default::default(),
            limit: u32::MAX,
        }
    }
}

impl LimitedEventQueue {
    pub fn new(limit: u32) -> Self {
        Self {
            queue: VecDeque::with_capacity(limit as usize),
            limit,
        }
    }

    pub fn push(&mut self, event: Event) {
        if self.queue.len() < (self.limit as usize) {
            self.queue.push_back(event);
        }
    }

    pub fn pop(&mut self) -> Option<Event> {
        self.queue.pop_front()
    }
}
