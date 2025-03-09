// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::{
    border::BorderBuilder,
    core::{pool::Handle, reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*},
    decorator::{DecoratorBuilder, DecoratorMessage},
    define_constructor,
    message::{MessageDirection, UiMessage},
    style::{resource::StyleResourceExt, Style},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, Thickness, UiNode, UserInterface,
};

use fyrox_graph::constructor::{ConstructorProvider, GraphNodeConstructor};
use std::ops::{Deref, DerefMut};

#[derive(Default, Clone, Visit, Reflect, Debug, TypeUuidProvider, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
#[type_uuid(id = "8d8f114d-7fc6-4d7e-8f57-cd4e39958c36")]
pub struct ToggleButton {
    pub widget: Widget,
    pub decorator: Handle<UiNode>,
    pub is_toggled: bool,
    pub content: Handle<UiNode>,
}

/// Messages that can be emitted by [`ToggleButton`] widget (or can be sent to the widget).
#[derive(Debug, Clone, PartialEq)]
pub enum ToggleButtonMessage {
    Toggled(bool),
    Content(Handle<UiNode>),
}

impl ToggleButtonMessage {
    define_constructor!(
        ToggleButtonMessage:Toggled => fn toggled(bool), layout: false
    );
    define_constructor!(
        ToggleButtonMessage:Content => fn content(Handle<UiNode>), layout: false
    );
}

impl ToggleButton {
    /// A name of style property, that defines corner radius of a toggle button.
    pub const CORNER_RADIUS: &'static str = "ToggleButton.CornerRadius";
    /// A name of style property, that defines border thickness of a toggle button.
    pub const BORDER_THICKNESS: &'static str = "ToggleButton.BorderThickness";

    /// Returns a style of the widget. This style contains only widget-specific properties.
    pub fn style() -> Style {
        Style::default()
            .with(Self::CORNER_RADIUS, 4.0f32)
            .with(Self::BORDER_THICKNESS, Thickness::uniform(1.0))
    }
}

impl ConstructorProvider<UiNode, UserInterface> for ToggleButton {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("ToggleButton", |ui| {
                ToggleButtonBuilder::new(WidgetBuilder::new().with_name("ToggleButton"))
                    .build(&mut ui.build_ctx())
                    .into()
            })
            .with_group("Input")
    }
}

crate::define_widget_deref!(ToggleButton);

impl Control for ToggleButton {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<WidgetMessage>() {
            if (message.destination() == self.handle()
                || self.has_descendant(message.destination(), ui))
                && message.direction() == MessageDirection::FromWidget
            {
                match msg {
                    WidgetMessage::MouseDown { .. } => {
                        ui.capture_mouse(self.handle());
                    }
                    WidgetMessage::MouseUp { .. } => {
                        if ui.captured_node() == self.handle() {
                            let new_state = !self.is_toggled;

                            ui.send_message(ToggleButtonMessage::toggled(
                                self.handle(),
                                MessageDirection::ToWidget,
                                new_state,
                            ));

                            ui.release_mouse_capture();
                        }
                    }
                    _ => {}
                }
            }
        } else if let Some(msg) = message.data::<ToggleButtonMessage>() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    ToggleButtonMessage::Toggled(value) => {
                        if self.is_toggled != *value {
                            self.is_toggled = *value;

                            ui.send_message(DecoratorMessage::select(
                                self.decorator,
                                MessageDirection::ToWidget,
                                self.is_toggled,
                            ));

                            ui.send_message(message.reverse());
                        }
                    }
                    ToggleButtonMessage::Content(content) => {
                        ui.send_message(WidgetMessage::remove(
                            self.content,
                            MessageDirection::ToWidget,
                        ));
                        ui.send_message(WidgetMessage::link(
                            *content,
                            MessageDirection::ToWidget,
                            self.decorator,
                        ));
                    }
                }
            }
        }
    }
}

pub struct ToggleButtonBuilder {
    widget_builder: WidgetBuilder,
    is_toggled: bool,
    content: Handle<UiNode>,
}

impl ToggleButtonBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            is_toggled: false,
            content: Default::default(),
        }
    }

    pub fn with_toggled(mut self, is_toggled: bool) -> Self {
        self.is_toggled = is_toggled;
        self
    }

    pub fn with_content(mut self, content: Handle<UiNode>) -> Self {
        self.content = content;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let decorator = DecoratorBuilder::new(
            BorderBuilder::new(WidgetBuilder::new().with_child(self.content))
                .with_corner_radius(ctx.style.property(ToggleButton::CORNER_RADIUS))
                .with_stroke_thickness(ctx.style.property(ToggleButton::BORDER_THICKNESS))
                .with_pad_by_corner_radius(true),
        )
        .with_pressable(true)
        .with_selected_brush(ctx.style.property(Style::BRUSH_BRIGHT_BLUE))
        .with_selected(self.is_toggled)
        .build(ctx);

        let canvas = ToggleButton {
            widget: self.widget_builder.with_child(decorator).build(ctx),
            decorator,
            is_toggled: self.is_toggled,
            content: self.content,
        };
        ctx.add_node(UiNode::new(canvas))
    }
}

#[cfg(test)]
mod test {
    use crate::{test::test_widget_deletion, toggle::ToggleButtonBuilder, widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| ToggleButtonBuilder::new(WidgetBuilder::new()).build(ctx));
    }
}
