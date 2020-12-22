use crate::core::algebra::Vector3;
use crate::message::{MessageData, MessageDirection};
use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::{color::Color, pool::Handle},
    grid::{Column, GridBuilder, Row},
    message::{NumericUpDownMessage, UiMessage, UiMessageData, Vec3EditorMessage},
    node::UINode,
    numeric::NumericUpDownBuilder,
    text::TextBuilder,
    BuildContext, Control, NodeHandleMapping, Thickness, UserInterface, VerticalAlignment, Widget,
    WidgetBuilder,
};
use std::ops::{Deref, DerefMut};

#[derive(Clone)]
pub struct Vec3Editor<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    x_field: Handle<UINode<M, C>>,
    y_field: Handle<UINode<M, C>>,
    z_field: Handle<UINode<M, C>>,
    value: Vector3<f32>,
}

crate::define_widget_deref!(Vec3Editor<M, C>);

impl<M: MessageData, C: Control<M, C>> Control<M, C> for Vec3Editor<M, C> {
    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        node_map.resolve(&mut self.x_field);
        node_map.resolve(&mut self.y_field);
        node_map.resolve(&mut self.z_field);
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        match &message.data() {
            UiMessageData::NumericUpDown(msg)
                if message.direction() == MessageDirection::FromWidget =>
            {
                if let NumericUpDownMessage::Value(value) = *msg {
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
            }
            UiMessageData::Vec3Editor(msg) if message.direction() == MessageDirection::ToWidget => {
                if let Vec3EditorMessage::Value(value) = *msg {
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
            _ => (),
        }
    }
}

pub struct Vec3EditorBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    value: Vector3<f32>,
}

pub fn make_numeric_input<M: MessageData, C: Control<M, C>>(
    ctx: &mut BuildContext<M, C>,
    column: usize,
) -> Handle<UINode<M, C>> {
    NumericUpDownBuilder::new(
        WidgetBuilder::new()
            .on_row(0)
            .on_column(column)
            .with_margin(Thickness {
                left: 1.0,
                top: 0.0,
                right: 1.0,
                bottom: 0.0,
            }),
    )
    .build(ctx)
}

pub fn make_mark<M: MessageData, C: Control<M, C>>(
    ctx: &mut BuildContext<M, C>,
    text: &str,
    column: usize,
    color: Color,
) -> Handle<UINode<M, C>> {
    BorderBuilder::new(
        WidgetBuilder::new()
            .on_row(0)
            .on_column(column)
            .with_background(Brush::Solid(color))
            .with_foreground(Brush::Solid(Color::TRANSPARENT))
            .with_child(
                TextBuilder::new(WidgetBuilder::new())
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .with_text(text)
                    .build(ctx),
            ),
    )
    .build(ctx)
}

impl<M: MessageData, C: Control<M, C>> Vec3EditorBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            value: Default::default(),
        }
    }

    pub fn with_value(mut self, value: Vector3<f32>) -> Self {
        self.value = value;
        self
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let x_field;
        let y_field;
        let z_field;
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_mark(ctx, "X", 0, Color::opaque(120, 0, 0)))
                .with_child({
                    x_field = make_numeric_input(ctx, 1);
                    x_field
                })
                .with_child(make_mark(ctx, "Y", 2, Color::opaque(0, 120, 0)))
                .with_child({
                    y_field = make_numeric_input(ctx, 3);
                    y_field
                })
                .with_child(make_mark(ctx, "Z", 4, Color::opaque(0, 0, 120)))
                .with_child({
                    z_field = make_numeric_input(ctx, 5);
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

        ctx.add_node(UINode::Vec3Editor(node))
    }
}
