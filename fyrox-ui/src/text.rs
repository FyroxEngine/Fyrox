//! Text is a simple widget that allows you to print text on screen. See [`Text`] docs for more info and
//! examples.

#![warn(missing_docs)]

use crate::{
    brush::Brush,
    core::{
        algebra::Vector2, color::Color, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*,
    },
    define_constructor,
    draw::DrawingContext,
    font::FontResource,
    formatted_text::{FormattedText, FormattedTextBuilder, WrapMode},
    message::{MessageDirection, UiMessage},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, HorizontalAlignment, UiNode, UserInterface, VerticalAlignment,
};
use fyrox_core::uuid_provider;
use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
};

/// Possible messages that can be used to alternate [`Text`] widget state at runtime.
#[derive(Debug, Clone, PartialEq)]
pub enum TextMessage {
    /// Used to set new text of the widget.
    Text(String),
    /// Used to set new text wrapping mode of the widget. See [Text](Text#text-alignment-and-word-wrapping) for usage
    /// examples.
    Wrap(WrapMode),
    /// Used to set new font of the widget.  See [Text](Text#fonts-and_colors) for usage examples.
    Font(FontResource),
    /// Used to set new vertical alignment of the widget. See [Text](Text#text-alignment-and-word-wrapping) for usage
    /// examples.
    VerticalAlignment(VerticalAlignment),
    /// Used to set new horizontal alignment of the widget. See [Text](Text#text-alignment-and-word-wrapping) for usage
    /// examples.
    HorizontalAlignment(HorizontalAlignment),
    /// Used to enable/disable shadow casting of the widget. See [Text](Text#shadows) for usage examples.
    Shadow(bool),
    /// Used to set new dilation factor of the shadows. See [Text](Text#shadows) for usage examples.
    ShadowDilation(f32),
    /// Used to set new brush that will be used to draw the shadows. See [Text](Text#shadows) for usage examples.
    ShadowBrush(Brush),
    /// Used to set how much the shadows will be offset from the widget. See [Text](Text#shadows) for usage examples.
    ShadowOffset(Vector2<f32>),
    /// Used to set font height of the widget.
    FontSize(f32),
}

impl TextMessage {
    define_constructor!(
        /// Creates new [`TextMessage::Text`] message.
        TextMessage:Text => fn text(String), layout: false
    );

    define_constructor!(
        /// Creates new [`TextMessage::Wrap`] message.
        TextMessage:Wrap => fn wrap(WrapMode), layout: false
    );

    define_constructor!(
        /// Creates new [`TextMessage::Font`] message.
        TextMessage:Font => fn font(FontResource), layout: false
    );

    define_constructor!(
        /// Creates new [`TextMessage::VerticalAlignment`] message.
        TextMessage:VerticalAlignment => fn vertical_alignment(VerticalAlignment), layout: false
    );

    define_constructor!(
        /// Creates new [`TextMessage::HorizontalAlignment`] message.
        TextMessage:HorizontalAlignment => fn horizontal_alignment(HorizontalAlignment), layout: false
    );

    define_constructor!(
        /// Creates new [`TextMessage::Shadow`] message.
        TextMessage:Shadow => fn shadow(bool), layout: false
    );

    define_constructor!(
        /// Creates new [`TextMessage::ShadowDilation`] message.
        TextMessage:ShadowDilation => fn shadow_dilation(f32), layout: false
    );

    define_constructor!(
        /// Creates new [`TextMessage::ShadowBrush`] message.
        TextMessage:ShadowBrush => fn shadow_brush(Brush), layout: false
    );

    define_constructor!(
        /// Creates new [`TextMessage::ShadowOffset`] message.
        TextMessage:ShadowOffset => fn shadow_offset(Vector2<f32>), layout: false
    );

    define_constructor!(
        /// Creates new [`TextMessage::FontSize`] message.
        TextMessage:FontSize => fn font_size(f32), layout: false
    );
}

