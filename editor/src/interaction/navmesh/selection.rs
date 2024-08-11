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
    core::{math::TriangleEdge, pool::Handle},
    scene::node::Node,
};
use crate::scene::SelectionContainer;
use std::{
    cell::{Cell, Ref, RefCell},
    collections::BTreeSet,
};

#[derive(PartialEq, Clone, Debug, Eq)]
pub enum NavmeshEntity {
    Vertex(usize),
    Edge(TriangleEdge),
}

#[derive(PartialEq, Clone, Debug, Eq)]
pub struct NavmeshSelection {
    dirty: Cell<bool>,
    navmesh_node: Handle<Node>,
    entities: Vec<NavmeshEntity>,
    unique_vertices: RefCell<BTreeSet<usize>>,
}

impl SelectionContainer for NavmeshSelection {
    fn len(&self) -> usize {
        self.entities.len()
    }
}

impl NavmeshSelection {
    pub fn empty(navmesh: Handle<Node>) -> Self {
        Self {
            dirty: Cell::new(false),
            navmesh_node: navmesh,
            entities: vec![],
            unique_vertices: Default::default(),
        }
    }

    pub fn new(navmesh: Handle<Node>, entities: Vec<NavmeshEntity>) -> Self {
        Self {
            dirty: Cell::new(true),
            navmesh_node: navmesh,
            entities,
            unique_vertices: Default::default(),
        }
    }

    pub fn navmesh_node(&self) -> Handle<Node> {
        self.navmesh_node
    }

    pub fn add(&mut self, entity: NavmeshEntity) {
        self.entities.push(entity);
        self.dirty.set(true);
    }

    pub fn clear(&mut self) {
        self.entities.clear();
        self.unique_vertices.borrow_mut().clear();
        self.dirty.set(false);
    }

    pub fn first(&self) -> Option<&NavmeshEntity> {
        self.entities.first()
    }

    pub fn unique_vertices(&self) -> Ref<'_, BTreeSet<usize>> {
        if self.dirty.get() {
            let mut unique_vertices = self.unique_vertices.borrow_mut();
            unique_vertices.clear();
            for entity in self.entities.iter() {
                match entity {
                    NavmeshEntity::Vertex(v) => {
                        unique_vertices.insert(*v);
                    }
                    NavmeshEntity::Edge(edge) => {
                        unique_vertices.insert(edge.a as usize);
                        unique_vertices.insert(edge.b as usize);
                    }
                }
            }
        }

        self.unique_vertices.borrow()
    }

    pub fn entities(&self) -> &[NavmeshEntity] {
        &self.entities
    }

    pub fn contains_edge(&self, edge: TriangleEdge) -> bool {
        self.entities.contains(&NavmeshEntity::Edge(edge))
    }
}
