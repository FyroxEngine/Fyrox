use crate::numeric::{NumericType, NumericUpDownMessage};
use crate::{
    core::{algebra::Vector3, color::Color, pool::Handle},
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage, UiMessageData},
    vec::{make_mark, make_numeric_input},
    BuildContext, Control, NodeHandleMapping, UiNode, UserInterface, Widget, WidgetBuilder,
};
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, PartialEq)]
pub enum Vec3EditorMessage<T: NumericType> {
    Value(Vector3<T>),
}

impl<T: NumericType> Vec3EditorMessage<T> {
    pub fn value(
        destination: Handle<UiNode>,
        direction: MessageDirection,
        value: Vector3<T>,
    ) -> UiMessage {
        UiMessage::user(
            destination,
            direction,
            Box::new(Vec3EditorMessage::Value(value)),
        )
    }
}

#[derive(Clone)]
pub struct Vec3Editor<T: NumericType> {
    widget: Widget,
    x_field: Handle<UiNode>,
    y_field: Handle<UiNode>,
    z_field: Handle<UiNode>,
    value: Vector3<T>,
}

impl<T: NumericType> Deref for Vec3Editor<T> {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T: NumericType> DerefMut for Vec3Editor<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<T: NumericType> Control for Vec3Editor<T> {
    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        node_map.resolve(&mut self.x_field);
        node_map.resolve(&mut self.y_field);
        node_map.resolve(&mut self.z_field);
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
                            ui.send_message(Vec3EditorMessage::value(
                                self.handle(),
                                MessageDirection::ToWidget,
                                Vector3::new(value, self.value.y, self.value.z),
                            ));
                        } else if message.destination() == self.y_field {
                            ui.send_message(Vec3EditorMessage::value(
                                self.handle(),
                                MessageDirection::ToWidget,
                                Vector3::new(self.value.x, value, self.value.z),
                            ));
                        } else if message.destination() == self.z_field {
                            ui.send_message(Vec3EditorMessage::value(
                                self.handle(),
                                MessageDirection::ToWidget,
                                Vector3::new(self.value.x, self.value.y, value),
                            ));
                        }
                    }
                } else if let Some(Vec3EditorMessage::Value(value)) =
                    msg.cast::<Vec3EditorMessage<T>>()
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

pub struct Vec3EditorBuilder<T: NumericType> {
    widget_builder: WidgetBuilder,
    value: Vector3<T>,
}

impl<T: NumericType> Vec3EditorBuilder<T> {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: Vector3::new(T::zero(), T::zero(), T::zero()),
        }
    }

    pub fn with_value(mut self, value: Vector3<T>) -> Self {
        self.value = value;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let x_field;
        let y_field;
        let z_field;
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
                }),
        )
        .add_row(Row::stretch())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .build(ctx);

        let node = Vec3Editor {
            widget: self.widget_builder.with_child(grid).build(),
            x_field,
            y_field,
            z_field,
            value: self.value,
        };

        ctx.add_node(UiNode::new(node))
    }
}
