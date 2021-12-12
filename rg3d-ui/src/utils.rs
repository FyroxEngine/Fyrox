use crate::{
    border::BorderBuilder,
    core::{algebra::Vector2, pool::Handle},
    formatted_text::WrapMode,
    text::TextBuilder,
    vector_image::{Primitive, VectorImageBuilder},
    widget::WidgetBuilder,
    Brush, BuildContext, HorizontalAlignment, UiNode, VerticalAlignment, BRUSH_BRIGHT,
};
use rg3d_core::color::Color;

pub enum ArrowDirection {
    Top,
    Bottom,
    Left,
    Right,
}

pub fn make_arrow(
    ctx: &mut BuildContext,
    orientation: ArrowDirection,
    size: f32,
) -> Handle<UiNode> {
    VectorImageBuilder::new(
        WidgetBuilder::new()
            .with_foreground(BRUSH_BRIGHT)
            .with_horizontal_alignment(HorizontalAlignment::Center)
            .with_vertical_alignment(VerticalAlignment::Center),
    )
    .with_primitives(vec![match orientation {
        ArrowDirection::Top => Primitive::Triangle {
            points: [
                Vector2::new(size * 0.5, 0.0),
                Vector2::new(size, size),
                Vector2::new(0.0, size),
            ],
        },
        ArrowDirection::Bottom => Primitive::Triangle {
            points: [
                Vector2::new(0.0, 0.0),
                Vector2::new(size, 0.0),
                Vector2::new(size * 0.5, size),
            ],
        },
        ArrowDirection::Right => Primitive::Triangle {
            points: [
                Vector2::new(0.0, 0.0),
                Vector2::new(size, size * 0.5),
                Vector2::new(0.0, size),
            ],
        },
        ArrowDirection::Left => Primitive::Triangle {
            points: [
                Vector2::new(0.0, size * 0.5),
                Vector2::new(size, 0.0),
                Vector2::new(size, size),
            ],
        },
    }])
    .build(ctx)
}

pub fn make_simple_tooltip(ctx: &mut BuildContext, text: &str) -> Handle<UiNode> {
    BorderBuilder::new(
        WidgetBuilder::new()
            .with_visibility(false)
            .with_foreground(Brush::Solid(Color::opaque(160, 160, 160)))
            .with_max_size(Vector2::new(250.0, f32::INFINITY))
            .with_child(
                TextBuilder::new(WidgetBuilder::new())
                    .with_wrap(WrapMode::Word)
                    .with_text(text)
                    .build(ctx),
            ),
    )
    .build(ctx)
}
