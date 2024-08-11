// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

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
