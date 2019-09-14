use crate::{
    gui::{
        builder::{CommonBuilderFields, GenericNodeBuilder},
        node::{UINode, UINodeKind},
        UserInterface,
        HorizontalAlignment,
        VerticalAlignment,
        Thickness,
        event::{RoutedEventHandlerType, RoutedEventHandler},
        text::TextBuilder,
        border::BorderBuilder,
    },
};

use rg3d_core::{
    color::Color,
    pool::Handle,
    math::vec2::Vec2
};

pub type ButtonClickEventHandler = dyn FnMut(&mut UserInterface, Handle<UINode>);

pub struct Button {
    pub(in crate::gui) owner_handle: Handle<UINode>,
    click: Option<Box<ButtonClickEventHandler>>,
}

impl Button {
    pub fn set_on_click(&mut self, handler: Box<ButtonClickEventHandler>) {
        self.click = Some(handler);
    }
}

pub enum ButtonContent {
    Text(String),
    Node(Handle<UINode>),
}

pub struct ButtonBuilder {
    content: Option<ButtonContent>,
    click: Option<Box<ButtonClickEventHandler>>,
    common: CommonBuilderFields,
}

impl ButtonBuilder {
    pub fn new() -> Self {
        Self {
            content: None,
            click: None,
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

    pub fn with_click(mut self, handler: Box<ButtonClickEventHandler>) -> Self {
        self.click = Some(handler);
        self
    }

    pub fn build(self, ui: &mut UserInterface) -> Handle<UINode> {
        let normal_color = Color::opaque(120, 120, 120);
        let pressed_color = Color::opaque(100, 100, 100);
        let hover_color = Color::opaque(160, 160, 160);

        let mut button = Button {
            owner_handle: Handle::none(),
            click: None,
        };
        button.click = self.click;

        GenericNodeBuilder::new(
            UINodeKind::Button(button), self.common)
            .with_handler(RoutedEventHandlerType::MouseDown, Box::new(move |ui, handle, _evt| {
                ui.capture_mouse(handle);
            }))
            .with_handler(RoutedEventHandlerType::MouseUp, Box::new(move |ui, handle, evt| {
                // Take-Call-PutBack trick to bypass borrow checker
                let mut click_handler = None;

                if let Some(button_node) = ui.nodes.borrow_mut(handle) {
                    if let UINodeKind::Button(button) = button_node.get_kind_mut() {
                        click_handler = button.click.take();
                    }
                }

                if let Some(ref mut handler) = click_handler {
                    handler(ui, handle);
                    evt.handled = true;
                }

                // Second check required because event handler can remove node.
                if let Some(button_node) = ui.nodes.borrow_mut(handle) {
                    if let UINodeKind::Button(button) = button_node.get_kind_mut() {
                        button.click = click_handler;
                    }
                }

                ui.release_mouse_capture();
            }))
            .with_child(BorderBuilder::new()
                .with_stroke_color(Color::opaque(200, 200, 200))
                .with_stroke_thickness(Thickness { left: 1.0, right: 1.0, top: 1.0, bottom: 1.0 })
                .with_color(normal_color)
                .with_handler(RoutedEventHandlerType::MouseEnter, Box::new(move |ui, handle, _evt| {
                    if let Some(back) = ui.nodes.borrow_mut(handle) {
                        back.color = hover_color;
                    }
                }))
                .with_handler(RoutedEventHandlerType::MouseLeave, Box::new(move |ui, handle, _evt| {
                    if let Some(back) = ui.nodes.borrow_mut(handle) {
                        back.color = normal_color;
                    }
                }))
                .with_handler(RoutedEventHandlerType::MouseDown, Box::new(move |ui, handle, _evt| {
                    if let Some(back) = ui.nodes.borrow_mut(handle) {
                        back.color = pressed_color;
                    }
                }))
                .with_handler(RoutedEventHandlerType::MouseUp, Box::new(move |ui, handle, _evt| {
                    if let Some(back) = ui.nodes.borrow_mut(handle) {
                        if back.is_mouse_over {
                            back.color = hover_color;
                        } else {
                            back.color = normal_color;
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
                        Handle::none()
                    })
                .build(ui))
            .build(ui)
    }
}