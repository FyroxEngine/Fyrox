#![allow(missing_docs)] // TODO

pub mod tileset;

use crate::{
    core::{
        algebra::Vector2, math::aabb::AxisAlignedBoundingBox, pool::Handle, reflect::prelude::*,
        type_traits::prelude::*, variable::InheritableVariable, visitor::prelude::*,
    },
    graph::BaseSceneGraph,
    renderer::bundle::RenderContext,
    scene::{
        base::Base, base::BaseBuilder, graph::Graph, node::Node, node::NodeTrait,
        node::RdcControlFlow, tilemap::tileset::TileSet, tilemap::tileset::TileSetResource,
    },
};
use std::ops::{Deref, DerefMut};

#[derive(Clone, Reflect, Default, Debug, PartialEq, Visit, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "e429ca1b-a311-46c3-b580-d5a2f49db7e2")]
pub struct Tile {
    position: Vector2<i32>,
}

#[derive(Clone, Reflect, Default, Debug, Visit, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "aa9a3385-a4af-4faf-a69a-8d3af1a3aa67")]
pub struct TileMap {
    base: Base,
    tile_set: InheritableVariable<Option<TileSetResource>>,
    tiles: InheritableVariable<Vec<Tile>>,
}

impl Deref for TileMap {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for TileMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl NodeTrait for TileMap {
    crate::impl_query_component!();

    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        AxisAlignedBoundingBox::unit()
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.local_bounding_box()
            .transform(&self.global_transform())
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn collect_render_data(&self, _ctx: &mut RenderContext) -> RdcControlFlow {
        RdcControlFlow::Continue
    }
}

pub struct TileMapBuilder {
    base_builder: BaseBuilder,
    tile_set: Option<TileSet>,
    tiles: Vec<Tile>,
}

impl TileMapBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            tile_set: None,
            tiles: Default::default(),
        }
    }

    pub fn with_tile_set(mut self, tile_set: TileSet) -> Self {
        self.tile_set = Some(tile_set);
        self
    }

    pub fn with_tiles(mut self, tiles: Vec<Tile>) -> Self {
        self.tiles = tiles;
        self
    }

    pub fn build_node(self) -> Node {
        Node::new(TileMap {
            base: self.base_builder.build_base(),
            tile_set: Default::default(),
            tiles: Default::default(),
        })
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
