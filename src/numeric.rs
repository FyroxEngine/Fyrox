use std::ops::{Deref, DerefMut};
use crate::{
    message::{
        UiMessage,
        UiMessageData,
        TextBoxMessage,
        WidgetMessage,
        NumericUpDownMessage,
        KeyCode,
    },
    node::UINode,
    Control,
    UserInterface,
    widget::{Widget, WidgetBuilder},
    core::pool::Handle,
    NodeHandleMapping,
    grid::{
        GridBuilder,
        Row,
        Column,
    },
    text_box::TextBoxBuilder,
    button::ButtonBuilder,
    VerticalAlignment,
    HorizontalAlignment,
};
use crate::message::ButtonMessage;

pub struct NumericUpDown<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    field: Handle<UINode<M, C>>,
    increase: Handle<UINode<M, C>>,
    decrease: Handle<UINode<M, C>>,
    value: f32,
    step: f32,
    min_value: f32,
    max_value: f32,
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for NumericUpDown<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for NumericUpDown<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Clone for NumericUpDown<M, C> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.raw_copy(),
            field: self.field,
            increase: self.increase,
            decrease: self.decrease,
            value: self.value,
            step: self.step,
            min_value: self.min_value,
            max_value: self.max_value,
        }
    }
}

impl<M: 'static, C: 'static + Control<M, C>> NumericUpDown<M, C> {
    fn try_parse_value(&mut self, ui: &mut UserInterface<M, C>) {
        // Parse input only when focus is lost from text field.
        if let UINode::TextBox(field) = ui.node(self.field) {
            if let Ok(value) = field.text().parse::<f32>() {
                let value = value.min(self.max_value).max(self.min_value);
                ui.send_message(UiMessage {
                    handled: false,
                    data: UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value)),
                    destination: self.handle,
                });
            }
        }
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for NumericUpDown<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::NumericUpDown(self.clone())
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.field = *node_map.get(&self.field).unwrap();
        self.increase = *node_map.get(&self.increase).unwrap();
        self.decrease = *node_map.get(&self.decrease).unwrap();
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_routed_message(ui, message);


        match &message.data {
            UiMessageData::Widget(msg) => {
                if message.destination == self.field {
                    match msg {
                        WidgetMessage::LostFocus => {
                            self.try_parse_value(ui);
                        }
                        WidgetMessage::KeyDown(key) => {
                            if let KeyCode::Return = key {
                                self.try_parse_value(ui);
                            }
                        }
                        _ => {}
                    }
                }
            }
            UiMessageData::NumericUpDown(msg) => {
                if message.destination == self.handle {
                    if let &NumericUpDownMessage::Value(value) = msg {
                        if value != self.value {
                            self.value = value;

                            // Sync text field.
                            ui.send_message(UiMessage {
                                handled: false,
                                data: UiMessageData::TextBox(TextBoxMessage::Text(self.value.to_string())),
                                destination: self.field,
                            });
                        }
                    }
                }
            }
            UiMessageData::Button(msg) => {
                if let ButtonMessage::Click = msg {
                    if message.destination == self.decrease {
                        let value = (self.value - self.step).min(self.max_value).max(self.min_value);
                        ui.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value)),
                            destination: self.handle,
                        });
                    } else if message.destination == self.increase {
                        let value = (self.value + self.step).min(self.max_value).max(self.min_value);
                        ui.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value)),
                            destination: self.handle,
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct NumericUpDownBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    value: f32,
    step: f32,
    min_value: f32,
    max_value: f32,
}

impl<M, C: 'static + Control<M, C>> NumericUpDownBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            value: 0.0,
            step: 0.1,
            min_value: -std::f32::MAX,
            max_value: std::f32::MAX,
        }
    }

    fn set_value(&mut self, value: f32) {
        self.value = value.max(self.min_value).min(self.max_value);
    }

    pub fn with_min_value(mut self, value: f32) -> Self {
        self.min_value = value;
        self.set_value(self.value);
        self
    }

    pub fn with_max_value(mut self, value: f32) -> Self {
        self.max_value = value;
        self.set_value(self.value);
        self
    }

    pub fn with_value(mut self, value: f32) -> Self {
        self.value = value;
        self.set_value(value);
        self
    }

    pub fn with_step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        let increase;
        let decrease;
        let field;
        let grid = GridBuilder::new(WidgetBuilder::new()
            .with_child({
                field = TextBoxBuilder::new(WidgetBuilder::new()
                    .on_row(0)
                    .on_column(0))
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .with_horizontal_text_alignment(HorizontalAlignment::Left)
                    .with_wrap(true)
                    .with_text(self.value.to_string())
                    .build(ui);
                field
            })
            .with_child(GridBuilder::new(WidgetBuilder::new()
                .on_column(1)
                .with_child({
                    increase = ButtonBuilder::new(WidgetBuilder::new()
                        .on_row(0))
                        .with_text("^")
                        .build(ui);
                    increase
                })
                .with_child({
                    decrease = ButtonBuilder::new(WidgetBuilder::new()
                        .on_row(1))
                        .with_text("v")
                        .build(ui);
                    decrease
                }))
                .add_column(Column::auto())
                .add_row(Row::stretch())
                .add_row(Row::stretch())
                .build(ui)))
            .add_row(Row::stretch())
            .add_column(Column::stretch())
            .add_column(Column::auto())
            .build(ui);

        let node = NumericUpDown {
            widget: self.widget_builder
                .with_child(grid)
                .build(ui.sender()),
            increase,
            decrease,
            field,
            value: self.value,
            step: self.step,
            min_value: self.min_value,
            max_value: self.max_value,
        };

        let handle = ui.add_node(UINode::NumericUpDown(node));

        ui.flush_messages();

        handle
    }
}