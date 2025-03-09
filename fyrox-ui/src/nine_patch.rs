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
    define_constructor,
    draw::{CommandTexture, Draw, DrawingContext},
    message::{compare_and_set, MessageDirection, UiMessage},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UiNode, UserInterface,
};

use fyrox_graph::{
    constructor::{ConstructorProvider, GraphNodeConstructor},
    BaseSceneGraph,
};
use fyrox_texture::{TextureKind, TextureResource};
use std::ops::{Deref, DerefMut};
use strum_macros::{AsRefStr, EnumString, VariantNames};

/// Stretch mode for the middle sections of [`NinePatch`] widget.
#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    Hash,
    PartialEq,
    Eq,
    Reflect,
    Visit,
    AsRefStr,
    EnumString,
    VariantNames,
    TypeUuidProvider,
)]
#[type_uuid(id = "c5bb0a5c-6581-45f7-899c-78aa1da8b659")]
pub enum StretchMode {
    /// Stretches middle sections of the widget. Could lead to distorted image.
    #[default]
    Stretch,
    /// Tiles middle sections of the widget. Prevents distortion of the image.
    Tile,
}

/// A set of possible nine patch messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NinePatchMessage {
    LeftMargin(u32),
    RightMargin(u32),
    TopMargin(u32),
    BottomMargin(u32),
    TextureRegion(Rect<u32>),
    Texture(Option<TextureResource>),
    DrawCenter(bool),
}

impl NinePatchMessage {
    define_constructor!(
        /// Creates [`NinePatchMessage::LeftMargin`] message.
        NinePatchMessage:LeftMargin => fn left_margin(u32), layout: false
    );
    define_constructor!(
        /// Creates [`NinePatchMessage::RightMargin`] message.
        NinePatchMessage:RightMargin => fn right_margin(u32), layout: false
    );
    define_constructor!(
        /// Creates [`NinePatchMessage::TopMargin`] message.
        NinePatchMessage:TopMargin => fn top_margin(u32), layout: false
    );
    define_constructor!(
        /// Creates [`NinePatchMessage::BottomMargin`] message.
        NinePatchMessage:BottomMargin => fn bottom_margin(u32), layout: false
    );
    define_constructor!(
        /// Creates [`NinePatchMessage::TextureRegion`] message.
        NinePatchMessage:TextureRegion => fn texture_region(Rect<u32>), layout: false
    );
    define_constructor!(
        /// Creates [`NinePatchMessage::Texture`] message.
        NinePatchMessage:Texture => fn texture(Option<TextureResource>), layout: false
    );
    define_constructor!(
        /// Creates [`NinePatchMessage::DrawCenter`] message.
        NinePatchMessage:DrawCenter => fn draw_center(bool), layout: false
    );
}

/// A texture slice that defines a region in a texture and margins that will be used to split the
/// section in nine pieces.
#[derive(Default, Clone, Visit, Reflect, Debug, PartialEq)]
pub struct TextureSlice {
    /// Texture of the slice. This field is used only for editing purposes in the UI. Can be [`None`]
    /// if no editing is needed.
    pub texture_source: Option<TextureResource>,
    /// Offset from the bottom side of the texture region.
    pub bottom_margin: InheritableVariable<u32>,
    /// Offset from the left side of the texture region.
    pub left_margin: InheritableVariable<u32>,
    /// Offset from the right side of the texture region.
    pub right_margin: InheritableVariable<u32>,
    /// Offset from the top of the texture region.
    pub top_margin: InheritableVariable<u32>,
    /// Region in the texture. Default is all zeros, which means that the entire texture is used.
    pub texture_region: InheritableVariable<Rect<u32>>,
}

impl TextureSlice {
    /// Returns the top left point.
    pub fn margin_min(&self) -> Vector2<u32> {
        Vector2::new(
            self.texture_region.position.x + *self.left_margin,
            self.texture_region.position.y + *self.top_margin,
        )
    }

    /// Returns the bottom right point.
    pub fn margin_max(&self) -> Vector2<u32> {
        Vector2::new(
            self.texture_region.position.x
                + self
                    .texture_region
                    .size
                    .x
                    .saturating_sub(*self.right_margin),
            self.texture_region.position.y
                + self
                    .texture_region
                    .size
                    .y
                    .saturating_sub(*self.bottom_margin),
        )
    }
}

