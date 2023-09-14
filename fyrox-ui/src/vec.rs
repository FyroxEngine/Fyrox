use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::{algebra::SVector, color::Color, num_traits, pool::Handle},
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage},
    numeric::{NumericType, NumericUpDownBuilder, NumericUpDownMessage},
    widget::WidgetBuilder,
    BuildContext, Control, NodeHandleMapping, Thickness, UiNode, UserInterface, Widget,
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

fn make_numeric_input<T: NumericType>(
    ctx: &mut BuildContext,
    column: usize,
    value: T,
    min: T,
    max: T,
    step: T,
    editable: bool,
) -> Handle<UiNode> {
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
    .with_precision(3)
    .with_value(value)
    .with_min_value(min)
    .with_max_value(max)
    .with_step(step)
    .with_editable(editable)
    .build(ctx)
}

pub fn make_mark(ctx: &mut BuildContext, column: usize, color: Color) -> Handle<UiNode> {
    BorderBuilder::new(
        WidgetBuilder::new()
            .on_row(0)
            .on_column(column)
            .with_background(Brush::Solid(color))
            .with_foreground(Brush::Solid(Color::TRANSPARENT))
            .with_width(4.0),
    )
    .build(ctx)
}

#[derive(Debug, Clone, PartialEq)]
pub enum VecEditorMessage<T, const D: usize>
where
    T: NumericType,
{
    Value(SVector<T, D>),
}

impl<T, const D: usize> VecEditorMessage<T, D>
where
    T: NumericType,
{
    define_constructor!(VecEditorMessage:Value => fn value(SVector<T, D>), layout: false);
}

#[derive(Clone)]
pub struct VecEditor<T, const D: usize>
where
    T: NumericType,
{
    pub widget: Widget,
    pub fields: Vec<Handle<UiNode>>,
    pub value: SVector<T, D>,
    pub min: SVector<T, D>,
    pub max: SVector<T, D>,
    pub step: SVector<T, D>,
}

impl<T, const D: usize> Deref for VecEditor<T, D>
where
    T: NumericType,
{
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T, const D: usize> DerefMut for VecEditor<T, D>
where
    T: NumericType,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<T, const D: usize> Control for VecEditor<T, D>
where
    T: NumericType,
{
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        node_map.resolve_slice(&mut self.fields);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(&NumericUpDownMessage::Value(value)) = message.data::<NumericUpDownMessage<T>>()
        {
            if message.direction() == MessageDirection::FromWidget {
                for (i, field) in self.fields.iter().enumerate() {
                    if message.destination() == *field {
                        let mut new_value = self.value;
                        new_value[i] = value;
                        ui.send_message(VecEditorMessage::value(
                            self.handle(),
                            MessageDirection::ToWidget,
                            new_value,
                        ));
                    }
                }
            }
        } else if let Some(&VecEditorMessage::Value(new_value)) =
            message.data::<VecEditorMessage<T, D>>()
        {
            if message.direction() == MessageDirection::ToWidget {
                let mut changed = false;

                for i in 0..D {
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

pub struct VecEditorBuilder<T, const D: usize>
where
    T: NumericType,
{
    widget_builder: WidgetBuilder,
    value: SVector<T, D>,
    editable: bool,
    min: SVector<T, D>,
    max: SVector<T, D>,
    step: SVector<T, D>,
}

impl<T, const D: usize> VecEditorBuilder<T, D>
where
    T: NumericType,
{
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: SVector::repeat(Default::default()),
            editable: true,
            min: SVector::repeat(T::min_value()),
            max: SVector::repeat(T::max_value()),
            step: SVector::repeat(T::one()),
        }
    }

    pub fn with_value(mut self, value: SVector<T, D>) -> Self {
        self.value = value;
        self
    }

    pub fn with_editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    pub fn with_min(mut self, min: SVector<T, D>) -> Self {
        self.min = min;
        self
    }

    pub fn with_max(mut self, max: SVector<T, D>) -> Self {
        self.max = max;
        self
    }

    pub fn with_step(mut self, step: SVector<T, D>) -> Self {
        self.step = step;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let mut fields = vec![];
        let mut children = vec![];
        let mut columns = vec![];

        let colors = [
            Color::opaque(120, 0, 0),
            Color::opaque(0, 120, 0),
            Color::opaque(0, 0, 120),
            Color::opaque(120, 0, 120),
            Color::opaque(0, 120, 120),
            Color::opaque(120, 120, 0),
        ];

        for i in 0..D {
            children.push(make_mark(
                ctx,
                i * 2,
                colors.get(i).cloned().unwrap_or(Color::ORANGE),
            ));

            let field = make_numeric_input(
                ctx,
                i * 2 + 1,
                self.value[i],
                self.min[i],
                self.max[i],
                self.step[i],
                self.editable,
            );
            children.push(field);
            fields.push(field);

            columns.push(Column::auto());
            columns.push(Column::stretch());
        }

        let grid = GridBuilder::new(WidgetBuilder::new().with_children(children))
            .add_row(Row::stretch())
            .add_columns(columns)
            .build(ctx);

        let node = VecEditor {
            widget: self.widget_builder.with_child(grid).build(),
            fields,
            value: self.value,
            min: self.min,
            max: self.max,
            step: self.step,
        };

        ctx.add_node(UiNode::new(node))
    }
}

pub type Vec2Editor<T> = VecEditor<T, 2>;
pub type Vec3Editor<T> = VecEditor<T, 3>;
pub type Vec4Editor<T> = VecEditor<T, 4>;
pub type Vec5Editor<T> = VecEditor<T, 5>;
pub type Vec6Editor<T> = VecEditor<T, 6>;

pub type Vec2EditorMessage<T> = VecEditorMessage<T, 2>;
pub type Vec3EditorMessage<T> = VecEditorMessage<T, 3>;
pub type Vec4EditorMessage<T> = VecEditorMessage<T, 4>;
pub type Vec5EditorMessage<T> = VecEditorMessage<T, 5>;
pub type Vec6EditorMessage<T> = VecEditorMessage<T, 6>;

pub type Vec2EditorBuilder<T> = VecEditorBuilder<T, 2>;
pub type Vec3EditorBuilder<T> = VecEditorBuilder<T, 3>;
pub type Vec4EditorBuilder<T> = VecEditorBuilder<T, 4>;
pub type Vec5EditorBuilder<T> = VecEditorBuilder<T, 5>;
pub type Vec6EditorBuilder<T> = VecEditorBuilder<T, 6>;
