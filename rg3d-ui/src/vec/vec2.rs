use crate::numeric::{NumericType, NumericUpDownMessage};
use crate::{
    core::{algebra::Vector2, color::Color, pool::Handle},
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage, UiMessageData},
    vec::{make_mark, make_numeric_input},
    BuildContext, Control, NodeHandleMapping, UiNode, UserInterface, Widget, WidgetBuilder,
};
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, PartialEq)]
pub enum Vec2EditorMessage<T: NumericType> {
    Value(Vector2<T>),
}

impl<T: NumericType> Vec2EditorMessage<T> {
    pub fn value(
        destination: Handle<UiNode>,
        direction: MessageDirection,
        value: Vector2<T>,
    ) -> UiMessage {
        UiMessage::user(
            destination,
            direction,
            Box::new(Vec2EditorMessage::Value(value)),
        )
    }
}

#[derive(Clone)]
pub struct Vec2Editor<T: NumericType> {
    widget: Widget,
    x_field: Handle<UiNode>,
    y_field: Handle<UiNode>,
    value: Vector2<T>,
}

impl<T: NumericType> Deref for Vec2Editor<T> {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T: NumericType> DerefMut for Vec2Editor<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<T: NumericType> Control for Vec2Editor<T> {
    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        node_map.resolve(&mut self.x_field);
        node_map.resolve(&mut self.y_field);
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
                            ui.send_message(Vec2EditorMessage::value(
                                self.handle(),
                                MessageDirection::ToWidget,
                                Vector2::new(value, self.value.y),
                            ));
                        } else if message.destination() == self.y_field {
                            ui.send_message(Vec2EditorMessage::value(
                                self.handle(),
                                MessageDirection::ToWidget,
                                Vector2::new(self.value.x, value),
                            ));
                        }
                    }
                } else if let Some(Vec2EditorMessage::Value(value)) =
                    msg.cast::<Vec2EditorMessage<T>>()
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

pub struct Vec2EditorBuilder<T: NumericType> {
    widget_builder: WidgetBuilder,
    value: Vector2<T>,
}

impl<T: NumericType> Vec2EditorBuilder<T> {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: Vector2::new(T::zero(), T::zero()),
        }
    }

    pub fn with_value(mut self, value: Vector2<T>) -> Self {
        self.value = value;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let x_field;
        let y_field;
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
                }),
        )
        .add_row(Row::stretch())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .build(ctx);

        let node = Vec2Editor {
            widget: self.widget_builder.with_child(grid).build(),
            x_field,
            y_field,
            value: self.value,
        };

        ctx.add_node(UiNode::new(node))
    }
}
