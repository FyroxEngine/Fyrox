//! Defines a clickable widget with arbitrary content. See [`Button`] dos for more info and examples.

#![warn(missing_docs)]

use crate::{
    border::BorderBuilder,
    core::pool::Handle,
    core::{reflect::prelude::*, visitor::prelude::*},
    decorator::DecoratorBuilder,
    define_constructor,
    font::FontResource,
    message::{MessageDirection, UiMessage},
    text::TextBuilder,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, HorizontalAlignment, NodeHandleMapping, Thickness, UiNode,
    UserInterface, VerticalAlignment, BRUSH_DARKER, BRUSH_LIGHT, BRUSH_LIGHTER, BRUSH_LIGHTEST,
};
use fyrox_core::uuid_provider;
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

/// Messages that can be emitted by [`Button`] widget (or can be sent to the widget).
#[derive(Debug, Clone, PartialEq)]
pub enum ButtonMessage {
    /// Emitted by the button widget when it was clicked by any mouse button. Click is a press with a following release
    /// of a mouse button withing the button bounds. This message can be only emitted, not sent. See [`Button`] docs
    /// for usage examples.
    Click,
    /// A message, that can be used to set new content of the button. See [`ButtonContent`] for usage examples.
    Content(ButtonContent),
}

impl ButtonMessage {
    define_constructor!(
        /// A shortcut method to create [`ButtonMessage::Click`] message.
        ButtonMessage:Click => fn click(), layout: false
    );
    define_constructor!(
        /// A shortcut method to create [`ButtonMessage::Content`] message.
        ButtonMessage:Content => fn content(ButtonContent), layout: false
    );
}

/// Defines a clickable widget with arbitrary content. The content could be any kind of widget, usually it
/// is just a text or an image.
///
/// ## Examples
///
/// To create a simple button with text you should do something like this:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     button::ButtonBuilder, widget::WidgetBuilder, UiNode, UserInterface
/// # };
/// fn create_button(ui: &mut UserInterface) -> Handle<UiNode> {
///     ButtonBuilder::new(WidgetBuilder::new())
///         .with_text("Click me!")
///         .build(&mut ui.build_ctx())
/// }
/// ```
///
/// To do something when your button was clicked you need to "listen" to user interface messages from the
/// queue and check if there's [`ButtonMessage::Click`] message from your button:
///
/// ```rust
/// # use fyrox_ui::{button::ButtonMessage, core::pool::Handle, message::UiMessage};
/// fn on_ui_message(message: &UiMessage) {
/// #   let your_button_handle = Handle::NONE;
///     if let Some(ButtonMessage::Click) = message.data() {
///         if message.destination() == your_button_handle {
///             println!("{} button was clicked!", message.destination());
///         }
///     }
/// }
/// ```
#[derive(Default, Clone, Visit, Reflect, Debug)]
pub struct Button {
    /// Base widget of the button.
    pub widget: Widget,
    /// Current content holder of the button.
    pub decorator: Handle<UiNode>,
    /// Current content of the button. It is attached to the content holder.
    pub content: Handle<UiNode>,
}

crate::define_widget_deref!(Button);

uuid_provider!(Button = "2abcf12b-2f19-46da-b900-ae8890f7c9c6");

impl Control for Button {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        node_map.resolve(&mut self.content);
        node_map.resolve(&mut self.decorator);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<WidgetMessage>() {
            if message.destination() == self.handle()
                || self.has_descendant(message.destination(), ui)
            {
                match msg {
                    WidgetMessage::MouseUp { .. } => {
                        ui.send_message(ButtonMessage::click(
                            self.handle(),
                            MessageDirection::FromWidget,
                        ));
                        ui.release_mouse_capture();
                        message.set_handled(true);
                    }
                    WidgetMessage::MouseDown { .. } => {
                        ui.capture_mouse(message.destination());
                        message.set_handled(true);
                    }
                    WidgetMessage::TouchStarted { .. } => {
                        ui.capture_mouse(message.destination());
                        message.set_handled(true);
                    }
                    WidgetMessage::TouchMoved { .. } => {
                        ui.capture_mouse(message.destination());
                        message.set_handled(true);
                    }
                    WidgetMessage::TouchEnded { .. } => {
                        ui.send_message(ButtonMessage::click(
                            self.handle(),
                            MessageDirection::FromWidget,
                        ));
                        ui.release_mouse_capture();
                        message.set_handled(true);
                    }
                    _ => (),
                }
            }
        } else if let Some(msg) = message.data::<ButtonMessage>() {
            if message.destination() == self.handle() {
                match msg {
                    ButtonMessage::Click => (),
                    ButtonMessage::Content(content) => {
                        if self.content.is_some() {
                            ui.send_message(WidgetMessage::remove(
                                self.content,
                                MessageDirection::ToWidget,
                            ));
                        }
                        self.content = content.build(&mut ui.build_ctx());
                        ui.send_message(WidgetMessage::link(
                            self.content,
                            MessageDirection::ToWidget,
                            self.decorator,
                        ));
                    }
                }
            }
        }
    }
}

