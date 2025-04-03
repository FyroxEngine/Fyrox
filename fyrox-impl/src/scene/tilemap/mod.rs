// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Tile map is a 2D "image", made out of a small blocks called tiles. Tile maps used in 2D games to
//! build game worlds quickly and easily. See [`TileMap`] docs for more info and usage examples.

mod autotile;
pub mod brush;
mod data;
mod effect;
mod property;
mod tile_collider;
mod tile_rect;
mod tile_source;
pub mod tileset;
mod transform;
mod update;

pub use autotile::*;
use brush::*;
pub use data::*;
pub use effect::*;
use fxhash::FxHashSet;
use fyrox_core::{
    math::{frustum::Frustum, plane::Plane, ray::Ray},
    parking_lot::Mutex,
};
use fyrox_resource::Resource;
pub use tile_collider::*;
pub use tile_rect::*;
pub use tile_source::*;
use tileset::*;
pub use transform::*;
pub use update::*;

use super::{dim2::rectangle::RectangleVertex, node::constructor::NodeConstructor};
use crate::lazy_static::*;
use crate::{
    asset::{untyped::ResourceKind, ResourceDataRef},
    core::{
        algebra::{Matrix4, Vector2, Vector3},
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, Matrix4Ext, TriangleDefinition},
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        variable::InheritableVariable,
        visitor::prelude::*,
        ImmutableString,
    },
    graph::{constructor::ConstructorProvider, BaseSceneGraph},
    material::{Material, MaterialResource, STANDARD_2D},
    renderer::{self, bundle::RenderContext},
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        mesh::{
            buffer::{
                VertexAttributeDataType, VertexAttributeDescriptor, VertexAttributeUsage,
                VertexTrait,
            },
            RenderPath,
        },
        node::{Node, NodeTrait, RdcControlFlow},
        Scene,
    },
};
use bytemuck::{Pod, Zeroable};
use fyrox_resource::manager::ResourceManager;
use std::{
    error::Error,
    fmt::Display,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

/// Current implementation version marker.
pub const VERSION: u8 = 1;

lazy_static! {
    /// The default material for tiles that have no material set.
    pub static ref DEFAULT_TILE_MATERIAL: MaterialResource = MaterialResource::new_ok(
        uuid!("36bf5b66-b4fa-4bca-80eb-33a271d8f825"),
        ResourceKind::External,
        Material::standard_tile()
    );
}

/// Context for rendering tiles in a tile map. It is especially used by
/// [`TileMapEffect`] objects.
pub struct TileMapRenderContext<'a, 'b> {
    /// The underlying render context that tiles will be rendered into.
    pub context: &'a mut RenderContext<'b>,
    /// The handle of the TileMap.
    tile_map_handle: Handle<Node>,
    /// The global transformation of the TileMap.
    transform: Matrix4<f32>,
    /// The visible tile positions.
    bounds: OptionTileRect,
    hidden_tiles: &'a mut FxHashSet<Vector2<i32>>,
    tile_set: OptionTileSet<'a>,
}

impl TileMapRenderContext<'_, '_> {
    /// The transformation to apply before rendering
    pub fn transform(&self) -> &Matrix4<f32> {
        &self.transform
    }
    /// The handle of the [`TileMap`] node
    pub fn tile_map_handle(&self) -> Handle<Node> {
        self.tile_map_handle
    }
    /// The global position of the TileMap
    pub fn position(&self) -> Vector3<f32> {
        self.transform.position()
    }
    /// The area of tiles that are touching the frustum
    pub fn visible_bounds(&self) -> OptionTileRect {
        self.bounds
    }
    /// Set a position to false in order to prevent later effects from rendering
    /// a tile at this position. All positions are true by default.
    /// Normally, once a tile has been rendered at a position, the position
    /// should be set to false to prevent a second tile from being rendered
    /// at the same position.
    pub fn set_tile_visible(&mut self, position: Vector2<i32>, is_visible: bool) {
        if is_visible {
            let _ = self.hidden_tiles.remove(&position);
        } else {
            let _ = self.hidden_tiles.insert(position);
        }
    }
    /// True if tiles should be rendered at that position.
    /// Normally this should always be checked before rendering a tile
    /// to prevent the rendering from conflicting with some previous
    /// effect that has set the position to false.
    pub fn is_tile_visible(&self, position: Vector2<i32>) -> bool {
        !self.hidden_tiles.contains(&position)
    }
    /// The handle of the tile that should be rendered at the current time in order
    /// to animate the tile at the given handle.
    pub fn get_animated_version(&self, handle: TileDefinitionHandle) -> TileDefinitionHandle {
        self.tile_set
            .get_animated_version(self.context.elapsed_time, handle)
            .unwrap_or(handle)
    }
    /// Render the tile with the given handle at the given position.
    /// Normally [`TileMapRenderContext::is_tile_visible`] should be checked before calling this method
    /// to ensure that tiles are permitted to be rendered at this position,
    /// and then [`TileMapRenderContext::set_tile_visible`] should be used to set the position to false
    /// to prevent any future effects from rendering at this position.
    pub fn draw_tile(&mut self, position: Vector2<i32>, handle: TileDefinitionHandle) {
        if handle.is_empty() {
            return;
        }
        let Some(data) = self.tile_set.get_tile_render_data(handle.into()) else {
            return;
        };
        self.push_tile(position, &data);
    }

    /// Render the given tile data at the given cell position. This makes it possible to render
    /// a tile that is not in the tile map's tile set.
    pub fn push_tile(&mut self, position: Vector2<i32>, data: &TileRenderData) {
        if data.is_empty() {
            return;
        }
        let color = data.color;
        if let Some(tile_bounds) = data.material_bounds.as_ref() {
            let material = &tile_bounds.material;
            let bounds = &tile_bounds.bounds;
            self.push_material_tile(position, material, bounds, color);
        } else {
            self.push_color_tile(position, color);
        }
    }

    fn push_color_tile(&mut self, position: Vector2<i32>, color: Color) {
        let position = position.cast::<f32>();
        let vertices = [(0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)]
            .map(|(x, y)| Vector2::new(x, y))
            .map(|p| make_rect_vertex(&self.transform, position + p, color));

        let triangles = [[0, 1, 2], [2, 3, 0]].map(TriangleDefinition);

        let sort_index = self.context.calculate_sorting_index(self.position());

        self.context.storage.push_triangles(
            self.context.dynamic_surface_cache,
            RectangleVertex::layout(),
            &STANDARD_2D.resource,
            RenderPath::Forward,
            sort_index,
            self.tile_map_handle,
            &mut move |mut vertex_buffer, mut triangle_buffer| {
                let start_vertex_index = vertex_buffer.vertex_count();

                vertex_buffer.push_vertices(&vertices).unwrap();

                triangle_buffer
                    .push_triangles_iter_with_offset(start_vertex_index, triangles.into_iter());
            },
        );
    }

    fn push_material_tile(
        &mut self,
        position: Vector2<i32>,
        material: &MaterialResource,
        bounds: &TileBounds,
        color: Color,
    ) {
        let position = position.cast::<f32>();
        let uvs = [
            bounds.right_top_corner,
            bounds.left_top_corner,
            bounds.left_bottom_corner,
            bounds.right_bottom_corner,
        ];
        let vertices = [
            (1.0, 1.0, uvs[0]),
            (0.0, 1.0, uvs[1]),
            (0.0, 0.0, uvs[2]),
            (1.0, 0.0, uvs[3]),
        ]
        .map(|(x, y, uv)| (Vector2::new(x, y), uv))
        .map(|(p, uv)| make_tile_vertex(&self.transform, position + p, uv, color));

        let triangles = [[0, 1, 2], [2, 3, 0]].map(TriangleDefinition);

        let sort_index = self.context.calculate_sorting_index(self.position());

        self.context.storage.push_triangles(
            self.context.dynamic_surface_cache,
            TileVertex::layout(),
            material,
            RenderPath::Forward,
            sort_index,
            self.tile_map_handle,
            &mut move |mut vertex_buffer, mut triangle_buffer| {
                let start_vertex_index = vertex_buffer.vertex_count();

                vertex_buffer.push_vertices(&vertices).unwrap();

                triangle_buffer
                    .push_triangles_iter_with_offset(start_vertex_index, triangles.into_iter());
            },
        );
    }
}

fn make_rect_vertex(
    transform: &Matrix4<f32>,
    position: Vector2<f32>,
    color: Color,
) -> RectangleVertex {
    RectangleVertex {
        position: transform
            .transform_point(&position.to_homogeneous().into())
            .coords,
        tex_coord: Vector2::default(),
        color,
    }
}

fn make_tile_vertex(
    transform: &Matrix4<f32>,
    position: Vector2<f32>,
    tex_coord: Vector2<u32>,
    color: Color,
) -> TileVertex {
    TileVertex {
        position: transform
            .transform_point(&position.to_homogeneous().into())
            .coords,
        tex_coord: tex_coord.cast::<f32>(),
        color,
    }
}

/// A record whether a change has happened since the most recent save.
#[derive(Default, Debug, Copy, Clone)]
pub struct ChangeFlag(bool);

impl ChangeFlag {
    /// True if there are changes.
    #[inline]
    pub fn needs_save(&self) -> bool {
        self.0
    }
    /// Reset the flag to indicate that there are no unsaved changes.
    #[inline]
    pub fn reset(&mut self) {
        self.0 = false;
    }
    /// Set the flat to indicate that there could be unsaved changes.
    #[inline]
    pub fn set(&mut self) {
        self.0 = true;
    }
}

/// A vertex for tiles.
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
#[repr(C)] // OpenGL expects this structure packed as in C
pub struct TileVertex {
    /// Position of vertex in local coordinates.
    pub position: Vector3<f32>,
    /// Texture coordinates measured in pixels.
    pub tex_coord: Vector2<f32>,
    /// Diffuse color.
    pub color: Color,
}

impl VertexTrait for TileVertex {
    fn layout() -> &'static [VertexAttributeDescriptor] {
        &[
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Position,
                data_type: VertexAttributeDataType::F32,
                size: 3,
                divisor: 0,
                shader_location: 0,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::TexCoord0,
                data_type: VertexAttributeDataType::F32,
                size: 2,
                divisor: 0,
                shader_location: 1,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Color,
                data_type: VertexAttributeDataType::U8,
                size: 4,
                divisor: 0,
                shader_location: 2,
                normalized: true,
            },
        ]
    }
}

