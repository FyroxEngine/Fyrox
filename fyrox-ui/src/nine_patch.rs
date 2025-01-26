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
    brush::Brush,
    core::{
        algebra::Vector2, color::Color, math::Rect, pool::Handle, reflect::prelude::*,
        some_or_return, type_traits::prelude::*, variable::InheritableVariable,
        visitor::prelude::*,
    },
    draw::{CommandTexture, Draw, DrawingContext},
    message::UiMessage,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UiNode, UserInterface,
};
use fyrox_graph::{
    constructor::{ConstructorProvider, GraphNodeConstructor},
    BaseSceneGraph,
};
use fyrox_texture::{TextureKind, TextureResource};
use std::ops::{Deref, DerefMut};

#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "c345033e-8c10-4186-b101-43f73b85981d")]
pub struct NinePatch {
    pub widget: Widget,
    pub texture: InheritableVariable<Option<TextureResource>>,
    pub bottom_margin: InheritableVariable<u32>,
    pub left_margin: InheritableVariable<u32>,
    pub right_margin: InheritableVariable<u32>,
    pub top_margin: InheritableVariable<u32>,
    pub texture_region: InheritableVariable<Option<Rect<u32>>>,
    pub draw_center: InheritableVariable<bool>,
}

impl ConstructorProvider<UiNode, UserInterface> for NinePatch {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("Nine Patch", |ui| {
                NinePatchBuilder::new(WidgetBuilder::new().with_name("Nine Patch"))
                    .build(&mut ui.build_ctx())
                    .into()
            })
            .with_group("Visual")
    }
}

crate::define_widget_deref!(NinePatch);

fn draw_image(
    image: &TextureResource,
    bounds: Rect<f32>,
    tex_coords: &[Vector2<f32>; 4],
    clip_bounds: Rect<f32>,
    background: Brush,
    drawing_context: &mut DrawingContext,
) {
    drawing_context.push_rect_filled(&bounds, Some(tex_coords));
    let texture = CommandTexture::Texture(image.clone());
    drawing_context.commit(clip_bounds, background, texture, None);
}

impl Control for NinePatch {
    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        let mut size: Vector2<f32> = available_size;

        let column1_width_pixels = *self.left_margin as f32;
        let column3_width_pixels = *self.right_margin as f32;

        let row1_height_pixels = *self.top_margin as f32;
        let row3_height_pixels = *self.bottom_margin as f32;

        let x_overflow = column1_width_pixels + column3_width_pixels;
        let y_overflow = row1_height_pixels + row3_height_pixels;

        let center_size =
            Vector2::new(available_size.x - x_overflow, available_size.y - y_overflow);

        for &child in self.children.iter() {
            ui.measure_node(child, center_size);
            let desired_size = ui.node(child).desired_size();
            size.x = size.x.max(desired_size.x.ceil());
            size.y = size.y.max(desired_size.y.ceil());
        }
        size
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        let column1_width_pixels = *self.left_margin as f32;
        let column3_width_pixels = *self.right_margin as f32;

        let row1_height_pixels = *self.top_margin as f32;
        let row3_height_pixels = *self.bottom_margin as f32;

        let x_overflow = column1_width_pixels + column3_width_pixels;
        let y_overflow = row1_height_pixels + row3_height_pixels;

        let final_rect = Rect::new(
            column1_width_pixels,
            row1_height_pixels,
            final_size.x - x_overflow,
            final_size.y - y_overflow,
        );

        for &child in self.children.iter() {
            ui.arrange_node(child, &final_rect);
        }

        final_size
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let texture = some_or_return!(self.texture.as_ref());

        let texture_state = texture.state();
        let texture_state = some_or_return!(texture_state.data_ref());

        // Only 2D textures can be used with nine-patch.
        let TextureKind::Rectangle { width, height } = texture_state.kind() else {
            return;
        };

        let texture_width = width as f32;
        let texture_height = height as f32;

        let patch_bounds = self.widget.bounding_rect();

        let left_margin = *self.left_margin as f32;
        let right_margin = *self.right_margin as f32;
        let top_margin = *self.top_margin as f32;
        let bottom_margin = *self.bottom_margin as f32;

        let region = self
            .texture_region
            .map(|region| Rect {
                position: region.position.cast::<f32>(),
                size: region.size.cast::<f32>(),
            })
            .unwrap_or_else(|| Rect::new(0.0, 0.0, texture_width, texture_height));

