use crate::brush::GradientPoint;
use crate::core::algebra::Vector2;
use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::pool::Handle,
    decorator::DecoratorBuilder,
    message::{
        ButtonMessage, MessageData, MessageDirection, UiMessage, UiMessageData, WidgetMessage,
    },
    text::TextBuilder,
    ttf::SharedFont,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, HorizontalAlignment, NodeHandleMapping, Thickness, UINode,
    UserInterface, VerticalAlignment, BRUSH_LIGHT, BRUSH_LIGHTER, BRUSH_LIGHTEST, COLOR_DARKEST,
    COLOR_LIGHTEST,
};
use std::ops::{Deref, DerefMut};

#[derive(Clone)]
pub struct Button<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    decorator: Handle<UINode<M, C>>,
    content: Handle<UINode<M, C>>,
}

crate::define_widget_deref!(Button<M, C>);

impl<M: MessageData, C: Control<M, C>> Button<M, C> {
    pub fn new(
        widget: Widget<M, C>,
        body: Handle<UINode<M, C>>,
        content: Handle<UINode<M, C>>,
    ) -> Self {
        Self {
            widget,
            decorator: body,
            content,
        }
    }

    pub fn content(&self) -> Handle<UINode<M, C>> {
        self.content
    }

    pub fn set_content(&mut self, content: Handle<UINode<M, C>>) -> &mut Self {
        self.content = content;
        self
    }
}

impl<M: MessageData, C: Control<M, C>> Control<M, C> for Button<M, C> {
    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        node_map.resolve(&mut self.content);
        node_map.resolve(&mut self.decorator);
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        match &message.data() {
            UiMessageData::Widget(msg) => {
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
            }
            UiMessageData::Button(msg) => {
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
            _ => (),
        }
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        if self.content == handle {
            self.content = Handle::NONE;
        }
    }
}

pub enum ButtonContent<M: MessageData, C: Control<M, C>> {
    Text(String),
    Node(Handle<UINode<M, C>>),
}

pub struct ButtonBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    content: Option<ButtonContent<M, C>>,
    font: Option<SharedFont>,
    back: Option<Handle<UINode<M, C>>>,
}

impl<M: MessageData, C: Control<M, C>> ButtonBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
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

    pub fn with_content(mut self, node: Handle<UINode<M, C>>) -> Self {
        self.content = Some(ButtonContent::Node(node));
        self
    }

    pub fn with_font(mut self, font: SharedFont) -> Self {
        self.font = Some(font);
        self
    }

    pub fn with_back(mut self, decorator: Handle<UINode<M, C>>) -> Self {
        self.back = Some(decorator);
        self
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
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
        ctx.link(content, back);

        let button = Button {
            widget: self.widget_builder.with_child(back).build(),
            decorator: back,
            content,
        };
        ctx.add_node(UINode::Button(button))
    }
}
