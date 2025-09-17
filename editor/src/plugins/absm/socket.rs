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

use crate::fyrox::core::pool::ErasedHandle;
use crate::fyrox::{
    core::{
        algebra::Vector2, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        uuid_provider, visitor::prelude::*,
    },
    gui::{
        define_constructor, define_widget_deref,
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, MouseButton, UiMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        vector_image::{Primitive, VectorImageBuilder},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, Orientation, Thickness, UiNode, UserInterface, VerticalAlignment,
    },
};
use fyrox::core::pool::NodeVariant;

use fyrox::gui::style::resource::StyleResourceExt;
use fyrox::gui::style::Style;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocketMessage {
    // Occurs when user clicks on socket and starts dragging it.
    StartDragging,
}

impl SocketMessage {
    define_constructor!(SocketMessage:StartDragging => fn start_dragging(), layout: false);
}

#[derive(Copy, Clone, PartialEq, Hash, Debug, Eq, Visit, Reflect, Default)]
pub enum SocketDirection {
    #[default]
    Input,
    Output,
}

#[derive(Clone, Debug, Visit, Reflect, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct Socket {
    widget: Widget,
    click_position: Option<Vector2<f32>>,
    pub parent_node: ErasedHandle,
    pub direction: SocketDirection,
    #[allow(dead_code)] // TODO
    editor: Handle<UiNode>,
    pin: Handle<UiNode>,
    pub index: usize,
}

impl NodeVariant<UiNode> for Socket {}

define_widget_deref!(Socket);

const RADIUS: f32 = 8.0;

uuid_provider!(Socket = "a6c0473e-7073-4e91-a681-cf88795af52a");

impl Control for Socket {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<WidgetMessage>() {
            match msg {
                WidgetMessage::MouseDown { button, pos } => {
                    if *button == MouseButton::Left && message.destination() == self.pin {
                        self.click_position = Some(*pos);

                        ui.capture_mouse(self.handle());

                        message.set_handled(true);
                    }
                }
                WidgetMessage::MouseUp { button, .. } => {
                    if *button == MouseButton::Left {
                        self.click_position = None;

                        ui.release_mouse_capture();

                        message.set_handled(true);
                    }
                }
                WidgetMessage::MouseMove { pos, .. } => {
                    if let Some(click_position) = self.click_position {
                        if click_position.metric_distance(pos) >= 5.0 {
                            ui.send_message(SocketMessage::start_dragging(
                                self.handle(),
                                MessageDirection::FromWidget,
                            ));

                            self.click_position = None;
                        }
                    }
                }
                WidgetMessage::MouseLeave => {
                    ui.send_message(WidgetMessage::foreground(
                        self.pin,
                        MessageDirection::ToWidget,
                        ui.style.property(Style::BRUSH_BRIGHT),
                    ));
                }
                WidgetMessage::MouseEnter => {
                    ui.send_message(WidgetMessage::foreground(
                        self.pin,
                        MessageDirection::ToWidget,
                        ui.style.property(Style::BRUSH_BRIGHTEST),
                    ));
                }
                _ => (),
            }
        }
    }
}

pub struct SocketBuilder {
    widget_builder: WidgetBuilder,
    parent_node: ErasedHandle,
    direction: SocketDirection,
    editor: Handle<UiNode>,
    index: usize,
    show_index: bool,
}

impl SocketBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            parent_node: Default::default(),
            direction: SocketDirection::Input,
            editor: Default::default(),
            index: 0,
            show_index: true,
        }
    }

    pub fn with_parent_node(mut self, parent_node: ErasedHandle) -> Self {
        self.parent_node = parent_node;
        self
    }

    pub fn with_direction(mut self, direction: SocketDirection) -> Self {
        self.direction = direction;
        self
    }

    #[allow(dead_code)] // TODO
    pub fn with_editor(mut self, editor: Handle<UiNode>) -> Self {
        self.editor = editor;
        self
    }

    pub fn with_index(mut self, index: usize) -> Self {
        self.index = index;
        self
    }

    pub fn with_show_index(mut self, show_index: bool) -> Self {
        self.show_index = show_index;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        if let Ok(editor) = ctx.try_get_node_mut(self.editor) {
            editor.set_row(0).set_column(1);
        }

        let pin;
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_child({
                                pin = VectorImageBuilder::new(
                                    WidgetBuilder::new()
                                        .on_row(0)
                                        .on_column(0)
                                        .with_foreground(ctx.style.property(Style::BRUSH_BRIGHT)),
                                )
                                .with_primitives(vec![Primitive::Circle {
                                    center: Vector2::new(RADIUS, RADIUS),
                                    radius: RADIUS,
                                    segments: 16,
                                }])
                                .build(ctx);
                                pin
                            })
                            .with_child(if self.show_index {
                                TextBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::left(2.0)),
                                )
                                .with_vertical_text_alignment(VerticalAlignment::Center)
                                .with_text(format!("{:?}", self.index))
                                .build(ctx)
                            } else {
                                Handle::NONE
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                )
                .with_child(self.editor),
        )
        .add_row(Row::auto())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .build(ctx);

        let socket = Socket {
            widget: self.widget_builder.with_child(grid).build(ctx),
            click_position: Default::default(),
            parent_node: self.parent_node,
            direction: self.direction,
            editor: self.editor,
            pin,
            index: self.index,
        };

        ctx.add_node(UiNode::new(socket))
    }
}

#[cfg(test)]
mod test {
    use crate::plugins::absm::socket::SocketBuilder;
    use fyrox::{gui::test::test_widget_deletion, gui::widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| SocketBuilder::new(WidgetBuilder::new()).build(ctx));
    }
}