/// Each brush and tile set has two palette areas: the pages and the tiles within each page.
/// These two areas are called stages, and each of the two stages needs to be handled separately.
/// Giving a particular `TilePaletteStage` to a tile map palette will control which kind of
/// tiles it will display.
#[derive(Clone, Copy, Default, Debug, Visit, Reflect, PartialEq)]
pub enum TilePaletteStage {
    /// The page tile stage. These tiles allow the user to select which page they want to use.
    #[default]
    Pages,
    /// The stage for tiles within a page.
    Tiles,
}

/// Tile pages come in these types.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PageType {
    /// A page where tiles get their material from a single shared tile atlas,
    /// and the UV coordinates of the tile are based on its grid coordinates.
    Atlas,
    /// A page where each tile can be assigned any material and UV coordinates.
    Freeform,
    /// A page that contains no tile data, but contains handles referencing tiles
    /// on other pages and specifies how tiles can be flipped and rotated.
    Transform,
    /// A page that contains no tile data, but contains handles referencing tiles
    /// on other pages and specifies how tiles animate over time.
    /// Animations proceed from left-to-right, with increasing x-coordinate,
    /// along continuous rows of tiles, until an empty cell is found, and then
    /// the animation returns to the start of the sequence and repeats.
    Animation,
    /// A brush page contains no tile data, but contains handles into a tile set
    /// where tile data can be found.
    Brush,
}

/// The position of a page or a tile within a tile resource.
/// Despite the difference between pages and tiles, they have enough similarities
/// that it is sometimes useful to view them abstractly as the same.
/// Both pages and tiles have a `Vecto2<i32>` position.
/// Both pages and tiles have a TileDefinitionHandle and are rendered using
/// [`TileRenderData`]. For pages this is due to having an icon to allow the user to select the page.
/// Both pages and tiles can be selected by the user, moved, and deleted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResourceTilePosition {
    /// This position refers to some page, and so it lacks tile coordinates.
    Page(Vector2<i32>),
    /// This position refers to some tile, and so it has page coordinates and
    /// the coordinates of the tile within the page.
    Tile(Vector2<i32>, Vector2<i32>),
}

