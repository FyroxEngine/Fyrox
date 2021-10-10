use crate::numeric::{NumericType, NumericUpDownMessage};
use crate::{
    core::{algebra::Vector4, color::Color, pool::Handle},
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage, UiMessageData},
    vec::{make_mark, make_numeric_input},
    BuildContext, Control, NodeHandleMapping, UiNode, UserInterface, Widget, WidgetBuilder,
};
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, PartialEq)]
pub enum Vec4EditorMessage<T: NumericType> {
    Value(Vector4<T>),
}

impl<T: NumericType> Vec4EditorMessage<T> {
    pub fn value(
        destination: Handle<UiNode>,
        direction: MessageDirection,
        value: Vector4<T>,
    ) -> UiMessage {
        UiMessage::user(
            destination,
            direction,
            Box::new(Vec4EditorMessage::Value(value)),
        )
    }
}

#[derive(Clone)]
pub struct Vec4Editor<T: NumericType> {
    widget: Widget,
    x_field: Handle<UiNode>,
    y_field: Handle<UiNode>,
    z_field: Handle<UiNode>,
    w_field: Handle<UiNode>,
    value: Vector4<T>,
}

impl<T: NumericType> Deref for Vec4Editor<T> {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T: NumericType> DerefMut for Vec4Editor<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<T: NumericType> Control for Vec4Editor<T> {
    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        node_map.resolve(&mut self.x_field);
        node_map.resolve(&mut self.y_field);
        node_map.resolve(&mut self.z_field);
        node_map.resolve(&mut self.w_field);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        match message.data() {
            UiMessageData::User(msg) => {
                if let Some(&NumericUpDownMessage::Value(value)) =
                    msg.cast::<NumericUpDownMessage<T>>()
                {
                    if message.direction() == MessageDirection::FromWidget {
                        if message.destination() == self.x_field {
                            ui.send_message(Vec4EditorMessage::value(
                                self.handle(),
                                MessageDirection::ToWidget,
                                Vector4::new(value, self.value.y, self.value.z, self.value.w),
                            ));
                        } else if message.destination() == self.y_field {
                            ui.send_message(Vec4EditorMessage::value(
                                self.handle(),
                                MessageDirection::ToWidget,
                                Vector4::new(self.value.x, value, self.value.z, self.value.w),
                            ));
                        } else if message.destination() == self.z_field {
                            ui.send_message(Vec4EditorMessage::value(
                                self.handle(),
                                MessageDirection::ToWidget,
                                Vector4::new(self.value.x, self.value.y, value, self.value.w),
                            ));
                        } else if message.destination() == self.w_field {
                            ui.send_message(Vec4EditorMessage::value(
                                self.handle(),
                                MessageDirection::ToWidget,
                                Vector4::new(self.value.x, self.value.y, self.value.z, value),
                            ));
                        }
                    }
                } else if let Some(Vec4EditorMessage::Value(value)) =
                    msg.cast::<Vec4EditorMessage<T>>()
                {
                    if message.direction() == MessageDirection::ToWidget {
                        let mut changed = false;
                        if self.value.x != value.x {
                            self.value.x = value.x;
                            ui.send_message(NumericUpDownMessage::value(
                                self.x_field,
                                MessageDirection::ToWidget,
                                value.x,
                            ));
                            changed = true;
                        }
                        if self.value.y != value.y {
                            self.value.y = value.y;
                            ui.send_message(NumericUpDownMessage::value(
                                self.y_field,
                                MessageDirection::ToWidget,
                                value.y,
                            ));
                            changed = true;
                        }
                        if self.value.z != value.z {
                            self.value.z = value.z;
                            ui.send_message(NumericUpDownMessage::value(
                                self.z_field,
                                MessageDirection::ToWidget,
                                value.z,
                            ));
                            changed = true;
                        }
                        if self.value.w != value.w {
                            self.value.w = value.w;
                            ui.send_message(NumericUpDownMessage::value(
                                self.w_field,
                                MessageDirection::ToWidget,
                                value.w,
                            ));
                            changed = true;
                        }
                        if changed {
                            ui.send_message(message.reverse());
                        }
                    }
                }
            }

            _ => (),
        }
    }
}

pub struct Vec4EditorBuilder<T: NumericType> {
    widget_builder: WidgetBuilder,
    value: Vector4<T>,
}

impl<T: NumericType> Vec4EditorBuilder<T> {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: Vector4::new(T::zero(), T::zero(), T::zero(), T::zero()),
        }
    }

    pub fn with_value(mut self, value: Vector4<T>) -> Self {
        self.value = value;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let x_field;
        let y_field;
        let z_field;
        let w_field;
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_mark(ctx, "X", 0, Color::opaque(120, 0, 0)))
                .with_child({
                    x_field = make_numeric_input(ctx, 1, self.value.x);
                    x_field
                })
                .with_child(make_mark(ctx, "Y", 2, Color::opaque(0, 120, 0)))
                .with_child({
                    y_field = make_numeric_input(ctx, 3, self.value.y);
                    y_field
                })
                .with_child(make_mark(ctx, "Z", 4, Color::opaque(0, 0, 120)))
                .with_child({
                    z_field = make_numeric_input(ctx, 5, self.value.z);
                    z_field
                })
                .with_child(make_mark(ctx, "W", 6, Color::opaque(120, 0, 120)))
                .with_child({
                    w_field = make_numeric_input(ctx, 7, self.value.w);
                    w_field
                }),
        )
        .add_row(Row::stretch())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .build(ctx);

        let node = Vec4Editor {
            widget: self.widget_builder.with_child(grid).build(),
            x_field,
            y_field,
            z_field,
            w_field,
            value: self.value,
        };

        ctx.add_node(UiNode::new(node))
    }
}
