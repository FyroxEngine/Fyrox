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

//! Composite node is a container for children nodes. Composite node could be either
//! `Sequence` or `Selector`. `Sequence` node will execute children nodes consecutively
//! until `Status::Failure` is returned from any descendant node. In other words `Sequence`
//! implement AND logical function. `Selector` node will execute children until `Status::Success`
//! is returned from any descendant node. In other worlds `Selector` implement OR logical
//! function.

use crate::{
    core::{pool::Handle, visitor::prelude::*},
    utils::behavior::{BehaviorNode, BehaviorTree},
};

/// Defines exact behavior of the composite node.
#[derive(Debug, PartialEq, Visit, Eq, Clone)]
pub enum CompositeNodeKind {
    /// `Sequence` node will execute children nodes consecutively
    /// until `Status::Failure` is returned from any descendant node. In other words `Sequence`
    /// implement AND logical function.
    Sequence,
    /// `Selector` node will execute children until `Status::Success`
    /// is returned from any descendant node. In other worlds `Selector` implement OR logical
    /// function.
    Selector,
}

impl Default for CompositeNodeKind {
    fn default() -> Self {
        Self::Sequence
    }
}

/// See module docs.
#[derive(Debug, PartialEq, Visit, Eq, Clone)]
pub struct CompositeNode<B>
where
    B: Clone,
{
    /// A set of children.
    pub children: Vec<Handle<BehaviorNode<B>>>,
    /// Current kind of the node.
    pub kind: CompositeNodeKind,
}

impl<B> Default for CompositeNode<B>
where
    B: Clone,
{
    fn default() -> Self {
        Self {
            children: Default::default(),
            kind: Default::default(),
        }
    }
}

impl<B> CompositeNode<B>
where
    B: Clone + 'static,
{
    /// Creates new composite node of given kind and set of children nodes.
    pub fn new(kind: CompositeNodeKind, children: Vec<Handle<BehaviorNode<B>>>) -> Self {
        Self { children, kind }
    }

    /// Creates new sequence composite node with a set of children nodes.
    pub fn new_sequence(children: Vec<Handle<BehaviorNode<B>>>) -> Self {
        Self {
            children,
            kind: CompositeNodeKind::Sequence,
        }
    }

    /// Creates new selector composite node with a set of children nodes.
    pub fn new_selector(children: Vec<Handle<BehaviorNode<B>>>) -> Self {
        Self {
            children,
            kind: CompositeNodeKind::Selector,
        }
    }

    /// Adds self to the tree and return handle to self.
    pub fn add_to(self, tree: &mut BehaviorTree<B>) -> Handle<BehaviorNode<B>> {
        tree.add_node(BehaviorNode::Composite(self))
    }
}
