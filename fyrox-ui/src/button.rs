use crate::{
    border::BorderBuilder,
    core::pool::Handle,
    decorator::DecoratorBuilder,
    define_constructor,
    message::{MessageDirection, UiMessage},
    text::TextBuilder,
    ttf::SharedFont,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, HorizontalAlignment, NodeHandleMapping, Thickness, UiNode,
    UserInterface, VerticalAlignment, BRUSH_DARKER, BRUSH_LIGHT, BRUSH_LIGHTER, BRUSH_LIGHTEST,
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq)]
pub enum ButtonMessage {
    Click,
    Content(ButtonContent),
}

impl ButtonMessage {
    define_constructor!(ButtonMessage:Click => fn click(), layout: false);
    define_constructor!(ButtonMessage:Content => fn content(ButtonContent), layout: false);
}

#[derive(Clone)]
pub struct Button {
    pub widget: Widget,
    pub decorator: Handle<UiNode>,
    pub content: Handle<UiNode>,
}

crate::define_widget_deref!(Button);

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

#[derive(Debug, Clone, PartialEq)]
pub enum ButtonContent {
    Text {
        text: String,
        font: Option<SharedFont>,
    },
    Node(Handle<UiNode>),
}

impl ButtonContent {
    pub fn text<S: AsRef<str>>(s: S) -> Self {
        Self::Text {
            text: s.as_ref().to_owned(),
            font: None,
        }
    }

    pub fn text_with_font<S: AsRef<str>>(s: S, font: SharedFont) -> Self {
        Self::Text {
            text: s.as_ref().to_owned(),
            font: Some(font),
        }
    }

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

pub struct ButtonBuilder {
    widget_builder: WidgetBuilder,
    content: Option<ButtonContent>,
    back: Option<Handle<UiNode>>,
}

impl ButtonBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            content: None,
            back: None,
        }
    }

    pub fn with_text(mut self, text: &str) -> Self {
        self.content = Some(ButtonContent::text(text));
        self
    }

    pub fn with_text_and_font(mut self, text: &str, font: SharedFont) -> Self {
        self.content = Some(ButtonContent::text_with_font(text, font));
        self
    }

    pub fn with_content(mut self, node: Handle<UiNode>) -> Self {
        self.content = Some(ButtonContent::Node(node));
        self
    }

    pub fn with_back(mut self, decorator: Handle<UiNode>) -> Self {
        self.back = Some(decorator);
        self
    }

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
