use crate::message::{MessageData, MessageDirection};
use crate::{
    button::ButtonBuilder,
    core::pool::Handle,
    grid::{Column, GridBuilder, Row},
    message::ButtonMessage,
    message::{
        KeyCode, NumericUpDownMessage, TextBoxMessage, UiMessage, UiMessageData, WidgetMessage,
    },
    node::UINode,
    text_box::TextBoxBuilder,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, HorizontalAlignment, NodeHandleMapping, Thickness, UserInterface,
    VerticalAlignment,
};
use std::ops::{Deref, DerefMut};

#[derive(Clone)]
pub struct NumericUpDown<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    field: Handle<UINode<M, C>>,
    increase: Handle<UINode<M, C>>,
    decrease: Handle<UINode<M, C>>,
    value: f32,
    step: f32,
    min_value: f32,
    max_value: f32,
    precision: usize,
}

impl<M: MessageData, C: Control<M, C>> Deref for NumericUpDown<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: MessageData, C: Control<M, C>> DerefMut for NumericUpDown<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: MessageData, C: Control<M, C>> NumericUpDown<M, C> {
    fn try_parse_value(&mut self, ui: &mut UserInterface<M, C>) {
        // Parse input only when focus is lost from text field.
        if let UINode::TextBox(field) = ui.node(self.field) {
            if let Ok(value) = field.text().parse::<f32>() {
                let value = value.min(self.max_value).max(self.min_value);
                ui.send_message(NumericUpDownMessage::value(
                    self.handle(),
                    MessageDirection::ToWidget,
                    value,
                ));
            }
        }
    }
}

impl<M: MessageData, C: Control<M, C>> Control<M, C> for NumericUpDown<M, C> {
    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.field = *node_map.get(&self.field).unwrap();
        self.increase = *node_map.get(&self.increase).unwrap();
        self.decrease = *node_map.get(&self.decrease).unwrap();
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        match &message.data() {
            UiMessageData::Widget(msg) => {
                if message.destination() == self.field {
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
            UiMessageData::NumericUpDown(msg)
                if message.direction() == MessageDirection::ToWidget
                    && message.destination() == self.handle() =>
            {
                if let NumericUpDownMessage::Value(value) = *msg {
                    if value != self.value {
                        self.value = value;

                        // Sync text field.
                        ui.send_message(TextBoxMessage::text(
                            self.field,
                            MessageDirection::ToWidget,
                            format!("{:.1$}", self.value, self.precision),
                        ));

                        ui.send_message(message.reverse());
                    }
                }
            }
            UiMessageData::Button(msg) => {
                if let ButtonMessage::Click = msg {
                    if message.destination() == self.decrease {
                        let value = (self.value - self.step)
                            .min(self.max_value)
                            .max(self.min_value);
                        ui.send_message(NumericUpDownMessage::value(
                            self.handle(),
                            MessageDirection::ToWidget,
                            value,
                        ));
                    } else if message.destination() == self.increase {
                        let value = (self.value + self.step)
                            .min(self.max_value)
                            .max(self.min_value);
                        ui.send_message(NumericUpDownMessage::value(
                            self.handle(),
                            MessageDirection::ToWidget,
                            value,
                        ));
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct NumericUpDownBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    value: f32,
    step: f32,
    min_value: f32,
    max_value: f32,
    precision: usize,
}

impl<M: MessageData, C: Control<M, C>> NumericUpDownBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            value: 0.0,
            step: 0.1,
            min_value: -std::f32::MAX,
            max_value: std::f32::MAX,
            precision: 3,
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

    pub fn with_precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let increase;
        let decrease;
        let field;
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    field = TextBoxBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
                        .with_vertical_text_alignment(VerticalAlignment::Center)
                        .with_horizontal_text_alignment(HorizontalAlignment::Left)
                        .with_wrap(true)
                        .with_text(self.value.to_string())
                        .build(ctx);
                    field
                })
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .on_column(1)
                            .with_child({
                                increase = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::right(1.0))
                                        .on_row(0),
                                )
                                .with_text("^")
                                .build(ctx);
                                increase
                            })
                            .with_child({
                                decrease = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::right(1.0))
                                        .on_row(1),
                                )
                                .with_text("v")
                                .build(ctx);
                                decrease
                            }),
                    )
                    .add_column(Column::auto())
                    .add_row(Row::stretch())
                    .add_row(Row::stretch())
                    .build(ctx),
                ),
        )
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .build(ctx);

        let node = NumericUpDown {
            widget: self.widget_builder.with_child(grid).build(),
            increase,
            decrease,
            field,
            value: self.value,
            step: self.step,
            min_value: self.min_value,
            max_value: self.max_value,
            precision: self.precision,
        };

        ctx.add_node(UINode::NumericUpDown(node))
    }
}
