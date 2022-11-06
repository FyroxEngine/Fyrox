use crate::{
    border::BorderBuilder,
    core::{algebra::Vector2, pool::Handle},
    formatted_text::WrapMode,
    text::TextBuilder,
    vector_image::{Primitive, VectorImageBuilder},
    widget::WidgetBuilder,
    Brush, BuildContext, HorizontalAlignment, Thickness, UiNode, VerticalAlignment, BRUSH_BRIGHT,
};
use fyrox_core::color::Color;
use std::rc::Rc;

pub enum ArrowDirection {
    Top,
    Bottom,
    Left,
    Right,
}

pub fn make_arrow_primitives(orientation: ArrowDirection, size: f32) -> Vec<Primitive> {
    vec![match orientation {
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
    }]
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
    .with_primitives(make_arrow_primitives(orientation, size))
    .build(ctx)
}

pub fn make_cross(ctx: &mut BuildContext, size: f32, thickness: f32) -> Handle<UiNode> {
    VectorImageBuilder::new(
        WidgetBuilder::new()
            .with_horizontal_alignment(HorizontalAlignment::Center)
            .with_vertical_alignment(VerticalAlignment::Center)
            .with_foreground(BRUSH_BRIGHT),
    )
    .with_primitives(vec![
        Primitive::Line {
            begin: Vector2::new(0.0, 0.0),
            end: Vector2::new(size, size),
            thickness,
        },
        Primitive::Line {
            begin: Vector2::new(size, 0.0),
            end: Vector2::new(0.0, size),
            thickness,
        },
    ])
    .build(ctx)
}

pub fn make_simple_tooltip(ctx: &mut BuildContext, text: &str) -> Rc<Handle<UiNode>> {
    Rc::new(
        BorderBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false)
                .with_foreground(Brush::Solid(Color::opaque(160, 160, 160)))
                .with_max_size(Vector2::new(300.0, f32::INFINITY))
                .with_child(
                    TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(2.0)))
                        .with_wrap(WrapMode::Word)
                        .with_text(text)
                        .build(ctx),
                ),
        )
        .build(ctx),
    )
}
