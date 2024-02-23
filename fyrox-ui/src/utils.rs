use crate::{
    border::BorderBuilder,
    core::{algebra::Vector2, color::Color, pool::Handle},
    formatted_text::WrapMode,
    text::TextBuilder,
    vector_image::{Primitive, VectorImageBuilder},
    widget::WidgetBuilder,
    Brush, BuildContext, HorizontalAlignment, RcUiNodeHandle, Thickness, UiNode, VerticalAlignment,
    BRUSH_BRIGHT, BRUSH_DARKER, BRUSH_DARKEST,
};

pub enum ArrowDirection {
    Top,
    Bottom,
    Left,
    Right,
}

pub fn make_arrow_primitives_non_uniform_size(
    orientation: ArrowDirection,
    width: f32,
    height: f32,
) -> Vec<Primitive> {
    vec![match orientation {
        ArrowDirection::Top => Primitive::Triangle {
            points: [
                Vector2::new(width * 0.5, 0.0),
                Vector2::new(width, height),
                Vector2::new(0.0, height),
            ],
        },
        ArrowDirection::Bottom => Primitive::Triangle {
            points: [
                Vector2::new(0.0, 0.0),
                Vector2::new(width, 0.0),
                Vector2::new(width * 0.5, height),
            ],
        },
        ArrowDirection::Right => Primitive::Triangle {
            points: [
                Vector2::new(0.0, 0.0),
                Vector2::new(width, height * 0.5),
                Vector2::new(0.0, height),
            ],
        },
        ArrowDirection::Left => Primitive::Triangle {
            points: [
                Vector2::new(0.0, height * 0.5),
                Vector2::new(width, 0.0),
                Vector2::new(width, height),
            ],
        },
    }]
}

pub fn make_arrow_primitives(orientation: ArrowDirection, size: f32) -> Vec<Primitive> {
    make_arrow_primitives_non_uniform_size(orientation, size, size)
}

pub fn make_arrow_non_uniform_size(
    ctx: &mut BuildContext,
    orientation: ArrowDirection,
    width: f32,
    height: f32,
) -> Handle<UiNode> {
    VectorImageBuilder::new(
        WidgetBuilder::new()
            .with_foreground(BRUSH_BRIGHT)
            .with_horizontal_alignment(HorizontalAlignment::Center)
            .with_vertical_alignment(VerticalAlignment::Center),
    )
    .with_primitives(make_arrow_primitives_non_uniform_size(
        orientation,
        width,
        height,
    ))
    .build(ctx)
}

pub fn make_arrow(
    ctx: &mut BuildContext,
    orientation: ArrowDirection,
    size: f32,
) -> Handle<UiNode> {
    make_arrow_non_uniform_size(ctx, orientation, size, size)
}

pub fn make_cross_primitive(size: f32, thickness: f32) -> Vec<Primitive> {
    vec![
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
    ]
}

pub fn make_cross(ctx: &mut BuildContext, size: f32, thickness: f32) -> Handle<UiNode> {
    VectorImageBuilder::new(
        WidgetBuilder::new()
            .with_horizontal_alignment(HorizontalAlignment::Center)
            .with_vertical_alignment(VerticalAlignment::Center)
            .with_foreground(BRUSH_BRIGHT),
    )
    .with_primitives(make_cross_primitive(size, thickness))
    .build(ctx)
}

pub fn make_simple_tooltip(ctx: &mut BuildContext, text: &str) -> RcUiNodeHandle {
    let handle = BorderBuilder::new(
        WidgetBuilder::new()
            .with_visibility(false)
            .with_foreground(BRUSH_DARKEST)
            .with_background(Brush::Solid(Color::opaque(230, 230, 230)))
            .with_max_size(Vector2::new(300.0, f32::INFINITY))
            .with_child(
                TextBuilder::new(
                    WidgetBuilder::new()
                        .with_margin(Thickness::uniform(2.0))
                        .with_foreground(BRUSH_DARKER),
                )
                .with_wrap(WrapMode::Word)
                .with_text(text)
                .build(ctx),
            ),
    )
    .build(ctx);
    RcUiNodeHandle::new(handle, ctx.sender())
}
