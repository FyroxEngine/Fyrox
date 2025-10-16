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
    core::{
        algebra::Vector2, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*,
    },
    define_constructor,
    message::{ButtonState, MessageDirection, MouseButton, UiMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, UiNode, UserInterface,
};

use fyrox_graph::constructor::{ConstructorProvider, GraphNodeConstructor};
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, PartialEq)]
pub enum ThumbMessage {
    DragStarted { position: Vector2<f32> },
    DragDelta { offset: Vector2<f32> },
    DragCompleted { position: Vector2<f32> },
}

impl ThumbMessage {
    define_constructor!(ThumbMessage:DragStarted => fn drag_started(position: Vector2<f32>), layout: false);
    define_constructor!(ThumbMessage:DragDelta => fn drag_delta(offset: Vector2<f32>), layout: false);
    define_constructor!(ThumbMessage:DragCompleted => fn drag_completed(position: Vector2<f32>), layout: false);
}

/// A helper widget that is used to provide basic dragging interaction. The widget itself does not
/// move when being dragged, instead it captures mouse input when clicked and calculates the
/// difference between its current position and its initial position. The user is responsible for
/// the actual movement of the thumb widget.
///
/// Typical use case of a thumb widget is a draggable widget such as indicator in a scroll bar,
/// splitter in a docking manager's tile, etc.
///
/// ```rust
/// use fyrox_ui::{
///     border::BorderBuilder,
///     brush::Brush,
///     core::{color::Color, pool::Handle},
///     decorator::DecoratorBuilder,
///     message::{CursorIcon, MessageDirection, UiMessage},
///     thumb::{ThumbBuilder, ThumbMessage},
///     widget::WidgetBuilder,
///     BuildContext, UiNode,
/// };
///
/// fn create_thumb(ctx: &mut BuildContext) -> Handle<UiNode> {
///     ThumbBuilder::new(
///         WidgetBuilder::new().with_child(
///             DecoratorBuilder::new(BorderBuilder::new(
///                 WidgetBuilder::new()
///                     .with_width(5.0)
///                     .with_cursor(Some(CursorIcon::Grab))
///                     .with_foreground(Brush::Solid(Color::opaque(0, 150, 0)).into()),
///             ))
///             .with_pressable(false)
///             .with_selected(false)
///             .with_normal_brush(Brush::Solid(Color::opaque(0, 150, 0)).into())
///             .with_hover_brush(Brush::Solid(Color::opaque(0, 255, 0)).into())
///             .build(ctx),
///         ),
///     )
///     .build(ctx)
/// }
///
/// fn process_thumb_messages(my_thumb: Handle<UiNode>, message: &UiMessage) {
///     if message.destination() == my_thumb && message.direction() == MessageDirection::FromWidget
///     {
///         if let Some(msg) = message.data::<ThumbMessage>() {
///             match msg {
///                 ThumbMessage::DragStarted { position } => {
///                     // Sent by a thumb when it just started dragging. The provided position
///                     // expressed in local coordinates of the thumb.
///                 }
///                 ThumbMessage::DragDelta { offset } => {
///                     // Sent by a thumb when it is being dragged. The provided offset
///                     // expressed in local coordinates of the thumb.
///                 }
///                 ThumbMessage::DragCompleted { position } => {
///                     // Sent by a thumb when it just stopped dragging. The provided position
///                     // expressed in local coordinates of the thumb.
///                 }
///             }
///         }
///     }
/// }
/// ```
///
/// This example creates a new thumb widget 5px wide and shows how to use its messages to get
/// information about the actual movement.
#[derive(Default, Clone, Visit, Reflect, Debug, TypeUuidProvider, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
#[type_uuid(id = "71ad2ff4-6e9e-461d-b7c2-867bd4039684")]
pub struct Thumb {
    pub widget: Widget,
    pub click_pos: Vector2<f32>,
}

impl ConstructorProvider<UiNode, UserInterface> for Thumb {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("Thumb", |ui| {
                ThumbBuilder::new(WidgetBuilder::new().with_name("Thumb"))
                    .build(&mut ui.build_ctx())
                    .into()
            })
            .with_group("Input")
    }
}

crate::define_widget_deref!(Thumb);

impl Control for Thumb {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<WidgetMessage>() {
            match msg {
                WidgetMessage::MouseDown { pos, button } => {
                    if !message.handled() && *button == MouseButton::Left {
                        ui.capture_mouse(self.handle);
                        message.set_handled(true);
                        self.click_pos = *pos;
                        ui.send_message(ThumbMessage::drag_started(
                            self.handle,
                            MessageDirection::FromWidget,
                            self.actual_local_position(),
                        ));
                    }
                }
                WidgetMessage::MouseUp { button, .. } => {
                    if ui.captured_node() == self.handle && *button == MouseButton::Left {
                        ui.send_message(ThumbMessage::drag_completed(
                            self.handle,
                            MessageDirection::FromWidget,
                            self.actual_local_position(),
                        ));

                        ui.release_mouse_capture();
                    }
                }
                WidgetMessage::MouseMove { pos, state } => {
                    if ui.captured_node() == self.handle && state.left == ButtonState::Pressed {
                        ui.send_message(ThumbMessage::drag_delta(
                            self.handle,
                            MessageDirection::FromWidget,
                            self.visual_transform()
                                .try_inverse()
                                .unwrap_or_default()
                                .transform_vector(&(*pos - self.click_pos)),
                        ));
                    }
                }
                _ => (),
            }
        }
    }
}

pub struct ThumbBuilder {
    widget_builder: WidgetBuilder,
}

impl ThumbBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let thumb = Thumb {
            widget: self.widget_builder.build(ctx),
            click_pos: Default::default(),
        };
        ctx.add_node(UiNode::new(thumb))
    }
}

#[cfg(test)]
mod test {
    use crate::thumb::ThumbBuilder;
    use crate::{test::test_widget_deletion, widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| ThumbBuilder::new(WidgetBuilder::new()).build(ctx));
    }
}
