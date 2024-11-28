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

use fyrox::scene::tilemap::tileset::{TileSetPage, TileSetPageSource};
use fyrox::scene::tilemap::TileSource;

use crate::asset::item::AssetItem;
use crate::fyrox::{
    core::{
        algebra::{Matrix3, Point2, Vector2},
        color::Color,
        math::{OptionRect, Rect},
        parking_lot::Mutex,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        visitor::prelude::*,
    },
    fxhash::FxHashMap,
    graph::{BaseSceneGraph, SceneGraph},
    gui::{
        brush::Brush,
        define_constructor, define_widget_deref,
        draw::{CommandTexture, Draw, DrawingContext},
        formatted_text::{FormattedText, FormattedTextBuilder},
        message::{KeyCode, MessageDirection, MouseButton, UiMessage},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, UiNode, UserInterface,
    },
    material::{Material, MaterialResource},
    resource::texture::TextureKind,
    scene::tilemap::{
        tileset::TileSetResource, Stamp, TileDefinitionHandle, TilePaletteStage, TileRect,
        TileRenderData, TileResource, TileSetUpdate, Tiles, TransTilesUpdate,
    },
};
use std::ops::{Deref, DerefMut};

use super::*;

pub const DEFAULT_MATERIAL_COLOR: Color = Color::from_rgba(255, 255, 255, 125);

#[derive(Clone, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "5356a864-c026-4bd7-a4b1-30bacf77d8fa")]
pub struct PanelPreview {
    widget: Widget,
    #[reflect(hidden)]
    pub state: TileDrawStateRef,
    tile_size: Vector2<f32>,
    transform: Matrix3<f32>,
}

define_widget_deref!(PanelPreview);

fn apply_transform(trans: &Matrix3<f32>, point: Vector2<f32>) -> Vector2<f32> {
    trans.transform_point(&Point2::from(point)).coords
}

fn invert_transform(trans: &Matrix3<f32>) -> Matrix3<f32> {
    trans.try_inverse().unwrap_or(Matrix3::identity())
}

fn draw_tile(
    position: Rect<f32>,
    clip_bounds: Rect<f32>,
    tile: &TileRenderData,
    drawing_context: &mut DrawingContext,
) {
    let color = tile.color;
    if let Some(material_bounds) = &tile.material_bounds {
        if let Some(texture) = material_bounds
            .material
            .state()
            .data()
            .and_then(|m| m.texture("diffuseTexture"))
        {
            let kind = texture.data_ref().kind();
            if let TextureKind::Rectangle { width, height } = kind {
                let size = Vector2::new(width, height);
                let bounds = &material_bounds.bounds;
                drawing_context.push_rect_filled(
                    &position,
                    Some(&[
                        bounds.left_bottom_uv(size),
                        bounds.right_bottom_uv(size),
                        bounds.right_top_uv(size),
                        bounds.left_top_uv(size),
                    ]),
                );
                drawing_context.commit(
                    clip_bounds,
                    Brush::Solid(color),
                    CommandTexture::Texture(texture.into()),
                    None,
                );
            }
        } else {
            drawing_context.push_rect_filled(&position, None);
            drawing_context.commit(clip_bounds, Brush::Solid(color), CommandTexture::None, None);
        }
    } else {
        drawing_context.push_rect_filled(&position, None);
        drawing_context.commit(clip_bounds, Brush::Solid(color), CommandTexture::None, None);
    }
}

impl PanelPreview {
    fn sync_to_state(&mut self) {
        let bounds = self.state.lock().stamp.bounding_rect();
        let Some(bounds) = *bounds else {
            self.transform = Matrix3::identity();
            return;
        };
        let t = self.tile_size.cast::<f32>();
        let view_size = self.actual_local_size.get();
        let bounds_size = bounds.size.cast::<f32>();
        let bounds_size = Vector2::new(bounds_size.x * t.x, bounds_size.y * t.y);
        let bounds_position = bounds.position.cast::<f32>();
        let bounds_position = Vector2::new(bounds_position.x * t.x, bounds_position.y * t.y);
        let mid_point = bounds_position + bounds_size * 0.5;
        let zoom_x = view_size.x / bounds_size.x;
        let zoom_y = view_size.y / bounds_size.y;
        let zoom = zoom_x.min(zoom_y);
        let scale = Vector2::new(zoom, -zoom);
        let translate =
            view_size * 0.5 - Vector2::new(mid_point.x * scale.x, mid_point.y * scale.y);
        self.transform =
            Matrix3::new_translation(&translate) * Matrix3::new_nonuniform_scaling(&scale);
    }

    fn grid_pos_to_rect(&self, pos: Vector2<i32>) -> Rect<f32> {
        let size = self.tile_size;
        let position = Vector2::new(pos.x as f32 * size.x, pos.y as f32 * size.y);
        Rect { position, size }
    }
    fn push_cell_rect(&self, position: Vector2<i32>, thickness: f32, ctx: &mut DrawingContext) {
        let size = self.tile_size;
        let position = Vector2::new(position.x as f32 * size.x, position.y as f32 * size.y);
        let rect = Rect { position, size }.inflate(thickness * 0.5, thickness * 0.5);
        ctx.push_rect(&rect, thickness);
    }
    fn push_cell_rect_filled(&self, position: Vector2<i32>, ctx: &mut DrawingContext) {
        let size = self.tile_size;
        let position = Vector2::new(position.x as f32 * size.x, position.y as f32 * size.y);
        let rect = Rect { position, size };
        ctx.push_rect_filled(&rect, None);
    }
    fn commit_color(&self, color: Color, ctx: &mut DrawingContext) {
        ctx.commit(
            self.clip_bounds(),
            Brush::Solid(color),
            CommandTexture::None,
            None,
        );
    }
}

impl Control for PanelPreview {
    fn draw(&self, ctx: &mut DrawingContext) {
        let bounds = self.bounding_rect();
        ctx.push_rect_filled(&bounds, None);
        ctx.commit(
            self.clip_bounds(),
            self.widget.background(),
            CommandTexture::None,
            None,
        );
        let state = self.state.lock();
        let stamp = &state.stamp;
        let Some(mut tile_set) = state.tile_set.as_ref().map(|t| t.state()) else {
            return;
        };
        let Some(tile_set) = tile_set.data() else {
            return;
        };

        ctx.transform_stack
            .push(self.visual_transform() * self.transform);

        for (pos, handle) in stamp.iter() {
            let data = tile_set
                .get_transformed_render_data(stamp.transformation(), *handle)
                .unwrap_or_else(TileRenderData::missing_data);
            let t = self.tile_size;
            let position = Vector2::new(pos.x as f32 * t.x, pos.y as f32 * t.y);
            let rect = Rect { position, size: t };
            draw_tile(rect, self.clip_bounds(), &data, ctx);
        }

        ctx.transform_stack.pop();
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(PaletteMessage::SyncToState) = message.data::<PaletteMessage>() {
            self.sync_to_state();
        }
    }
}

pub struct PanelPreviewBuilder {
    widget_builder: WidgetBuilder,
    state: TileDrawStateRef,
}

impl PanelPreviewBuilder {
    pub fn new(widget_builder: WidgetBuilder, state: TileDrawStateRef) -> Self {
        Self {
            widget_builder,
            state,
        }
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        ctx.add_node(UiNode::new(PanelPreview {
            widget: self.widget_builder.with_clip_to_bounds(false).build(),
            state: self.state,
            tile_size: Vector2::repeat(32.0),
            transform: Matrix3::identity(),
        }))
    }
}
