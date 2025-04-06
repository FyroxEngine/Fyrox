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

use crate::{
    core::{algebra::Vector3, pool::Handle},
    scene::node::Node,
};
use fxhash::FxHashMap;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Visibility {
    Invisible,
    Visible,
}

impl From<bool> for Visibility {
    fn from(value: bool) -> Self {
        match value {
            true => Visibility::Visible,
            false => Visibility::Invisible,
        }
    }
}

impl Visibility {
    pub fn should_be_rendered(self) -> bool {
        match self {
            Visibility::Visible => true,
            Visibility::Invisible => false,
        }
    }
}

#[derive(Debug, Default)]
pub struct NodeVisibilityMap {
    map: FxHashMap<Handle<Node>, Visibility>,
}

impl NodeVisibilityMap {
    pub fn mark(&mut self, node: Handle<Node>, visibility: Visibility) {
        *self.map.entry(node).or_insert(visibility) = visibility;
    }

    pub fn is_visible(&self, node: Handle<Node>) -> bool {
        self.map
            .get(&node)
            .is_none_or(|vis| vis.should_be_rendered())
    }

    pub fn needs_occlusion_query(&self, node: Handle<Node>) -> bool {
        self.map
            .get(&node)
            .is_none_or(|vis| *vis == Visibility::Invisible)
    }
}

impl Deref for NodeVisibilityMap {
    type Target = FxHashMap<Handle<Node>, Visibility>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl DerefMut for NodeVisibilityMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

/// Volumetric visibility cache based on occlusion query.
#[derive(Debug)]
pub struct GridCache {
    cells: FxHashMap<Vector3<i32>, NodeVisibilityMap>,
    granularity: Vector3<u32>,
}

fn world_to_grid(world_position: Vector3<f32>, granularity: Vector3<u32>) -> Vector3<i32> {
    Vector3::new(
        (world_position.x * (granularity.x as f32)).round() as i32,
        (world_position.y * (granularity.y as f32)).round() as i32,
        (world_position.z * (granularity.z as f32)).round() as i32,
    )
}

impl GridCache {
    /// Creates new visibility cache with the given granularity and distance discard threshold.
    /// Granularity in means how much the cache should subdivide the world. For example 2 means that
    /// 1 meter cell will be split into 8 blocks by 0.5 meters. Distance discard threshold means how
    /// far an observer can without discarding visibility info about distant objects.
    pub fn new(granularity: Vector3<u32>) -> Self {
        Self {
            cells: Default::default(),
            granularity,
        }
    }

    /// Transforms the given world-space position into internal grid-space position.
    pub fn world_to_grid(&self, world_position: Vector3<f32>) -> Vector3<i32> {
        world_to_grid(world_position, self.granularity)
    }

    pub fn cell(&self, observer_position: Vector3<f32>) -> Option<&NodeVisibilityMap> {
        self.cells.get(&self.world_to_grid(observer_position))
    }

    pub fn get_or_insert_cell(
        &mut self,
        observer_position: Vector3<f32>,
    ) -> &mut NodeVisibilityMap {
        self.cells
            .entry(self.world_to_grid(observer_position))
            .or_default()
    }
}
