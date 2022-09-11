use crate::interaction::navmesh::{
    data_model::{Navmesh, NavmeshEdge},
    NavmeshEntity, NavmeshVertex,
};
use fyrox::core::pool::Handle;
use std::cell::Ref;
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
};

#[derive(PartialEq, Clone, Debug, Eq)]
pub struct NavmeshSelection {
    dirty: Cell<bool>,
    navmesh: Handle<Navmesh>,
    entities: Vec<NavmeshEntity>,
    unique_vertices: RefCell<HashSet<Handle<NavmeshVertex>>>,
}

impl NavmeshSelection {
    pub fn empty(navmesh: Handle<Navmesh>) -> Self {
        Self {
            dirty: Cell::new(false),
            navmesh,
            entities: vec![],
            unique_vertices: Default::default(),
        }
    }

    pub fn new(navmesh: Handle<Navmesh>, entities: Vec<NavmeshEntity>) -> Self {
        Self {
            dirty: Cell::new(true),
            navmesh,
            entities,
            unique_vertices: Default::default(),
        }
    }

    pub fn navmesh(&self) -> Handle<Navmesh> {
        self.navmesh
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

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub fn is_single_selection(&self) -> bool {
        self.entities.len() == 1
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn unique_vertices(&self) -> Ref<'_, HashSet<Handle<NavmeshVertex>>> {
        if self.dirty.get() {
            let mut unique_vertices = self.unique_vertices.borrow_mut();
            unique_vertices.clear();
            for entity in self.entities.iter() {
                match entity {
                    NavmeshEntity::Vertex(v) => {
                        unique_vertices.insert(*v);
                    }
                    NavmeshEntity::Edge(edge) => {
                        unique_vertices.insert(edge.begin);
                        unique_vertices.insert(edge.end);
                    }
                }
            }
        }

        self.unique_vertices.borrow()
    }

    pub fn entities(&self) -> &[NavmeshEntity] {
        &self.entities
    }

    pub fn contains_edge(&self, edge: NavmeshEdge) -> bool {
        self.entities.contains(&NavmeshEntity::Edge(edge))
    }
}
