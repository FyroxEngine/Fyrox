use crate::{
    core::{
        algebra::{Scalar, Vector2},
        math::Rect,
        num_traits::{cast::*, NumAssign},
        pool::Handle,
    },
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage, UiMessageData, Vec2EditorMessage},
    text::TextBuilder,
    vec::vec2::Vec2EditorBuilder,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, Thickness, UiNode, UserInterface, VerticalAlignment,
};
use std::{
    any::Any,
    fmt::Debug,
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq)]
pub enum RectEditorMessage<T>
where
    T: NumAssign + Scalar + PartialOrd + Debug + Copy + Send + Sync + NumCast + 'static,
{
    Value(Rect<T>),
}

#[derive(Debug, Clone)]
pub struct RectEditor<T>
where
    T: NumAssign + Scalar + PartialOrd + Debug + Copy + Send + Sync + NumCast + 'static,
{
    widget: Widget,
    position: Handle<UiNode>,
    size: Handle<UiNode>,
    value: Rect<T>,
}

impl<T> Deref for RectEditor<T>
where
    T: NumAssign + Scalar + PartialOrd + Copy + Send + Sync + NumCast + 'static,
{
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T> DerefMut for RectEditor<T>
where
    T: NumAssign + Scalar + Copy + PartialOrd + Send + Sync + NumCast + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

// TODO: Fix this when VecNEditor become generic over internal type.
fn to_vec2f32<T>(value: Vector2<T>) -> Vector2<f32>
where
    T: NumAssign + Scalar + Copy + PartialOrd + Send + Sync + NumCast + 'static,
{
    Vector2::new(
        NumCast::from(value.x).unwrap_or_default(),
        NumCast::from(value.y).unwrap_or_default(),
    )
}

impl<T> Control for RectEditor<T>
where
    T: NumAssign + Scalar + Copy + PartialOrd + Send + Sync + NumCast + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn clone_boxed(&self) -> Box<dyn Control> {
        Box::new(self.clone())
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        match message.data() {
            UiMessageData::Vec2Editor(Vec2EditorMessage::Value(value)) => {
                if message.direction() == MessageDirection::FromWidget {
                    if message.destination() == self.position {
                        let position = to_vec2f32(self.value.position);
                        if position != *value {
                            ui.send_message(UiMessage::user(
                                self.handle,
                                MessageDirection::ToWidget,
                                Box::new(RectEditorMessage::Value(Rect::new(
                                    value.x,
                                    value.y,
                                    NumCast::from(self.value.size.x).unwrap_or_default(),
                                    NumCast::from(self.value.size.y).unwrap_or_default(),
                                ))),
                            ));
                        }
                    } else if message.destination() == self.size {
                        let size = to_vec2f32(self.value.size);
                        if size != *value {
                            ui.send_message(UiMessage::user(
                                self.handle,
                                MessageDirection::ToWidget,
                                Box::new(RectEditorMessage::Value(Rect::new(
                                    NumCast::from(self.value.position.x).unwrap_or_default(),
                                    NumCast::from(self.value.position.y).unwrap_or_default(),
                                    value.x,
                                    value.y,
                                ))),
                            ));
                        }
                    }
                }
            }
            UiMessageData::User(msg) => {
                if message.destination() == self.handle
                    && message.direction() == MessageDirection::ToWidget
                {
                    if let Some(msg) = msg.cast::<RectEditorMessage<T>>() {
                        match msg {
                            RectEditorMessage::Value(value) => {
                                if *value != self.value {
                                    self.value = *value;

                                    ui.send_message(message.reverse());
                                }
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }
}

pub struct RectEditorBuilder<T>
where
    T: NumAssign + Scalar + PartialOrd + Copy + Send + Sync + NumCast + 'static,
{
    widget_builder: WidgetBuilder,
    value: Rect<T>,
}

fn create_field(
    ctx: &mut BuildContext,
    name: &str,
    value: Vector2<f32>,
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
    T: NumAssign + Scalar + PartialOrd + Copy + Send + Sync + NumCast + 'static,
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
        let (position_grid, position) =
            create_field(ctx, "Position", to_vec2f32(self.value.position), 0);
        let (size_grid, size) = create_field(ctx, "Size", to_vec2f32(self.value.size), 1);
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
