//! Tile map is a 2D "image", made out of a small blocks called tiles. Tile maps used in 2D games to
//! build game worlds quickly and easily. See [`TileMap`] docs for more info and usage examples.

pub mod brush;
pub mod tileset;

use crate::{
    core::{
        algebra::{Vector2, Vector3},
        math::{aabb::AxisAlignedBoundingBox, Rect, TriangleDefinition},
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        value_as_u8_slice,
        variable::InheritableVariable,
        visitor::prelude::*,
    },
    graph::BaseSceneGraph,
    rand::{seq::IteratorRandom, thread_rng},
    renderer::{self, bundle::RenderContext},
    scene::{
        base::{Base, BaseBuilder},
        dim2::rectangle::RectangleVertex,
        graph::Graph,
        mesh::{buffer::VertexTrait, RenderPath},
        node::{Node, NodeTrait, RdcControlFlow},
        tilemap::{
            brush::{TileMapBrush, TileMapBrushResource},
            tileset::{TileDefinitionHandle, TileSetResource},
        },
        Scene,
    },
};
use fxhash::{FxHashMap, FxHashSet};
use std::ops::{Deref, DerefMut};

/// Tile is a base block of a tile map. It has a position and a handle of tile definition, stored
/// in the respective tile set.
#[derive(Clone, Reflect, Default, Debug, PartialEq, Visit, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "e429ca1b-a311-46c3-b580-d5a2f49db7e2")]
pub struct Tile {
    /// Position of the tile (in grid coordinates).
    pub position: Vector2<i32>,
    /// A handle of the tile definition.
    pub definition_handle: TileDefinitionHandle,
}

/// A set of tiles.
#[derive(Clone, Reflect, Debug, Default, PartialEq)]
pub struct Tiles(FxHashMap<Vector2<i32>, Tile>);

impl Visit for Tiles {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)
    }
}

impl Deref for Tiles {
    type Target = FxHashMap<Vector2<i32>, Tile>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Tiles {
    /// Inserts a tile in the tile container. Returns previous tile, located at the same position as
    /// the new one (if any).
    #[inline]
    pub fn insert(&mut self, tile: Tile) -> Option<Tile> {
        self.0.insert(tile.position, tile)
    }

    /// Tries to remove a tile at the given position.
    #[inline]
    pub fn remove(&mut self, position: Vector2<i32>) -> Option<Tile> {
        self.0.remove(&position)
    }
}

/// Tile map is a 2D "image", made out of a small blocks called tiles. Tile maps used in 2D games to
/// build game worlds quickly and easily.
///
/// ## Example
///
/// The following example creates a simple tile map with two tile types - grass and stone. It creates
/// stone foundation and lays grass on top of it.
///
/// ```rust
/// use fyrox_impl::{
///     asset::untyped::ResourceKind,
///     core::{algebra::Vector2, color::Color, math::Rect, pool::Handle},
///     material::{Material, MaterialResource},
///     scene::{
///         base::BaseBuilder,
///         graph::Graph,
///         node::Node,
///         tilemap::{
///             tileset::{TileCollider, TileDefinition, TileSet, TileSetResource},
///             Tile, TileMapBuilder, Tiles,
///         },
///     },
/// };
///
/// fn create_tile_map(graph: &mut Graph) -> Handle<Node> {
///     // Each tile could have its own material, for simplicity it is just a standard 2D material.
///     let material = MaterialResource::new_ok(ResourceKind::Embedded, Material::standard_2d());
///
///     // Create a tile set - it is a data source for the tile map. Tile map will reference the tiles
///     // stored in the tile set by handles. We'll create two tile types with different colors.
///     let mut tile_set = TileSet::default();
///     let stone_tile = tile_set.add_tile(TileDefinition {
///         material: material.clone(),
///         uv_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
///         collider: TileCollider::Rectangle,
///         color: Color::BROWN,
///     });
///     let grass_tile = tile_set.add_tile(TileDefinition {
///         material,
///         uv_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
///         collider: TileCollider::Rectangle,
///         color: Color::GREEN,
///     });
///     let tile_set = TileSetResource::new_ok(ResourceKind::Embedded, tile_set);
///
///     let mut tiles = Tiles::default();
///
///     // Create stone foundation.
///     for x in 0..10 {
///         for y in 0..2 {
///             tiles.insert(Tile {
///                 position: Vector2::new(x, y),
///                 definition_handle: stone_tile,
///             });
///         }
///     }
///
///     // Add grass on top of it.
///     for x in 0..10 {
///         tiles.insert(Tile {
///             position: Vector2::new(x, 2),
///             definition_handle: grass_tile,
///         });
///     }
///
///     // Finally create the tile map.
///     TileMapBuilder::new(BaseBuilder::new())
///         .with_tile_set(tile_set)
///         .with_tiles(tiles)
///         .build(graph)
/// }
/// ```
#[derive(Clone, Reflect, Debug, Visit, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "aa9a3385-a4af-4faf-a69a-8d3af1a3aa67")]
pub struct TileMap {
    base: Base,
    tile_set: InheritableVariable<Option<TileSetResource>>,
    #[reflect(read_only)]
    tiles: InheritableVariable<Tiles>,
    tile_scale: InheritableVariable<Vector2<f32>>,
    brushes: InheritableVariable<Vec<Option<TileMapBrushResource>>>,
    active_brush: InheritableVariable<Option<TileMapBrushResource>>,
}

impl TileMap {
    /// Returns a reference to the current tile set (if any).
    #[inline]
    pub fn tile_set(&self) -> Option<&TileSetResource> {
        self.tile_set.as_ref()
    }

