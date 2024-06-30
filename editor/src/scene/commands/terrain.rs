use fyrox::scene::terrain::brushstroke::ChunkData;

use crate::command::CommandContext;
use crate::fyrox::{
    core::pool::Handle,
    resource::texture::TextureResource,
    scene::{node::Node, terrain::Layer},
};
use crate::{
    command::CommandTrait, create_terrain_layer_material, scene::commands::GameSceneContext,
};

#[derive(Debug)]
pub struct AddTerrainLayerCommand {
    terrain: Handle<Node>,
    layer: Option<Layer>,
    masks: Vec<TextureResource>,
}

impl AddTerrainLayerCommand {
    pub fn new(terrain_handle: Handle<Node>) -> Self {
        Self {
            terrain: terrain_handle,
            layer: Some(Layer {
                material: create_terrain_layer_material(),
                ..Default::default()
            }),
            masks: Default::default(),
        }
    }
}

impl CommandTrait for AddTerrainLayerCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Add Terrain Layer".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let terrain = context.scene.graph[self.terrain].as_terrain_mut();
        terrain.add_layer(self.layer.take().unwrap(), std::mem::take(&mut self.masks));
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let terrain = context.scene.graph[self.terrain].as_terrain_mut();
        let (layer, masks) = terrain.pop_layer().unwrap();
        self.layer = Some(layer);
        self.masks = masks;
    }
}

#[derive(Debug)]
pub struct DeleteTerrainLayerCommand {
    terrain: Handle<Node>,
    layer: Option<Layer>,
    index: usize,
    masks: Vec<TextureResource>,
}

impl DeleteTerrainLayerCommand {
    pub fn new(terrain: Handle<Node>, index: usize) -> Self {
        Self {
            terrain,
            layer: Default::default(),
            index,
            masks: Default::default(),
        }
    }
}

impl CommandTrait for DeleteTerrainLayerCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Delete Terrain Layer".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let (layer, masks) = context.scene.graph[self.terrain]
            .as_terrain_mut()
            .remove_layer(self.index);

        self.layer = Some(layer);
        self.masks = masks;
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let terrain = context.scene.graph[self.terrain].as_terrain_mut();
        terrain.insert_layer(
            self.layer.take().unwrap(),
            std::mem::take(&mut self.masks),
            self.index,
        );
    }
}

#[derive(Debug)]
pub struct ModifyTerrainHeightCommand {
    terrain: Handle<Node>,
    heightmaps: Vec<ChunkData>,
    skip_first_execute: bool,
}

impl ModifyTerrainHeightCommand {
    pub fn new(terrain: Handle<Node>, heightmaps: Vec<ChunkData>) -> Self {
        Self {
            terrain,
            heightmaps,
            skip_first_execute: true,
        }
    }

    pub fn swap(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let terrain = context.scene.graph[self.terrain].as_terrain_mut();
        let current_chunks = terrain.chunks_mut();
        for c in self.heightmaps.iter_mut() {
            c.swap_height_from_list(current_chunks);
        }
        terrain.update_quad_trees();
    }
}

impl CommandTrait for ModifyTerrainHeightCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Modify Terrain Height".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        if self.skip_first_execute {
            self.skip_first_execute = false;
            return;
        }
        self.swap(context);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.swap(context);
    }
}

#[derive(Debug)]
pub struct ModifyTerrainLayerMaskCommand {
    terrain: Handle<Node>,
    masks: Vec<ChunkData>,
    layer: usize,
    skip_first_execute: bool,
}

impl ModifyTerrainLayerMaskCommand {
    pub fn new(terrain: Handle<Node>, masks: Vec<ChunkData>, layer: usize) -> Self {
        Self {
            terrain,
            masks,
            layer,
            skip_first_execute: true,
        }
    }

    pub fn swap(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let terrain = context.scene.graph[self.terrain].as_terrain_mut();
        let current_chunks = terrain.chunks_mut();
        for c in self.masks.iter_mut() {
            c.swap_layer_mask_from_list(current_chunks, self.layer);
        }
    }
}

impl CommandTrait for ModifyTerrainLayerMaskCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Modify Terrain Layer Mask".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        if self.skip_first_execute {
            self.skip_first_execute = false;
            return;
        }
        self.swap(context);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.swap(context);
    }
}
