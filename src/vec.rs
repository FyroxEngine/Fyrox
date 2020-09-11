use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::{color::Color, math::vec3::Vec3, pool::Handle},
    grid::{Column, GridBuilder, Row},
    message::{NumericUpDownMessage, UiMessage, UiMessageData, Vec3EditorMessage},
    node::UINode,
    numeric::NumericUpDownBuilder,
    text::TextBuilder,
    BuildContext, Control, NodeHandleMapping, Thickness, UserInterface, VerticalAlignment, Widget,
    WidgetBuilder,
};
use std::ops::{Deref, DerefMut};

pub struct Vec3Editor<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    x_field: Handle<UINode<M, C>>,
    y_field: Handle<UINode<M, C>>,
    z_field: Handle<UINode<M, C>>,
    value: Vec3,
}

impl<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> Deref for Vec3Editor<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> DerefMut for Vec3Editor<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> Clone for Vec3Editor<M, C> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.raw_copy(),
            x_field: self.x_field,
            y_field: self.y_field,
            z_field: self.z_field,
            value: self.value,
        }
    }
}

impl<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> Control<M, C> for Vec3Editor<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::Vec3Editor(self.clone())
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.x_field = *node_map.get(&self.x_field).unwrap();
        self.y_field = *node_map.get(&self.y_field).unwrap();
        self.z_field = *node_map.get(&self.z_field).unwrap();
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        match &message.data {
            UiMessageData::NumericUpDown(msg) => {
                if let NumericUpDownMessage::Value(value) = *msg {
                    if message.destination == self.x_field {
                        if self.value.x != value {
                            ui.send_message(UiMessage {
                                handled: false,
                                data: UiMessageData::Vec3Editor(Vec3EditorMessage::Value(
                                    Vec3::new(value, self.value.y, self.value.z),
                                )),
                                destination: self.handle(),
                            });
                        }
                    } else if message.destination == self.y_field {
                        if self.value.y != value {
                            ui.send_message(UiMessage {
                                handled: false,
                                data: UiMessageData::Vec3Editor(Vec3EditorMessage::Value(
                                    Vec3::new(self.value.x, value, self.value.z),
                                )),
                                destination: self.handle(),
                            });
                        }
                    } else if message.destination == self.z_field && self.value.z != value {
                        ui.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::Vec3Editor(Vec3EditorMessage::Value(Vec3::new(
                                self.value.x,
                                self.value.y,
                                value,
                            ))),
                            destination: self.handle(),
                        });
                    }
                }
            }
            UiMessageData::Vec3Editor(msg) => {
                if let Vec3EditorMessage::Value(value) = *msg {
                    if self.value.x != value.x {
                        self.value.x = value.x;
                        ui.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::NumericUpDown(NumericUpDownMessage::Value(
                                value.x,
                            )),
                            destination: self.x_field,
                        });
                    }
                    if self.value.y != value.y {
                        self.value.y = value.y;
                        ui.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::NumericUpDown(NumericUpDownMessage::Value(
                                value.y,
                            )),
                            destination: self.y_field,
                        });
                    }
                    if self.value.z != value.z {
                        self.value.z = value.z;
                        ui.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::NumericUpDown(NumericUpDownMessage::Value(
                                value.z,
                            )),
                            destination: self.z_field,
                        });
                    }
                }
            }
            _ => (),
        }
    }
}

pub struct Vec3EditorBuilder<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    value: Vec3,
}

pub fn make_numeric_input<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>>(
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

pub fn make_mark<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>>(
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

impl<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> Vec3EditorBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            value: Default::default(),
        }
    }

    pub fn with_value(mut self, value: Vec3) -> Self {
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
