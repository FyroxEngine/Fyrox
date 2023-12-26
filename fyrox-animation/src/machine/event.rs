//! State machine could produces a fixed set of events during its work, this module contains all the stuff
//! needed to works with such events.

use crate::{
    core::pool::Handle,
    machine::{State, Transition},
};
use std::collections::VecDeque;

/// Specific state machine event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Occurs when enter some state. See module docs for example.
    StateEnter(Handle<State>),

    /// Occurs when leaving some state. See module docs for example.
    StateLeave(Handle<State>),

    /// Occurs when a transition is done and a new active state was set.
    ActiveStateChanged {
        /// Previously active state.
        prev: Handle<State>,

        /// New active state.
        new: Handle<State>,
    },

    /// Occurs when active transition was changed.
    ActiveTransitionChanged(Handle<Transition>),
}

/// A simple event queue with fixed capacity. It is used to store a fixed amount of events and discard any
/// events when the queue is full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedEventQueue {
    queue: VecDeque<Event>,
    limit: u32,
}

impl Default for FixedEventQueue {
    fn default() -> Self {
        Self {
            queue: Default::default(),
            limit: u32::MAX,
        }
    }
}

impl FixedEventQueue {
    /// Creates a new queue with given limit.
    pub fn new(limit: u32) -> Self {
        Self {
            queue: VecDeque::with_capacity(limit as usize),
            limit,
        }
    }

    /// Pushes an event to the queue.
    pub fn push(&mut self, event: Event) {
        if self.queue.len() < (self.limit as usize) {
            self.queue.push_back(event);
        }
    }

    /// Pops an event from the queue.
    pub fn pop(&mut self) -> Option<Event> {
        self.queue.pop_front()
    }
}