        let center_uv_x_min = (region.position.x + left_margin) / texture_width;
        let center_uv_x_max = (region.position.x + region.size.x - right_margin) / texture_width;
        let center_uv_y_min = (region.position.y + top_margin) / texture_height;
        let center_uv_y_max = (region.position.y + region.size.y - bottom_margin) / texture_height;
        let uv_x_min = region.position.x / texture_width;
        let uv_x_max = (region.position.x + region.size.x) / texture_width;
        let uv_y_min = region.position.y / texture_height;
        let uv_y_max = (region.position.y + region.size.y) / texture_height;

        let x_overflow = left_margin + right_margin;
        let y_overflow = top_margin + bottom_margin;

        //top left
        let bounds = Rect {
            position: patch_bounds.position,
            size: Vector2::new(left_margin, top_margin),
        };
        let tex_coords = [
            Vector2::new(uv_x_min, uv_y_min),
            Vector2::new(center_uv_x_min, uv_y_min),
            Vector2::new(center_uv_x_min, center_uv_y_min),
            Vector2::new(uv_x_min, center_uv_y_min),
        ];
        draw_image(
            texture,
            bounds,
            &tex_coords,
            self.clip_bounds(),
            self.widget.background(),
            drawing_context,
        );

        //top center
        let bounds = Rect {
            position: Vector2::new(
                patch_bounds.position.x + left_margin,
                patch_bounds.position.y,
            ),
            size: Vector2::new(patch_bounds.size.x - x_overflow, top_margin),
        };
        let tex_coords = [
            Vector2::new(center_uv_x_min, uv_y_min),
            Vector2::new(center_uv_x_max, uv_y_min),
            Vector2::new(center_uv_x_max, center_uv_y_min),
            Vector2::new(center_uv_x_min, center_uv_y_min),
        ];
        draw_image(
            texture,
            bounds,
            &tex_coords,
            self.clip_bounds(),
            self.widget.background(),
            drawing_context,
        );

        //top right
        let bounds = Rect {
            position: Vector2::new(
                (patch_bounds.position.x + patch_bounds.size.x) - right_margin,
                patch_bounds.position.y,
            ),
            size: Vector2::new(right_margin, top_margin),
        };
        let tex_coords = [
            Vector2::new(center_uv_x_max, uv_y_min),
            Vector2::new(uv_x_max, uv_y_min),
            Vector2::new(uv_x_max, center_uv_y_min),
            Vector2::new(center_uv_x_max, center_uv_y_min),
        ];
        draw_image(
            texture,
            bounds,
            &tex_coords,
            self.clip_bounds(),
            self.widget.background(),
            drawing_context,
        );
        ////////////////////////////////////////////////////////////////////////////////
        //middle left
        let bounds = Rect {
            position: Vector2::new(
                patch_bounds.position.x,
                patch_bounds.position.y + top_margin,
            ),
            size: Vector2::new(left_margin, patch_bounds.size.y - y_overflow),
        };
        let tex_coords = [
            Vector2::new(uv_x_min, center_uv_y_min),
            Vector2::new(center_uv_x_min, center_uv_y_min),
            Vector2::new(center_uv_x_min, center_uv_y_max),
            Vector2::new(uv_x_min, center_uv_y_max),
        ];
        draw_image(
            texture,
            bounds,
            &tex_coords,
            self.clip_bounds(),
            self.widget.background(),
            drawing_context,
        );

        if *self.draw_center {
            //middle center
            let bounds = Rect {
                position: Vector2::new(
                    patch_bounds.position.x + left_margin,
                    patch_bounds.position.y + top_margin,
                ),
                size: Vector2::new(
                    patch_bounds.size.x - x_overflow,
                    patch_bounds.size.y - y_overflow,
                ),
            };
            let tex_coords = [
                Vector2::new(center_uv_x_min, center_uv_y_min),
                Vector2::new(center_uv_x_max, center_uv_y_min),
                Vector2::new(center_uv_x_max, center_uv_y_max),
                Vector2::new(center_uv_x_min, center_uv_y_max),
            ];
            draw_image(
                texture,
                bounds,
                &tex_coords,
                self.clip_bounds(),
                self.widget.background(),
                drawing_context,
            );
        }

        //middle right
        let bounds = Rect {
            position: Vector2::new(
                (patch_bounds.position.x + patch_bounds.size.x) - right_margin,
                patch_bounds.position.y + top_margin,
            ),
            size: Vector2::new(right_margin, patch_bounds.size.y - y_overflow),
        };
        let tex_coords = [
            Vector2::new(center_uv_x_max, center_uv_y_min),
            Vector2::new(uv_x_max, center_uv_y_min),
            Vector2::new(uv_x_max, center_uv_y_max),
            Vector2::new(center_uv_x_max, center_uv_y_max),
        ];
        draw_image(
            texture,
            bounds,
            &tex_coords,
            self.clip_bounds(),
            self.widget.background(),
            drawing_context,
        );

