use crate::interaction::navmesh::data_model::NavmeshEdge;
use crate::interaction::navmesh::{NavmeshEntity, NavmeshVertex};
use rg3d::core::pool::Handle;
use std::collections::HashSet;

#[derive(PartialEq, Default, Clone, Debug)]
pub struct NavmeshSelection {
    dirty: bool,
    entities: Vec<NavmeshEntity>,
    unique_vertices: HashSet<Handle<NavmeshVertex>>,
}

impl NavmeshSelection {
    pub fn add(&mut self, entity: NavmeshEntity) {
        self.entities.push(entity);
        self.dirty = true;
    }

    pub fn clear(&mut self) {
        self.entities.clear();
        self.unique_vertices.clear();
        self.dirty = false;
    }

    pub fn first(&self) -> Option<&NavmeshEntity> {
        self.entities.first()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub fn unique_vertices(&mut self) -> &HashSet<Handle<NavmeshVertex>> {
        if self.dirty {
            self.unique_vertices.clear();
            for entity in self.entities.iter() {
                match entity {
                    NavmeshEntity::Vertex(v) => {
                        self.unique_vertices.insert(*v);
                    }
                    NavmeshEntity::Edge(edge) => {
                        self.unique_vertices.insert(edge.begin);
                        self.unique_vertices.insert(edge.end);
                    }
                }
            }
        }

        &self.unique_vertices
    }

    pub fn entities(&self) -> &[NavmeshEntity] {
        &self.entities
    }

    pub fn contains_edge(&self, edge: NavmeshEdge) -> bool {
        self.entities.contains(&NavmeshEntity::Edge(edge))
    }
}
