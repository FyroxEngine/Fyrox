use crate::draw::Draw;
use crate::{
    core::{algebra::Vector2, color::Color, math::Vector2Ext, pool::Handle},
    draw::{CommandTexture, DrawingContext},
    message::{MessageData, UiMessage},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UINode, UserInterface,
};
use std::ops::{Deref, DerefMut};

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
}

fn line_thickness_vector(a: Vector2<f32>, b: Vector2<f32>, thickness: f32) -> Vector2<f32> {
    if let Some(dir) = (b - a).try_normalize(std::f32::EPSILON) {
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
        }
    }
}

#[derive(Clone)]
pub struct VectorImage<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    primitives: Vec<Primitive>,
}

crate::define_widget_deref!(VectorImage<M, C>);

impl<M: MessageData, C: Control<M, C>> Control<M, C> for VectorImage<M, C> {
    fn measure_override(
        &self,
        _ui: &UserInterface<M, C>,
        _available_size: Vector2<f32>,
    ) -> Vector2<f32> {
        if self.primitives.is_empty() {
            Default::default()
        } else {
            let mut max = Vector2::new(-std::f32::MAX, -std::f32::MAX);
            let mut min = Vector2::new(std::f32::MAX, std::f32::MAX);

            for primitive in self.primitives.iter() {
                let (pmin, pmax) = primitive.bounds();
                min = min.per_component_min(&pmin);
                max = max.per_component_max(&pmax);
            }

            max - min
        }
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.screen_bounds();

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
            }
        }
        drawing_context.commit(
            self.clip_bounds(),
            self.widget.foreground(),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);
    }
}

pub struct VectorImageBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    primitives: Vec<Primitive>,
}

impl<M: MessageData, C: Control<M, C>> VectorImageBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            primitives: Default::default(),
        }
    }

    pub fn with_primitives(mut self, primitives: Vec<Primitive>) -> Self {
        self.primitives = primitives;
        self
    }

    pub fn build_node(self) -> UINode<M, C> {
        let image = VectorImage {
            widget: self.widget_builder.build(),
            primitives: self.primitives,
        };
        UINode::VectorImage(image)
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        ctx.add_node(self.build_node())
    }
}
