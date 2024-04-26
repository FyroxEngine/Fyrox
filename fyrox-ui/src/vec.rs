use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::{
        algebra::SVector, color::Color, num_traits, pool::Handle, reflect::prelude::*,
        type_traits::prelude::*, visitor::prelude::*,
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
    value: T,
    min: T,
    max: T,
    step: T,
    editable: bool,
    precision: usize,
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
    .with_precision(precision)
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
    .with_corner_radius(2.0)
    .with_pad_by_corner_radius(false)
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

#[derive(Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct VecEditor<T, const D: usize>
where
    T: NumericType,
{
    pub widget: Widget,
    pub fields: Vec<Handle<UiNode>>,
    #[reflect(hidden)]
    #[visit(skip)]
    pub value: SVector<T, D>,
    #[reflect(hidden)]
    #[visit(skip)]
    pub min: SVector<T, D>,
    #[reflect(hidden)]
    #[visit(skip)]
    pub max: SVector<T, D>,
    #[reflect(hidden)]
    #[visit(skip)]
    pub step: SVector<T, D>,
}

impl<T, const D: usize> Default for VecEditor<T, D>
where
    T: NumericType,
{
    fn default() -> Self {
        Self {
            widget: Default::default(),
            fields: Default::default(),
            value: SVector::from([T::default(); D]),
            min: SVector::from([T::default(); D]),
            max: SVector::from([T::default(); D]),
            step: SVector::from([T::default(); D]),
        }
    }
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

// TODO: Is 16 enough?
const DIM_UUIDS: [Uuid; 16] = [
    uuid!("11ec6ec2-9780-4dbe-827a-935cb9ec5bb0"),
    uuid!("af532488-8833-443a-8ece-d8380e5ad148"),
    uuid!("6738154a-9663-4628-bb9d-f61d453eafcd"),
    uuid!("448dab8c-b4e6-478e-a704-ea0b0db628aa"),
    uuid!("67246977-8802-4e72-a19f-6e4f60b6eced"),
    uuid!("f711a9f8-288a-4a28-b30e-e7bcfdf26ab0"),
    uuid!("c92ac3ad-5dc5-41dd-abbd-7fb9aacb9a5f"),
    uuid!("88d9a035-4424-40d2-af62-f701025bd767"),
    uuid!("dda09036-18d8-40bc-ae9d-1a69f45e2ba0"),
    uuid!("b6fe9585-6ebc-4b4d-be66-484b4d7b3d5b"),
    uuid!("03c41033-e8fe-420d-b246-e7c9dcd7c01b"),
    uuid!("14ea7e95-0f94-4b15-a53c-97d7d2e58d4e"),
    uuid!("0149f666-33cf-4e39-b4bd-58502994b162"),
    uuid!("abb9f691-0958-464b-a37d-a3336b4d33f9"),
    uuid!("6f37cfd5-9bec-40ec-9dbc-e532d43b81b7"),
    uuid!("fa786077-95b9-4e7c-9268-7d0314c005ba"),
];

impl<T: NumericType, const D: usize> TypeUuidProvider for VecEditor<T, D> {
    fn type_uuid() -> Uuid {
        combine_uuids(
            combine_uuids(
                uuid!("0332144f-c70e-456a-812b-f9b89980d2ba"),
                T::type_uuid(),
            ),
            DIM_UUIDS[D],
        )
    }
}

impl<T, const D: usize> Control for VecEditor<T, D>
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
    precision: usize,
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
            precision: 3,
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

    pub fn with_precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let mut fields = Vec::new();
        let mut children = Vec::new();
        let mut columns = Vec::new();

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
                self.precision,
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
