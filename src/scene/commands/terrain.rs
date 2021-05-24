use crate::{command::Command, scene::commands::SceneContext};
use rg3d::core::algebra::Vector2;
use rg3d::scene::graph::Graph;
use rg3d::{
    core::pool::Handle,
    scene::{node::Node, terrain::Layer},
};

#[derive(Debug)]
pub struct AddTerrainLayerCommand {
    terrain: Handle<Node>,
    layers: Vec<Layer>,
}

impl AddTerrainLayerCommand {
    pub fn new(terrain_handle: Handle<Node>, graph: &Graph) -> Self {
        let terrain = graph[terrain_handle].as_terrain();

        Self {
            terrain: terrain_handle,
            layers: terrain
                .chunks_ref()
                .iter()
                .map(|_| terrain.create_layer(Vector2::new(10.0, 10.0), 0))
                .collect(),
        }
    }
}

impl<'a> Command<'a> for AddTerrainLayerCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Add Terrain Layer".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        let terrain = context.scene.graph[self.terrain].as_terrain_mut();
        for (layer, chunk) in self.layers.drain(..).zip(terrain.chunks_mut()) {
            chunk.add_layer(layer);
        }
    }

    fn revert(&mut self, context: &mut Self::Context) {
        let terrain = context.scene.graph[self.terrain].as_terrain_mut();
        self.layers.clear();
        for chunk in terrain.chunks_mut() {
            self.layers.push(chunk.pop_layer().unwrap());
        }
    }
}

#[derive(Debug)]
pub struct DeleteTerrainLayerCommand {
    terrain: Handle<Node>,
    layers: Vec<Layer>,
    index: usize,
}

impl DeleteTerrainLayerCommand {
    pub fn new(terrain: Handle<Node>, index: usize) -> Self {
        Self {
            terrain,
            layers: Default::default(),
            index,
        }
    }
}

impl<'a> Command<'a> for DeleteTerrainLayerCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Delete Terrain Layer".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        self.layers = context.scene.graph[self.terrain]
            .as_terrain_mut()
            .chunks_mut()
            .iter_mut()
            .map(|c| c.remove_layer(self.index))
            .collect();
    }

    fn revert(&mut self, context: &mut Self::Context) {
        let terrain = context.scene.graph[self.terrain].as_terrain_mut();

        for (layer, chunk) in self.layers.drain(..).zip(terrain.chunks_mut()) {
            chunk.insert_layer(layer, self.index);
        }
    }
}
