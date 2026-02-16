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

//! Text is a simple widget that allows you to print text on screen. See [`Text`] docs for more info and
//! examples.

#![warn(missing_docs)]

use crate::{
    brush::Brush,
    core::{
        algebra::Vector2, color::Color, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        uuid_provider, visitor::prelude::*,
    },
    draw::DrawingContext,
    font::FontResource,
    formatted_text::{FormattedText, FormattedTextBuilder, Run, RunSet, WrapMode},
    message::{MessageData, UiMessage},
    style::{resource::StyleResourceExt, Style, StyledProperty},
    widget::{Widget, WidgetBuilder},
    BBCode, BuildContext, Control, HorizontalAlignment, UiNode, UserInterface, VerticalAlignment,
};
use fyrox_core::algebra::Matrix3;
use fyrox_core::variable::InheritableVariable;
use fyrox_graph::constructor::{ConstructorProvider, GraphNodeConstructor};
use std::cell::RefCell;

/// Possible messages that can be used to alternate [`Text`] widget state at runtime.
#[derive(Debug, Clone, PartialEq)]
pub enum TextMessage {
    /// Used to set a new text and runs with BBCode tags.
    BBCode(String),
    /// Used to set a new text or to receive the changed text.
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
    FontSize(StyledProperty<f32>),
    /// Used to set the new set of runs in the text.
    Runs(RunSet),
}
impl MessageData for TextMessage {}

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
/// #     text::{Text, TextBuilder}, widget::WidgetBuilder, UiNode, UserInterface
/// # };
/// fn create_text(ui: &mut UserInterface, text: &str) -> Handle<Text> {
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
/// #     text::{Text, TextBuilder}, widget::WidgetBuilder, HorizontalAlignment, UiNode, UserInterface,
/// #     VerticalAlignment,
/// # };
/// fn create_centered_text(ui: &mut UserInterface, text: &str) -> Handle<Text> {
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
/// #     formatted_text::WrapMode, text::{Text, TextBuilder}, widget::WidgetBuilder, UiNode,
/// #     UserInterface,
/// # };
/// fn create_text_with_word_wrap(ui: &mut UserInterface, text: &str) -> Handle<Text> {
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
/// # use fyrox_ui::border::Border;
/// #
/// fn create_text_with_background(ui: &mut UserInterface, text: &str) -> Handle<Border> {
///     let text_widget =
///         TextBuilder::new(WidgetBuilder::new().with_foreground(Brush::Solid(Color::RED).into()))
///             .with_text(text)
///             .build(&mut ui.build_ctx());
///     BorderBuilder::new(
///         WidgetBuilder::new()
///             .with_child(text_widget) // <-- Text is now a child of the border
///             .with_background(Brush::Solid(Color::opaque(50, 50, 50)).into()),
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
/// To set a color of the text, just use [`WidgetBuilder::with_foreground`] while building the text instance:
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     core::{color::Color, pool::Handle},
/// #     brush::Brush, text::{Text, TextBuilder}, widget::WidgetBuilder, UiNode, UserInterface
/// # };
/// fn create_text(ui: &mut UserInterface, text: &str) -> Handle<Text> {
///     //               vvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvv
///     TextBuilder::new(WidgetBuilder::new().with_foreground(Brush::Solid(Color::RED).into()))
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
/// #     text::{Text, TextBuilder},
/// #     font::{Font, FontResource},
/// #     widget::WidgetBuilder,
/// #     UiNode, UserInterface,
/// # };
///
/// fn create_text(ui: &mut UserInterface, resource_manager: &ResourceManager, text: &str) -> Handle<Text> {
///     TextBuilder::new(WidgetBuilder::new())
///         .with_font(resource_manager.request::<Font>("path/to/your/font.ttf"))
///         .with_text(text)
///         .with_font_size(20.0f32.into())
///         .build(&mut ui.build_ctx())
/// }
/// ```
///
/// Please refer to [`crate::font::Font`] chapter to learn more about fonts.
///
/// ### Font size
///
/// Use [`TextBuilder::with_font_size`] or send [`TextMessage::FontSize`] to your Text widget instance
/// to set the font size of it.
///
/// ## Shadows
///
/// Text widget supports shadows effect to add contrast to your text, which could be useful to make text readable independent
///  of the background colors. This effect could be used for subtitles. Shadows are pretty easy to add, all you need to do
/// is to enable them, setup desired thickness, offset and brush (solid color or gradient).
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     core::{algebra::Vector2, color::Color, pool::Handle},
/// #     brush::Brush, text::{Text, TextBuilder}, widget::WidgetBuilder, UiNode, UserInterface
/// # };
/// #
/// fn create_red_text_with_black_shadows(ui: &mut UserInterface, text: &str) -> Handle<Text> {
///     TextBuilder::new(WidgetBuilder::new().with_foreground(Brush::Solid(Color::RED).into()))
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
/// ## Runs
///
/// Formatting options such as fonts, shadows, sizes, and brushes can be independently controlled
/// for each character in text by adding formatting runs to the text with [`Run`].
/// Each run has a range of `char` positions within the text and fields to control formatting.
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     core::{algebra::Vector2, color::Color, pool::Handle},
/// #     brush::Brush, text::{Text, TextBuilder}, widget::WidgetBuilder, UiNode, UserInterface,
/// #     formatted_text::Run,
/// # };
/// #
/// fn create_text_with_red_run(ui: &mut UserInterface, text: &str) -> Handle<Text> {
///     TextBuilder::new(WidgetBuilder::new())
///         .with_text(text)
///         .with_run(Run::new(5..22).with_brush(Brush::Solid(Color::RED)).with_shadow(true))
///         .build(&mut ui.build_ctx())
/// }
/// ```
///
/// ## BBCode
///
/// Text widget supports BBCode to add runs of various formatting to text.
/// The available tags are:
/// * `[b]` **bold text** `[/b]`
/// * `[i]` *italic text* `[/i]`
/// * `[color=red]` red text `[/color]` (can be shortened to `[c=red]`... `[/c]`, and can use hex color as in `[color=#FF0000]`)
/// * `[size=24]` large text `[/size]` (can be shortened to `[s=24]` ... `[/s]`)
/// * `[shadow]` shadowed text `[/shadow]` (can be shortened to `[sh]` ... `[/sh]` and can change shadow color with `[shadow=blue]`)
/// * `[br]` for a line break.
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     core::{algebra::Vector2, color::Color, pool::Handle},
/// #     brush::Brush, text::{Text, TextBuilder}, widget::WidgetBuilder, UiNode, UserInterface
/// # };
/// #
/// fn create_text_with_bbcode(ui: &mut UserInterface) -> Handle<Text> {
///     TextBuilder::new(WidgetBuilder::new())
///         .with_bbcode("BBCode example: [b][c=blue]bold and blue[/c][/b]")
///         .build(&mut ui.build_ctx())
/// }
/// ```
///
/// ## Messages
///
/// Text widget can accept the following list of messages at runtime (respective constructors are named with small letter -
/// `TextMessage::Text -> TextMessage::text(widget_handle, direction, text)`):
///
/// - [`TextMessage::BBCode`] - sets the text and formatting runs using BBCode.
/// - [`TextMessage::Text`] - sets new text for a `Text` widget.
/// - [`TextMessage::Wrap`] - sets new [wrapping mode](Text#text-alignment-and-word-wrapping).
/// - [`TextMessage::Font`] - sets new [font](Text#fonts-and-colors)
/// - [`TextMessage::VerticalAlignment`] and `TextMessage::HorizontalAlignment` sets
/// [vertical and horizontal](Text#text-alignment-and-word-wrapping) text alignment respectively.
/// - [`TextMessage::Shadow`] - enables or disables [shadow casting](Text#shadows)
/// - [`TextMessage::ShadowDilation`] - sets "thickness" of the shadows under the tex.
/// - [`TextMessage::ShadowBrush`] - sets shadow brush (allows you to change color and even make shadow with color gradients).
/// - [`TextMessage::ShadowOffset`] - sets offset of the shadows.
/// - [`TextMessage::Runs`] - sets the formatting runs for the text.
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
///     ui.send(text_widget_handle, TextMessage::Text(text.to_owned()))
/// }
/// ```
///
/// Please keep in mind, that like any other situation when you "changing" something via messages, you should remember
/// that the change is **not** immediate.
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct Text {
    /// Base widget of the Text widget.
    pub widget: Widget,
    /// Text that may have BBCode tags to automatically generate formatting runs.
    /// The available tags are:
    /// * `[b]` **bold text** `[/b]`
    /// * `[i]` *italic text* `[/i]`
    /// * `[color=red]` red text `[/color]` (can be shortened to `[c=red]`... `[/c]`, and can use hex color as in `[color=#FF0000]`)
    /// * `[size=24]` large text `[/size]` (can be shortened to `[s=24]` ... `[/s]`)
    /// * `[shadow]` shadowed text `[/shadow]` (can be shortened to `[sh]` ... `[/sh]` and can change shadow color with `[shadow=blue]`)
    /// * `[br]` for a line break.
    #[visit(optional)]
    #[reflect(hidden)]
    pub bbcode: InheritableVariable<String>,
    /// [`FormattedText`] instance that is used to layout text and generate drawing commands.
    pub formatted_text: RefCell<FormattedText>,
}

