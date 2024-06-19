use crate::{
    core::{
        algebra::Matrix2, num_traits, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*,
    },
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage},
    numeric::{NumericType, NumericUpDownBuilder, NumericUpDownMessage},
    widget::WidgetBuilder,
    BuildContext, Control, Thickness, UiNode, UserInterface, Widget,
};
use std::ops::{Deref, DerefMut};

fn make_numeric_input<T: NumericType>(
    ctx: &mut BuildContext,
    column: usize,
    row: usize,
    value: T,
    min: T,
    max: T,
    step: T,
    editable: bool,
    precision: usize,
) -> Handle<UiNode> {
    NumericUpDownBuilder::new(
        WidgetBuilder::new()
            .on_row(row)
            .on_column(column)
            .with_margin(Thickness {
                left: 1.0,
                top: 0.0,
                right: 1.0,
                bottom: 0.0,
            }),
    )
    .with_precision(precision)
    .with_value(value)
    .with_min_value(min)
    .with_max_value(max)
    .with_step(step)
    .with_editable(editable)
    .build(ctx)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Matrix2EditorMessage<T>
where
    T: NumericType,
{
    Value(Matrix2<T>),
}

impl<T> Matrix2EditorMessage<T>
where
    T: NumericType,
{
    define_constructor!(Matrix2EditorMessage:Value => fn value(Matrix2<T>), layout: false);
}

#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct Matrix2Editor<T>
where
    T: NumericType,
{
    pub widget: Widget,
    pub fields: [Handle<UiNode>; 4],
    #[reflect(hidden)]
    #[visit(skip)]
    pub value: Matrix2<T>,
    #[reflect(hidden)]
    #[visit(skip)]
    pub min: Matrix2<T>,
    #[reflect(hidden)]
    #[visit(skip)]
    pub max: Matrix2<T>,
    #[reflect(hidden)]
    #[visit(skip)]
    pub step: Matrix2<T>,
}

impl<T> Deref for Matrix2Editor<T>
where
    T: NumericType,
{
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T> DerefMut for Matrix2Editor<T>
where
    T: NumericType,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<T: NumericType> TypeUuidProvider for Matrix2Editor<T> {
    fn type_uuid() -> Uuid {
        combine_uuids(
            uuid!("9f05427a-5862-4574-bb21-ebaf52aa8c72"),
            T::type_uuid(),
        )
    }
}

impl<T> Control for Matrix2Editor<T>
where
    T: NumericType,
{
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(&NumericUpDownMessage::Value(value)) = message.data::<NumericUpDownMessage<T>>()
        {
            if message.direction() == MessageDirection::FromWidget {
                for (i, field) in self.fields.iter().enumerate() {
                    if message.destination() == *field {
                        let mut new_value = self.value;
                        new_value[i] = value;
                        ui.send_message(Matrix2EditorMessage::value(
                            self.handle(),
                            MessageDirection::ToWidget,
                            new_value,
                        ));
                    }
                }
            }
        } else if let Some(&Matrix2EditorMessage::Value(new_value)) =
            message.data::<Matrix2EditorMessage<T>>()
        {
            if message.direction() == MessageDirection::ToWidget {
                let mut changed = false;

                for i in 0..4 {
                    let editor = self.fields[i];
                    let current = &mut self.value[i];
                    let min = self.min[i];
                    let max = self.max[i];
                    let new = num_traits::clamp(new_value[i], min, max);

                    if *current != new {
                        *current = new;
                        ui.send_message(NumericUpDownMessage::value(
                            editor,
                            MessageDirection::ToWidget,
                            new,
                        ));
                        changed = true;
                    }
                }

                if changed {
                    ui.send_message(message.reverse());
                }
            }
        }
    }
}

pub struct Matrix2EditorBuilder<T>
where
    T: NumericType,
{
    widget_builder: WidgetBuilder,
    value: Matrix2<T>,
    editable: bool,
    min: Matrix2<T>,
    max: Matrix2<T>,
    step: Matrix2<T>,
    precision: usize,
}

impl<T> Matrix2EditorBuilder<T>
where
    T: NumericType,
{
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: Matrix2::identity(),
            editable: true,
            min: Matrix2::repeat(T::min_value()),
            max: Matrix2::repeat(T::max_value()),
            step: Matrix2::repeat(T::one()),
            precision: 3,
        }
    }

    pub fn with_value(mut self, value: Matrix2<T>) -> Self {
        self.value = value;
        self
    }

    pub fn with_editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    pub fn with_min(mut self, min: Matrix2<T>) -> Self {
        self.min = min;
        self
    }

    pub fn with_max(mut self, max: Matrix2<T>) -> Self {
        self.max = max;
        self
    }

    pub fn with_step(mut self, step: Matrix2<T>) -> Self {
        self.step = step;
        self
    }

    pub fn with_precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let mut fields = Vec::new();
        let mut children = Vec::new();

        for y in 0..2 {
            for x in 0..2 {
                let field = make_numeric_input(
                    ctx,
                    x,
                    y,
                    self.value[(y, x)],
                    self.min[(y, x)],
                    self.max[(y, x)],
                    self.step[(y, x)],
                    self.editable,
                    self.precision,
                );
                children.push(field);
                fields.push(field);
            }
        }

        let grid = GridBuilder::new(WidgetBuilder::new().with_children(children))
            .add_row(Row::stretch())
            .add_row(Row::stretch())
            .add_column(Column::stretch())
            .add_column(Column::stretch())
            .build(ctx);

        let node = Matrix2Editor {
            widget: self.widget_builder.with_child(grid).build(),
            fields: fields.try_into().unwrap(),
            value: self.value,
            min: self.min,
            max: self.max,
            step: self.step,
        };

        ctx.add_node(UiNode::new(node))
    }
}