impl Display for ResourceTilePosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceTilePosition::Page(p) => write!(f, "Page({},{})", p.x, p.y),
            ResourceTilePosition::Tile(page, pos) => {
                write!(f, "({},{}):({},{})", page.x, page.y, pos.x, pos.y)
            }
        }
    }
}

impl From<TileDefinitionHandle> for ResourceTilePosition {
    fn from(value: TileDefinitionHandle) -> Self {
        Self::Tile(value.page(), value.tile())
    }
}

impl ResourceTilePosition {
    /// Construct a position from the given stage, page, and tile.
    /// If the stage is [`TilePaletteStage::Pages`] then this position is refering to some page
    /// as if it were a tile, and therefore the `page` argument is ignored and the `tile` argument
    /// is taken as the page's position.
    pub fn new(stage: TilePaletteStage, page: Vector2<i32>, tile: Vector2<i32>) -> Self {
        match stage {
            TilePaletteStage::Pages => Self::Page(tile),
            TilePaletteStage::Tiles => Self::Tile(page, tile),
        }
    }
    /// This position refers to some page.
    pub fn is_page(&self) -> bool {
        matches!(self, Self::Page(_))
    }
    /// This position refers to a tile within a page.
    pub fn is_tile(&self) -> bool {
        matches!(self, Self::Tile(_, _))
    }
    /// The stage that contains this position.
    pub fn stage(&self) -> TilePaletteStage {
        match self {
            Self::Page(_) => TilePaletteStage::Pages,
            Self::Tile(_, _) => TilePaletteStage::Tiles,
        }
    }
    /// The position within the stage. For a page position, this is the page's coordinates.
    /// For a tile position, this is the tile's coordinates.
    pub fn stage_position(&self) -> Vector2<i32> {
        match self {
            Self::Page(p) => *p,
            Self::Tile(_, p) => *p,
        }
    }
    /// The page coordinates of the position. For a page position, this is
    pub fn page(&self) -> Vector2<i32> {
        match self {
            Self::Page(p) => *p,
            Self::Tile(p, _) => *p,
        }
    }
    /// The handle associated with this position, if this is a tile position.
    pub fn handle(&self) -> Option<TileDefinitionHandle> {
        if let Self::Tile(p, t) = self {
            TileDefinitionHandle::try_new(*p, *t)
        } else {
            None
        }
    }
}

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

/// Adapt an iterator over positions into an iterator over `(Vector2<i32>, TileHandleDefinition)`.
#[derive(Debug, Clone)]
pub struct TileIter<I> {
    source: TileBook,
    stage: TilePaletteStage,
    page: Vector2<i32>,
    positions: I,
}

impl<I: Iterator<Item = Vector2<i32>>> Iterator for TileIter<I> {
    type Item = (Vector2<i32>, TileDefinitionHandle);

    fn next(&mut self) -> Option<Self::Item> {
        self.positions.find_map(|p| {
            let h = self
                .source
                .get_tile_handle(ResourceTilePosition::new(self.stage, self.page, p))?;
            Some((p, h))
        })
    }
}

#[derive(Debug, Default, Clone, PartialEq, Visit, Reflect)]
/// Abstract source of tiles, which can either be a tile set or a brush.
/// It is called a "book" because each of these tile resources contains
/// pages of tiles.
pub enum TileBook {
    /// A tile resource containing no tiles.
    #[default]
    Empty,
    /// Getting tiles from a tile set
    TileSet(TileSetResource),
    /// Getting tiles from a brush
    Brush(TileMapBrushResource),
}

