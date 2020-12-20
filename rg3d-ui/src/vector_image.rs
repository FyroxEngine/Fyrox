use crate::{
    core::{algebra::Vector2, color::Color, math::Vector2Ext, pool::Handle},
    draw::{CommandKind, CommandTexture, DrawingContext},
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
    pub fn size(&self) -> Vector2<f32> {
        match self {
            Primitive::Triangle { points } => {
                let min = points[0]
                    .per_component_min(&points[1])
                    .per_component_min(&points[2]);
                let max = points[0]
                    .per_component_max(&points[1])
                    .per_component_max(&points[2]);
                max - min
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
                max - min
            }
            Primitive::Circle { radius, .. } => {
                let diameter = *radius * 2.0;
                Vector2::new(diameter, diameter)
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
        let mut size: Vector2<f32> = Vector2::default();

        for primitive in self.primitives.iter() {
            size = size.per_component_max(&primitive.size());
        }

        size
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
            CommandKind::Geometry,
            self.widget.foreground(),
            CommandTexture::None,
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
