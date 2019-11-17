use crate::{
    gui::{
        node::UINode,
        widget::{
            Widget,
            WidgetBuilder,
            AsWidget,
        },
        UserInterface,
        HorizontalAlignment,
        VerticalAlignment,
        Thickness,
        text::TextBuilder,
        border::BorderBuilder,
        event::{UIEvent, UIEventKind},
        Layout,
        Draw,
        draw::DrawingContext,
        Update,
    },
    resource::ttf::Font,
};
use crate::core::{
    color::Color,
    pool::Handle,
    math::vec2::Vec2,
};
use std::{
    rc::Rc,
    cell::RefCell,
};

/// Button
///
/// # Events
///
/// [`Click`] - spawned when user click button.
pub struct Button {
    widget: Widget,
}

impl AsWidget for Button {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }
}

impl Layout for Button {
    fn measure_override(&self, ui: &UserInterface, available_size: Vec2) -> Vec2 {
        self.widget.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vec2) -> Vec2 {
        self.widget.arrange_override(ui, final_size)
    }
}

impl Draw for Button {
    fn draw(&mut self, drawing_context: &mut DrawingContext) {
        self.widget.draw(drawing_context)
    }
}

impl Update for Button {
    fn update(&mut self, dt: f32) {
        self.widget.update(dt)
    }
}

pub enum ButtonContent {
    Text(String),
    Node(Handle<UINode>),
}

pub struct ButtonBuilder {
    widget_builder: WidgetBuilder,
    content: Option<ButtonContent>,
    font: Option<Rc<RefCell<Font>>>,
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

    pub fn with_node(mut self, node: Handle<UINode>) -> Self {
        self.content = Some(ButtonContent::Node(node));
        self
    }

    pub fn with_font(mut self, font: Rc<RefCell<Font>>) -> Self {
        self.font = Some(font);
        self
    }

    pub fn build(self, ui: &mut UserInterface) -> Handle<UINode> {
        let normal_color = Color::opaque(120, 120, 120);
        let pressed_color = Color::opaque(100, 100, 100);
        let hover_color = Color::opaque(160, 160, 160);

        let button = UINode::Button(Button {
            widget: self.widget_builder
                .with_event_handler(Box::new(move |ui, handle, evt| {
                    if evt.source == handle || ui.is_node_child_of(evt.source, handle) {
                        match evt.kind {
                            UIEventKind::MouseUp { .. } => {
                                // Generate Click event
                                ui.get_node_mut(handle).widget_mut().events.borrow_mut().push_back(UIEvent::new(UIEventKind::Click));
                                ui.release_mouse_capture();
                            }
                            UIEventKind::MouseDown { .. } => {
                                ui.capture_mouse(evt.source);
                            }
                            _ => ()
                        }
                    }
                }))
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .with_color(normal_color)
                    .with_event_handler(Box::new(move |ui, handle, evt| {
                        if evt.source == handle || ui.is_node_child_of(evt.source, handle) {
                            let back = ui.nodes.borrow_mut(handle).widget_mut();
                            match evt.kind {
                                UIEventKind::MouseDown { .. } => back.set_color(pressed_color),
                                UIEventKind::MouseUp { .. } => {
                                    if back.is_mouse_over {
                                        back.set_color(hover_color);
                                    } else {
                                        back.set_color(normal_color);
                                    }
                                }
                                UIEventKind::MouseLeave => back.set_color(normal_color),
                                UIEventKind::MouseEnter => back.set_color(hover_color),
                                _ => ()
                            }
                        }
                    }))
                    .with_child(
                        if let Some(content) = self.content {
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
                        }))
                    .with_stroke_color(Color::opaque(200, 200, 200))
                    .with_stroke_thickness(Thickness { left: 1.0, right: 1.0, top: 1.0, bottom: 1.0 })
                    .build(ui))
                .build(),
        });
        ui.add_node(button)
    }
}