impl TileBook {
    /// The TileDefinitionHandle of the icon that represents the page at the given position.
    #[inline]
    pub fn page_icon(&self, position: Vector2<i32>) -> Option<TileDefinitionHandle> {
        match self {
            TileBook::Empty => None,
            TileBook::TileSet(r) => r.state().data()?.page_icon(position),
            TileBook::Brush(r) => r.state().data()?.page_icon(position),
        }
    }
    /// Returns true if this resource is a tile set.
    #[inline]
    pub fn is_tile_set(&self) -> bool {
        matches!(self, TileBook::TileSet(_))
    }
    /// Returns true if this resource is a brush.
    #[inline]
    pub fn is_brush(&self) -> bool {
        matches!(self, TileBook::Brush(_))
    }
    /// Returns true if this contains no resource.
    #[inline]
    pub fn is_empty(&self) -> bool {
        matches!(self, TileBook::Empty)
    }
    /// Return the path of the resource as a String.
    pub fn name(&self, resource_manager: &ResourceManager) -> String {
        self.path(resource_manager)
            .map(|x| x.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Error".into())
    }
    /// Return the path of the resource.
    pub fn path(&self, resource_manager: &ResourceManager) -> Option<PathBuf> {
        match self {
            TileBook::Empty => None,
            TileBook::TileSet(r) => resource_manager.resource_path(r.as_ref()),
            TileBook::Brush(r) => resource_manager.resource_path(r.as_ref()),
        }
    }
    /// True if the resource is external and its `change_count` is not zero.
    pub fn needs_save(&self) -> bool {
        match self {
            TileBook::Empty => false,
            TileBook::TileSet(r) => {
                r.header().kind.is_external() && r.data_ref().change_flag.needs_save()
            }
            TileBook::Brush(r) => {
                r.header().kind.is_external() && r.data_ref().change_flag.needs_save()
            }
        }
    }
    /// Attempt to save the resource to its file, if it has one and if `change_flag` is set.
    /// Otherwise do nothing and return Ok to indicate success.
    pub fn save(&self, resource_manager: &ResourceManager) -> Result<(), Box<dyn Error>> {
        match self {
            TileBook::Empty => Ok(()),
            TileBook::TileSet(r) => {
                if r.header().kind.is_external() && r.data_ref().change_flag.needs_save() {
                    let result = r.save(&resource_manager.resource_path(r.as_ref()).unwrap());
                    if result.is_ok() {
                        r.data_ref().change_flag.reset();
                    }
                    result
                } else {
                    Ok(())
                }
            }
            TileBook::Brush(r) => {
                if r.header().kind.is_external() && r.data_ref().change_flag.needs_save() {
                    let result = r.save(&resource_manager.resource_path(r.as_ref()).unwrap());
                    if result.is_ok() {
                        r.data_ref().change_flag.reset();
                    }
                    result
                } else {
                    Ok(())
                }
            }
        }
    }
    /// A reference to the TileSetResource, if this is a TileSetResource.
    pub fn tile_set_ref(&self) -> Option<&TileSetResource> {
        match self {
            TileBook::TileSet(r) => Some(r),
            _ => None,
        }
    }
    /// A reference to the TileMapBrushResource, if this is a TileMapBrushResource.
    pub fn brush_ref(&self) -> Option<&TileMapBrushResource> {
        match self {
            TileBook::Brush(r) => Some(r),
            _ => None,
        }
    }
    /// Returns the tile set associated with this resource.
    /// If the resource is a tile set, the return that tile set.
    /// If the resource is a brush, then return the tile set used by that brush.
    pub fn get_tile_set(&self) -> Option<TileSetResource> {
        match self {
            TileBook::Empty => None,
            TileBook::TileSet(r) => Some(r.clone()),
            TileBook::Brush(r) => r.state().data()?.tile_set(),
        }
    }
    /// Build a list of the positions of all tiles on the given page.
    pub fn get_all_tile_positions(&self, page: Vector2<i32>) -> Vec<Vector2<i32>> {
        match self {
            TileBook::Empty => Vec::new(),
            TileBook::TileSet(r) => r
                .state()
                .data()
                .map(|r| r.keys_on_page(page))
                .unwrap_or_default(),
            TileBook::Brush(r) => r
                .state()
                .data()
                .and_then(|r| {
                    r.pages
                        .get(&page)
                        .map(|p| p.tiles.keys().copied().collect())
                })
                .unwrap_or_default(),
        }
    }
    /// Build a list of the posiitons of all pages.
    pub fn get_all_page_positions(&self) -> Vec<Vector2<i32>> {
        match self {
            TileBook::Empty => Vec::new(),
            TileBook::TileSet(r) => r.state().data().map(|r| r.page_keys()).unwrap_or_default(),
            TileBook::Brush(r) => r
                .state()
                .data()
                .map(|r| r.pages.keys().copied().collect())
                .unwrap_or_default(),
        }
    }
    /// True if there is a page at the given position.
    pub fn has_page_at(&self, position: Vector2<i32>) -> bool {
        match self {
            TileBook::Empty => false,
            TileBook::TileSet(r) => r
                .state()
                .data()
                .map(|r| r.pages.contains_key(&position))
                .unwrap_or(false),
            TileBook::Brush(r) => r
                .state()
                .data()
                .map(|r| r.pages.contains_key(&position))
                .unwrap_or(false),
        }
    }
    /// The type of the page at the given position, if there is one.
    pub fn page_type(&self, position: Vector2<i32>) -> Option<PageType> {
        match self {
            TileBook::Empty => None,
            TileBook::TileSet(r) => r.state().data()?.get_page(position).map(|p| p.page_type()),
            TileBook::Brush(r) => {
                if r.state().data()?.has_page_at(position) {
                    Some(PageType::Brush)
                } else {
                    None
                }
            }
        }
    }
    /// True if there is a atlas page at the given coordinates.
    pub fn is_atlas_page(&self, position: Vector2<i32>) -> bool {
        self.page_type(position) == Some(PageType::Atlas)
    }
    /// True if there is a free tile page at the given coordinates.
    pub fn is_free_page(&self, position: Vector2<i32>) -> bool {
        self.page_type(position) == Some(PageType::Freeform)
    }
    /// True if there is a transform page at the given coordinates.
    pub fn is_transform_page(&self, position: Vector2<i32>) -> bool {
        self.page_type(position) == Some(PageType::Transform)
    }
    /// True if there is a transform page at the given coordinates.
    pub fn is_animation_page(&self, position: Vector2<i32>) -> bool {
        self.page_type(position) == Some(PageType::Animation)
    }
    /// True if there is a brush page at the given coordinates.
    pub fn is_brush_page(&self, position: Vector2<i32>) -> bool {
        self.page_type(position) == Some(PageType::Brush)
    }
    /// Return true if there is a tile at the given position on the page at the given position.
    pub fn has_tile_at(&self, page: Vector2<i32>, tile: Vector2<i32>) -> bool {
        match self {
            TileBook::Empty => false,
            TileBook::TileSet(r) => r
                .state()
                .data()
                .map(|r| r.has_tile_at(page, tile))
                .unwrap_or(false),
            TileBook::Brush(r) => r
                .state()
                .data()
                .map(|r| r.has_tile_at(page, tile))
                .unwrap_or(false),
        }
    }
    /// Returns the TileDefinitionHandle that points to the data in the tile set that represents this tile.
    /// Even if this resource is actually a brush, the handle returned still refers to some page and position
    /// in the brush's tile set.
    pub fn get_tile_handle(&self, position: ResourceTilePosition) -> Option<TileDefinitionHandle> {
        match self {
            TileBook::Empty => None,
            TileBook::TileSet(r) => r.state().data()?.redirect_handle(position),
            TileBook::Brush(r) => r.state().data()?.redirect_handle(position),
        }
    }
    /// The StampElement for the given position in this resource.
    /// For brushes, the [`StampElement::handle`] refers to the ultimate location of the tile within the
    /// tile set, while the [`StampElement::source`] refers to the location of the tile within
    /// the brush or tile set that was used to create the stamp.
    pub fn get_stamp_element(&self, position: ResourceTilePosition) -> Option<StampElement> {
        match self {
            TileBook::Empty => None,
            TileBook::TileSet(r) => r.state().data()?.stamp_element(position),
            TileBook::Brush(r) => r.state().data()?.stamp_element(position),
        }
    }
    /// Returns an iterator over `(Vector2<i32>, TileDefinitionHandle)` where the first
    /// member of the pair is the position of the tile on the page as provided by `positions`
    /// and the second member is the handle that would be returned from [`get_tile_handle`](Self::get_tile_handle).
    pub fn get_tile_iter<I: Iterator<Item = Vector2<i32>>>(
        &self,
        stage: TilePaletteStage,
        page: Vector2<i32>,
        positions: I,
    ) -> TileIter<I> {
        TileIter {
            source: self.clone(),
            stage,
            page,
            positions,
        }
    }
    /// Construct a Tiles object holding the tile definition handles for the tiles
    /// at the given positions on the given page.
    pub fn get_tiles<I: Iterator<Item = Vector2<i32>>>(
        &self,
        stage: TilePaletteStage,
        page: Vector2<i32>,
        iter: I,
        tiles: &mut Tiles,
    ) {
        match self {
            TileBook::Empty => (),
            TileBook::TileSet(res) => {
                if let Some(tile_set) = res.state().data() {
                    tile_set.get_tiles(stage, page, iter, tiles);
                }
            }
            TileBook::Brush(res) => {
                if let Some(brush) = res.state().data() {
                    brush.get_tiles(stage, page, iter, tiles);
                }
            }
        }
    }

    /// Returns true if the resource is a brush that has no tile set.
    pub fn is_missing_tile_set(&self) -> bool {
        match self {
            TileBook::Empty => false,
            TileBook::TileSet(_) => false,
            TileBook::Brush(resource) => resource
                .state()
                .data()
                .map(|b| b.is_missing_tile_set())
                .unwrap_or(false),
        }
    }

    /// Return the `TileRenderData` needed to render the tile at the given position on the given page.
    /// If there is no tile at that position or the tile set is missing or not loaded, then None is returned.
    /// If there is a tile and a tile set, but the handle of the tile does not exist in the tile set,
    /// then the rendering data for an error tile is returned using `TileRenderData::missing_tile()`.
    ///
    /// Beware that this method is *slow.* Like most methods in `TileBook`, this method involves
    /// locking a resource, so none of them should be called many times per frame, as one might be
    /// tempted to do with this method. Do *not* render a `TileBook` by repeatedly calling this
    /// method. Use [`tile_render_loop`](Self::tile_render_loop) instead.
    pub fn get_tile_render_data(&self, position: ResourceTilePosition) -> Option<TileRenderData> {
        match self {
            TileBook::Empty => None,
            TileBook::TileSet(resource) => resource.state().data()?.get_tile_render_data(position),
            TileBook::Brush(resource) => resource.state().data()?.get_tile_render_data(position),
        }
    }

    /// Repeatedly call the given function with each tile for the given stage and page.
    /// The function is given the position of the tile within the palette and the
    /// data for rendering the tile.
    pub fn tile_render_loop<F>(&self, stage: TilePaletteStage, page: Vector2<i32>, func: F)
    where
        F: FnMut(Vector2<i32>, TileRenderData),
    {
        match self {
            TileBook::Empty => (),
            TileBook::TileSet(res) => {
                if let Some(data) = res.state().data() {
                    data.palette_render_loop(stage, page, func)
                }
            }
            TileBook::Brush(res) => {
                if let Some(data) = res.state().data() {
                    data.palette_render_loop(stage, page, func)
                }
            }
        };
    }
    /// Repeatedly call the given function with each collider for each tile on the given page.
    /// The function is given the position of the tile
    pub fn tile_collider_loop<F>(&self, page: Vector2<i32>, func: F)
    where
        F: FnMut(Vector2<i32>, Uuid, Color, &TileCollider),
    {
        match self {
            TileBook::Empty => (),
            TileBook::TileSet(res) => {
                if let Some(data) = res.state().data() {
                    data.tile_collider_loop(page, func)
                }
            }
            TileBook::Brush(_) => (),
        };
    }
    /// Returns the rectangle within a material that a tile should show
    /// at the given stage and handle.
    pub fn get_tile_bounds(&self, position: ResourceTilePosition) -> Option<TileMaterialBounds> {
        match self {
            TileBook::Empty => None,
            TileBook::TileSet(res) => res
                .state()
                .data()
                .map(|d| d.get_tile_bounds(position))
                .unwrap_or_default(),
            TileBook::Brush(res) => res
                .state()
                .data()
                .map(|d| d.get_tile_bounds(position))
                .unwrap_or_default(),
        }
    }
    /// The bounds of the tiles on the given page.
    pub fn tiles_bounds(&self, stage: TilePaletteStage, page: Vector2<i32>) -> OptionTileRect {
        match self {
            TileBook::Empty => OptionTileRect::default(),
            TileBook::TileSet(res) => res.data_ref().tiles_bounds(stage, page),
            TileBook::Brush(res) => res.data_ref().tiles_bounds(stage, page),
        }
    }
}

/// The specification for how to render a tile.
#[derive(Clone, Default, Debug)]
pub struct TileRenderData {
    /// The material to use to render this tile.
    pub material_bounds: Option<TileMaterialBounds>,
    /// The color to use to render the tile
    pub color: Color,
    /// This data represents the empty tile
    empty: bool,
}

impl TileRenderData {
    /// Create a TileRenderData
    pub fn new(material_bounds: Option<TileMaterialBounds>, color: Color) -> Self {
        Self {
            material_bounds,
            color,
            empty: false,
        }
    }
    /// Create an empty TileRenderData
    pub fn empty() -> Self {
        Self {
            material_bounds: None,
            color: Color::WHITE,
            empty: true,
        }
    }
    /// True if the render data is for the empty tile.
    pub fn is_empty(&self) -> bool {
        self.empty
    }
    /// Returns TileRenderData to represent an error due to render data being unavailable.
    pub fn missing_data() -> Self {
        Self {
            material_bounds: None,
            color: Color::HOT_PINK,
            empty: false,
        }
    }
}

impl OrthoTransform for TileRenderData {
    fn x_flipped(mut self) -> Self {
        self.material_bounds = self.material_bounds.map(|b| b.x_flipped());
        self
    }

