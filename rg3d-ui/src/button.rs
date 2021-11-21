use crate::{
    border::BorderBuilder,
    brush::{Brush, GradientPoint},
    core::{algebra::Vector2, pool::Handle},
    decorator::DecoratorBuilder,
    define_constructor,
    message::{MessageDirection, UiMessage},
    text::TextBuilder,
    ttf::SharedFont,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, HorizontalAlignment, NodeHandleMapping, Thickness, UiNode,
    UserInterface, VerticalAlignment, BRUSH_LIGHT, BRUSH_LIGHTER, BRUSH_LIGHTEST, COLOR_DARKEST,
    COLOR_LIGHTEST,
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq)]
pub enum ButtonMessage {
    Click,
    Content(Handle<UiNode>),
}

impl ButtonMessage {
    define_constructor!(ButtonMessage:Click => fn click(), layout: false);
    define_constructor!(ButtonMessage:Content => fn content(Handle<UiNode>), layout: false);
}

#[derive(Clone)]
pub struct Button {
    widget: Widget,
    decorator: Handle<UiNode>,
    content: Handle<UiNode>,
}

crate::define_widget_deref!(Button);

impl Button {
    pub fn new(widget: Widget, body: Handle<UiNode>, content: Handle<UiNode>) -> Self {
        Self {
            widget,
            decorator: body,
            content,
        }
    }

    pub fn content(&self) -> Handle<UiNode> {
        self.content
    }

    pub fn set_content(&mut self, content: Handle<UiNode>) -> &mut Self {
        self.content = content;
        self
    }
}

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
                        self.content = *content;
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

pub enum ButtonContent {
    Text(String),
    Node(Handle<UiNode>),
}

pub struct ButtonBuilder {
    widget_builder: WidgetBuilder,
    content: Option<ButtonContent>,
    font: Option<SharedFont>,
    back: Option<Handle<UiNode>>,
}

impl ButtonBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            content: None,
            font: None,
            back: None,
        }
    }

    pub fn with_text(mut self, text: &str) -> Self {
        self.content = Some(ButtonContent::Text(text.to_owned()));
        self
    }

    pub fn with_content(mut self, node: Handle<UiNode>) -> Self {
        self.content = Some(ButtonContent::Node(node));
        self
    }

    pub fn with_font(mut self, font: SharedFont) -> Self {
        self.font = Some(font);
        self
    }

    pub fn with_back(mut self, decorator: Handle<UiNode>) -> Self {
        self.back = Some(decorator);
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let content = if let Some(content) = self.content {
            match content {
                ButtonContent::Text(txt) => TextBuilder::new(WidgetBuilder::new())
                    .with_text(txt.as_str())
                    .with_opt_font(self.font)
                    .with_horizontal_text_alignment(HorizontalAlignment::Center)
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ctx),
                ButtonContent::Node(node) => node,
            }
        } else {
            Handle::NONE
        };

        let back = self.back.unwrap_or_else(|| {
            DecoratorBuilder::new(
                BorderBuilder::new(
                    WidgetBuilder::new()
                        .with_foreground(Brush::LinearGradient {
                            from: Vector2::new(0.5, 0.0),
                            to: Vector2::new(0.5, 1.0),
                            stops: vec![
                                GradientPoint {
                                    stop: 0.0,
                                    color: COLOR_LIGHTEST,
                                },
                                GradientPoint {
                                    stop: 0.25,
                                    color: COLOR_LIGHTEST,
                                },
                                GradientPoint {
                                    stop: 1.0,
                                    color: COLOR_DARKEST,
                                },
                            ],
                        })
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
