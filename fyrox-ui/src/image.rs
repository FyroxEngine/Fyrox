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

//! Image widget is a rectangle with a texture, it is used draw custom bitmaps. See [`Image`] docs for more info
//! and usage examples.

#![warn(missing_docs)]

use crate::{
    brush::Brush,
    color::draw_checker_board,
    core::{
        algebra::Vector2, color::Color, math::Rect, pool::Handle, reflect::prelude::*,
        type_traits::prelude::*, variable::InheritableVariable, visitor::prelude::*,
    },
    define_constructor,
    draw::{CommandTexture, Draw, DrawingContext},
    message::{MessageDirection, UiMessage},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UiNode, UserInterface,
};

use fyrox_graph::constructor::{ConstructorProvider, GraphNodeConstructor};
use fyrox_texture::{TextureKind, TextureResource};
use std::ops::{Deref, DerefMut};

/// A set of messages that could be used to alter [`Image`] widget state at runtime.
#[derive(Debug, Clone, PartialEq)]
pub enum ImageMessage {
    /// Used to set new texture of the [`Image`] widget.
    Texture(Option<TextureResource>),
    /// Used to enable or disable texture flip of the [`Image`] widget. See respective [section](Image#vertical-flip)
    /// of the docs for more info.
    Flip(bool),
    /// Used to set specific portion of the texture. See respective [section](Image#drawing-only-a-portion-of-the-texture)
    /// of the docs for more info.
    UvRect(Rect<f32>),
    /// Used to enable or disable checkerboard background. See respective [section](Image#checkerboard-background) of the
    /// docs for more info.
    CheckerboardBackground(bool),
}

impl ImageMessage {
    define_constructor!(
        /// Creates [`ImageMessage::Texture`] message.
        ImageMessage:Texture => fn texture(Option<TextureResource>), layout: false
    );

    define_constructor!(
        /// Creates [`ImageMessage::Flip`] message.
        ImageMessage:Flip => fn flip(bool), layout: false
    );

    define_constructor!(
        /// Creates [`ImageMessage::UvRect`] message.
        ImageMessage:UvRect => fn uv_rect(Rect<f32>), layout: false
    );

    define_constructor!(
        /// Creates [`ImageMessage::CheckerboardBackground`] message.
        ImageMessage:CheckerboardBackground => fn checkerboard_background(bool), layout: false
    );
}