    fn rotated(mut self, amount: i8) -> Self {
        self.material_bounds = self.material_bounds.map(|b| b.rotated(amount));
        self
    }
}

/// Tile map is a 2D "image", made out of a small blocks called tiles. Tile maps used in 2D games to
/// build game worlds quickly and easily. Each tile is represented by a [`TileDefinitionHandle`] which
/// contains the position of a page and the position of a tile within that page.
///
/// When rendering the `TileMap`, the rendering data is fetched from the tile map's tile set resource,
/// which contains all the pages that may be referenced by the tile map's handles.
///
/// Optional [`TileMapEffect`] objects may be included in the `TileMap` to change how it renders.
#[derive(Reflect, Debug, ComponentProvider, TypeUuidProvider)]
#[reflect(derived_type = "Node")]
#[type_uuid(id = "aa9a3385-a4af-4faf-a69a-8d3af1a3aa67")]
pub struct TileMap {
    base: Base,
    /// The source of rendering data for tiles in this tile map.
    tile_set: InheritableVariable<Option<TileSetResource>>,
    /// Tile container of the tile map.
    #[reflect(hidden)]
    pub tiles: InheritableVariable<Option<TileMapDataResource>>,
    tile_scale: InheritableVariable<Vector2<f32>>,
    active_brush: InheritableVariable<Option<TileMapBrushResource>>,
    /// Temporary space to store which tiles are invisible during `collect_render_data`.
    /// This is part of how [`TileMapEffect`] can prevent a tile from being rendered.
    #[reflect(hidden)]
    hidden_tiles: Mutex<FxHashSet<Vector2<i32>>>,
    /// Special rendering effects that may change how the tile map renders.
    /// These effects are processed in order before the tile map performs the
    /// normal rendering of tiles, and they can prevent some times from being
    /// rendered and render other tiles in place of what would normally be
    /// rendered.
    #[reflect(hidden)]
    pub before_effects: Vec<TileMapEffectRef>,
    /// Special rendering effects that may change how the tile map renders.
    /// These effects are processed in order after the tile map performs the
    /// normal rendering of tiles.
    #[reflect(hidden)]
    pub after_effects: Vec<TileMapEffectRef>,
}

impl Visit for TileMap {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;
        let mut version = if region.is_reading() { 0 } else { VERSION };
        let _ = version.visit("Version", &mut region);
        self.base.visit("Base", &mut region)?;
        self.tile_set.visit("TileSet", &mut region)?;
        self.tile_scale.visit("TileScale", &mut region)?;
        self.active_brush.visit("ActiveBrush", &mut region)?;
        match version {
            0 => {
                let mut tiles = InheritableVariable::new_non_modified(Tiles::default());
                let result = tiles.visit("Tiles", &mut region);
                result?;
                let mut data = TileMapData::default();
                for (p, h) in tiles.iter() {
                    data.set(*p, *h);
                }
                self.tiles = Some(Resource::new_ok(
                    Uuid::new_v4(),
                    ResourceKind::Embedded,
                    data,
                ))
                .into();
            }
            VERSION => {
                self.tiles.visit("Tiles", &mut region)?;
            }
            _ => return Err(VisitError::User("Unknown version".into())),
        }
        Ok(())
    }
}

