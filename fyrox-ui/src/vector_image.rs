use crate::{
    core::{algebra::Vector2, color::Color, math::Rect, math::Vector2Ext, pool::Handle},
    draw::{CommandTexture, Draw, DrawingContext},
    message::UiMessage,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UiNode, UserInterface,
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

#[derive(Clone, Debug)]
pub enum Primitive {
    Triangle {
        points: [Vector2<f32>; 3],
    },
    Line {
        begin: Vector2<f32>,
        end: Vector2<f32>,
        thickness: f32,
    },
    Circle {
        center: Vector2<f32>,
        radius: f32,
        segments: usize,
    },
    Rectangle {
        rect: Rect<f32>,
        thickness: f32,
    },
    RectangleFilled {
        rect: Rect<f32>,
    },
}

fn line_thickness_vector(a: Vector2<f32>, b: Vector2<f32>, thickness: f32) -> Vector2<f32> {
    if let Some(dir) = (b - a).try_normalize(f32::EPSILON) {
        Vector2::new(dir.y, -dir.x).scale(thickness * 0.5)
    } else {
        Vector2::default()
    }
}

impl Primitive {
    pub fn bounds(&self) -> (Vector2<f32>, Vector2<f32>) {
        match self {
            Primitive::Triangle { points } => {
                let min = points[0]
                    .per_component_min(&points[1])
                    .per_component_min(&points[2]);
                let max = points[0]
                    .per_component_max(&points[1])
                    .per_component_max(&points[2]);
                (min, max)
            }
            Primitive::Line {
                begin,
                end,
                thickness,
            } => {
                let tv = line_thickness_vector(*begin, *end, *thickness);
                let mut min = begin + tv;
                let mut max = min;
                for v in &[begin - tv, end + tv, end - tv] {
                    min = min.per_component_min(v);
                    max = max.per_component_max(v);
                }
                (min, max)
            }
            Primitive::Circle { radius, center, .. } => {
                let radius = Vector2::new(*radius, *radius);
                (center - radius, center + radius)
            }
            Primitive::Rectangle { rect, .. } | Primitive::RectangleFilled { rect } => {
                (rect.left_top_corner(), rect.right_bottom_corner())
            }
        }
    }
}

#[derive(Clone)]
pub struct VectorImage {
    pub widget: Widget,
    pub primitives: Vec<Primitive>,
}

crate::define_widget_deref!(VectorImage);

impl Control for VectorImage {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn measure_override(&self, _ui: &UserInterface, _available_size: Vector2<f32>) -> Vector2<f32> {
        if self.primitives.is_empty() {
            Default::default()
        } else {
            let mut max = Vector2::new(-f32::MAX, -f32::MAX);
            let mut min = Vector2::new(f32::MAX, f32::MAX);

            for primitive in self.primitives.iter() {
                let (pmin, pmax) = primitive.bounds();
                min = min.per_component_min(&pmin);
                max = max.per_component_max(&pmax);
            }

            max - min
        }
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.bounding_rect();

        for primitive in self.primitives.iter() {
            match primitive {
                Primitive::Triangle { points } => {
                    let pts = [
                        bounds.position + points[0],
                        bounds.position + points[1],
                        bounds.position + points[2],
                    ];

                    drawing_context.push_triangle_filled(pts);
                }
                Primitive::Line {
                    begin,
                    end,
                    thickness,
                } => {
                    drawing_context.push_line(
                        bounds.position + *begin,
                        bounds.position + *end,
                        *thickness,
                    );
                }
                Primitive::Circle {
                    center,
                    radius,
                    segments,
                } => drawing_context.push_circle(
                    bounds.position + *center,
                    *radius,
                    *segments,
                    Color::WHITE,
                ),
                Primitive::RectangleFilled { rect } => drawing_context.push_rect_filled(rect, None),
                Primitive::Rectangle { rect, thickness } => {
                    drawing_context.push_rect(rect, *thickness)
                }
            }
        }
        drawing_context.commit(
            self.clip_bounds(),
            self.widget.foreground(),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
    }
}

pub struct VectorImageBuilder {
    widget_builder: WidgetBuilder,
    primitives: Vec<Primitive>,
}

impl VectorImageBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            primitives: Default::default(),
        }
    }

    pub fn with_primitives(mut self, primitives: Vec<Primitive>) -> Self {
        self.primitives = primitives;
        self
    }

    pub fn build_node(self) -> UiNode {
        let image = VectorImage {
            widget: self.widget_builder.build(),
            primitives: self.primitives,
        };
        UiNode::new(image)
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        ctx.add_node(self.build_node())
    }
}
