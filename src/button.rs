use crate::{
    core::{
        color::Color,
        pool::Handle,
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
    event::{
        UIEvent,
        UIEventKind,
    },
    Control,
    ControlTemplate,
    UINodeContainer,
    Builder,
    ttf::Font,
};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        Mutex,
    },
};
use crate::brush::{Brush, GradientPoint};
use rg3d_core::math::vec2::Vec2;

pub struct Button {
    widget: Widget,
    body: Handle<UINode>,
    content: Handle<UINode>,
    hover_brush: Brush,
    pressed_brush: Brush,
}

impl Button {
    pub fn template() -> ControlTemplate {
        let mut template = ControlTemplate::new();
        ButtonBuilder::new(WidgetBuilder::new()).build(&mut template);
        template
    }

    pub fn new(
        widget: Widget,
        body: Handle<UINode>,
        content: Handle<UINode>,
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

    pub fn content(&self) -> Handle<UINode> {
        self.content
    }
}

impl Control for Button {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }

    fn raw_copy(&self) -> Box<dyn Control> {
        Box::new(Self {
            widget: *self.widget.raw_copy().downcast::<Widget>().unwrap_or_else(|_| panic!()),
            body: self.body,
            content: self.content,
            hover_brush: self.hover_brush.clone(),
            pressed_brush: self.pressed_brush.clone(),
        })
    }

    fn resolve(&mut self, _: &ControlTemplate, node_map: &HashMap<Handle<UINode>, Handle<UINode>>) {
        self.body = *node_map.get(&self.body).unwrap();
        self.content = *node_map.get(&self.content).unwrap();
    }

    fn handle_event(&mut self, self_handle: Handle<UINode>, ui: &mut UserInterface, evt: &mut UIEvent) {
        if evt.source == self_handle || self.widget().has_descendant(evt.source, ui) {
            match evt.kind {
                UIEventKind::MouseUp { .. } => {
                    // Generate Click event
                    self.widget_mut()
                        .events
                        .borrow_mut()
                        .push_back(UIEvent::new(UIEventKind::Click));
                    ui.release_mouse_capture();
                }
                UIEventKind::MouseDown { .. } => {
                    ui.capture_mouse(evt.source);
                }
                _ => ()
            }
        }

        if evt.source == self.body || ui.is_node_child_of(evt.source, self.body) {
            let back = ui.nodes.borrow_mut(self.body).widget_mut();
            match evt.kind {
                UIEventKind::MouseDown { .. } => {
                    back.set_background(self.pressed_brush.clone());
                }
                UIEventKind::MouseUp { .. } => {
                    if back.is_mouse_over {
                        back.set_background(self.hover_brush.clone());
                    } else {
                        back.set_background(self.widget.background());
                    }
                }
                UIEventKind::MouseLeave => {
                    back.set_background(self.widget.background());
                }
                UIEventKind::MouseEnter => {
                    back.set_background(self.hover_brush.clone());
                }
                _ => ()
            }
        }
    }
}

pub enum ButtonContent {
    Text(String),
    Node(Handle<UINode>),
}

pub struct ButtonBuilder {
    widget_builder: WidgetBuilder,
    content: Option<ButtonContent>,
    font: Option<Arc<Mutex<Font>>>,
    hover_brush: Option<Brush>,
    pressed_brush: Option<Brush>,
    body: Option<Handle<UINode>>,
}

impl ButtonBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            content: None,
            font: None,
            pressed_brush: None,
            hover_brush: None,
            body: Default::default()
        }
    }

    pub fn with_text(mut self, text: &str) -> Self {
        self.content = Some(ButtonContent::Text(text.to_owned()));
        self
    }

    pub fn with_content(mut self, node: Handle<UINode>) -> Self {
        self.content = Some(ButtonContent::Node(node));
        self
    }

    pub fn with_font(mut self, font: Arc<Mutex<Font>>) -> Self {
        self.font = Some(font);
        self
    }

    pub fn with_body(mut self, body: Handle<UINode>) -> Self {
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

    pub fn build(self, ui: &mut dyn UINodeContainer) -> Handle<UINode> {
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
                .with_background(brush.clone())
                .with_foreground(Brush::Solid(Color::opaque(65, 65, 65))))
                .with_stroke_thickness(Thickness { left: 1.0, right: 1.0, top: 1.0, bottom: 1.0 })
                .build(ui)
        });

        ui.link_nodes(content, body);

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

        ui.add_node(Box::new(button))
    }
}