    /// Sets new tile set.
    #[inline]
    pub fn set_tile_set(&mut self, tile_set: Option<TileSetResource>) {
        self.tile_set.set_value_and_mark_modified(tile_set);
    }

    /// Returns a reference to the tile container.
    #[inline]
    pub fn tiles(&self) -> &Tiles {
        &self.tiles
    }

    /// Sets new tiles.
    #[inline]
    pub fn set_tiles(&mut self, tiles: Tiles) {
        self.tiles.set_value_and_mark_modified(tiles);
    }

    /// Returns current tile scaling.
    #[inline]
    pub fn tile_scale(&self) -> Vector2<f32> {
        *self.tile_scale
    }

    /// Sets new tile scaling, which defines tile size.
    #[inline]
    pub fn set_tile_scale(&mut self, tile_scale: Vector2<f32>) {
        self.tile_scale.set_value_and_mark_modified(tile_scale);
    }

    /// Inserts a tile in the tile map. Returns previous tile, located at the same position as
    /// the new one (if any).
    #[inline]
    pub fn insert_tile(&mut self, tile: Tile) -> Option<Tile> {
        self.tiles.insert(tile)
    }

    /// Removes a tile from the tile map.
    #[inline]
    pub fn remove_tile(&mut self, position: Vector2<i32>) -> Option<Tile> {
        self.tiles.remove(position)
    }

    /// Returns active brush of the tile map.
    #[inline]
    pub fn active_brush(&self) -> Option<TileMapBrushResource> {
        (*self.active_brush).clone()
    }

    /// Sets new active brush of the tile map.
    #[inline]
    pub fn set_active_brush(&mut self, brush: Option<TileMapBrushResource>) {
        self.active_brush.set_value_and_mark_modified(brush);
    }

    /// Returns a reference to the set of brushes.
    #[inline]
    pub fn brushes(&self) -> &[Option<TileMapBrushResource>] {
        &self.brushes
    }

    /// Sets news brushes of the tile map. This set could be used to store the most used brushes.
    #[inline]
    pub fn set_brushes(&mut self, brushes: Vec<Option<TileMapBrushResource>>) {
        self.brushes.set_value_and_mark_modified(brushes);
    }

    /// Draws on the tile map using the given brush.
    #[inline]
    pub fn draw(&mut self, origin: Vector2<i32>, brush: &TileMapBrush) {
        for brush_tile in brush.tiles.iter() {
            self.insert_tile(Tile {
                position: origin + brush_tile.local_position,
                definition_handle: brush_tile.definition_handle,
            });
        }
    }

    /// Erases the tiles under the given brush.
    #[inline]
    pub fn erase(&mut self, origin: Vector2<i32>, brush: &TileMapBrush) {
        for brush_tile in brush.tiles.iter() {
            self.remove_tile(origin + brush_tile.local_position);
        }
    }