impl ConstructorProvider<UiNode, UserInterface> for Text {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("Text", |ui| {
                TextBuilder::new(WidgetBuilder::new().with_name("Text"))
                    .with_text("Text")
                    .build(&mut ui.build_ctx())
                    .to_base()
                    .into()
            })
            .with_group("Visual")
    }
}

crate::define_widget_deref!(Text);

uuid_provider!(Text = "22f7f502-7622-4ecb-8c5f-ba436e7ee823");

impl Control for Text {
    fn measure_override(&self, _: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.formatted_text
            .borrow_mut()
            .set_super_sampling_scale(self.visual_max_scaling())
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
            &self.material,
            &self.formatted_text.borrow(),
        );
    }

    fn on_visual_transform_changed(
        &self,
        _old_transform: &Matrix3<f32>,
        _new_transform: &Matrix3<f32>,
    ) {
        let text = self.formatted_text.borrow_mut();
        let new_super_sampling_scale = self.visual_max_scaling();
        if new_super_sampling_scale != text.super_sampling_scale() {
            self.formatted_text
                .borrow_mut()
                .set_super_sampling_scale(new_super_sampling_scale)
                .build();
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle() {
            if let Some(msg) = message.data::<TextMessage>() {
                let mut text_ref = self.formatted_text.borrow_mut();
                match msg {
                    TextMessage::BBCode(text) => {
                        drop(text_ref);
                        self.set_bbcode(text.clone());
                    }
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
                    TextMessage::FontSize(height) => {
                        if text_ref.font_size() != height {
                            text_ref.set_font_size(height.clone());
                            drop(text_ref);
                            self.invalidate_layout();
                        }
                    }
                    TextMessage::Runs(runs) => {
                        text_ref.set_runs(runs.clone());
                        drop(text_ref);
                        self.invalidate_layout();
                    }
                }
            }
        }
    }
}

