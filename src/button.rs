use std::sync::{
    Arc,
    Mutex,
};
use crate::{
    brush::{Brush, GradientPoint},
    core::{
        color::Color,
        pool::Handle,
        math::vec2::Vec2,
    },
    UINode,
    widget::{
        Widget,
        WidgetBuilder,
    },
    UserInterface,
    HorizontalAlignment,
    VerticalAlignment,
    Thickness,
    text::TextBuilder,
    border::BorderBuilder,
    Control,
    ControlTemplate,
    UINodeContainer,
    Builder,
    ttf::Font,
    message::{
        WidgetMessage,
        UiMessage,
        UiMessageData,
        ButtonMessage,
    },
    NodeHandleMapping,
};

pub struct Button<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    body: Handle<UINode<M, C>>,
    content: Handle<UINode<M, C>>,
    hover_brush: Brush,
    pressed_brush: Brush,
}

impl<M, C: 'static + Control<M, C>> Button<M, C> {
    pub fn template() -> ControlTemplate<M, C> {
        let mut template = ControlTemplate::new();
        ButtonBuilder::new(WidgetBuilder::new()).build(&mut template);
        template
    }

    pub fn new(
        widget: Widget<M, C>,
        body: Handle<UINode<M, C>>,
        content: Handle<UINode<M, C>>,
        hover_brush: Brush,
        pressed_brush: Brush,
    ) -> Self {
        Self {
            widget,
            body,
            content,
            hover_brush,
            pressed_brush,
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

impl<M, C: 'static + Control<M, C>> Control<M, C> for Button<M, C> {
    fn widget(&self) -> &Widget<M, C> {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget<M, C> {
        &mut self.widget
    }

    fn raw_copy(&self) -> UINode<M, C> {
        UINode::Button(Self {
            widget: self.widget.raw_copy(),
            body: self.body,
            content: self.content,
            hover_brush: self.hover_brush.clone(),
            pressed_brush: self.pressed_brush.clone(),
        })
    }

    fn resolve(&mut self, _: &ControlTemplate<M, C>, node_map: &NodeHandleMapping<M, C>) {
        self.body = *node_map.get(&self.body).unwrap();
        if let Some(content) = node_map.get(&self.content) {
            self.content = *content;
        }
    }

    fn handle_message(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_message(self_handle, ui, message);

        match &message.data {
            UiMessageData::Widget(msg) => {
                if message.source == self.body || ui.is_node_child_of(message.source, self.body) {
                    let back = ui.nodes.borrow_mut(self.body).widget_mut();
                    match msg {
                        WidgetMessage::MouseDown { .. } => {
                            back.set_background(self.pressed_brush.clone());
                        }
                        WidgetMessage::MouseUp { .. } => {
                            if back.is_mouse_over {
                                back.set_background(self.hover_brush.clone());
                            } else {
                                back.set_background(self.widget.background());
                            }
                        }
                        WidgetMessage::MouseLeave => {
                            back.set_background(self.widget.background());
                        }
                        WidgetMessage::MouseEnter => {
                            back.set_background(self.hover_brush.clone());
                        }
                        _ => ()
                    }
                }

                if message.source == self_handle || self.widget().has_descendant(message.source, ui) {
                    match msg {
                        WidgetMessage::MouseUp { .. } => {
                            // Generate Click event
                            self.widget_mut()
                                .outgoing_messages
                                .borrow_mut()
                                .push_back(UiMessage::new(UiMessageData::Button(ButtonMessage::Click)));
                            ui.release_mouse_capture();
                        }
                        WidgetMessage::MouseDown { .. } => {
                            ui.capture_mouse(message.source);
                        }
                        _ => ()
                    }
                }
            }
            UiMessageData::Button(msg) => {
                if message.target() == self_handle {
                    match msg {
                        ButtonMessage::Click => (),
                        ButtonMessage::Content(content) => {
                            if self.content.is_some() {
                                ui.remove_node(self.content);
                            }
                            self.content = *content;
                            ui.link_nodes(self.content, self.body);
                        }
                        ButtonMessage::BorderThickness(thickness) => {
                            if let UINode::Border(body) = ui.node_mut(self.body) {
                                body.set_stroke_thickness(*thickness);
                            }
                        }
                        ButtonMessage::BorderBrush(brush) => {
                            ui.node_mut(self.body)
                                .widget_mut()
                                .set_foreground(brush.clone());
                        }
                    }
                }
            }
            _ => ()
        }
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        if self.content == handle {
            self.content = Handle::NONE;
        }
    }
}

pub enum ButtonContent<M: 'static, C: 'static + Control<M, C>> {
    Text(String),
    Node(Handle<UINode<M, C>>),
}

pub struct ButtonBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    content: Option<ButtonContent<M, C>>,
    font: Option<Arc<Mutex<Font>>>,
    hover_brush: Option<Brush>,
    pressed_brush: Option<Brush>,
    body: Option<Handle<UINode<M, C>>>,
}

impl<M, C: 'static + Control<M, C>> ButtonBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            content: None,
            font: None,
            pressed_brush: None,
            hover_brush: None,
            body: Default::default(),
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

    pub fn with_font(mut self, font: Arc<Mutex<Font>>) -> Self {
        self.font = Some(font);
        self
    }

    pub fn with_body(mut self, body: Handle<UINode<M, C>>) -> Self {
        self.body = Some(body);
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

    pub fn build(self, ui: &mut dyn UINodeContainer<M, C>) -> Handle<UINode<M, C>> {
        let content = if let Some(content) = self.content {
            match content {
                ButtonContent::Text(txt) => {
                    TextBuilder::new(WidgetBuilder::new())
                        .with_text(txt.as_str())
                        .with_opt_font(self.font)
                        .with_horizontal_text_alignment(HorizontalAlignment::Center)
                        .with_vertical_text_alignment(VerticalAlignment::Center)
                        .build(ui)
                }
                ButtonContent::Node(node) => node
            }
        } else {
            Handle::NONE
        };

        let body = self.body.unwrap_or_else(|| {
            let brush = Brush::LinearGradient {
                from: Vec2::new(0.5, 0.0),
                to: Vec2::new(0.5, 1.0),
                stops: vec![
                    GradientPoint { stop: 0.0, color: Color::opaque(85, 85, 85) },
                    GradientPoint { stop: 0.46, color: Color::opaque(85, 85, 85) },
                    GradientPoint { stop: 0.5, color: Color::opaque(65, 65, 65) },
                    GradientPoint { stop: 0.54, color: Color::opaque(75, 75, 75) },
                    GradientPoint { stop: 1.0, color: Color::opaque(75, 75, 75) },
                ],
            };

            BorderBuilder::new(WidgetBuilder::new()
                .with_background(brush)
                .with_foreground(Brush::Solid(Color::opaque(65, 65, 65))))
                .with_stroke_thickness(Thickness { left: 1.0, right: 1.0, top: 1.0, bottom: 1.0 })
                .build(ui)
        });

        if content.is_some() {
            ui.link_nodes(content, body);
        }

        let button = Button {
            widget: self.widget_builder
                .with_background(ui.node(body).widget().background())
                .with_child(body)
                .build(),
            body,
            content,
            hover_brush: self.hover_brush.unwrap_or_else(|| {
                Brush::LinearGradient {
                    from: Vec2::new(0.5, 0.0),
                    to: Vec2::new(0.5, 1.0),
                    stops: vec![
                        GradientPoint { stop: 0.0, color: Color::opaque(105, 95, 85) },
                        GradientPoint { stop: 0.46, color: Color::opaque(105, 95, 85) },
                        GradientPoint { stop: 0.5, color: Color::opaque(85, 75, 65) },
                        GradientPoint { stop: 0.54, color: Color::opaque(95, 85, 75) },
                        GradientPoint { stop: 1.0, color: Color::opaque(95, 85, 75) },
                    ],
                }
            }),
            pressed_brush: self.pressed_brush.unwrap_or_else(|| {
                Brush::LinearGradient {
                    from: Vec2::new(0.5, 0.0),
                    to: Vec2::new(0.5, 1.0),
                    stops: vec![
                        GradientPoint { stop: 0.0, color: Color::opaque(65, 65, 65) },
                        GradientPoint { stop: 0.46, color: Color::opaque(65, 65, 65) },
                        GradientPoint { stop: 0.5, color: Color::opaque(45, 45, 45) },
                        GradientPoint { stop: 0.54, color: Color::opaque(55, 55, 55) },
                        GradientPoint { stop: 1.0, color: Color::opaque(55, 55, 55) },
                    ],
                }
            }),
        };

        ui.add_node(UINode::Button(button))
    }
}