/// A reference to the tile data of a some tile in a tile set.
pub struct TileMapDataRef<'a> {
    tile_set: ResourceDataRef<'a, TileSet>,
    handle: TileDefinitionHandle,
}

impl Deref for TileMapDataRef<'_> {
    type Target = TileData;

    fn deref(&self) -> &Self::Target {
        self.tile_set.tile_data(self.handle).unwrap()
    }
}

/// An error in finding a property for a tile.
#[derive(Debug)]
pub enum TilePropertyError {
    /// The tile map has no tile set, so no tile data is available.
    MissingTileSet,
    /// The tile map has a tile set, but it is not yet loaded.
    TileSetNotLoaded,
    /// There is no property with the given name in the tile set.
    UnrecognizedName(ImmutableString),
    /// There is no property with the given UUID in the tile set.
    UnrecognizedUuid(Uuid),
    /// The property has the wrong type.
    WrongType(&'static str),
}

impl Display for TilePropertyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TilePropertyError::MissingTileSet => write!(f, "The tile map has no tile set."),
            TilePropertyError::TileSetNotLoaded => {
                write!(f, "The tile map's tile set is not loaded.")
            }
            TilePropertyError::UnrecognizedName(name) => {
                write!(f, "There is no property with this name: {name}")
            }
            TilePropertyError::UnrecognizedUuid(uuid) => {
                write!(f, "There is no property with this UUID: {uuid}")
            }
            TilePropertyError::WrongType(message) => write!(f, "Property type error: {message}"),
        }
    }
}

impl Error for TilePropertyError {}