/// `NinePatch` widget is used to split an image in nine sections, where each corner section will
/// remain the same, while the middle parts between each corner will be used to evenly fill the
/// space. This widget is primarily used in the UI to create resizable frames, buttons, windows, etc.
///
/// ## Example
///
/// The following examples shows how to create a nine-patch widget with a texture and some margins.
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::{math::Rect, pool::Handle},
/// #     nine_patch::NinePatchBuilder,
/// #     widget::WidgetBuilder,
/// #     UiNode, UserInterface,
/// # };
/// # use fyrox_texture::TextureResource;
/// #
/// fn create_nine_patch(texture: TextureResource, ui: &mut UserInterface) -> Handle<UiNode> {
///     NinePatchBuilder::new(WidgetBuilder::new())
///         // Specify margins for each side in pixels.
///         .with_left_margin(50)
///         .with_right_margin(50)
///         .with_top_margin(40)
///         .with_bottom_margin(40)
///         .with_texture(texture)
///         // Optionally, you can also specify a region in a texture to use. It is useful if you
///         // have a texture atlas where most of the UI elements are packed.
///         .with_texture_region(Rect::new(200, 200, 400, 400))
///         .build(&mut ui.build_ctx())
/// }
/// ```
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "c345033e-8c10-4186-b101-43f73b85981d")]
#[reflect(derived_type = "UiNode")]
pub struct NinePatch {
    pub widget: Widget,
    pub texture_slice: TextureSlice,
    pub draw_center: InheritableVariable<bool>,
    #[reflect(setter = "set_texture")]
    pub texture: InheritableVariable<Option<TextureResource>>,
    pub stretch_mode: InheritableVariable<StretchMode>,
}

impl NinePatch {
    pub fn set_texture(&mut self, texture: Option<TextureResource>) {
        self.texture.set_value_and_mark_modified(texture.clone());
        self.texture_slice.texture_source = texture;
    }
}