/// Image widget is a rectangle with a texture, it is used draw custom bitmaps. The UI in the engine is vector-based, Image
/// widget is the only way to draw a bitmap. Usage of the Image is very simple:
///
/// ## Usage
///
/// ```rust,no_run
/// # use fyrox_texture::TextureResource;
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     image::ImageBuilder, widget::WidgetBuilder, BuildContext, UiNode,
/// # };
///
/// fn create_image(ctx: &mut BuildContext, texture: TextureResource) -> Handle<UiNode> {
///     ImageBuilder::new(WidgetBuilder::new())
///         .with_texture(texture)
///         .build(ctx)
/// }
/// ```
///
/// By default, the Image widget will try to use the size of the texture as its desired size for layout
/// process. This means that the widget will be as large as the texture if the outer bounds allows
/// that. You can specify the desired width and height manually and the image will shrink/expand
/// automatically.
///
/// Keep in mind, that texture is a resource, and it could be loaded asynchronously, and during that
/// process, the UI can't fetch texture's size, and it will be collapsed into a point. After it fully
/// loaded, the widget will take texture's size as normal.
///
/// ## Vertical Flip
///
/// In some rare cases you need to flip your source image before showing it, there is `.with_flip` option for that:
///
/// ```rust,no_run
/// # use fyrox_texture::TextureResource;
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     image::ImageBuilder, widget::WidgetBuilder, BuildContext, UiNode
/// # };
///
/// fn create_image(ctx: &mut BuildContext, texture: TextureResource) -> Handle<UiNode> {
///     ImageBuilder::new(WidgetBuilder::new().with_width(100.0).with_height(100.0))
///         .with_flip(true) // Flips an image vertically
///         .with_texture(texture)
///         .build(ctx)
/// }
/// ```
///
/// There are few places where it can be helpful:
///
/// - You're using render target as a source texture for your [`Image`] instance, render targets are vertically flipped due
/// to mismatch of coordinates of UI and graphics API. The UI has origin at left top corner, the graphics API - bottom left.
/// - Your source image is vertically mirrored.
///
/// ## Checkerboard background
///
/// Image widget supports checkerboard background that could be useful for images with alpha channel (transparency). It can
/// be enabled either when building the widget or via [`ImageMessage::CheckerboardBackground`] message:
///
/// ```rust,no_run
/// # use fyrox_texture::TextureResource;
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     image::ImageBuilder, widget::WidgetBuilder, BuildContext, UiNode
/// # };
///
/// fn create_image(ctx: &mut BuildContext, texture: TextureResource) -> Handle<UiNode> {
///     ImageBuilder::new(WidgetBuilder::new().with_width(100.0).with_height(100.0))
///         .with_checkerboard_background(true) // Turns on checkerboard background.
///         .with_texture(texture)
///         .build(ctx)
/// }
/// ```
///
/// ## Drawing only a portion of the texture
///
/// Specific cases requires to be able to draw a specific rectangular portion of the texture. It could be done by using
/// custom UV rect (UV stands for XY coordinates, but texture related):
///
/// ```rust,no_run
/// # use fyrox_texture::TextureResource;
/// # use fyrox_ui::{
/// #     core::{pool::Handle, math::Rect},
/// #     image::ImageBuilder, widget::WidgetBuilder, BuildContext, UiNode
/// # };
///
/// fn create_image(ctx: &mut BuildContext, texture: TextureResource) -> Handle<UiNode> {
///     ImageBuilder::new(WidgetBuilder::new().with_width(100.0).with_height(100.0))
///         .with_uv_rect(Rect::new(0.0, 0.0, 0.25, 0.25)) // Uses top-left quadrant of the texture.
///         .with_texture(texture)
///         .build(ctx)
/// }
/// ```
///
/// Keep in mind, that the rectangle uses _normalized_ coordinates. This means that the entire image dimensions (for both
/// X and Y axes) "compressed" to `0.0..1.0` range. In this case 0.0 means left corner for X axis and top for Y axis, while
/// 1.0 means right corner for X axis and bottom for Y axis.
///
/// It is useful if you have many custom UI elements packed in a single texture atlas. Drawing using atlases is much more
/// efficient and faster. This could also be used for animations, when you have multiple frames packed in a single atlas
/// and changing texture coordinates over the time.
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "18e18d0f-cb84-4ac1-8050-3480a2ec3de5")]
#[visit(optional)]
#[reflect(derived_type = "UiNode")]
pub struct Image {
    /// Base widget of the image.
    pub widget: Widget,
    /// Current texture of the image.
    pub texture: InheritableVariable<Option<TextureResource>>,
    /// Defines whether to vertically flip the image or not.
    pub flip: InheritableVariable<bool>,
    /// Specifies an arbitrary portion of the texture.
    pub uv_rect: InheritableVariable<Rect<f32>>,
    /// Defines whether to use the checkerboard background or not.
    pub checkerboard_background: InheritableVariable<bool>,
    /// Defines whether the image should keep its aspect ratio or stretch to the available size.
    pub keep_aspect_ratio: InheritableVariable<bool>,
}

impl ConstructorProvider<UiNode, UserInterface> for Image {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("Image", |ui| {
                ImageBuilder::new(
                    WidgetBuilder::new()
                        .with_height(32.0)
                        .with_width(32.0)
                        .with_name("Image"),
                )
                .build(&mut ui.build_ctx())
                .into()
            })
            .with_group("Visual")
    }
}

crate::define_widget_deref!(Image);

impl Control for Image {
    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        let mut size: Vector2<f32> = self.widget.measure_override(ui, available_size);

        if let Some(texture) = self.texture.as_ref() {
            let state = texture.state();
            if let Some(data) = state.data_ref() {
                if let TextureKind::Rectangle { width, height } = data.kind() {
                    let width = width as f32;
                    let height = height as f32;

                    if *self.keep_aspect_ratio {
                        let aspect_ratio = width / height;
                        size.x = size.x.max(width).min(available_size.x);
                        size.y = size.x * aspect_ratio;
                    } else {
                        size.x = size.x.max(width);
                        size.y = size.y.max(height);
                    }
                }
            }
        }

        size
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.bounding_rect();

        if *self.checkerboard_background {
            draw_checker_board(bounds, self.clip_bounds(), 8.0, drawing_context);
        }