/// Text is a simple widget that allows you to print text on screen. It has various options like word wrapping, text
/// alignment, and so on.
///
/// ## How to create
///
/// An instance of the [`Text`] widget could be created like so:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     text::TextBuilder, widget::WidgetBuilder, UiNode, UserInterface
/// # };
/// fn create_text(ui: &mut UserInterface, text: &str) -> Handle<UiNode> {
///     TextBuilder::new(WidgetBuilder::new())
///         .with_text(text)
///         .build(&mut ui.build_ctx())
/// }
/// ```
///
/// ## Text alignment and word wrapping
///
/// There are various text alignment options for both vertical and horizontal axes. Typical alignment values are:
/// [`HorizontalAlignment::Left`], [`HorizontalAlignment::Center`], [`HorizontalAlignment::Right`] for horizontal axis,
/// and [`VerticalAlignment::Top`], [`VerticalAlignment::Center`], [`VerticalAlignment::Bottom`] for vertical axis. An
/// instance of centered text could be created like so:
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     text::TextBuilder, widget::WidgetBuilder, HorizontalAlignment, UiNode, UserInterface,
/// #     VerticalAlignment,
/// # };
/// fn create_centered_text(ui: &mut UserInterface, text: &str) -> Handle<UiNode> {
///     TextBuilder::new(WidgetBuilder::new())
///         .with_horizontal_text_alignment(HorizontalAlignment::Center)
///         .with_vertical_text_alignment(VerticalAlignment::Center)
///     .with_text(text)
///     .build(&mut ui.build_ctx())
/// }
/// ```
///
/// What's the difference between widget's alignment and text-specific? Widget's alignment operates on a bounding rectangle
/// of the text and text-specific alignment operates on line-basis. This means that if you set [`HorizontalAlignment::Center`]
/// as widget's alignment, your text lines won't be centered, instead they'll be aligned at the left and the entire text block
/// will be aligned at center.
///
/// Long text is usually needs to wrap on available bounds, there are three possible options for word wrapping:
/// [`WrapMode::NoWrap`], [`WrapMode::Letter`], [`WrapMode::Word`]. An instance of text with word-based wrapping could
/// be created like so:
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     formatted_text::WrapMode, text::TextBuilder, widget::WidgetBuilder, UiNode,
/// #     UserInterface,
/// # };
/// fn create_text_with_word_wrap(ui: &mut UserInterface, text: &str) -> Handle<UiNode> {
///     TextBuilder::new(WidgetBuilder::new())
///         .with_wrap(WrapMode::Word)
///         .with_text(text)
///         .build(&mut ui.build_ctx())
/// }
/// ```
///
/// ## Background
///
/// If you need to have a text with some background, you should use [`crate::border::Border`] widget as a parent widget of your
/// text. **Caveat:** [`WidgetBuilder::with_background`] is ignored for [`Text`] widget!
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     core::{color::Color, pool::Handle},
/// #     border::BorderBuilder, brush::Brush, text::TextBuilder, widget::WidgetBuilder, UiNode,
/// #     UserInterface,
/// # };
/// #
/// fn create_text_with_background(ui: &mut UserInterface, text: &str) -> Handle<UiNode> {
///     let text_widget =
///         TextBuilder::new(WidgetBuilder::new().with_foreground(Brush::Solid(Color::RED)))
///             .with_text(text)
///             .build(&mut ui.build_ctx());
///     BorderBuilder::new(
///         WidgetBuilder::new()
///             .with_child(text_widget) // <-- Text is now a child of the border
///             .with_background(Brush::Solid(Color::opaque(50, 50, 50))),
///     )
///     .build(&mut ui.build_ctx())
/// }
/// ```
///
/// Keep in mind that now the text widget is a child widget of the border, so if you need to position the text, you should
/// position the border, not the text.
///
/// ## Fonts and colors
///
/// To set a color of the text just use [`WidgetBuilder::with_foreground`] while building the text instance:
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     core::{color::Color, pool::Handle},
/// #     brush::Brush, text::TextBuilder, widget::WidgetBuilder, UiNode, UserInterface
/// # };
/// fn create_text(ui: &mut UserInterface, text: &str) -> Handle<UiNode> {
///     //               vvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvv
///     TextBuilder::new(WidgetBuilder::new().with_foreground(Brush::Solid(Color::RED)))
///         .with_text(text)
///         .build(&mut ui.build_ctx())
/// }
/// ```
///
/// By default, text is created with default font, however it is possible to set any custom font:
///
/// ```rust
/// # use fyrox_resource::manager::ResourceManager;
/// # use fyrox_ui::{
/// #     core::{futures::executor::block_on, pool::Handle},
/// #     text::TextBuilder,
/// #     font::{Font, FontResource},
/// #     widget::WidgetBuilder,
/// #     UiNode, UserInterface,
/// # };
///
/// fn create_text(ui: &mut UserInterface, resource_manager: &ResourceManager, text: &str) -> Handle<UiNode> {
///     TextBuilder::new(WidgetBuilder::new())
///         .with_font(resource_manager.request::<Font>("path/to/your/font.ttf"))
///         .with_text(text)
///         .with_font_size(20.0)
///         .build(&mut ui.build_ctx())
/// }
/// ```
///
/// Please refer to [`crate::font::Font`] chapter to learn more about fonts.
///
/// ### Font size
///
/// Use [`TextBuilder::with_font_size`] or send [`TextMessage::font_size`] to your Text widget instance
/// to set the font size of it.
///
/// ## Shadows
///
/// Text widget supports shadows effect to add contrast to your text, which could be useful to make text readable independent
/// on the background colors. This effect could be used for subtitles. Shadows are pretty easy to add, all you need to do
/// is to enable them, setup desired thickness, offset and brush (solid color or gradient).
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     core::{algebra::Vector2, color::Color, pool::Handle},
/// #     brush::Brush, text::TextBuilder, widget::WidgetBuilder, UiNode, UserInterface
/// # };
/// #
/// fn create_red_text_with_black_shadows(ui: &mut UserInterface, text: &str) -> Handle<UiNode> {
///     TextBuilder::new(WidgetBuilder::new().with_foreground(Brush::Solid(Color::RED)))
///         .with_text(text)
///         // Enable shadows.
///         .with_shadow(true)
///         // Black shadows.
///         .with_shadow_brush(Brush::Solid(Color::BLACK))
///         // 1px thick.
///         .with_shadow_dilation(1.0)
///         // Offset the shadow slightly to the right-bottom.
///         .with_shadow_offset(Vector2::new(1.0, 1.0))
///         .build(&mut ui.build_ctx())
/// }
/// ```
///
/// ## Messages
///
/// Text widget can accept the following list of messages at runtime (respective constructors are name with small letter -
/// `TextMessage::Text -> TextMessage::text(widget_handle, direction, text)`):
///
/// - [`TextMessage::Text`] - sets new text for a `Text` widget.
/// - [`TextMessage::Wrap`] - sets new [wrapping mode](Text#text-alignment-and-word-wrapping).
/// - [`TextMessage::Font`] - sets new [font](Text#fonts-and-colors)
/// - [`TextMessage::VerticalAlignment`] and `TextMessage::HorizontalAlignment` sets
/// [vertical and horizontal](Text#text-alignment-and-word-wrapping) text alignment respectively.
/// - [`TextMessage::Shadow`] - enables or disables [shadow casting](Text#shadows)
/// - [`TextMessage::ShadowDilation`] - sets "thickness" of the shadows under the tex.
/// - [`TextMessage::ShadowBrush`] - sets shadow brush (allows you to change color and even make shadow with color gradients).
/// - [`TextMessage::ShadowOffset`] - sets offset of the shadows.
///
/// An example of changing text at runtime could be something like this:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     message::{MessageDirection},
/// #     UiNode, UserInterface,
/// #     text::TextMessage
/// # };
/// fn request_change_text(ui: &UserInterface, text_widget_handle: Handle<UiNode>, text: &str) {
///     ui.send_message(TextMessage::text(
///         text_widget_handle,
///         MessageDirection::ToWidget,
///         text.to_owned(),
///     ))
/// }
/// ```
///
/// Please keep in mind, that like any other situation when you "changing" something via messages, you should remember
/// that the change is **not** immediate.
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct Text {
    /// Base widget of the Text widget.
    pub widget: Widget,
    /// [`FormattedText`] instance that is used to layout text and generate drawing commands.
    pub formatted_text: RefCell<FormattedText>,
}