impl TileMap {
    /// The handle that is stored in the tile map at the given position to refer to some tile in the tile set.
    pub fn tile_handle(&self, position: Vector2<i32>) -> Option<TileDefinitionHandle> {
        let tiles = self.tiles.as_ref()?.data_ref();
        tiles.as_loaded_ref()?.get(position)
    }
    /// The tile data for the tile at the given position, if that position has a tile and this tile map
    /// has a tile set that contains data for the tile's handle.
    pub fn tile_data(&self, position: Vector2<i32>) -> Option<TileMapDataRef> {
        let handle = self.tile_handle(position)?;
        let tile_set = self.tile_set.as_ref()?.data_ref();
        if tile_set.as_loaded_ref()?.tile_data(handle).is_some() {
            Some(TileMapDataRef { tile_set, handle })
        } else {
            None
        }
    }
    /// The property value for the property of the given name for the tile at the given position in this tile map.
    /// This requires that the tile map has a loaded tile set and the tile set contains a property with the given name.
    /// Otherwise an error is returned to indicate which of these conditions failed.
    /// If the only problem is that there is no tile at the given position, then the default value for the property's value type
    /// is returned.
    pub fn tile_property_value<T>(
        &self,
        position: Vector2<i32>,
        property_id: Uuid,
    ) -> Result<T, TilePropertyError>
    where
        T: TryFrom<TileSetPropertyValue, Error = TilePropertyError> + Default,
    {
        let Some(handle) = self.tile_handle(position) else {
            return Ok(T::default());
        };
        let tile_set = self
            .tile_set
            .as_ref()
            .ok_or(TilePropertyError::MissingTileSet)?
            .data_ref();
        let tile_set = tile_set
            .as_loaded_ref()
            .ok_or(TilePropertyError::TileSetNotLoaded)?;
        tile_set.tile_property_value(handle, property_id)
    }
    /// The property value for the property of the given name for the tile at the given position in this tile map.
    /// This requires that the tile map has a loaded tile set and the tile set contains a property with the given name.
    /// Otherwise an error is returned to indicate which of these conditions failed.
    /// If the only problem is that there is no tile at the given position, then the default value for the property's value type
    /// is returned.
    pub fn tile_property_value_by_name(
        &self,
        position: Vector2<i32>,
        property_name: &ImmutableString,
    ) -> Result<TileSetPropertyValue, TilePropertyError> {
        let tile_set = self
            .tile_set
            .as_ref()
            .ok_or(TilePropertyError::MissingTileSet)?
            .data_ref();
        let tile_set = tile_set
            .as_loaded_ref()
            .ok_or(TilePropertyError::TileSetNotLoaded)?;
        let property = tile_set
            .find_property_by_name(property_name)
            .ok_or_else(|| TilePropertyError::UnrecognizedName(property_name.clone()))?;
        let Some(handle) = self.tile_handle(position) else {
            return Ok(property.prop_type.default_value());
        };
        Ok(tile_set
            .property_value(handle, property.uuid)
            .unwrap_or_else(|| property.prop_type.default_value()))
    }
    /// The property value for the property of the given UUID for the tile at the given position in this tile map.
    /// This requires that the tile map has a loaded tile set and the tile set contains a property with the given UUID.
    /// Otherwise an error is returned to indicate which of these conditions failed.
    /// If the only problem is that there is no tile at the given position, then the default value for the property's value type
    /// is returned.
    pub fn tile_property_value_by_uuid_untyped(
        &self,
        position: Vector2<i32>,
        property_id: Uuid,
    ) -> Result<TileSetPropertyValue, TilePropertyError> {
        let tile_set = self
            .tile_set
            .as_ref()
            .ok_or(TilePropertyError::MissingTileSet)?
            .data_ref();
        let tile_set = tile_set
            .as_loaded_ref()
            .ok_or(TilePropertyError::TileSetNotLoaded)?;
        let value = if let Some(handle) = self.tile_handle(position) {
            tile_set
                .tile_data(handle)
                .and_then(|d| d.properties.get(&property_id))
                .cloned()
        } else {
            None
        };
        if let Some(value) = value {
            Ok(value)
        } else {
            let property = tile_set
                .find_property(property_id)
                .ok_or(TilePropertyError::UnrecognizedUuid(property_id))?;
            Ok(property.prop_type.default_value())
        }
    }
    /// The global transform of the tile map with initial x-axis flip applied, so the positive x-axis points left instead of right.
    pub fn tile_map_transform(&self) -> Matrix4<f32> {
        self.global_transform()
            .prepend_nonuniform_scaling(&Vector3::new(-1.0, 1.0, 1.0))
    }
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
    pub fn tiles(&self) -> Option<&TileMapDataResource> {
        self.tiles.as_ref()
    }

    /// Sets new tiles.
    #[inline]
    pub fn set_tiles(&mut self, tiles: TileMapDataResource) {
        self.tiles.set_value_and_mark_modified(Some(tiles));
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
    pub fn insert_tile(
        &mut self,
        position: Vector2<i32>,
        tile: TileDefinitionHandle,
    ) -> Option<TileDefinitionHandle> {
        self.tiles
            .as_ref()?
            .data_ref()
            .as_loaded_mut()?
            .replace(position, Some(tile))
    }

    /// Removes a tile from the tile map.
    #[inline]
    pub fn remove_tile(&mut self, position: Vector2<i32>) -> Option<TileDefinitionHandle> {
        self.tiles
            .as_ref()?
            .data_ref()
            .as_loaded_mut()?
            .replace(position, None)
    }

    /// Returns active brush of the tile map.
    #[inline]
    pub fn active_brush(&self) -> Option<&TileMapBrushResource> {
        self.active_brush.as_ref()
    }

    /// Sets new active brush of the tile map.
    #[inline]
    pub fn set_active_brush(&mut self, brush: Option<TileMapBrushResource>) {
        self.active_brush.set_value_and_mark_modified(brush);
    }

    /// Calculates bounding rectangle in grid coordinates.
    #[inline]
    pub fn bounding_rect(&self) -> OptionTileRect {
        let Some(tiles) = self.tiles.as_ref().map(|r| r.data_ref()) else {
            return OptionTileRect::default();
        };
        let Some(tiles) = tiles.as_loaded_ref() else {
            return OptionTileRect::default();
        };
        tiles.bounding_rect()
    }

    /// Calculates grid-space position (tile coordinates) from world-space. Could be used to find
    /// tile coordinates from arbitrary point in world space. It is especially useful, if the tile
    /// map is rotated or shifted.
    #[inline]
    pub fn world_to_grid(&self, world_position: Vector3<f32>) -> Vector2<i32> {
        let inv_global_transform = self.tile_map_transform().try_inverse().unwrap_or_default();
        let local_space_position = inv_global_transform.transform_point(&world_position.into());
        Vector2::new(
            local_space_position.x.floor() as i32,
            local_space_position.y.floor() as i32,
        )
    }

    /// Calculates world-space position from grid-space position (tile coordinates).
    #[inline]
    pub fn grid_to_world(&self, grid_position: Vector2<i32>) -> Vector3<f32> {
        let v3 = grid_position.cast::<f32>().to_homogeneous();
        self.tile_map_transform().transform_point(&v3.into()).coords
    }

    fn cells_touching_frustum(&self, frustum: &Frustum) -> OptionTileRect {
        let global_transform = self.global_transform();

        fn make_ray(a: Vector3<f32>, b: Vector3<f32>) -> Ray {
            Ray {
                origin: a,
                dir: b - a,
            }
        }

        let left_top_ray = make_ray(
            frustum.left_top_front_corner(),
            frustum.left_top_back_corner(),
        );
        let right_top_ray = make_ray(
            frustum.right_top_front_corner(),
            frustum.right_top_back_corner(),
        );
        let left_bottom_ray = make_ray(
            frustum.left_bottom_front_corner(),
            frustum.left_bottom_back_corner(),
        );
        let right_bottom_ray = make_ray(
            frustum.right_bottom_front_corner(),
            frustum.right_bottom_back_corner(),
        );

        let plane =
            Plane::from_normal_and_point(&global_transform.look(), &global_transform.position())
                .unwrap_or_default();

        let Some(left_top) = left_top_ray.plane_intersection_point(&plane) else {
            return None.into();
        };
        let Some(right_top) = right_top_ray.plane_intersection_point(&plane) else {
            return None.into();
        };
        let Some(left_bottom) = left_bottom_ray.plane_intersection_point(&plane) else {
            return None.into();
        };
        let Some(right_bottom) = right_bottom_ray.plane_intersection_point(&plane) else {
            return None.into();
        };
        let mut bounds = OptionTileRect::default();
        for corner in [left_top, right_top, left_bottom, right_bottom] {
            bounds.push(self.world_to_grid(corner))
        }
        bounds
    }
}

impl Default for TileMap {
    fn default() -> Self {
        Self {
            base: Default::default(),
            tile_set: Default::default(),
            tiles: Default::default(),
            tile_scale: Vector2::repeat(1.0).into(),
            active_brush: Default::default(),
            hidden_tiles: Mutex::default(),
            before_effects: Vec::default(),
            after_effects: Vec::default(),
        }
    }
}

impl Clone for TileMap {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            tile_set: self.tile_set.clone(),
            tiles: self.tiles.clone(),
            tile_scale: self.tile_scale.clone(),
            active_brush: self.active_brush.clone(),
            hidden_tiles: Mutex::default(),
            before_effects: self.before_effects.clone(),
            after_effects: self.after_effects.clone(),
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

impl ConstructorProvider<Node, Graph> for TileMap {
    fn constructor() -> NodeConstructor {
        NodeConstructor::new::<Self>()
            .with_variant("Tile Map", |_| {
                TileMapBuilder::new(BaseBuilder::new().with_name("Tile Map"))
                    .build_node()
                    .into()
            })
            .with_group("2D")
    }
}

impl NodeTrait for TileMap {
    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        let Some(rect) = *self.bounding_rect() else {
            return AxisAlignedBoundingBox::default();
        };

