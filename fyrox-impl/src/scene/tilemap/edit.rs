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

/// A reference to TileMapEditorData. A TileMap keeps one of these if it is in the editor
/// being edited so that the TileMap can render a partially-constructed state.
pub type TileMapEditorDataRef = Arc<Mutex<TileMapEditorData>>;

/// The state of a tile map that is being edited.
#[derive(Default, Debug)]
pub struct TileMapEditorData {
    /// Position to render cursor highlight
    pub cursor_position: Option<Vector2<i32>>,
    /// Material for cursor highlight.
    pub cursor_material: Option<MaterialResource>,
    /// The highlighted cells.
    pub selected: FxHashSet<Vector2<i32>>,
    /// The color of the current highlight.
    pub select_material: Option<MaterialResource>,
    /// An area where only overlay tiles will be drawn.
    pub erased_area: FxHashSet<Vector2<i32>>,
    /// Temporary tiles to render in place of whatever tiles are currently at the given positions,
    /// after being offet by `overlay_offset`.
    pub overlay: FxHashMap<Vector2<i32>, TileRenderData>,
    /// An overlay that is drawn using `erase_material` after being offset bo `overlay_offset` to indicate tiles that
    /// may soon be deleted from the tile map.
    pub erase_overlay: FxHashSet<Vector2<i32>>,
    /// The position of the overlay.
    pub overlay_offset: Option<Vector2<i32>>,
    /// The maerial for the erase overlay.
    pub erase_material: Option<MaterialResource>,
    /// Tiles that are currently in the process of being modified.
    pub update: TransTilesUpdate,
}

impl TileMapEditorData {
    /// True if the tile at the given position is not to be rendered, unless it has been replaced
    /// by an overlay or by an update.
    pub fn erased_at(&self, position: Vector2<i32>) -> bool {
        self.erased_area.contains(&position)
            || self
                .overlay_offset
                .map(|off| self.overlay.contains_key(&(position - off)))
                .unwrap_or_default()
            || self.update.contains_key(&position)
    }
    /// Renders tile map overlay tiles and highlights.
    pub fn collect_render_data(
        &self,
        render_position: &TileRenderPosition,
        tile_set: TileSetRef,
        ctx: &mut RenderContext,
    ) {
        for (position, data) in self.update.iter() {
            let Some((transform, handle)) = data else {
                continue;
            };
            if self.overlay_offset.is_some()
                && self
                    .overlay
                    .contains_key(&(position - self.overlay_offset.unwrap()))
            {
                continue;
            }
            let handle = tile_set
                .get_transformed_version(*transform, *handle)
                .unwrap_or(*handle);
            let Some(data) = tile_set.get_tile_render_data(TilePaletteStage::Tiles, handle) else {
                continue;
            };
            render_position.push_tile(*position, &data, ctx);
        }
        if let Some(offset) = self.overlay_offset {
            for (&position, data) in self.overlay.iter() {
                render_position.push_tile(position + offset, data, ctx);
            }
            if let Some(material) = self.erase_material.as_ref() {
                for &position in self.erase_overlay.iter() {
                    push_highlight(render_position, position + offset, material, 0.1, ctx);
                }
            }
        }
        if let Some(material) = self.select_material.as_ref() {
            for position in self.selected.iter() {
                push_highlight(render_position, *position, material, 0.1, ctx);
            }
        }
        if let Some(material) = self.cursor_material.as_ref() {
            if let Some(position) = self.cursor_position {
                push_cursor(render_position, position, material, ctx);
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
    render_position: &TileRenderPosition,
    position: Vector2<i32>,
    material: &MaterialResource,
    thickness: f32,
    ctx: &mut RenderContext,
) {
    let transform = &render_position.transform;
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

    let sort_index = ctx.calculate_sorting_index(render_position.position());

    ctx.storage.push_triangles(
        StaticVertex::layout(),
        material,
        RenderPath::Forward,
        sort_index,
        render_position.tile_map_handle,
        &mut move |mut vertex_buffer, mut triangle_buffer| {
            let start_vertex_index = vertex_buffer.vertex_count();

            vertex_buffer.push_vertices(&vertices).unwrap();

            triangle_buffer
                .push_triangles_iter_with_offset(start_vertex_index, triangles.into_iter());
        },
    );
}

fn push_cursor(
    render_position: &TileRenderPosition,
    position: Vector2<i32>,
    material: &MaterialResource,
    ctx: &mut RenderContext,
) {
    let transform = &render_position.transform;
    let position = position.cast::<f32>();
    let vertices = [(0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)]
        .map(|(x, y)| Vector2::new(x, y))
        .map(|p| make_highlight_vertex(transform, position + p));

    let triangles = [[0, 1, 2], [0, 2, 3]].map(TriangleDefinition);

    let sort_index = ctx.calculate_sorting_index(render_position.position());

    ctx.storage.push_triangles(
        StaticVertex::layout(),
        material,
        RenderPath::Forward,
        sort_index,
        render_position.tile_map_handle,
        &mut move |mut vertex_buffer, mut triangle_buffer| {
            let start_vertex_index = vertex_buffer.vertex_count();

            vertex_buffer.push_vertices(&vertices).unwrap();

            triangle_buffer
                .push_triangles_iter_with_offset(start_vertex_index, triangles.into_iter());
        },
    );
}
