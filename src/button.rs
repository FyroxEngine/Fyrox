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

/// Button
///
/// # Events
///
/// [`Click`] - spawned when user click button.
pub struct Button {
    widget: Widget,
    body: Handle<UINode>,
    content: Handle<UINode>,
}

impl Button {
    pub fn new(widget: Widget, body: Handle<UINode>, content: Handle<UINode>) -> Self {
        Self {
            widget,
            body,
            content,
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
        })
    }

    fn resolve(&mut self, _: &ControlTemplate, node_map: &HashMap<Handle<UINode>, Handle<UINode>>) {
        self.body = *node_map.get(&self.body).unwrap();
        self.content = *node_map.get(&self.content).unwrap();
    }

    fn handle_event(&mut self, self_handle: Handle<UINode>, ui: &mut UserInterface, evt: &mut UIEvent) {
        let normal_color = Color::opaque(120, 120, 120);
        let pressed_color = Color::opaque(100, 100, 100);
        let hover_color = Color::opaque(160, 160, 160);

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
                    back.set_background(pressed_color);
                }
                UIEventKind::MouseUp { .. } => {
                    if back.is_mouse_over {
                        back.set_background(hover_color);
                    } else {
                        back.set_background(normal_color);
                    }
                }
                UIEventKind::MouseLeave => {
                    back.set_background(normal_color);
                }
                UIEventKind::MouseEnter => {
                    back.set_background(hover_color);
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
}

impl ButtonBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            content: None,
            font: None,
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

    pub fn build(self, ui: &mut dyn UINodeContainer) -> Handle<UINode> {
        let normal_color = Color::opaque(120, 120, 120);

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

        let body = BorderBuilder::new(WidgetBuilder::new()
            .with_background(normal_color)
            .with_foreground(Color::opaque(200, 200, 200))
            .with_child(content))
            .with_stroke_thickness(Thickness { left: 1.0, right: 1.0, top: 1.0, bottom: 1.0 })
            .build(ui);

        let button = Button {
            widget: self.widget_builder
                .with_child(body)
                .build(),
            body,
            content,
        };
        ui.add_node(Box::new(button))
    }
}