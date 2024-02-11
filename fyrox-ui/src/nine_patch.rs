use fyrox_core::{scope_profile, uuid_provider};

use crate::{
    brush::Brush,
    core::{
        algebra::Vector2, color::Color, math::Rect, pool::Handle, reflect::prelude::*,
        type_traits::prelude::*, visitor::prelude::*,
    },
    draw::{CommandTexture, Draw, DrawingContext},
    message::UiMessage,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UiNode, UserInterface,
};
use fyrox_core::variable::InheritableVariable;
use fyrox_graph::BaseSceneGraph;
use fyrox_resource::untyped::UntypedResource;
use std::ops::{Deref, DerefMut};

/// Automatically arranges children by rows and columns
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct NinePatch {
    pub widget: Widget,
    pub texture: InheritableVariable<Option<UntypedResource>>,
    pub bottom_margin_uv: InheritableVariable<f32>,
    pub left_margin_uv: InheritableVariable<f32>,
    pub right_margin_uv: InheritableVariable<f32>,
    pub top_margin_uv: InheritableVariable<f32>,

    pub bottom_margin_pixel: InheritableVariable<u32>,
    pub left_margin_pixel: InheritableVariable<u32>,
    pub right_margin_pixel: InheritableVariable<u32>,
    pub top_margin_pixel: InheritableVariable<u32>,
}

crate::define_widget_deref!(NinePatch);

uuid_provider!(NinePatch = "c345033e-8c10-4186-b101-43f73b85981d");

impl Control for NinePatch {
    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        scope_profile!();
        let mut size: Vector2<f32> = available_size;

        let column1_width_pixels = *self.left_margin_pixel as f32;
        let column3_width_pixels = *self.right_margin_pixel as f32;

        let row1_height_pixels = *self.top_margin_pixel as f32;
        let row3_height_pixels = *self.bottom_margin_pixel as f32;

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
        scope_profile!();

        let column1_width_pixels = *self.left_margin_pixel as f32;
        let column3_width_pixels = *self.right_margin_pixel as f32;

        let row1_height_pixels = *self.top_margin_pixel as f32;
        let row3_height_pixels = *self.bottom_margin_pixel as f32;

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
        let texture = self.texture.as_ref().unwrap();

        let patch_bounds = self.widget.bounding_rect();

        let column1_width_pixels = *self.left_margin_pixel as f32;
        let column3_width_pixels = *self.right_margin_pixel as f32;

        let row1_height_pixels = *self.top_margin_pixel as f32;
        let row3_height_pixels = *self.bottom_margin_pixel as f32;

        let x_fence_post1_uv = *self.left_margin_uv;
        let x_fence_post2_uv = 1.0 - *self.right_margin_uv;
        let y_fence_post1_uv = *self.top_margin_uv;
        let y_fence_post2_uv = 1.0 - *self.bottom_margin_uv;

        let x_overflow = column1_width_pixels + column3_width_pixels;
        let y_overlfow = row1_height_pixels + row3_height_pixels;