crate::define_widget_deref!(Text);

uuid_provider!(Text = "22f7f502-7622-4ecb-8c5f-ba436e7ee823");

impl Control for Text {
    fn measure_override(&self, _: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.formatted_text
            .borrow_mut()
            .set_constraint(available_size)
            .build()
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.formatted_text
            .borrow_mut()
            .set_brush(self.widget.foreground());
        let bounds = self.widget.bounding_rect();
        drawing_context.draw_text(
            self.clip_bounds(),
            bounds.position,
            &self.formatted_text.borrow(),
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle() {
            if let Some(msg) = message.data::<TextMessage>() {
                let mut text_ref = self.formatted_text.borrow_mut();
                match msg {
                    TextMessage::Text(text) => {
                        text_ref.set_text(text);
                        drop(text_ref);
                        self.invalidate_layout();
                    }
                    &TextMessage::Wrap(wrap) => {
                        if text_ref.wrap_mode() != wrap {
                            text_ref.set_wrap(wrap);
                            drop(text_ref);
                            self.invalidate_layout();
                        }
                    }
                    TextMessage::Font(font) => {
                        if &text_ref.get_font() != font {
                            text_ref.set_font(font.clone());
                            drop(text_ref);
                            self.invalidate_layout();
                        }
                    }
                    &TextMessage::HorizontalAlignment(horizontal_alignment) => {
                        if text_ref.horizontal_alignment() != horizontal_alignment {
                            text_ref.set_horizontal_alignment(horizontal_alignment);
                            drop(text_ref);
                            self.invalidate_layout();
                        }
                    }
                    &TextMessage::VerticalAlignment(vertical_alignment) => {
                        if text_ref.vertical_alignment() != vertical_alignment {
                            text_ref.set_vertical_alignment(vertical_alignment);
                            drop(text_ref);
                            self.invalidate_layout();
                        }
                    }
                    &TextMessage::Shadow(shadow) => {
                        if *text_ref.shadow != shadow {
                            text_ref.set_shadow(shadow);
                            drop(text_ref);
                            self.invalidate_layout();
                        }
                    }
                    TextMessage::ShadowBrush(brush) => {
                        if &*text_ref.shadow_brush != brush {
                            text_ref.set_shadow_brush(brush.clone());
                            drop(text_ref);
                            self.invalidate_layout();
                        }
                    }
                    &TextMessage::ShadowDilation(dilation) => {
                        if *text_ref.shadow_dilation != dilation {
                            text_ref.set_shadow_dilation(dilation);
                            drop(text_ref);
                            self.invalidate_layout();
                        }
                    }
                    &TextMessage::ShadowOffset(offset) => {
                        if *text_ref.shadow_offset != offset {
                            text_ref.set_shadow_offset(offset);
                            drop(text_ref);
                            self.invalidate_layout();
                        }
                    }
                    &TextMessage::FontSize(height) => {
                        if text_ref.font_size() != height {
                            text_ref.set_font_size(height);
                            drop(text_ref);
                            self.invalidate_layout();
                        }
                    }
                }
            }
        }
    }
}

impl Text {
    /// Returns current text wrapping mode of the widget.
    pub fn wrap_mode(&self) -> WrapMode {
        self.formatted_text.borrow().wrap_mode()
    }

