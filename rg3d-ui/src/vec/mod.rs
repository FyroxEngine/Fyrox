use crate::{
    border::BorderBuilder, brush::Brush, core::color::Color, core::pool::Handle,
    message::MessageData, node::UINode, numeric::NumericUpDownBuilder, text::TextBuilder,
    widget::WidgetBuilder, BuildContext, Control, Thickness, VerticalAlignment,
};

pub mod vec2;
pub mod vec3;
pub mod vec4;

pub fn make_numeric_input<M: MessageData, C: Control<M, C>>(
    ctx: &mut BuildContext<M, C>,
    column: usize,
    value: f32,
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
    .with_value(value)
    .build(ctx)
}

pub fn make_mark<M: MessageData, C: Control<M, C>>(
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