        //top left
        let bounds = Rect {
            position: patch_bounds.position,
            size: Vector2::new(column1_width_pixels, row1_height_pixels),
        };
        let tex_coords = [
            Vector2::<f32>::new(0.0, 0.0),
            Vector2::new(x_fence_post1_uv, 0.0),
            Vector2::new(x_fence_post1_uv, y_fence_post1_uv),
            Vector2::new(0.0, y_fence_post1_uv),
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
                patch_bounds.position.x + column1_width_pixels,
                patch_bounds.position.y,
            ),
            size: Vector2::new(patch_bounds.size.x - x_overflow, row1_height_pixels),
        };
        let tex_coords = [
            Vector2::<f32>::new(x_fence_post1_uv, 0.0),
            Vector2::new(x_fence_post2_uv, 0.0),
            Vector2::new(x_fence_post2_uv, y_fence_post1_uv),
            Vector2::new(x_fence_post1_uv, y_fence_post1_uv),
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
                (patch_bounds.position.x + patch_bounds.size.x) - column3_width_pixels,
                patch_bounds.position.y,
            ),
            size: Vector2::new(column3_width_pixels, row1_height_pixels),
        };
        let tex_coords = [
            Vector2::<f32>::new(x_fence_post2_uv, 0.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(1.0, y_fence_post1_uv),
            Vector2::new(x_fence_post2_uv, y_fence_post1_uv),
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
                patch_bounds.position.y + row1_height_pixels,
            ),
            size: Vector2::new(column1_width_pixels, patch_bounds.size.y - y_overlfow),
        };
        let tex_coords = [
            Vector2::<f32>::new(0.0, y_fence_post1_uv),
            Vector2::new(x_fence_post1_uv, y_fence_post1_uv),
            Vector2::new(x_fence_post1_uv, y_fence_post2_uv),
            Vector2::new(0.0, y_fence_post2_uv),
        ];
        draw_image(
            texture,
            bounds,
            &tex_coords,
            self.clip_bounds(),
            self.widget.background(),
            drawing_context,
        );

        //middle center
        let bounds = Rect {
            position: Vector2::new(
                patch_bounds.position.x + column1_width_pixels,
                patch_bounds.position.y + row1_height_pixels,
            ),
            size: Vector2::new(
                patch_bounds.size.x - x_overflow,
                patch_bounds.size.y - y_overlfow,
            ),
        };
        let tex_coords = [
            Vector2::<f32>::new(x_fence_post1_uv, y_fence_post1_uv),
            Vector2::new(x_fence_post2_uv, y_fence_post1_uv),
            Vector2::new(x_fence_post2_uv, y_fence_post2_uv),
            Vector2::new(x_fence_post1_uv, y_fence_post2_uv),
        ];
        draw_image(
            texture,
            bounds,
            &tex_coords,
            self.clip_bounds(),
            self.widget.background(),
            drawing_context,
        );

        //middle right
        let bounds = Rect {
            position: Vector2::new(
                (patch_bounds.position.x + patch_bounds.size.x) - column3_width_pixels,
                patch_bounds.position.y + row1_height_pixels,
            ),
            size: Vector2::new(column3_width_pixels, patch_bounds.size.y - y_overlfow),
        };
        let tex_coords = [
            Vector2::<f32>::new(x_fence_post2_uv, y_fence_post1_uv),
            Vector2::new(1.0, y_fence_post1_uv),
            Vector2::new(1.0, y_fence_post2_uv),
            Vector2::new(x_fence_post2_uv, y_fence_post2_uv),
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
                (patch_bounds.position.y + patch_bounds.size.y) - row3_height_pixels,
            ),
            size: Vector2::new(column1_width_pixels, row3_height_pixels),
        };
        let tex_coords = [
            Vector2::<f32>::new(0.0, y_fence_post2_uv),
            Vector2::new(x_fence_post1_uv, y_fence_post2_uv),
            Vector2::new(x_fence_post1_uv, 1.0),
            Vector2::new(0.0, 1.0),
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
                patch_bounds.position.x + column1_width_pixels,
                (patch_bounds.position.y + patch_bounds.size.y) - row3_height_pixels,
            ),
            size: Vector2::new(patch_bounds.size.x - x_overflow, row3_height_pixels),
        };
        let tex_coords = [
            Vector2::<f32>::new(x_fence_post1_uv, y_fence_post2_uv),
            Vector2::new(x_fence_post2_uv, y_fence_post2_uv),
            Vector2::new(x_fence_post2_uv, 1.0),
            Vector2::new(x_fence_post1_uv, 1.0),
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
                (patch_bounds.position.x + patch_bounds.size.x) - column3_width_pixels,
                (patch_bounds.position.y + patch_bounds.size.y) - row3_height_pixels,
            ),
            size: Vector2::new(column3_width_pixels, row3_height_pixels),
        };
        let tex_coords = [
            Vector2::<f32>::new(x_fence_post2_uv, y_fence_post2_uv),
            Vector2::new(1.0, y_fence_post2_uv),
            Vector2::new(1.0, 1.0),
            Vector2::new(x_fence_post2_uv, 1.0),
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
    widget_builder: WidgetBuilder,
    texture: Option<UntypedResource>,

    pub bottom_margin_pixel: Option<u32>,
    pub left_margin_pixel: Option<u32>,
    pub right_margin_pixel: Option<u32>,
    pub top_margin_pixel: Option<u32>,

    pub bottom_margin_uv: Option<f32>,
    pub left_margin_uv: Option<f32>,
    pub right_margin_uv: Option<f32>,
    pub top_margin_uv: Option<f32>,
}