    /// Calculates bounding rectangle in grid coordinates.
    #[inline]
    pub fn bounding_rect(&self) -> Rect<i32> {
        let mut min = Vector2::repeat(i32::MAX);
        let mut max = Vector2::repeat(i32::MIN);

        for tile in self.tiles.values() {
            min = tile.position.inf(&min);
            max = tile.position.sup(&max);
        }

        Rect::from_points(min, max)
    }

    /// Tries to fetch tile definition index at the given point.
    #[inline]
    pub fn definition_at(&self, point: Vector2<i32>) -> Option<TileDefinitionHandle> {
        self.tiles.get(&point).map(|tile| tile.definition_handle)
    }

    /// Fills the tile map at the given point using random tiles from the given brush. This method
    /// extends tile map when trying to fill at a point that lies outside the bounding rectangle.
    /// Keep in mind, that flood fill is only possible either on free cells or on cells with the same
    /// tile kind.
    #[inline]
    pub fn flood_fill(&mut self, start_point: Vector2<i32>, brush: &TileMapBrush) {
        let mut bounds = self.bounding_rect();
        bounds.push(start_point);

        let allowed_definition = self.definition_at(start_point);
        let mut visited = FxHashSet::default();

        let mut stack = vec![start_point];
        while let Some(position) = stack.pop() {
            let definition = self.definition_at(position);
            if definition == allowed_definition && !visited.contains(&position) {
                if let Some(random_tile) = brush.tiles.iter().choose(&mut thread_rng()) {
                    self.insert_tile(Tile {
                        position,
                        definition_handle: random_tile.definition_handle,
                    });
                }

                visited.insert(position);

                // Continue on neighbours.
                for neighbour_position in [
                    Vector2::new(position.x - 1, position.y),
                    Vector2::new(position.x + 1, position.y),
                    Vector2::new(position.x, position.y - 1),
                    Vector2::new(position.x, position.y + 1),
                ] {
                    if bounds.contains(neighbour_position) {
                        stack.push(neighbour_position);
                    }
                }
            }
        }
    }

    /// Fills the given rectangle using the specified brush.
    #[inline]
    pub fn rect_fill(&mut self, rect: Rect<i32>, brush: &TileMapBrush) {
        let brush_rect = brush.bounding_rect();
        for y in
            (rect.position.y..(rect.position.y + rect.size.y)).step_by(brush_rect.size.y as usize)
        {
            for x in (rect.position.x..(rect.position.x + rect.size.x))
                .step_by(brush_rect.size.x as usize)
            {
                for brush_tile in brush.tiles.iter() {
                    let position = Vector2::new(x, y) + brush_tile.local_position;
                    if rect.contains(position) {
                        self.insert_tile(Tile {
                            position,
                            definition_handle: brush_tile.definition_handle,
                        });
                    }
                }
            }
        }
    }
}

impl Default for TileMap {
    fn default() -> Self {
        Self {
            base: Default::default(),
            tile_set: Default::default(),
            tiles: Default::default(),
            tile_scale: Vector2::repeat(1.0).into(),
            brushes: Default::default(),
            active_brush: Default::default(),
        }
    }
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
        let rect = self.bounding_rect();

        let min_pos = rect.position.cast::<f32>().to_homogeneous();
        let max_pos = (rect.position + rect.size).cast::<f32>().to_homogeneous();

