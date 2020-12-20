use crate::{
    core::algebra::Vector2,
    core::pool::Handle,
    message::MessageData,
    node::UINode,
    vector_image::{Primitive, VectorImageBuilder},
    widget::WidgetBuilder,
    BuildContext, Control, HorizontalAlignment, VerticalAlignment, BRUSH_BRIGHT,
};

pub enum ArrowDirection {
    North,
    South,
    West,
    East,
}

pub fn make_arrow<M: MessageData, C: Control<M, C>>(
    ctx: &mut BuildContext<M, C>,
    orientation: ArrowDirection,
    size: f32,
) -> Handle<UINode<M, C>> {
    VectorImageBuilder::new(
        WidgetBuilder::new()
            .with_foreground(BRUSH_BRIGHT)
            .with_horizontal_alignment(HorizontalAlignment::Center)
            .with_vertical_alignment(VerticalAlignment::Center),
    )
    .with_primitives(vec![match orientation {
        ArrowDirection::North => Primitive::Triangle {
            points: [
                Vector2::new(size * 0.5, 0.0),
                Vector2::new(size, size),
                Vector2::new(0.0, size),
            ],
        },
        ArrowDirection::South => Primitive::Triangle {
            points: [
                Vector2::new(0.0, 0.0),
                Vector2::new(size, 0.0),
                Vector2::new(size * 0.5, size),
            ],
        },
        ArrowDirection::West => Primitive::Triangle {
            points: [
                Vector2::new(0.0, 0.0),
                Vector2::new(size, size * 0.5),
                Vector2::new(0.0, size),
            ],
        },
        ArrowDirection::East => Primitive::Triangle {
            points: [
                Vector2::new(0.0, size * 0.5),
                Vector2::new(size, 0.0),
                Vector2::new(size, size),
            ],
        },
    }])
    .build(ctx)
}
