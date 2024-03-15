use crate::command::CommandContext;
use crate::fyrox::core::log::Log;
use crate::fyrox::resource::texture::{
    TextureKind, TexturePixelKind, TextureResourceExtension, TextureWrapMode,
};
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

    pub fn swap(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let terrain = context.scene.graph[self.terrain].as_terrain_mut();
        let heigth_map_size = terrain.height_map_size();
        for (chunk, (old, new)) in terrain.chunks_mut().iter_mut().zip(
            self.old_heightmaps
                .iter_mut()
                .zip(self.new_heightmaps.iter_mut()),
        ) {
            let height_map = TextureResource::from_bytes(
                TextureKind::Rectangle {
                    width: heigth_map_size.x,
                    height: heigth_map_size.y,
                },
                TexturePixelKind::R32F,
                fyrox::core::transmute_vec_as_bytes(new.clone()),
                Default::default(),
            )
            .unwrap();

            let mut data = height_map.data_ref();
            data.set_s_wrap_mode(TextureWrapMode::ClampToEdge);
            data.set_t_wrap_mode(TextureWrapMode::ClampToEdge);
            drop(data);

            chunk.replace_height_map(height_map).unwrap();
            std::mem::swap(old, new);
        }
    }
}

impl CommandTrait for ModifyTerrainHeightCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Modify Terrain Height".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.swap(context);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
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

    pub fn swap(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let terrain = context.scene.graph[self.terrain].as_terrain_mut();

        for (i, chunk) in terrain.chunks_mut().iter_mut().enumerate() {
            if i >= self.old_masks.len() || i >= self.new_masks.len() {
                Log::err("Invalid mask index.")
            } else {
                let old = &mut self.old_masks[i];
                let new = &mut self.new_masks[i];
                let chunk_mask = &mut chunk.layer_masks[self.layer];

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
}

impl CommandTrait for ModifyTerrainLayerMaskCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Modify Terrain Layer Mask".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.swap(context);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.swap(context);
    }
}