        ////////////////////////////////////////////////////////////////////////////////
        //bottom left
        let bounds = Rect {
            position: Vector2::new(
                patch_bounds.position.x,
                (patch_bounds.position.y + patch_bounds.size.y) - bottom_margin,
            ),
            size: Vector2::new(left_margin, bottom_margin),
        };
        let tex_coords = [
            Vector2::new(uv_x_min, center_uv_y_max),
            Vector2::new(center_uv_x_min, center_uv_y_max),
            Vector2::new(center_uv_x_min, uv_y_max),
            Vector2::new(uv_x_min, uv_y_max),
        ];
        draw_image(
            texture,
            bounds,
            &tex_coords,
            self.clip_bounds(),
            self.widget.background(),
            drawing_context,
        );

        //bottom center
        let bounds = Rect {
            position: Vector2::new(
                patch_bounds.position.x + left_margin,
                (patch_bounds.position.y + patch_bounds.size.y) - bottom_margin,
            ),
            size: Vector2::new(patch_bounds.size.x - x_overflow, bottom_margin),
        };
        let tex_coords = [
            Vector2::new(center_uv_x_min, center_uv_y_max),
            Vector2::new(center_uv_x_max, center_uv_y_max),
            Vector2::new(center_uv_x_max, uv_y_max),
            Vector2::new(center_uv_x_min, uv_y_max),
        ];
        draw_image(
            texture,
            bounds,
            &tex_coords,
            self.clip_bounds(),
            self.widget.background(),
            drawing_context,
        );

        //bottom right
        let bounds = Rect {
            position: Vector2::new(
                (patch_bounds.position.x + patch_bounds.size.x) - right_margin,
                (patch_bounds.position.y + patch_bounds.size.y) - bottom_margin,
            ),
            size: Vector2::new(right_margin, bottom_margin),
        };
        let tex_coords = [
            Vector2::new(center_uv_x_max, center_uv_y_max),
            Vector2::new(uv_x_max, center_uv_y_max),
            Vector2::new(uv_x_max, uv_y_max),
            Vector2::new(center_uv_x_max, uv_y_max),
        ];
        draw_image(
            texture,
            bounds,
            &tex_coords,
            self.clip_bounds(),
            self.widget.background(),
            drawing_context,
        );

        //end drawing
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
    }
}

pub struct NinePatchBuilder {
    pub widget_builder: WidgetBuilder,
    pub texture: Option<TextureResource>,
    pub bottom_margin: u32,
    pub left_margin: u32,
    pub right_margin: u32,
    pub top_margin: u32,
    pub texture_region: Option<Rect<u32>>,
    pub draw_center: bool,
}

impl NinePatchBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            texture: None,
            bottom_margin: 0,
            left_margin: 0,
            right_margin: 0,
            top_margin: 0,
            texture_region: None,
            draw_center: true,
        }
    }

    pub fn with_texture(mut self, texture: TextureResource) -> Self {
        self.texture = Some(texture);
        self
    }

    pub fn with_bottom_margin(mut self, margin: u32) -> Self {
        self.bottom_margin = margin;
        self
    }

    pub fn with_left_margin(mut self, margin: u32) -> Self {
        self.left_margin = margin;
        self
    }

    pub fn with_right_margin(mut self, margin: u32) -> Self {
        self.right_margin = margin;
        self
    }

    pub fn with_top_margin(mut self, margin: u32) -> Self {
        self.top_margin = margin;
        self
    }

    pub fn with_texture_region(mut self, rect: Rect<u32>) -> Self {
        self.texture_region = Some(rect);
        self
    }

    pub fn with_draw_center(mut self, draw_center: bool) -> Self {
        self.draw_center = draw_center;
        self
    }

    pub fn build(mut self, ctx: &mut BuildContext) -> Handle<UiNode> {
        if self.widget_builder.background.is_none() {
            self.widget_builder.background = Some(Brush::Solid(Color::WHITE).into())
        }

        ctx.add_node(UiNode::new(NinePatch {
            widget: self.widget_builder.build(ctx),
            texture: self.texture.into(),
            bottom_margin: self.bottom_margin.into(),
            left_margin: self.left_margin.into(),
            right_margin: self.right_margin.into(),
            top_margin: self.top_margin.into(),
            texture_region: self.texture_region.into(),
            draw_center: self.draw_center.into(),
        }))
    }
}

#[cfg(test)]
mod test {
    use crate::nine_patch::NinePatchBuilder;
    use crate::{test::test_widget_deletion, widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| NinePatchBuilder::new(WidgetBuilder::new()).build(ctx));
    }
}
