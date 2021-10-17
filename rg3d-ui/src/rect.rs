use crate::{
    core::{algebra::Vector2, math::Rect, pool::Handle},
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage, UiMessageData},
    numeric::NumericType,
    text::TextBuilder,
    vec::vec2::{Vec2EditorBuilder, Vec2EditorMessage},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, Thickness, UiNode, UserInterface, VerticalAlignment,
};
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq)]
pub enum RectEditorMessage<T>
where
    T: NumericType,
{
    Value(Rect<T>),
}

#[derive(Debug, Clone)]
pub struct RectEditor<T>
where
    T: NumericType,
{
    widget: Widget,
    position: Handle<UiNode>,
    size: Handle<UiNode>,
    value: Rect<T>,
}

impl<T> Deref for RectEditor<T>
where
    T: NumericType,
{
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T> DerefMut for RectEditor<T>
where
    T: NumericType,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<T> Control for RectEditor<T>
where
    T: NumericType,
{
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let UiMessageData::User(msg) = message.data() {
            if let Some(RectEditorMessage::Value(value)) = msg.cast::<RectEditorMessage<T>>() {
                if message.destination() == self.handle
                    && message.direction() == MessageDirection::ToWidget
                    && *value != self.value
                {
                    self.value = *value;

                    ui.send_message(message.reverse());
                }
            } else if let Some(Vec2EditorMessage::Value(value)) = msg.cast::<Vec2EditorMessage<T>>()
            {
                if message.direction() == MessageDirection::FromWidget {
                    if message.destination() == self.position {
                        if self.value.position != *value {
                            ui.send_message(UiMessage::user(
                                self.handle,
                                MessageDirection::ToWidget,
                                Box::new(RectEditorMessage::Value(Rect::new(
                                    value.x,
                                    value.y,
                                    self.value.size.x,
                                    self.value.size.y,
                                ))),
                            ));
                        }
                    } else if message.destination() == self.size && self.value.size != *value {
                        ui.send_message(UiMessage::user(
                            self.handle,
                            MessageDirection::ToWidget,
                            Box::new(RectEditorMessage::Value(Rect::new(
                                self.value.position.x,
                                self.value.position.y,
                                value.x,
                                value.y,
                            ))),
                        ));
                    }
                }
            }
        }
    }
}

pub struct RectEditorBuilder<T>
where
    T: NumericType,
{
    widget_builder: WidgetBuilder,
    value: Rect<T>,
}

fn create_field<T: NumericType>(
    ctx: &mut BuildContext,
    name: &str,
    value: Vector2<T>,
    row: usize,
) -> (Handle<UiNode>, Handle<UiNode>) {
    let editor;
    let grid = GridBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::left(10.0))
            .on_row(row)
            .with_child(
                TextBuilder::new(WidgetBuilder::new())
                    .with_text(name)
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ctx),
            )
            .with_child({
                editor = Vec2EditorBuilder::new(WidgetBuilder::new().on_column(1))
                    .with_value(value)
                    .build(ctx);
                editor
            }),
    )
    .add_column(Column::strict(70.0))
    .add_column(Column::stretch())
    .add_row(Row::stretch())
    .build(ctx);
    (grid, editor)
}

impl<T> RectEditorBuilder<T>
where
    T: NumericType,
{
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: Default::default(),
        }
    }

    pub fn with_value(mut self, value: Rect<T>) -> Self {
        self.value = value;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let (position_grid, position) = create_field(ctx, "Position", self.value.position, 0);
        let (size_grid, size) = create_field(ctx, "Size", self.value.size, 1);
        let node = RectEditor {
            widget: self
                .widget_builder
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child(position_grid)
                            .with_child(size_grid),
                    )
                    .add_row(Row::stretch())
                    .add_row(Row::stretch())
                    .add_column(Column::stretch())
                    .build(ctx),
                )
                .build(),
            value: self.value,
            position,
            size,
        };

        ctx.add_node(UiNode::new(node))
    }
}