        let mut min_pos = rect.position.cast::<f32>().to_homogeneous();
        let mut max_pos = (rect.position + rect.size).cast::<f32>().to_homogeneous();
        min_pos.x *= -1.0;
        max_pos.x *= -1.0;
        let (min, max) = min_pos.inf_sup(&max_pos);

        AxisAlignedBoundingBox::from_min_max(min, max)
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.local_bounding_box()
            .transform(&self.global_transform())
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn collect_render_data(&self, ctx: &mut RenderContext) -> RdcControlFlow {
        if !self.should_be_rendered(ctx.frustum) {
            return RdcControlFlow::Continue;
        }

        if renderer::is_shadow_pass(ctx.render_pass_name) {
            return RdcControlFlow::Continue;
        }

        let Some(ref tile_set_resource) = *self.tile_set else {
            return RdcControlFlow::Continue;
        };

        let mut tile_set_lock = TileSetRef::new(tile_set_resource);
        let tile_set = tile_set_lock.as_loaded();

        let mut hidden_tiles = self.hidden_tiles.lock();
        hidden_tiles.clear();

        let bounds = ctx
            .frustum
            .as_ref()
            .map(|f| self.cells_touching_frustum(f))
            .unwrap_or_default();

        let mut tile_render_context = TileMapRenderContext {
            tile_map_handle: self.handle(),
            transform: self.tile_map_transform(),
            hidden_tiles: &mut hidden_tiles,
            context: ctx,
            bounds,
            tile_set,
        };

        for effect in self.before_effects.iter() {
            effect.lock().render_special_tiles(&mut tile_render_context);
        }
        let bounds = tile_render_context.visible_bounds();
        let Some(tiles) = self.tiles.as_ref().map(|r| r.data_ref()) else {
            return RdcControlFlow::Continue;
        };
        let Some(tiles) = tiles.as_loaded_ref() else {
            return RdcControlFlow::Continue;
        };
        if bounds.is_some() {
            for (position, handle) in tiles.bounded_iter(bounds) {
                if bounds.contains(position) && tile_render_context.is_tile_visible(position) {
                    let handle = tile_render_context.get_animated_version(handle);
                    tile_render_context.draw_tile(position, handle);
                }
            }
        } else {
            for (position, handle) in tiles.iter() {
                if tile_render_context.is_tile_visible(position) {
                    let handle = tile_render_context.get_animated_version(handle);
                    tile_render_context.draw_tile(position, handle);
                }
            }
        }
        for effect in self.after_effects.iter() {
            effect.lock().render_special_tiles(&mut tile_render_context);
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
    tiles: TileMapData,
    tile_scale: Vector2<f32>,
    before_effects: Vec<TileMapEffectRef>,
    after_effects: Vec<TileMapEffectRef>,
}

impl TileMapBuilder {
    /// Creates new tile map builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            tile_set: None,
            tiles: TileMapData::default(),
            tile_scale: Vector2::repeat(1.0),
            before_effects: Default::default(),
            after_effects: Default::default(),
        }
    }

    /// Sets the desired tile set.
    pub fn with_tile_set(mut self, tile_set: TileSetResource) -> Self {
        self.tile_set = Some(tile_set);
        self
    }

    /// Sets the actual tiles of the tile map.
    pub fn with_tiles(mut self, tiles: &Tiles) -> Self {
        for (pos, handle) in tiles.iter() {
            self.tiles.set(*pos, *handle);
        }
        self
    }

    /// Sets the actual tile scaling.
    pub fn with_tile_scale(mut self, tile_scale: Vector2<f32>) -> Self {
        self.tile_scale = tile_scale;
        self
    }

    /// Adds an effect to the tile map which will run before the tiles render.
    pub fn with_before_effect(mut self, effect: TileMapEffectRef) -> Self {
        self.before_effects.push(effect);
        self
    }

    /// Adds an effect to the tile map which will run after the tiles render.
    pub fn with_after_effect(mut self, effect: TileMapEffectRef) -> Self {
        self.after_effects.push(effect);
        self
    }

    /// Builds tile map scene node, but not adds it to a scene graph.
    pub fn build_node(self) -> Node {
        Node::new(TileMap {
            base: self.base_builder.build_base(),
            tile_set: self.tile_set.into(),
            tiles: Some(Resource::new_ok(
                Uuid::new_v4(),
                ResourceKind::Embedded,
                self.tiles,
            ))
            .into(),
            tile_scale: self.tile_scale.into(),
            active_brush: Default::default(),
            hidden_tiles: Mutex::default(),
            before_effects: self.before_effects,
            after_effects: self.after_effects,
        })
    }

    /// Finishes tile map building and adds it to the specified scene graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
