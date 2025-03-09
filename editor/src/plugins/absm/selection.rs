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

use crate::fyrox::{
    core::pool::Handle,
    generic_animation::machine::{PoseNode, State, Transition},
};
use crate::scene::SelectionContainer;

use fyrox::core::reflect::Reflect;
use std::fmt::{Debug, Formatter};

#[derive(Eq)]
pub enum SelectedEntity<N: Reflect> {
    Transition(Handle<Transition<Handle<N>>>),
    State(Handle<State<Handle<N>>>),
    PoseNode(Handle<PoseNode<Handle<N>>>),
}

impl<N> Debug for SelectedEntity<N>
where
    N: Reflect,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transition(v) => write!(f, "{v}"),
            Self::State(v) => write!(f, "{v}"),
            Self::PoseNode(v) => write!(f, "{v}"),
        }
    }
}

impl<N> Clone for SelectedEntity<N>
where
    N: Reflect,
{
    fn clone(&self) -> Self {
        match self {
            Self::Transition(v) => Self::Transition(*v),
            Self::State(v) => Self::State(*v),
            Self::PoseNode(v) => Self::PoseNode(*v),
        }
    }
}

impl<N> PartialEq for SelectedEntity<N>
where
    N: Reflect,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Transition(a), Self::Transition(b)) => *a == *b,
            (Self::State(a), Self::State(b)) => *a == *b,
            (Self::PoseNode(a), Self::PoseNode(b)) => *a == *b,
            _ => false,
        }
    }
}

#[derive(Eq, Default)]
pub struct AbsmSelection<N: Reflect> {
    pub absm_node_handle: Handle<N>,
    pub layer: Option<usize>,
    pub entities: Vec<SelectedEntity<N>>,
}

impl<N> Debug for AbsmSelection<N>
where
    N: Reflect,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {:?} {:?}",
            self.absm_node_handle, self.layer, self.entities
        )
    }
}

impl<N> Clone for AbsmSelection<N>
where
    N: Reflect,
{
    fn clone(&self) -> Self {
        Self {
            absm_node_handle: self.absm_node_handle,
            layer: self.layer,
            entities: self.entities.clone(),
        }
    }
}

impl<N> PartialEq for AbsmSelection<N>
where
    N: Reflect,
{
    fn eq(&self, other: &Self) -> bool {
        self.entities == other.entities
            && self.layer == other.layer
            && self.absm_node_handle == other.absm_node_handle
    }
}

impl<N: Reflect> SelectionContainer for AbsmSelection<N> {
    fn len(&self) -> usize {
        self.entities.len()
    }
}