impl Text {
    /// Modifies the content of the text, with BBCode tags used to set the formatting runs.
    /// The available tags are:
    /// * `[b]` **bold text** `[/b]`
    /// * `[i]` *italic text* `[/i]`
    /// * `[color=red]` red text `[/color]` (can be shortened to `[c=red]`... `[/c]`, and can use hex color as in `[color=#FF0000]`)
    /// * `[size=24]` large text `[/size]` (can be shortened to `[s=24]` ... `[/s]`)
    /// * `[shadow]` shadowed text `[/shadow]` (can be shortened to `[sh]` ... `[/sh]` and can change shadow color with `[shadow=blue]`)
    /// * `[br]` for a line break.
    pub fn set_bbcode(&mut self, code: String) {
        self.bbcode.set_value_and_mark_modified(code);
        let code: BBCode = self.bbcode.parse().unwrap();
        let mut formatted = self.formatted_text.borrow_mut();
        let font = formatted.get_font();
        formatted.set_runs(code.build_runs(&font));
        formatted.set_text(code.text);
        self.invalidate_layout();
    }
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
    bbcode: Option<String>,
    text: Option<String>,
    font: Option<FontResource>,
    vertical_text_alignment: VerticalAlignment,
    horizontal_text_alignment: HorizontalAlignment,
    wrap: WrapMode,
    shadow: bool,
    shadow_brush: Brush,
    shadow_dilation: f32,
    shadow_offset: Vector2<f32>,
    font_size: Option<StyledProperty<f32>>,
    runs: Vec<Run>,
}