impl ConstructorProvider<UiNode, UserInterface> for NinePatch {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("Nine Patch", |ui| {
                NinePatchBuilder::new(
                    WidgetBuilder::new()
                        .with_name("Nine Patch")
                        .with_width(200.0)
                        .with_height(200.0),
                )
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

fn draw_tiled_image(
    image: &TextureResource,
    texture_width: f32,
    texture_height: f32,
    bounds: Rect<f32>,
    tex_coords: &[Vector2<f32>; 4],
    clip_bounds: Rect<f32>,
    background: Brush,
    drawing_context: &mut DrawingContext,
) {
    let region_bounds = Rect::new(
        tex_coords[0].x * texture_width,
        tex_coords[0].y * texture_height,
        (tex_coords[1].x - tex_coords[0].x) * texture_width,
        (tex_coords[2].y - tex_coords[0].y) * texture_height,
    );

    let nx = (bounds.size.x / region_bounds.size.x).ceil() as usize;
    let ny = (bounds.size.y / region_bounds.size.y).ceil() as usize;

    for y in 0..ny {
        for x in 0..nx {
            let tile_bounds = Rect::new(
                bounds.position.x + x as f32 * region_bounds.size.x,
                bounds.position.y + y as f32 * region_bounds.size.y,
                region_bounds.size.x,
                region_bounds.size.y,
            );

            drawing_context.push_rect_filled(&tile_bounds, Some(tex_coords));
        }
    }

    drawing_context.commit(
        clip_bounds,
        background,
        CommandTexture::Texture(image.clone()),
        None,
    );
}

impl Control for NinePatch {
    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        let mut size: Vector2<f32> = available_size;

        let column1_width_pixels = *self.texture_slice.left_margin as f32;
        let column3_width_pixels = *self.texture_slice.right_margin as f32;

        let row1_height_pixels = *self.texture_slice.top_margin as f32;
        let row3_height_pixels = *self.texture_slice.bottom_margin as f32;

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
        let column1_width_pixels = *self.texture_slice.left_margin as f32;
        let column3_width_pixels = *self.texture_slice.right_margin as f32;

        let row1_height_pixels = *self.texture_slice.top_margin as f32;
        let row3_height_pixels = *self.texture_slice.bottom_margin as f32;

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

        let left_margin = *self.texture_slice.left_margin as f32;
        let right_margin = *self.texture_slice.right_margin as f32;
        let top_margin = *self.texture_slice.top_margin as f32;
        let bottom_margin = *self.texture_slice.bottom_margin as f32;

        let mut region = Rect {
            position: self.texture_slice.texture_region.position.cast::<f32>(),
            size: self.texture_slice.texture_region.size.cast::<f32>(),
        };

        if region.size.x == 0.0 && region.size.y == 0.0 {
            region.size.x = texture_width;
            region.size.y = texture_height;
        }

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

        let stretch_mode = *self.stretch_mode;
        let mut draw_piece = |bounds: Rect<f32>, tex_coords: &[Vector2<f32>; 4]| match stretch_mode
        {
            StretchMode::Stretch => {
                draw_image(
                    texture,
                    bounds,
                    tex_coords,
                    self.clip_bounds(),
                    self.widget.background(),
                    drawing_context,
                );
            }
            StretchMode::Tile => draw_tiled_image(
                texture,
                texture_width,
                texture_height,
                bounds,
                tex_coords,
                self.clip_bounds(),
                self.widget.background(),
                drawing_context,
            ),
        };

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
        draw_piece(bounds, &tex_coords);

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
        draw_piece(bounds, &tex_coords);

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
        draw_piece(bounds, &tex_coords);
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
        draw_piece(bounds, &tex_coords);

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
            draw_piece(bounds, &tex_coords);
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
        draw_piece(bounds, &tex_coords);

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
        draw_piece(bounds, &tex_coords);

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
        draw_piece(bounds, &tex_coords);

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
        draw_piece(bounds, &tex_coords);

        //end drawing
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<NinePatchMessage>() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
            {
                let slice = &mut self.texture_slice;
                match msg {
                    NinePatchMessage::LeftMargin(margin) => {
                        compare_and_set(slice.left_margin.deref_mut(), margin, message, ui);
                    }
                    NinePatchMessage::RightMargin(margin) => {
                        compare_and_set(slice.right_margin.deref_mut(), margin, message, ui);
                    }
                    NinePatchMessage::TopMargin(margin) => {
                        compare_and_set(slice.top_margin.deref_mut(), margin, message, ui);
                    }
                    NinePatchMessage::BottomMargin(margin) => {
                        compare_and_set(slice.bottom_margin.deref_mut(), margin, message, ui);
                    }
                    NinePatchMessage::TextureRegion(region) => {
                        compare_and_set(slice.texture_region.deref_mut(), region, message, ui);
                    }
                    NinePatchMessage::Texture(texture) => {
                        compare_and_set(&mut slice.texture_source, texture, message, ui);
                    }
                    NinePatchMessage::DrawCenter(draw_center) => {
                        compare_and_set(self.draw_center.deref_mut(), draw_center, message, ui);
                    }
                }
            }
        }
    }
}

/// Creates instances of [`NinePatch`] widget.
pub struct NinePatchBuilder {
    pub widget_builder: WidgetBuilder,
    pub texture: Option<TextureResource>,
    pub bottom_margin: u32,
    pub left_margin: u32,
    pub right_margin: u32,
    pub top_margin: u32,
    pub texture_region: Rect<u32>,
    pub draw_center: bool,
    pub stretch_mode: StretchMode,
}

impl NinePatchBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            texture: None,
            bottom_margin: 20,
            left_margin: 20,
            right_margin: 20,
            top_margin: 20,
            texture_region: Rect::new(0, 0, 200, 200),
            draw_center: true,
            stretch_mode: Default::default(),
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
        self.texture_region = rect;
        self
    }

    pub fn with_draw_center(mut self, draw_center: bool) -> Self {
        self.draw_center = draw_center;
        self
    }

    pub fn with_stretch_mode(mut self, stretch: StretchMode) -> Self {
        self.stretch_mode = stretch;
        self
    }

    pub fn build(mut self, ctx: &mut BuildContext) -> Handle<UiNode> {
        if self.widget_builder.background.is_none() {
            self.widget_builder.background = Some(Brush::Solid(Color::WHITE).into())
        }

        ctx.add_node(UiNode::new(NinePatch {
            widget: self.widget_builder.build(ctx),
            texture_slice: TextureSlice {
                texture_source: self.texture.clone(),
                bottom_margin: self.bottom_margin.into(),
                left_margin: self.left_margin.into(),
                right_margin: self.right_margin.into(),
                top_margin: self.top_margin.into(),
                texture_region: self.texture_region.into(),
            },
            draw_center: self.draw_center.into(),
            texture: self.texture.into(),
            stretch_mode: self.stretch_mode.into(),
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
