//! Vector image is used to create images, that consists from a fixed set of basic primitives, such as lines,
//! triangles, rectangles, etc. It could be used to create simple images that can be infinitely scaled without
//! aliasing issues. See [`VectorImage`] docs for more info and usage examples.

#![warn(missing_docs)]

use crate::{
    core::{
        algebra::Vector2, color::Color, math::Rect, math::Vector2Ext, pool::Handle,
        reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*,
    },
    draw::{CommandTexture, Draw, DrawingContext},
    message::UiMessage,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UiNode, UserInterface,
};
use fyrox_core::uuid_provider;
use fyrox_core::variable::InheritableVariable;
use std::ops::{Deref, DerefMut};
use strum_macros::{AsRefStr, EnumString, VariantNames};

/// Primitive is a simplest shape, that consists of one or multiple lines of the same thickness.
#[derive(Clone, Debug, PartialEq, Visit, Reflect, AsRefStr, EnumString, VariantNames)]
pub enum Primitive {
    /// Solid triangle primitive.
    Triangle {
        /// Points of the triangle in local coordinates.
        points: [Vector2<f32>; 3],
    },
    /// A line of fixed thickness between two points.  
    Line {
        /// Beginning of the line in local coordinates.
        begin: Vector2<f32>,
        /// End of the line in local coordinates.
        end: Vector2<f32>,
        /// Thickness of the line in absolute units.
        thickness: f32,
    },
    /// Solid circle primitive.
    Circle {
        /// Center of the circle in local coordinates.
        center: Vector2<f32>,
        /// Radius of the circle in absolute units.
        radius: f32,
        /// Amount of segments that is used to approximate the circle using triangles. The higher the value, the smoother the
        /// circle and vice versa.
        segments: usize,
    },
    /// Solid circle primitive.
    WireCircle {
        /// Center of the circle in local coordinates.
        center: Vector2<f32>,
        /// Radius of the circle in absolute units.
        radius: f32,
        /// Thickness of the circle.
        thickness: f32,
        /// Amount of segments that is used to approximate the circle using triangles. The higher the value, the smoother the
        /// circle and vice versa.
        segments: usize,
    },
    /// Wireframe rectangle primitive.
    Rectangle {
        /// Rectangle bounds in local coordinates.
        rect: Rect<f32>,
        /// Thickness of the lines on the rectangle in absolute units.
        thickness: f32,
    },
    /// Solid rectangle primitive.
    RectangleFilled {
        /// Rectangle bounds in local coordinates.
        rect: Rect<f32>,
    },
}

uuid_provider!(Primitive = "766be1b3-6d1c-4466-bcf3-7093017c9e31");

impl Default for Primitive {
    fn default() -> Self {
        Self::Line {
            begin: Default::default(),
            end: Default::default(),
            thickness: 0.0,
        }
    }
}

fn line_thickness_vector(a: Vector2<f32>, b: Vector2<f32>, thickness: f32) -> Vector2<f32> {
    if let Some(dir) = (b - a).try_normalize(f32::EPSILON) {
        Vector2::new(dir.y, -dir.x).scale(thickness * 0.5)
    } else {
        Vector2::default()
    }
}

impl Primitive {
    /// Returns current bounds of the primitive as `min, max` tuple.
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
            Primitive::Circle { radius, center, .. }
            | Primitive::WireCircle { radius, center, .. } => {
                let radius = Vector2::new(*radius, *radius);
                (center - radius, center + radius)
            }
            Primitive::Rectangle { rect, .. } | Primitive::RectangleFilled { rect } => {
                (rect.left_top_corner(), rect.right_bottom_corner())
            }
        }
    }
}

/// Vector image is used to create images, that consists from a fixed set of basic primitives, such as lines,
/// triangles, rectangles, etc. It could be used to create simple images that can be infinitely scaled without
/// aliasing issues.
///
/// ## Examples
///
/// The following example creates a cross shape with given size and thickness:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::{algebra::Vector2, pool::Handle},
/// #     vector_image::{Primitive, VectorImageBuilder},
/// #     widget::WidgetBuilder,
/// #     BuildContext, UiNode, BRUSH_BRIGHT,
/// # };
/// #
/// fn make_cross_vector_image(
///     ctx: &mut BuildContext,
///     size: f32,
///     thickness: f32,
/// ) -> Handle<UiNode> {
///     VectorImageBuilder::new(
///         WidgetBuilder::new()
///             // Color of the image is defined by the foreground brush of the base widget.
///             .with_foreground(BRUSH_BRIGHT),
///     )
///     .with_primitives(vec![
///         Primitive::Line {
///             begin: Vector2::new(0.0, 0.0),
///             end: Vector2::new(size, size),
///             thickness,
///         },
///         Primitive::Line {
///             begin: Vector2::new(size, 0.0),
///             end: Vector2::new(0.0, size),
///             thickness,
///         },
///     ])
///     .build(ctx)
/// }
/// ```
///
/// Keep in mind that all primitives located in local coordinates. The color of the vector image can be changed by
/// setting a new foreground brush.
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct VectorImage {
    /// Base widget of the image.
    pub widget: Widget,
    /// Current set of primitives that will be drawn.
    pub primitives: InheritableVariable<Vec<Primitive>>,
}

crate::define_widget_deref!(VectorImage);

uuid_provider!(VectorImage = "7e535b65-0178-414e-b310-e208afc0eeb5");

impl Control for VectorImage {
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
                } => drawing_context.push_circle_filled(
                    bounds.position + *center,
                    *radius,
                    *segments,
                    Color::WHITE,
                ),
                Primitive::WireCircle {
                    center,
                    radius,
                    thickness,
                    segments,
                } => drawing_context.push_circle(
                    bounds.position + *center,
                    *radius,
                    *segments,
                    *thickness,
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

/// Vector image builder creates [`VectorImage`] instances and adds them to the user interface.
pub struct VectorImageBuilder {
    widget_builder: WidgetBuilder,
    primitives: Vec<Primitive>,
}

impl VectorImageBuilder {
    /// Creates new vector image builder.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            primitives: Default::default(),
        }
    }

    /// Sets the desired set of primitives.
    pub fn with_primitives(mut self, primitives: Vec<Primitive>) -> Self {
        self.primitives = primitives;
        self
    }

    /// Builds the vector image widget.
    pub fn build_node(self) -> UiNode {
        let image = VectorImage {
            widget: self.widget_builder.build(),
            primitives: self.primitives.into(),
        };
        UiNode::new(image)
    }

    /// Finishes vector image building and adds it to the user interface.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        ctx.add_node(self.build_node())
    }
}
