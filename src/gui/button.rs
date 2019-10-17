use crate::gui::{
    builder::{CommonBuilderFields, GenericNodeBuilder},
    node::{UINode, UINodeKind},
    UserInterface,
    HorizontalAlignment,
    VerticalAlignment,
    Thickness,
    text::TextBuilder,
    border::BorderBuilder,
    event::{UIEvent, UIEventKind},
    EventSource,
};

use rg3d_core::{color::Color, pool::Handle};
use std::collections::VecDeque;

/// Button
///
/// # Events
///
/// [`Click`] - spawned when user click button.
pub struct Button {
    events: VecDeque<UIEvent>,
}

pub enum ButtonContent {
    Text(String),
    Node(Handle<UINode>),
}

pub struct ButtonBuilder {
    content: Option<ButtonContent>,
    common: CommonBuilderFields,
}

impl Default for ButtonBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ButtonBuilder {
    pub fn new() -> Self {
        Self {
            content: None,
            common: CommonBuilderFields::new(),
        }
    }

    impl_default_builder_methods!();

    pub fn with_text(mut self, text: &str) -> Self {
        self.content = Some(ButtonContent::Text(text.to_owned()));
        self
    }

    pub fn with_node(mut self, node: Handle<UINode>) -> Self {
        self.content = Some(ButtonContent::Node(node));
        self
    }

    pub fn build(self, ui: &mut UserInterface) -> Handle<UINode> {
        let normal_color = Color::opaque(120, 120, 120);
        let pressed_color = Color::opaque(100, 100, 100);
        let hover_color = Color::opaque(160, 160, 160);

        let button = Button {
            events: VecDeque::new(),
        };

        GenericNodeBuilder::new(
            UINodeKind::Button(button), self.common)
            .with_event_handler(Box::new(move |ui, handle, evt| {
                if evt.source == handle || ui.is_node_child_of(evt.source, handle) {
                    match evt.kind {
                        UIEventKind::MouseUp { .. } => {
                            // Generate Click event
                            let node = ui.get_node_mut(handle);
                            if let UINodeKind::Button(button) = node.get_kind_mut() {
                                button.events.push_back(UIEvent::new(UIEventKind::Click));
                            }
                            ui.release_mouse_capture();
                        }
                        UIEventKind::MouseDown { .. } => {
                            ui.capture_mouse(evt.source);
                        }
                        _ => ()
                    }
                }
            }))
            .with_child(BorderBuilder::new()
                .with_stroke_color(Color::opaque(200, 200, 200))
                .with_stroke_thickness(Thickness { left: 1.0, right: 1.0, top: 1.0, bottom: 1.0 })
                .with_color(normal_color)
                .with_event_handler(Box::new(move |ui, handle, evt| {
                    if evt.source == handle || ui.is_node_child_of(evt.source, handle) {
                        let back = ui.nodes.borrow_mut(handle);
                        match evt.kind {
                            UIEventKind::MouseDown { .. } => back.color = pressed_color,
                            UIEventKind::MouseUp { .. } => {
                                if back.is_mouse_over {
                                    back.color = hover_color;
                                } else {
                                    back.color = normal_color;
                                }
                            }
                            UIEventKind::MouseLeave => back.color = normal_color,
                            UIEventKind::MouseEnter => back.color = hover_color,
                            _ => ()
                        }
                    }
                }))
                .with_child(
                    if let Some(content) = self.content {
                        match content {
                            ButtonContent::Text(txt) => {
                                TextBuilder::new()
                                    .with_text(txt.as_str())
                                    .with_horizontal_text_alignment(HorizontalAlignment::Center)
                                    .with_vertical_text_alignment(VerticalAlignment::Center)
                                    .build(ui)
                            }
                            ButtonContent::Node(node) => node
                        }
                    } else {
                        Handle::NONE
                    })
                .build(ui))
            .build(ui)
    }
}

impl EventSource for Button {
    fn emit_event(&mut self) -> Option<UIEvent> {
        self.events.pop_front()
    }
}