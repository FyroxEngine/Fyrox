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
    core::{pool::Handle, uuid::Uuid},
    generic_animation::Animation,
};
use crate::scene::SelectionContainer;

use fyrox::core::reflect::Reflect;
use std::fmt::{Debug, Formatter};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SelectedEntity {
    Track(Uuid),
    Curve(Uuid),
    Signal(Uuid),
}

#[derive(Eq)]
pub struct AnimationSelection<N>
where
    N: Reflect,
{
    pub animation_player: Handle<N>,
    pub animation: Handle<Animation<Handle<N>>>,
    pub entities: Vec<SelectedEntity>,
}

impl<N> Debug for AnimationSelection<N>
where
    N: Reflect,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {:?}",
            self.animation_player, self.animation, self.entities
        )
    }
}

impl<N> Clone for AnimationSelection<N>
where
    N: Reflect,
{
    fn clone(&self) -> Self {
        Self {
            animation_player: self.animation_player,
            animation: self.animation,
            entities: self.entities.clone(),
        }
    }
}

impl<N> PartialEq for AnimationSelection<N>
where
    N: Reflect,
{
    fn eq(&self, other: &Self) -> bool {
        self.entities == other.entities
            && self.animation == other.animation
            && self.animation_player == other.animation_player
    }
}

impl<N> SelectionContainer for AnimationSelection<N>
where
    N: Reflect,
{
    fn len(&self) -> usize {
        self.entities.len()
    }
}

impl<N> AnimationSelection<N>
where
    N: Reflect,
{
    pub fn first_selected_track(&self) -> Option<Uuid> {
        self.entities.iter().find_map(|e| {
            if let SelectedEntity::Track(id) = e {
                Some(*id)
            } else {
                None
            }
        })
    }
}
