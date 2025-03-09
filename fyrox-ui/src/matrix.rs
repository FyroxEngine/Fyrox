// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::{
    core::{
        num_traits, pool::Handle, reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*,
    },
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage},
    numeric::{NumericType, NumericUpDownBuilder, NumericUpDownMessage},
    widget::WidgetBuilder,
    BuildContext, Control, Thickness, UiNode, UserInterface, Widget,
};
use fyrox_core::algebra::SMatrix;

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
pub enum MatrixEditorMessage<const R: usize, const C: usize, T>
where
    T: NumericType,
{
    Value(SMatrix<T, R, C>),
}

impl<const R: usize, const C: usize, T> MatrixEditorMessage<R, C, T>
where
    T: NumericType,
{
    define_constructor!(MatrixEditorMessage:Value => fn value(SMatrix<T, R, C>), layout: false);
}

#[derive(Clone, Visit, Reflect, Debug, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct MatrixEditor<const R: usize, const C: usize, T>
where
    T: NumericType,
{
    pub widget: Widget,
    pub fields: Vec<Handle<UiNode>>,
    #[reflect(hidden)]
    #[visit(skip)]
    pub value: SMatrix<T, R, C>,
    #[reflect(hidden)]
    #[visit(skip)]
    pub min: SMatrix<T, R, C>,
    #[reflect(hidden)]
    #[visit(skip)]
    pub max: SMatrix<T, R, C>,
    #[reflect(hidden)]
    #[visit(skip)]
    pub step: SMatrix<T, R, C>,
}

impl<const R: usize, const C: usize, T> Default for MatrixEditor<R, C, T>
where
    T: NumericType,
{
    fn default() -> Self {
        Self {
            widget: Default::default(),
            fields: Default::default(),
            value: SMatrix::repeat(T::zero()),
            min: SMatrix::repeat(T::min_value()),
            max: SMatrix::repeat(T::max_value()),
            step: SMatrix::repeat(T::one()),
        }
    }
}

impl<const R: usize, const C: usize, T> Deref for MatrixEditor<R, C, T>
where
    T: NumericType,
{
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<const R: usize, const C: usize, T> DerefMut for MatrixEditor<R, C, T>
where
    T: NumericType,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<const R: usize, const C: usize, T: NumericType> TypeUuidProvider for MatrixEditor<R, C, T> {
    fn type_uuid() -> Uuid {
        let r_id = Uuid::from_u64_pair(R as u64, R as u64);
        let c_id = Uuid::from_u64_pair(C as u64, C as u64);
        combine_uuids(
            c_id,
            combine_uuids(
                r_id,
                combine_uuids(
                    uuid!("9f05427a-5862-4574-bb21-ebaf52aa8c72"),
                    T::type_uuid(),
                ),
            ),
        )
    }
}

impl<const R: usize, const C: usize, T> Control for MatrixEditor<R, C, T>
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
                        ui.send_message(MatrixEditorMessage::value(
                            self.handle(),
                            MessageDirection::ToWidget,
                            new_value,
                        ));
                    }
                }
            }
        } else if let Some(&MatrixEditorMessage::Value(new_value)) =
            message.data::<MatrixEditorMessage<R, C, T>>()
        {
            if message.direction() == MessageDirection::ToWidget {
                let mut changed = false;

                for i in 0..self.fields.len() {
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

pub struct MatrixEditorBuilder<const R: usize, const C: usize, T>
where
    T: NumericType,
{
    widget_builder: WidgetBuilder,
    value: SMatrix<T, R, C>,
    editable: bool,
    min: SMatrix<T, R, C>,
    max: SMatrix<T, R, C>,
    step: SMatrix<T, R, C>,
    precision: usize,
}

impl<const R: usize, const C: usize, T> MatrixEditorBuilder<R, C, T>
where
    T: NumericType,
{
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: SMatrix::identity(),
            editable: true,
            min: SMatrix::repeat(T::min_value()),
            max: SMatrix::repeat(T::max_value()),
            step: SMatrix::repeat(T::one()),
            precision: 3,
        }
    }

    pub fn with_value(mut self, value: SMatrix<T, R, C>) -> Self {
        self.value = value;
        self
    }

    pub fn with_editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    pub fn with_min(mut self, min: SMatrix<T, R, C>) -> Self {
        self.min = min;
        self
    }

    pub fn with_max(mut self, max: SMatrix<T, R, C>) -> Self {
        self.max = max;
        self
    }

    pub fn with_step(mut self, step: SMatrix<T, R, C>) -> Self {
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

        for row in 0..R {
            for column in 0..C {
                let field = make_numeric_input(
                    ctx,
                    column,
                    row,
                    self.value[(row, column)],
                    self.min[(row, column)],
                    self.max[(row, column)],
                    self.step[(row, column)],
                    self.editable,
                    self.precision,
                );
                children.push(field);
                fields.push(field);
            }
        }

        let grid = GridBuilder::new(WidgetBuilder::new().with_children(children))
            .add_rows(vec![Row::stretch(); R])
            .add_columns(vec![Column::stretch(); C])
            .build(ctx);

        let node = MatrixEditor {
            widget: self.widget_builder.with_child(grid).build(ctx),
            fields,
            value: self.value,
            min: self.min,
            max: self.max,
            step: self.step,
        };

        ctx.add_node(UiNode::new(node))
    }
}

#[cfg(test)]
mod test {
    use crate::matrix::MatrixEditorBuilder;
    use crate::{test::test_widget_deletion, widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| {
            MatrixEditorBuilder::<2, 2, f32>::new(WidgetBuilder::new()).build(ctx)
        });
    }
}
