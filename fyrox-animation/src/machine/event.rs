//! State machine could produces a fixed set of events during its work, this module contains all the stuff
//! needed to works with such events.

use crate::{
    core::pool::Handle,
    machine::{State, Transition},
    EntityId,
};
use std::collections::VecDeque;

/// Specific state machine event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event<T: EntityId> {
    /// Occurs when enter some state. See module docs for example.
    StateEnter(Handle<State<T>>),

    /// Occurs when leaving some state. See module docs for example.
    StateLeave(Handle<State<T>>),

    /// Occurs when a transition is done and a new active state was set.
    ActiveStateChanged {
        /// Previously active state.
        prev: Handle<State<T>>,

        /// New active state.
        new: Handle<State<T>>,
    },

    /// Occurs when active transition was changed.
    ActiveTransitionChanged(Handle<Transition<T>>),
}

/// A simple event queue with fixed capacity. It is used to store a fixed amount of events and discard any
/// events when the queue is full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedEventQueue<T: EntityId> {
    queue: VecDeque<Event<T>>,
    limit: u32,
}

impl<T: EntityId> Default for FixedEventQueue<T> {
    fn default() -> Self {
        Self {
            queue: Default::default(),
            limit: u32::MAX,
        }
    }
}

impl<T: EntityId> FixedEventQueue<T> {
    /// Creates a new queue with given limit.
    pub fn new(limit: u32) -> Self {
        Self {
            queue: VecDeque::with_capacity(limit as usize),
            limit,
        }
    }

    /// Pushes an event to the queue.
    pub fn push(&mut self, event: Event<T>) {
        if self.queue.len() < (self.limit as usize) {
            self.queue.push_back(event);
        }
    }

    /// Pops an event from the queue.
    pub fn pop(&mut self) -> Option<Event<T>> {
        self.queue.pop_front()
    }
}