        AxisAlignedBoundingBox::from_min_max(min_pos, max_pos)
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.local_bounding_box()
            .transform(&self.global_transform())
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn collect_render_data(&self, ctx: &mut RenderContext) -> RdcControlFlow {
        if !self.global_visibility()
            || !self.is_globally_enabled()
            || (self.frustum_culling()
                && !ctx
                    .frustum
                    .map_or(true, |f| f.is_intersects_aabb(&self.world_bounding_box())))
        {
            return RdcControlFlow::Continue;
        }

        if renderer::is_shadow_pass(ctx.render_pass_name) {
            return RdcControlFlow::Continue;
        }

        let Some(ref tile_set_resource) = *self.tile_set else {
            return RdcControlFlow::Continue;
        };

        if !tile_set_resource.is_ok() {
            return RdcControlFlow::Continue;
        }

        let tile_set = tile_set_resource.data_ref();

        for tile in self.tiles.values() {
            let Some(tile_definition) = tile_set.tiles.try_borrow(tile.definition_handle) else {
                continue;
            };

            let global_transform = self.global_transform();

            type Vertex = RectangleVertex;

            let position = tile.position.cast::<f32>().to_homogeneous();

            let vertices = [
                Vertex {
                    position: global_transform
                        .transform_point(&(position + Vector3::new(0.0, 1.0, 0.0)).into())
                        .coords,
                    tex_coord: tile_definition.uv_rect.right_top_corner(),
                    color: tile_definition.color,
                },
                Vertex {
                    position: global_transform
                        .transform_point(&(position + Vector3::new(1.0, 1.0, 0.0)).into())
                        .coords,
                    tex_coord: tile_definition.uv_rect.left_top_corner(),
                    color: tile_definition.color,
                },
                Vertex {
                    position: global_transform
                        .transform_point(&(position + Vector3::new(1.00, 0.0, 0.0)).into())
                        .coords,
                    tex_coord: tile_definition.uv_rect.left_bottom_corner(),
                    color: tile_definition.color,
                },
                Vertex {
                    position: global_transform
                        .transform_point(&(position + Vector3::new(0.0, 0.0, 0.0)).into())
                        .coords,
                    tex_coord: tile_definition.uv_rect.right_bottom_corner(),
                    color: tile_definition.color,
                },
            ];

            let triangles = [TriangleDefinition([0, 1, 2]), TriangleDefinition([2, 3, 0])];

            let sort_index = ctx.calculate_sorting_index(self.global_position());

            ctx.storage.push_triangles(
                RectangleVertex::layout(),
                &tile_definition.material,
                RenderPath::Forward,
                0,
                sort_index,
                false,
                self.self_handle,
                &mut move |mut vertex_buffer, mut triangle_buffer| {
                    let start_vertex_index = vertex_buffer.vertex_count();

                    for vertex in vertices.iter() {
                        vertex_buffer
                            .push_vertex_raw(value_as_u8_slice(vertex))
                            .unwrap();
                    }

                    triangle_buffer
                        .push_triangles_iter_with_offset(start_vertex_index, triangles.into_iter());
                },
            );
        }

        RdcControlFlow::Continue
    }

    fn validate(&self, _scene: &Scene) -> Result<(), String> {
        if self.tile_set.is_none() {
            Err(
                "Tile set resource is not set. Tile map will not be rendered correctly!"
                    .to_string(),
            )
        } else {
            Ok(())
        }
    }
}

/// Tile map builder allows you to create [`TileMap`] scene nodes.
pub struct TileMapBuilder {
    base_builder: BaseBuilder,
    tile_set: Option<TileSetResource>,
    tiles: Tiles,
    tile_scale: Vector2<f32>,
    brushes: Vec<Option<TileMapBrushResource>>,
}

impl TileMapBuilder {
    /// Creates new tile map builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            tile_set: None,
            tiles: Default::default(),
            tile_scale: Vector2::repeat(1.0),
            brushes: Default::default(),
        }
    }

    /// Sets the desired tile set.
    pub fn with_tile_set(mut self, tile_set: TileSetResource) -> Self {
        self.tile_set = Some(tile_set);
        self
    }

    /// Sets the actual tiles of the tile map.
    pub fn with_tiles(mut self, tiles: Tiles) -> Self {
        self.tiles = tiles;
        self
    }

    /// Sets the actual tile scaling.
    pub fn with_tile_scale(mut self, tile_scale: Vector2<f32>) -> Self {
        self.tile_scale = tile_scale;
        self
    }

    /// Sets brushes of the tile map.
    pub fn with_brushes(mut self, brushes: Vec<Option<TileMapBrushResource>>) -> Self {
        self.brushes = brushes;
        self
    }

    /// Builds tile map scene node, but not adds it to a scene graph.
    pub fn build_node(self) -> Node {
        Node::new(TileMap {
            base: self.base_builder.build_base(),
            tile_set: self.tile_set.into(),
            tiles: self.tiles.into(),
            tile_scale: self.tile_scale.into(),
            brushes: self.brushes.into(),
            active_brush: Default::default(),
        })
    }

    /// Finishes tile map building and adds it to the specified scene graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