impl TextBuilder {
    /// Creates new [`TextBuilder`] instance using the provided base widget builder.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            bbcode: None,
            text: None,
            font: None,
            vertical_text_alignment: VerticalAlignment::Top,
            horizontal_text_alignment: HorizontalAlignment::Left,
            wrap: WrapMode::NoWrap,
            shadow: false,
            shadow_brush: Brush::Solid(Color::BLACK),
            shadow_dilation: 1.0,
            shadow_offset: Vector2::new(1.0, 1.0),
            font_size: None,
            runs: Vec::default(),
        }
    }

    /// Sets the desired text of the widget, with BBcode tags that will
    /// automatically generate the formatting runs and replace any other
    /// runs set through this builder.
    pub fn with_bbcode<P: Into<String>>(mut self, text: P) -> Self {
        self.bbcode = Some(text.into());
        self
    }

    /// Sets the desired text of the widget.
    pub fn with_text<P: Into<String>>(mut self, text: P) -> Self {
        self.text = Some(text.into());
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
    pub fn with_font_size(mut self, font_size: StyledProperty<f32>) -> Self {
        self.font_size = Some(font_size);
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

    /// Adds the given run to the text to set the style for a portion of the text.
    /// Later runs potentially overriding earlier runs if the ranges of the runs overlap and the later run
    /// sets a property that conflicts with an earlier run.
    pub fn with_run(mut self, run: Run) -> Self {
        self.runs.push(run);
        self
    }

    /// Adds multiple runs to the text to set the style of portions of the text.
    /// Later runs potentially overriding earlier runs if the ranges of the runs overlap and the later run
    /// sets a property that conflicts with an earlier run.
    pub fn with_runs<I: IntoIterator<Item = Run>>(mut self, runs: I) -> Self {
        self.runs.extend(runs);
        self
    }

    /// Finishes text widget creation and registers it in the user interface, returning its handle to you.
    pub fn build(mut self, ctx: &mut BuildContext) -> Handle<Text> {
        let font = if let Some(font) = self.font {
            font
        } else {
            ctx.default_font()
        };

        if self.widget_builder.foreground.is_none() {
            self.widget_builder.foreground = Some(ctx.style.property(Style::BRUSH_TEXT));
        }

        let text_builder = if let Some(bbcode) = &self.bbcode {
            let code: BBCode = bbcode.parse().unwrap();
            code.build_formatted_text(font)
        } else {
            FormattedTextBuilder::new(font)
                .with_text(self.text.unwrap_or_default())
                .with_runs(self.runs)
        };
        let formatted_text = text_builder
            .with_vertical_alignment(self.vertical_text_alignment)
            .with_horizontal_alignment(self.horizontal_text_alignment)
            .with_wrap(self.wrap)
            .with_shadow(self.shadow)
            .with_shadow_brush(self.shadow_brush)
            .with_shadow_dilation(self.shadow_dilation)
            .with_shadow_offset(self.shadow_offset)
            .with_font_size(
                self.font_size
                    .unwrap_or_else(|| ctx.style.property(Style::FONT_SIZE)),
            )
            .build();

        let text = Text {
            widget: self.widget_builder.build(ctx),
            bbcode: self.bbcode.unwrap_or_default().into(),
            formatted_text: RefCell::new(formatted_text),
        };
        ctx.add(text)
    }
}

#[cfg(test)]
mod test {
    use crate::text::TextBuilder;
    use crate::{test::test_widget_deletion, widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| TextBuilder::new(WidgetBuilder::new()).build(ctx));
    }
}
