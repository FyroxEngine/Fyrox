use crate::{
    command::Command, create_terrain_layer_material, define_node_command, get_set_swap,
    scene::commands::SceneContext,
};
use rg3d::{
    core::pool::Handle,
    scene::{graph::Graph, node::Node, terrain::Layer},
};

#[derive(Debug)]
pub struct AddTerrainLayerCommand {
    terrain: Handle<Node>,
    layer: Option<Layer>,
}

impl AddTerrainLayerCommand {
    pub fn new(terrain_handle: Handle<Node>, graph: &Graph) -> Self {
        let terrain = graph[terrain_handle].as_terrain();

        Self {
            terrain: terrain_handle,
            layer: Some(terrain.create_layer(
                0,
                create_terrain_layer_material(),
                "maskTexture".to_owned(),
            )),
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
        terrain.add_layer(self.layer.take().unwrap());
    }

    fn revert(&mut self, context: &mut Self::Context) {
        let terrain = context.scene.graph[self.terrain].as_terrain_mut();
        self.layer = terrain.pop_layer();
    }
}

#[derive(Debug)]
pub struct DeleteTerrainLayerCommand {
    terrain: Handle<Node>,
    layer: Option<Layer>,
    index: usize,
}

impl DeleteTerrainLayerCommand {
    pub fn new(terrain: Handle<Node>, index: usize) -> Self {
        Self {
            terrain,
            layer: Default::default(),
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
        self.layer = Some(
            context.scene.graph[self.terrain]
                .as_terrain_mut()
                .remove_layer(self.index),
        );
    }

    fn revert(&mut self, context: &mut Self::Context) {
        let terrain = context.scene.graph[self.terrain].as_terrain_mut();
        terrain.insert_layer(self.layer.take().unwrap(), self.index);
    }
}

#[derive(Debug)]
pub struct ModifyTerrainHeightCommand {
    terrain: Handle<Node>,
    // TODO: This is very memory-inefficient solution, it could be done
    //  better by either pack/unpack data on the fly, or by saving changes
    //  for sub-chunks.
    old_heightmaps: Vec<Vec<f32>>,
    new_heightmaps: Vec<Vec<f32>>,
}

impl ModifyTerrainHeightCommand {
    pub fn new(
        terrain: Handle<Node>,
        old_heightmaps: Vec<Vec<f32>>,
        new_heightmaps: Vec<Vec<f32>>,
    ) -> Self {
        Self {
            terrain,
            old_heightmaps,
            new_heightmaps,
        }
    }

    pub fn swap(&mut self, context: &mut SceneContext) {
        let terrain = context.scene.graph[self.terrain].as_terrain_mut();
        for (chunk, (old, new)) in terrain.chunks_mut().iter_mut().zip(
            self.old_heightmaps
                .iter_mut()
                .zip(self.new_heightmaps.iter_mut()),
        ) {
            chunk.set_heightmap(new.clone());
            std::mem::swap(old, new);
        }
    }
}

impl<'a> Command<'a> for ModifyTerrainHeightCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Modify Terrain Height".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        self.swap(context);
    }

    fn revert(&mut self, context: &mut Self::Context) {
        self.swap(context);
    }
}

#[derive(Debug)]
pub struct ModifyTerrainLayerMaskCommand {
    terrain: Handle<Node>,
    // TODO: This is very memory-inefficient solution, it could be done
    //  better by either pack/unpack data on the fly, or by saving changes
    //  for sub-chunks.
    old_masks: Vec<Vec<u8>>,
    new_masks: Vec<Vec<u8>>,
    layer: usize,
}

impl ModifyTerrainLayerMaskCommand {
    pub fn new(
        terrain: Handle<Node>,
        old_masks: Vec<Vec<u8>>,
        new_masks: Vec<Vec<u8>>,
        layer: usize,
    ) -> Self {
        Self {
            terrain,
            old_masks,
            new_masks,
            layer,
        }
    }

    pub fn swap(&mut self, context: &mut SceneContext) {
        let terrain = context.scene.graph[self.terrain].as_terrain_mut();
        for (chunk_mask, (old, new)) in terrain.layers_mut()[self.layer]
            .chunk_masks()
            .iter()
            .zip(self.old_masks.iter_mut().zip(self.new_masks.iter_mut()))
        {
            let mut texture_data = chunk_mask.data_ref();

            for (mask_pixel, new_pixel) in
                texture_data.modify().data_mut().iter_mut().zip(new.iter())
            {
                *mask_pixel = *new_pixel;
            }

            std::mem::swap(old, new);
        }
    }
}

impl<'a> Command<'a> for ModifyTerrainLayerMaskCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Modify Terrain Layer Mask".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        self.swap(context);
    }

    fn revert(&mut self, context: &mut Self::Context) {
        self.swap(context);
    }
}

define_node_command!(SetTerrainDecalLayerIndexCommand("Set Terrain Decal Layer Index", u8) where fn swap(self, node) {
    get_set_swap!(self, node.as_terrain_mut(), decal_layer_index, set_decal_layer_index);
});
