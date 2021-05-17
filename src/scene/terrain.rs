//! Everything related to terrains.

#![allow(missing_docs)] // Temporary

use crate::resource::texture::{TextureKind, TexturePixelKind};
use crate::{
    core::{algebra::Vector3, arrayvec::ArrayVec, pool::Handle, visitor::prelude::*},
    resource::texture::Texture,
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::Node,
    },
};
use std::ops::{Deref, DerefMut};

#[derive(Default, Debug, Clone, Visit)]
pub struct Layer {
    diffuse_texture: Option<Texture>,
    normal_texture: Option<Texture>,
    specular_texture: Option<Texture>,
    height_texture: Option<Texture>,
    mask: Option<Texture>,
}

#[derive(Default, Debug, Clone, Visit)]
pub struct Chunk {
    heightmap: Vec<f32>,
    layers: Vec<Layer>,
    position: Vector3<f32>,
}

#[derive(Visit, Debug, Default)]
pub struct Terrain {
    width: f32,
    length: f32,
    base: Base,
    chunks: Vec<Chunk>,
}

impl Deref for Terrain {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Terrain {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Terrain {
    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn length(&self) -> f32 {
        self.length
    }

    pub fn chunks(&self) -> &[Chunk] {
        &self.chunks
    }

    pub fn raw_copy(&self) -> Self {
        Self {
            width: self.width,
            length: self.length,
            base: self.base.raw_copy(),
            chunks: self.chunks.clone(),
        }
    }
}

#[derive(Copy, Clone)]
pub enum BrushKind {
    Circle { radius: f32 },
    Rectangle { width: f32, length: f32 },
}

#[derive(Clone, PartialEq, PartialOrd)]
pub enum BrushMode {
    ChangeHeight { amount: f32 },
    Draw { layers: ArrayVec<usize, 32> },
}

#[derive(Clone)]
pub struct Brush {
    position: Vector3<f32>,
    kind: BrushKind,
    mode: BrushMode,
}

pub struct LayerDefinition {
    diffuse_texture: Option<Texture>,
    normal_texture: Option<Texture>,
    specular_texture: Option<Texture>,
    height_texture: Option<Texture>,
}

pub struct TerrainBuilder {
    base_builder: BaseBuilder,
    width: f32,
    length: f32,
    mask_resolution: f32,
    width_chunks: usize,
    length_chunks: usize,
    resolution: f32,
    layers: Vec<LayerDefinition>,
}

impl TerrainBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            width: 32.0,
            length: 32.0,
            mask_resolution: 64.0,
            width_chunks: 2,
            length_chunks: 2,
            resolution: 32.0,
            layers: Default::default(),
        }
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn with_length(mut self, length: f32) -> Self {
        self.length = length;
        self
    }

    pub fn with_mask_resolution(mut self, resolution: f32) -> Self {
        self.mask_resolution = resolution;
        self
    }

    pub fn with_width_chunks(mut self, count: usize) -> Self {
        self.width_chunks = count;
        self
    }

    pub fn with_length_chunks(mut self, count: usize) -> Self {
        self.length_chunks = count;
        self
    }

    pub fn with_resolution(mut self, resolution: f32) -> Self {
        self.resolution = resolution;
        self
    }

    pub fn with_layers(mut self, layers: Vec<LayerDefinition>) -> Self {
        self.layers = layers;
        self
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        let mut chunks = Vec::new();
        let chunk_length = self.length / self.length_chunks as f32;
        let chunk_width = self.width / self.width_chunks as f32;
        let chunk_length_points = (chunk_length * self.resolution) as usize;
        let chunk_width_points = (chunk_width * self.resolution) as usize;
        let chunk_mask_width = (chunk_width * self.mask_resolution) as u32;
        let chunk_mask_height = (chunk_length * self.mask_resolution) as u32;
        for z in 0..self.length_chunks {
            for x in 0..self.width_chunks {
                chunks.push(Chunk {
                    heightmap: vec![0.0; chunk_length_points * chunk_width_points],
                    layers: self
                        .layers
                        .iter()
                        .map(|definition| Layer {
                            diffuse_texture: definition.diffuse_texture.clone(),
                            normal_texture: definition.normal_texture.clone(),
                            specular_texture: definition.specular_texture.clone(),
                            height_texture: definition.height_texture.clone(),
                            mask: Texture::from_bytes(
                                TextureKind::Rectangle {
                                    width: chunk_mask_width,
                                    height: chunk_mask_height,
                                },
                                TexturePixelKind::R8,
                                vec![255; (chunk_mask_width * chunk_mask_height) as usize],
                            ),
                        })
                        .collect(),
                    position: Vector3::new(x as f32, 0.0, z as f32),
                });
            }
        }

        let terrain = Terrain {
            width: self.width,
            length: self.length,
            base: self.base_builder.build_base(),
            chunks,
        };

        graph.add_node(Node::Terrain(terrain))
    }
}
