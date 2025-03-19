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

//! Rendering a tile map is often not as simple as rendering each tile in its
//! proper place. Sometimes tiles need to be made invisible or replaced by other
//! tiles. Sometimes additional rendering needs to happen along with some tile, such
//! as for creating highlights or other special effects.
//!
//! For this purpose, a tile map may optionally have a list of [`TileMapEffect`]
//! references which can intervene in the rendering process of the tile map.

use crate::{
    core::{
        algebra::{Matrix4, Vector2, Vector3, Vector4},
        parking_lot::Mutex,
    },
    scene::mesh::vertex::StaticVertex,
};
use fxhash::{FxHashMap, FxHashSet};
use std::{fmt::Debug, sync::Arc};

use super::*;

/// A reference to [`TileMapEffect`]. A TileMap keeps some of these if it needs to be
/// rendered specially.
pub type TileMapEffectRef = Arc<Mutex<dyn TileMapEffect>>;

/// A trait for objects that can perform specialized rendering for a tile map by
/// adding them to [`TileMap::before_effects`] or [`TileMap::after_effects`],
/// depending on whether the effect should render before the tile map renders
/// or after the tile map renders.
pub trait TileMapEffect: Send + Debug {
    /// Use the given context to render the special effect for the [`TileMap`].
    fn render_special_tiles(&self, context: &mut TileMapRenderContext);
}

/// Renders a rectangle of the given material at the given position in the tile map.
#[derive(Debug)]
pub struct TileCursorEffect {
    /// The position of the cursor in the tile map.
    pub position: Option<Vector2<i32>>,
    /// The material to use to render the cursor.
    pub material: Option<MaterialResource>,
}

impl TileMapEffect for TileCursorEffect {
    fn render_special_tiles(&self, context: &mut TileMapRenderContext) {
        if let Some(material) = self.material.as_ref() {
            if let Some(position) = self.position {
                push_cursor(position, material, context);
            }
        }
    }
}

/// Renders borders of the given material around the given positions in the tile map.
#[derive(Debug)]
pub struct TileSelectionEffect {
    /// This vector is added to the positions before rendering.
    pub offset: Option<Vector2<i32>>,
    /// The positions at which to draw the borders
    pub positions: FxHashSet<Vector2<i32>>,
    /// The size of the border
    pub thickness: f32,
    /// The material to use to render the border
    pub material: Option<MaterialResource>,
}

impl TileMapEffect for TileSelectionEffect {
    fn render_special_tiles(&self, context: &mut TileMapRenderContext) {
        if let (Some(material), Some(offset)) = (self.material.as_ref(), self.offset) {
            for &position in self.positions.iter() {
                let position = position + offset;
                push_highlight(position, material, self.thickness, context);
            }
        }
    }
}

/// Sets the tiles at the given positions to invisible.
#[derive(Debug)]
pub struct TileEraseEffect {
    /// The positions of the tiles to hide
    pub positions: FxHashSet<Vector2<i32>>,
}

impl TileMapEffect for TileEraseEffect {
    fn render_special_tiles(&self, context: &mut TileMapRenderContext) {
        for &position in self.positions.iter() {
            context.set_tile_visible(position, false);
        }
    }
}

/// Draws the given tiles with the given offset, and sets the drawn tile positions
/// to invisible so that no other tiles will be drawn there.
#[derive(Debug)]
pub struct TileOverlayEffect {
    /// True if the tiles are to be drawn. If false, then this effect does nothing.
    pub active: bool,
    /// Vector to be added to the positions of the tiles before rendering
    pub offset: Vector2<i32>,
    /// The tiles to render
    pub tiles: FxHashMap<Vector2<i32>, TileDefinitionHandle>,
}

impl TileMapEffect for TileOverlayEffect {
    fn render_special_tiles(&self, context: &mut TileMapRenderContext) {
        if !self.active {
            return;
        }
        for (&position, &handle) in self.tiles.iter() {
            let position = position + self.offset;
            if context.is_tile_visible(position) {
                context.draw_tile(position, handle);
                context.set_tile_visible(position, false);
            }
        }
    }
}