/// Possible button content. In general, button widget can contain any type of widget inside. This enum contains
/// a special shortcuts for most commonly used cases - button with the default font, button with custom font, or
/// button with any widget.
#[derive(Debug, Clone, PartialEq)]
pub enum ButtonContent {
    /// A shortcut to create a [crate::text::Text] widget as the button content. It is the same as creating Text
    /// widget yourself, but much shorter.
    Text {
        /// Text of the button.
        text: String,
        /// Optional font of the button. If [`None`], the default font will be used.
        font: Option<FontResource>,
    },
    /// Arbitrary widget handle. It could be any widget handle, for example a handle of [`crate::image::Image`]
    /// widget.
    Node(Handle<UiNode>),
}

impl ButtonContent {
    /// Creates [`ButtonContent::Text`] with default font.
    pub fn text<S: AsRef<str>>(s: S) -> Self {
        Self::Text {
            text: s.as_ref().to_owned(),
            font: None,
        }
    }

    /// Creates [`ButtonContent::Text`] with custom font.
    pub fn text_with_font<S: AsRef<str>>(s: S, font: FontResource) -> Self {
        Self::Text {
            text: s.as_ref().to_owned(),
            font: Some(font),
        }
    }

    /// Creates [`ButtonContent::Node`].
    pub fn node(node: Handle<UiNode>) -> Self {
        Self::Node(node)
    }

    fn build(&self, ctx: &mut BuildContext) -> Handle<UiNode> {
        match self {
            Self::Text { text, font } => TextBuilder::new(WidgetBuilder::new())
                .with_text(text)
                .with_horizontal_text_alignment(HorizontalAlignment::Center)
                .with_vertical_text_alignment(VerticalAlignment::Center)
                .with_font(font.clone().unwrap_or_else(|| ctx.default_font()))
                .build(ctx),
            Self::Node(node) => *node,
        }
    }
}

/// Button builder is used to create [`Button`] widget instances.
pub struct ButtonBuilder {
    widget_builder: WidgetBuilder,
    content: Option<ButtonContent>,
    back: Option<Handle<UiNode>>,
}

impl ButtonBuilder {
    /// Creates a new button builder with a widget builder instance.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            content: None,
            back: None,
        }
    }

    /// Sets the content of the button to be [`ButtonContent::Text`] (text with the default font).
    pub fn with_text(mut self, text: &str) -> Self {
        self.content = Some(ButtonContent::text(text));
        self
    }

    /// Sets the content of the button to be [`ButtonContent::Text`] (text with a custom font).
    pub fn with_text_and_font(mut self, text: &str, font: FontResource) -> Self {
        self.content = Some(ButtonContent::text_with_font(text, font));
        self
    }

    /// Sets the content of the button to be [`ButtonContent::Node`] (arbitrary widget handle).
    pub fn with_content(mut self, node: Handle<UiNode>) -> Self {
        self.content = Some(ButtonContent::Node(node));
        self
    }

    /// Specifies the widget that will be used as a content holder of the button. By default it is an
    /// instance of [`crate::decorator::Decorator`] widget. Usually, this widget should respond to mouse
    /// events to highlight button state (hovered, pressed, etc.)
    pub fn with_back(mut self, decorator: Handle<UiNode>) -> Self {
        self.back = Some(decorator);
        self
    }

    /// Finishes button build and adds to the user interface and returns its handle.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let content = self.content.map(|c| c.build(ctx)).unwrap_or_default();

        let back = self.back.unwrap_or_else(|| {
            DecoratorBuilder::new(
                BorderBuilder::new(
                    WidgetBuilder::new()
                        .with_foreground(BRUSH_DARKER)
                        .with_child(content),
                )
                .with_stroke_thickness(Thickness::uniform(1.0)),
            )
            .with_normal_brush(BRUSH_LIGHT)
            .with_hover_brush(BRUSH_LIGHTER)
            .with_pressed_brush(BRUSH_LIGHTEST)
            .build(ctx)
        });

        if content.is_some() {
            ctx.link(content, back);
        }

        let button = Button {
            widget: self.widget_builder.with_child(back).build(),
            decorator: back,
            content,
        };
        ctx.add_node(UiNode::new(button))
    }
}
