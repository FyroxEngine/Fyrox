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

//! The preview widget of the tile map control panel. This allows the user to see the
//! currently selected tile stamp, including whatever transformations have been applied
//! to the stamp.

use super::*;
use crate::fyrox::{
    core::{
        algebra::{Matrix3, Vector2},
        math::Rect,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        visitor::prelude::*,
    },
    gui::formatted_text::{FormattedText, FormattedTextBuilder},
    gui::{
        brush::Brush,
        define_widget_deref,
        draw::{CommandTexture, Draw, DrawingContext},
        message::UiMessage,
        widget::{Widget, WidgetBuilder},
        BuildContext, Control, UiNode, UserInterface,
    },
    resource::texture::TextureKind,
    scene::tilemap::{TileRenderData, TileSource},
};

use std::ops::{Deref, DerefMut};

/// The preview widget of the tile map control panel. This allows the user to see the
/// currently selected tile stamp, including whatever transformations have been applied
/// to the stamp.
#[derive(Clone, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "5356a864-c026-4bd7-a4b1-30bacf77d8fa")]
#[reflect(derived_type = "UiNode")]
pub struct PanelPreview {
    widget: Widget,
    /// The tile editing state that is shared with palette widgets, the tile map interaction mode,
    /// the tile map control panel, and others. It allows this widget access to determine
    /// what stamp to show to the user.
    #[reflect(hidden)]
    #[visit(skip)]
    pub state: TileDrawStateRef,
    tile_size: Vector2<f32>,
    transform: Matrix3<f32>,
    /// This text is used to show the tile handle when a single tile is selected.
    #[visit(skip)]
    #[reflect(hidden)]
    handle_text: FormattedText,
    #[visit(skip)]
    #[reflect(hidden)]
    /// This is the size of the handle text, for the purposes of positioning the text
    /// within the widget.
    handle_text_size: Vector2<f32>,
}

define_widget_deref!(PanelPreview);

fn draw_tile(
    position: Rect<f32>,
    clip_bounds: Rect<f32>,
    tile: &TileRenderData,
    drawing_context: &mut DrawingContext,
) {
    if tile.is_empty() {
        return;
    }
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
                    CommandTexture::Texture(texture),
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
        self.sync_handle_text();
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
    fn sync_handle_text(&mut self) {
        let text = self.get_handle_text();
        self.handle_text_size = self.handle_text.set_text(text).build();
    }
    fn get_handle_text(&self) -> String {
        let state = self.state.lock();
        let Some(mut tile_set) = state.tile_set.as_ref().map(|t| t.state()) else {
            return "".into();
        };
        let Some(tile_set) = tile_set.data() else {
            return "".into();
        };
        let stamp = &state.stamp;
        let mut iter = stamp.tile_iter();
        let Some(handle) = iter.next() else {
            return "".into();
        };
        if iter.next().is_some() {
            return "".into();
        }
        let transform = stamp.transformation();
        if let Some(handle) = tile_set.get_transformed_version(transform, handle) {
            handle.to_string()
        } else {
            format!("{}*", handle)
        }
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

        let time = ctx.elapsed_time;

        for (pos, &StampElement { handle, .. }) in stamp.iter() {
            let handle = tile_set
                .get_animated_version(time, handle)
                .unwrap_or(handle);
            let data = tile_set
                .get_transformed_render_data(stamp.transformation(), handle)
                .unwrap_or_else(TileRenderData::missing_data);
            let t = self.tile_size;
            let position = Vector2::new(pos.x as f32 * t.x, pos.y as f32 * t.y);
            let rect = Rect { position, size: t };
            draw_tile(rect, self.clip_bounds(), &data, ctx);
        }

        ctx.transform_stack.pop();
        ctx.draw_text(
            self.clip_bounds(),
            bounds.right_bottom_corner() - self.handle_text_size,
            &self.handle_text,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
        if message.destination() == self.handle()
            && message.direction() == MessageDirection::ToWidget
        {
            if let Some(PaletteMessage::SyncToState) = message.data::<PaletteMessage>() {
                self.sync_to_state();
            }
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
            widget: self.widget_builder.with_clip_to_bounds(false).build(ctx),
            state: self.state,
            tile_size: Vector2::repeat(32.0),
            transform: Matrix3::identity(),
            handle_text: FormattedTextBuilder::new(ctx.inner().default_font.clone())
                .with_brush(Brush::Solid(Color::WHITE))
                .build(),
            handle_text_size: Vector2::default(),
        }))
    }
}