impl NinePatchBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            texture: None,

            bottom_margin_uv: None,
            left_margin_uv: None,
            right_margin_uv: None,
            top_margin_uv: None,

            bottom_margin_pixel: None,
            left_margin_pixel: None,
            right_margin_pixel: None,
            top_margin_pixel: None,
        }
    }

    pub fn with_texture(mut self, texture: UntypedResource) -> Self {
        self.texture = Some(texture);
        self
    }
    pub fn with_bottom_margin_uv(mut self, margin: f32) -> Self {
        self.bottom_margin_uv = Some(margin);
        self
    }
    pub fn with_left_margin_uv(mut self, margin: f32) -> Self {
        self.left_margin_uv = Some(margin);
        self
    }
    pub fn with_right_margin_uv(mut self, margin: f32) -> Self {
        self.right_margin_uv = Some(margin);
        self
    }
    pub fn with_top_margin_uv(mut self, margin: f32) -> Self {
        self.top_margin_uv = Some(margin);
        self
    }
    pub fn with_bottom_margin_pixel(mut self, margin: u32) -> Self {
        self.bottom_margin_pixel = Some(margin);
        self
    }
    pub fn with_left_margin_pixel(mut self, margin: u32) -> Self {
        self.left_margin_pixel = Some(margin);
        self
    }
    pub fn with_right_margin_pixel(mut self, margin: u32) -> Self {
        self.right_margin_pixel = Some(margin);
        self
    }
    pub fn with_top_margin_pixel(mut self, margin: u32) -> Self {
        self.top_margin_pixel = Some(margin);
        self
    }
    pub fn build(mut self, ui: &mut BuildContext) -> Handle<UiNode> {
        if self.widget_builder.background.is_none() {
            self.widget_builder.background = Some(Brush::Solid(Color::WHITE))
        }

        // if one of the margins hasn't been set just mirror the opposite one.
        let (left_margin_pixel, right_margin_pixel) =
            match (self.left_margin_pixel, self.right_margin_pixel) {
                (Some(x), None) => (x, x),
                (None, Some(x)) => (x, x),
                (Some(one), Some(two)) => (one, two),
                (None, None) => (0, 0),
            };

        let (top_margin_pixel, bottom_margin_pixel) =
            match (self.top_margin_pixel, self.bottom_margin_pixel) {
                (Some(y), None) => (y, y),
                (None, Some(y)) => (y, y),
                (Some(one), Some(two)) => (one, two),
                (None, None) => (0, 0),
            };

        let (left_margin_uv, right_margin_uv) = match (self.left_margin_uv, self.right_margin_uv) {
            (Some(x), None) => (x, x),
            (None, Some(x)) => (x, x),
            (Some(one), Some(two)) => (one, two),
            (None, None) => (0.0, 0.0),
        };

        let (top_margin_uv, bottom_margin_uv) = match (self.top_margin_uv, self.bottom_margin_uv) {
            (Some(y), None) => (y, y),
            (None, Some(y)) => (y, y),
            (Some(one), Some(two)) => (one, two),
            (None, None) => (0.0, 0.0),
        };

        let grid = NinePatch {
            widget: self.widget_builder.build(),
            texture: self.texture.into(),
            bottom_margin_pixel: bottom_margin_pixel.into(),
            bottom_margin_uv: bottom_margin_uv.into(),
            left_margin_pixel: left_margin_pixel.into(),
            left_margin_uv: left_margin_uv.into(),
            right_margin_pixel: right_margin_pixel.into(),
            right_margin_uv: right_margin_uv.into(),
            top_margin_pixel: top_margin_pixel.into(),
            top_margin_uv: top_margin_uv.into(),
        };
        ui.add_node(UiNode::new(grid))
    }
}
fn draw_image(
    image: &UntypedResource,
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
