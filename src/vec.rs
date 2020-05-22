use std::ops::{Deref, DerefMut};
use crate::{
    message::{
        UiMessageData,
        NumericUpDownMessage,
        UiMessage,
        Vec3EditorMessage
    },
    border::BorderBuilder,
    node::UINode,
    Control,
    UserInterface,
    Widget,
    WidgetBuilder,
    Thickness,
    grid::{GridBuilder, Column, Row},
    numeric::NumericUpDownBuilder,
    text::TextBuilder,
    HorizontalAlignment,
    VerticalAlignment,
    NodeHandleMapping,
    core::{
        color::Color,
        pool::Handle,
        math::vec3::Vec3
    },
    brush::Brush,
};

pub struct Vec3Editor<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    x_field: Handle<UINode<M, C>>,
    y_field: Handle<UINode<M, C>>,
    z_field: Handle<UINode<M, C>>,
    value: Vec3
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for Vec3Editor<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for Vec3Editor<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Clone for Vec3Editor<M, C> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.raw_copy(),
            x_field: self.x_field,
            y_field: self.y_field,
            z_field: self.z_field,
            value: self.value
        }
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for Vec3Editor<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::Vec3Editor(self.clone())
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.x_field = *node_map.get(&self.x_field).unwrap();
        self.y_field = *node_map.get(&self.y_field).unwrap();
        self.z_field = *node_map.get(&self.z_field).unwrap();
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_routed_message(ui, message);

        match &message.data {
            UiMessageData::NumericUpDown(msg) => {
                if let &NumericUpDownMessage::Value(value) = msg {
                    if message.destination == self.x_field {
                        if self.value.x != value {
                            ui.send_message(UiMessage {
                                handled: false,
                                data: UiMessageData::Vec3Editor(Vec3EditorMessage::Value(Vec3::new(value, self.value.y, self.value.z))),
                                destination: self.handle
                            });
                        }
                    } else if message.destination == self.y_field {
                        if self.value.y != value {
                            ui.send_message(UiMessage {
                                handled: false,
                                data: UiMessageData::Vec3Editor(Vec3EditorMessage::Value(Vec3::new(self.value.x, value, self.value.z))),
                                destination: self.handle
                            });
                        }
                    } else if message.destination == self.z_field {
                        if self.value.z != value {
                            ui.send_message(UiMessage {
                                handled: false,
                                data: UiMessageData::Vec3Editor(Vec3EditorMessage::Value(Vec3::new(self.value.x, self.value.y, value))),
                                destination: self.handle
                            });
                        }
                    }
                }
            }
            UiMessageData::Vec3Editor(msg) => {
                if let &Vec3EditorMessage::Value(value) = msg {
                    if self.value != value {
                        self.value = value;
                        self.invalidate_layout();

                        ui.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value.x)),
                            destination: self.x_field
                        });
                        ui.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value.y)),
                            destination: self.y_field
                        });
                        ui.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value.z)),
                            destination: self.z_field
                        });
                    }
                }
            }
            _ => ()
        }
    }
}

pub struct Vec3EditorBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    value: Vec3
}

pub fn make_numeric_input<M, C: 'static + Control<M, C>>(ui: &mut UserInterface<M, C>, column: usize) -> Handle<UINode<M, C>> {
    NumericUpDownBuilder::new(WidgetBuilder::new()
        .on_row(0)
        .on_column(column)
        .with_margin(Thickness{
            left: 1.0,
            top: 0.0,
            right: 1.0,
            bottom: 0.0
        }))
        .build(ui)
}

pub fn make_mark<M, C: 'static + Control<M, C>>(ui: &mut UserInterface<M, C>, text: &str, column: usize, color: Color) -> Handle<UINode<M, C>> {
    BorderBuilder::new(WidgetBuilder::new()
        .on_row(0)
        .on_column(column)
        .with_background(Brush::Solid(color))
        .with_foreground(Brush::Solid(Color::TRANSPARENT))
        .with_child(TextBuilder::new(WidgetBuilder::new()
            .with_margin(Thickness::uniform(2.0)))
            .with_horizontal_text_alignment(HorizontalAlignment::Center)
            .with_vertical_text_alignment(VerticalAlignment::Center)
            .with_text(text)
            .build(ui)))
        .build(ui)
}

impl<M, C: 'static + Control<M, C>> Vec3EditorBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            value: Default::default()
        }
    }

    pub fn with_value(mut self, value: Vec3) -> Self {
        self.value = value;
        self
    }

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        let x_field;
        let y_field;
        let z_field;
        let grid = GridBuilder::new(WidgetBuilder::new()
            .with_child(make_mark(ui, "X", 0, Color::opaque(120, 0, 0)))
            .with_child({
                x_field = make_numeric_input(ui, 1);
                x_field
            })
            .with_child(make_mark(ui, "Y", 2, Color::opaque(0, 120, 0)))
            .with_child({
                y_field = make_numeric_input(ui, 3);
                y_field
            })
            .with_child(make_mark(ui, "Z", 4, Color::opaque(0, 0, 120)))
            .with_child({
                z_field = make_numeric_input(ui, 5);
                z_field
            }))
            .add_row(Row::stretch())
            .add_column(Column::auto())
            .add_column(Column::stretch())
            .add_column(Column::auto())
            .add_column(Column::stretch())
            .add_column(Column::auto())
            .add_column(Column::stretch())
            .build(ui);

        let node = Vec3Editor {
            widget: self.widget_builder
                .with_child(grid)
                .build(ui.sender()),
            x_field,
            y_field,
            z_field,
            value: self.value
        };

        let handle = ui.add_node(UINode::Vec3Editor(node));

        ui.flush_messages();

        handle
    }
}