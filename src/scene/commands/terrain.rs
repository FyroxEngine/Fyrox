use crate::{command::Command, scene::commands::SceneContext};
use rg3d::resource::texture::Texture;
use rg3d::{
    core::{algebra::Vector2, pool::Handle},
    scene::{graph::Graph, node::Node, terrain::Layer},
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

#[derive(Debug)]
pub enum TerrainLayerTextureKind {
    Diffuse,
    Normal,
    Specular,
    Roughness,
    Height,
}

#[derive(Debug)]
pub struct SetTerrainLayerTextureCommand {
    terrain: Handle<Node>,
    index: usize,
    kind: TerrainLayerTextureKind,
    texture: Option<Texture>,
}

impl SetTerrainLayerTextureCommand {
    pub fn new(
        terrain: Handle<Node>,
        index: usize,
        texture: Texture,
        kind: TerrainLayerTextureKind,
    ) -> Self {
        Self {
            kind,
            index,
            terrain,
            texture: Some(texture),
        }
    }

    fn swap(&mut self, context: &mut SceneContext) {
        let terrain = context.scene.graph[self.terrain].as_terrain_mut();
        let texture = self.texture.take();
        for chunk in terrain.chunks_mut() {
            let layer = &mut chunk.layers_mut()[self.index];
            let instance = match self.kind {
                TerrainLayerTextureKind::Diffuse => &mut layer.diffuse_texture,
                TerrainLayerTextureKind::Normal => &mut layer.normal_texture,
                TerrainLayerTextureKind::Specular => &mut layer.specular_texture,
                TerrainLayerTextureKind::Roughness => &mut layer.roughness_texture,
                TerrainLayerTextureKind::Height => &mut layer.height_texture,
            };

            if self.texture.is_none() {
                self.texture = instance.clone();
            }
            *instance = texture.clone()
        }
    }
}

impl<'a> Command<'a> for SetTerrainLayerTextureCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Set Terrain Layer Texture".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        self.swap(context);
    }

    fn revert(&mut self, context: &mut Self::Context) {
        self.swap(context);
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
        for (chunk, (old, new)) in terrain
            .chunks_mut()
            .iter_mut()
            .zip(self.old_masks.iter_mut().zip(self.new_masks.iter_mut()))
        {
            let mut texture_data = chunk.layers_mut()[self.layer]
                .mask
                .as_mut()
                .unwrap()
                .data_ref();

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
