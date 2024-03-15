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