/// Uses the given tile update to render the replacement tiles and make
/// the erased tiles invisible.
#[derive(Debug)]
pub struct TileUpdateEffect {
    /// True if the tiles are to be drawn. If false, then this effect does nothing.
    pub active: bool,
    /// The update data to render in the tile map
    pub update: TransTilesUpdate,
}

impl TileMapEffect for TileUpdateEffect {
    fn render_special_tiles(&self, context: &mut TileMapRenderContext) {
        if !self.active {
            return;
        }
        for (&position, value) in self.update.iter() {
            if context.is_tile_visible(position) {
                context.set_tile_visible(position, false);
                if let Some((transform, handle)) = value.as_ref().map(|v| v.pair()) {
                    let handle = context
                        .tile_set
                        .get_transformed_version(transform, handle)
                        .unwrap_or(handle);
                    context.draw_tile(position, handle);
                }
            }
        }
    }
}

fn make_highlight_vertex(transform: &Matrix4<f32>, position: Vector2<f32>) -> StaticVertex {
    StaticVertex {
        position: transform
            .transform_point(&position.to_homogeneous().into())
            .coords,
        tex_coord: Vector2::default(),
        normal: Vector3::new(0.0, 0.0, 1.0),
        tangent: Vector4::new(0.0, 1.0, 0.0, 1.0),
    }
}

fn push_highlight(
    position: Vector2<i32>,
    material: &MaterialResource,
    thickness: f32,
    ctx: &mut TileMapRenderContext,
) {
    let transform = ctx.transform();
    let position = position.cast::<f32>();
    let t = thickness;
    let vertices = [
        (0.0, 1.0),
        (1.0, 1.0),
        (1.0, 0.0),
        (0.0, 0.0),
        (t, 1.0 - t),
        (1.0 - t, 1.0 - t),
        (1.0 - t, t),
        (t, t),
    ]
    .map(|(x, y)| Vector2::new(x, y))
    .map(|p| make_highlight_vertex(transform, position + p));

    let triangles = [
        [0, 4, 5],
        [0, 1, 5],
        [1, 5, 6],
        [1, 2, 6],
        [2, 6, 7],
        [2, 3, 7],
        [3, 7, 4],
        [3, 0, 4],
    ]
    .map(TriangleDefinition);

    let sort_index = ctx
        .context
        .calculate_sorting_index(ctx.position())
        .saturating_sub(1);

    ctx.context.storage.push_triangles(
        ctx.context.dynamic_surface_cache,
        StaticVertex::layout(),
        material,
        RenderPath::Forward,
        sort_index,
        ctx.tile_map_handle(),
        &mut move |mut vertex_buffer, mut triangle_buffer| {
            let start_vertex_index = vertex_buffer.vertex_count();

            vertex_buffer.push_vertices(&vertices).unwrap();

            triangle_buffer
                .push_triangles_iter_with_offset(start_vertex_index, triangles.into_iter());
        },
    );
}

fn push_cursor(
    position: Vector2<i32>,
    material: &MaterialResource,
    ctx: &mut TileMapRenderContext,
) {
    let transform = ctx.transform();
    let position = position.cast::<f32>();
    let vertices = [(0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)]
        .map(|(x, y)| Vector2::new(x, y))
        .map(|p| make_highlight_vertex(transform, position + p));

    let triangles = [[0, 1, 2], [0, 2, 3]].map(TriangleDefinition);

    let sort_index = ctx.context.calculate_sorting_index(ctx.position());

    ctx.context.storage.push_triangles(
        ctx.context.dynamic_surface_cache,
        StaticVertex::layout(),
        material,
        RenderPath::Forward,
        sort_index,
        ctx.tile_map_handle(),
        &mut move |mut vertex_buffer, mut triangle_buffer| {
            let start_vertex_index = vertex_buffer.vertex_count();

            vertex_buffer.push_vertices(&vertices).unwrap();

            triangle_buffer
                .push_triangles_iter_with_offset(start_vertex_index, triangles.into_iter());
        },
    );
}
