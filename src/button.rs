use crate::message::{MessageData, MessageDirection};
use crate::ttf::SharedFont;
use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::pool::Handle,
    decorator::DecoratorBuilder,
    message::{ButtonMessage, UiMessage, UiMessageData, WidgetMessage},
    text::TextBuilder,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, HorizontalAlignment, NodeHandleMapping, UINode, UserInterface,
    VerticalAlignment,
};
use std::ops::{Deref, DerefMut};

#[derive(Clone)]
pub struct Button<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    decorator: Handle<UINode<M, C>>,
    content: Handle<UINode<M, C>>,
}

impl<M: MessageData, C: Control<M, C>> Deref for Button<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: MessageData, C: Control<M, C>> DerefMut for Button<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

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
    hover_brush: Option<Brush>,
    pressed_brush: Option<Brush>,
    decorator: Option<Handle<UINode<M, C>>>,
}

impl<M: MessageData, C: Control<M, C>> ButtonBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            content: None,
            font: None,
            pressed_brush: None,
            hover_brush: None,
            decorator: None,
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

    pub fn with_decorator(mut self, decorator: Handle<UINode<M, C>>) -> Self {
        self.decorator = Some(decorator);
        self
    }

    pub fn with_hover_brush(mut self, brush: Brush) -> Self {
        self.hover_brush = Some(brush);
        self
    }

    pub fn with_pressed_brush(mut self, brush: Brush) -> Self {
        self.pressed_brush = Some(brush);
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

        let decorator = self.decorator.unwrap_or_else(|| {
            DecoratorBuilder::new(BorderBuilder::new(WidgetBuilder::new().with_child(content)))
                .build(ctx)
        });
        ctx.link(content, decorator);

        let button = Button {
            widget: self.widget_builder.with_child(decorator).build(),
            decorator,
            content,
        };
        ctx.add_node(UINode::Button(button))
    }
}
