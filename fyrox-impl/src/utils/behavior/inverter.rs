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

//! A node, that inverts its child state ([`super::Status::Failure`] becomes [`super::Status::Success`] and vice versa, [`super::Status::Running`] remains
//! unchanged)

use crate::{
    core::{pool::Handle, visitor::prelude::*},
    utils::behavior::{BehaviorNode, BehaviorTree},
};

/// See module docs.
#[derive(Debug, PartialEq, Visit, Eq, Clone)]
pub struct Inverter<B>
where
    B: Clone,
{
    /// A handle of child node, the state of which will be inverted.
    pub child: Handle<BehaviorNode<B>>,
}

impl<B> Default for Inverter<B>
where
    B: Clone,
{
    fn default() -> Self {
        Self {
            child: Default::default(),
        }
    }
}

impl<B> Inverter<B>
where
    B: Clone + 'static,
{
    /// Creates new inverter node with given action.
    pub fn new(child: Handle<BehaviorNode<B>>) -> Self {
        Self { child }
    }

    /// Adds self to given behavior tree and returns handle to self.
    pub fn add_to(self, tree: &mut BehaviorTree<B>) -> Handle<BehaviorNode<B>> {
        tree.add_node(BehaviorNode::Inverter(self))
    }
}