    /// Returns current text of the widget.
    pub fn text(&self) -> String {
        self.formatted_text.borrow().text()
    }

    /// Returns current font of the widget.
    pub fn font(&self) -> FontResource {
        self.formatted_text.borrow().get_font()
    }

    /// Returns current vertical alignment of the widget.
    pub fn vertical_alignment(&self) -> VerticalAlignment {
        self.formatted_text.borrow().vertical_alignment()
    }

    /// Returns current horizontal alignment of the widget.
    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        self.formatted_text.borrow().horizontal_alignment()
    }
}

/// TextBuilder is used to create instances of [`Text`] widget and register them in the user interface.
pub struct TextBuilder {
    widget_builder: WidgetBuilder,
    text: Option<String>,
    font: Option<FontResource>,
    vertical_text_alignment: VerticalAlignment,
    horizontal_text_alignment: HorizontalAlignment,
    wrap: WrapMode,
    shadow: bool,
    shadow_brush: Brush,
    shadow_dilation: f32,
    shadow_offset: Vector2<f32>,
    font_size: f32,
}

impl TextBuilder {
    /// Creates new [`TextBuilder`] instance using the provided base widget builder.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            text: None,
            font: None,
            vertical_text_alignment: VerticalAlignment::Top,
            horizontal_text_alignment: HorizontalAlignment::Left,
            wrap: WrapMode::NoWrap,
            shadow: false,
            shadow_brush: Brush::Solid(Color::BLACK),
            shadow_dilation: 1.0,
            shadow_offset: Vector2::new(1.0, 1.0),
            font_size: 14.0,
        }
    }

    /// Sets the desired text of the widget.
    pub fn with_text<P: AsRef<str>>(mut self, text: P) -> Self {
        self.text = Some(text.as_ref().to_owned());
        self
    }

    /// Sets the desired font of the widget.
    pub fn with_font(mut self, font: FontResource) -> Self {
        self.font = Some(font);
        self
    }

    /// Sets the desired font of the widget using font wrapped in [`Option`].
    pub fn with_opt_font(mut self, font: Option<FontResource>) -> Self {
        self.font = font;
        self
    }

    /// Sets the desired vertical alignment of the widget.
    pub fn with_vertical_text_alignment(mut self, valign: VerticalAlignment) -> Self {
        self.vertical_text_alignment = valign;
        self
    }

    /// Sets the desired height of the text.
    pub fn with_font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    /// Sets the desired horizontal alignment of the widget.
    pub fn with_horizontal_text_alignment(mut self, halign: HorizontalAlignment) -> Self {
        self.horizontal_text_alignment = halign;
        self
    }

    /// Sets the desired word wrapping mode of the widget.
    pub fn with_wrap(mut self, wrap: WrapMode) -> Self {
        self.wrap = wrap;
        self
    }

    /// Whether the shadow enabled or not.
    pub fn with_shadow(mut self, shadow: bool) -> Self {
        self.shadow = shadow;
        self
    }

    /// Sets desired shadow brush. It will be used to render the shadow.
    pub fn with_shadow_brush(mut self, brush: Brush) -> Self {
        self.shadow_brush = brush;
        self
    }

    /// Sets desired shadow dilation in units. Keep in mind that the dilation is absolute,
    /// not percentage-based.
    pub fn with_shadow_dilation(mut self, thickness: f32) -> Self {
        self.shadow_dilation = thickness;
        self
    }

    /// Sets desired shadow offset in units.
    pub fn with_shadow_offset(mut self, offset: Vector2<f32>) -> Self {
        self.shadow_offset = offset;
        self
    }

    /// Finishes text widget creation and registers it in the user interface, returning its handle to you.
    pub fn build(mut self, ui: &mut BuildContext) -> Handle<UiNode> {
        let font = if let Some(font) = self.font {
            font
        } else {
            ui.default_font()
        };

        if self.widget_builder.foreground.is_none() {
            self.widget_builder.foreground = Some(Brush::Solid(Color::opaque(220, 220, 220)));
        }

        let text = Text {
            widget: self.widget_builder.build(),
            formatted_text: RefCell::new(
                FormattedTextBuilder::new(font)
                    .with_text(self.text.unwrap_or_default())
                    .with_vertical_alignment(self.vertical_text_alignment)
                    .with_horizontal_alignment(self.horizontal_text_alignment)
                    .with_wrap(self.wrap)
                    .with_shadow(self.shadow)
                    .with_shadow_brush(self.shadow_brush)
                    .with_shadow_dilation(self.shadow_dilation)
                    .with_shadow_offset(self.shadow_offset)
                    .with_font_size(self.font_size)
                    .build(),
            ),
        };
        ui.add_node(UiNode::new(text))
    }
}