        if self.texture.is_some() || !*self.checkerboard_background {
            let tex_coords = if *self.flip {
                Some([
                    Vector2::new(self.uv_rect.position.x, self.uv_rect.position.y),
                    Vector2::new(
                        self.uv_rect.position.x + self.uv_rect.size.x,
                        self.uv_rect.position.y,
                    ),
                    Vector2::new(
                        self.uv_rect.position.x + self.uv_rect.size.x,
                        self.uv_rect.position.y - self.uv_rect.size.y,
                    ),
                    Vector2::new(
                        self.uv_rect.position.x,
                        self.uv_rect.position.y - self.uv_rect.size.y,
                    ),
                ])
            } else {
                Some([
                    Vector2::new(self.uv_rect.position.x, self.uv_rect.position.y),
                    Vector2::new(
                        self.uv_rect.position.x + self.uv_rect.size.x,
                        self.uv_rect.position.y,
                    ),
                    Vector2::new(
                        self.uv_rect.position.x + self.uv_rect.size.x,
                        self.uv_rect.position.y + self.uv_rect.size.y,
                    ),
                    Vector2::new(
                        self.uv_rect.position.x,
                        self.uv_rect.position.y + self.uv_rect.size.y,
                    ),
                ])
            };
            drawing_context.push_rect_filled(&bounds, tex_coords.as_ref());
            let texture = self
                .texture
                .as_ref()
                .map_or(CommandTexture::None, |t| CommandTexture::Texture(t.clone()));
            drawing_context.commit(self.clip_bounds(), self.widget.background(), texture, None);
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<ImageMessage>() {
            if message.destination() == self.handle {
                match msg {
                    ImageMessage::Texture(tex) => {
                        self.texture.set_value_and_mark_modified(tex.clone());
                    }
                    &ImageMessage::Flip(flip) => {
                        self.flip.set_value_and_mark_modified(flip);
                    }
                    ImageMessage::UvRect(uv_rect) => {
                        self.uv_rect.set_value_and_mark_modified(*uv_rect);
                    }
                    ImageMessage::CheckerboardBackground(value) => {
                        self.checkerboard_background
                            .set_value_and_mark_modified(*value);
                    }
                }
            }
        }
    }
}

/// Image builder is used to create [`Image`] widget instances and register them in the user interface.
pub struct ImageBuilder {
    widget_builder: WidgetBuilder,
    texture: Option<TextureResource>,
    flip: bool,
    uv_rect: Rect<f32>,
    checkerboard_background: bool,
    keep_aspect_ratio: bool,
}

impl ImageBuilder {
    /// Creates new image builder with the base widget builder specified.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            texture: None,
            flip: false,
            uv_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            checkerboard_background: false,
            keep_aspect_ratio: true,
        }
    }

    /// Sets whether the image should be flipped vertically or not. See respective
    /// [section](Image#vertical-flip) of the docs for more info.
    pub fn with_flip(mut self, flip: bool) -> Self {
        self.flip = flip;
        self
    }

    /// Sets the texture that will be used for drawing.
    pub fn with_texture(mut self, texture: TextureResource) -> Self {
        self.texture = Some(texture);
        self
    }

    /// Specifies the texture that will be used for drawing.
    pub fn with_opt_texture(mut self, texture: Option<TextureResource>) -> Self {
        self.texture = texture;
        self
    }

    /// Specifies a portion of the texture in normalized coordinates. See respective
    /// [section](Image#drawing-only-a-portion-of-the-texture) of the docs for more info.
    pub fn with_uv_rect(mut self, uv_rect: Rect<f32>) -> Self {
        self.uv_rect = uv_rect;
        self
    }

    /// Sets whether the image should use checkerboard background or not. See respective
    /// [section](Image#checkerboard-background) of the docs for more info.
    pub fn with_checkerboard_background(mut self, checkerboard_background: bool) -> Self {
        self.checkerboard_background = checkerboard_background;
        self
    }

    /// Sets whether the image should keep its aspect ratio or stretch to the available size.
    pub fn with_keep_aspect_ratio(mut self, keep_aspect_ratio: bool) -> Self {
        self.keep_aspect_ratio = keep_aspect_ratio;
        self
    }

    /// Builds the [`Image`] widget, but does not add it to the UI.
    pub fn build_node(mut self, ctx: &BuildContext) -> UiNode {
        if self.widget_builder.background.is_none() {
            self.widget_builder.background = Some(Brush::Solid(Color::WHITE).into())
        }

        let image = Image {
            widget: self.widget_builder.build(ctx),
            texture: self.texture.into(),
            flip: self.flip.into(),
            uv_rect: self.uv_rect.into(),
            checkerboard_background: self.checkerboard_background.into(),
            keep_aspect_ratio: self.keep_aspect_ratio.into(),
        };
        UiNode::new(image)
    }

    /// Builds the [`Image`] widget and adds it to the UI and returns its handle.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        ctx.add_node(self.build_node(ctx))
    }
}

#[cfg(test)]
mod test {
    use crate::image::ImageBuilder;
    use crate::{test::test_widget_deletion, widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| ImageBuilder::new(WidgetBuilder::new()).build(ctx));
    }
}
