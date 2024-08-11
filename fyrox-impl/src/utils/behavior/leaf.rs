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

//! Leaf is a "final" node of a behavior tree. It contains user-defined action which
//! is able to mutate given context.

use crate::{
    core::{pool::Handle, visitor::prelude::*},
    utils::behavior::{BehaviorNode, BehaviorTree},
};
use std::cell::RefCell;

/// See module docs.
#[derive(Debug, PartialEq, Visit, Eq, Clone)]
pub struct LeafNode<B>
where
    B: Clone,
{
    /// User-defined behavior.
    pub behavior: Option<RefCell<B>>,
}

impl<B> Default for LeafNode<B>
where
    B: Clone,
{
    fn default() -> Self {
        Self { behavior: None }
    }
}

impl<B> LeafNode<B>
where
    B: Clone + 'static,
{
    /// Creates new leaf node with given action.
    pub fn new(behavior: B) -> Self {
        Self {
            behavior: Some(RefCell::new(behavior)),
        }
    }

    /// Adds self to given behavior tree and returns handle to self.
    pub fn add_to(self, tree: &mut BehaviorTree<B>) -> Handle<BehaviorNode<B>> {
        tree.add_node(BehaviorNode::Leaf(self))
    }
}
