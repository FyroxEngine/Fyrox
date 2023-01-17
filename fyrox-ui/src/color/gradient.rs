use crate::{
    brush::Brush,
    color::ColorFieldBuilder,
    core::{
        algebra::Vector2,
        color::Color,
        color_gradient::{ColorGradient, GradientPoint},
        math::Rect,
        pool::Handle,
    },
    define_constructor, define_widget_deref,
    draw::{CommandTexture, Draw, DrawingContext},
    grid::{Column, GridBuilder, Row},
    message::{CursorIcon, MessageDirection, MouseButton, UiMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, UiNode, UserInterface,
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq)]
pub enum ColorGradientEditorMessage {
    /// Sets new color gradient.
    Value(ColorGradient),
}

impl ColorGradientEditorMessage {
    define_constructor!(ColorGradientEditorMessage:Value => fn value(ColorGradient), layout: false);
}

#[derive(Clone)]
pub struct ColorGradientField {
    widget: Widget,
    color_gradient: ColorGradient,
}

define_widget_deref!(ColorGradientField);

impl Control for ColorGradientField {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        // Draw checkerboard background.
        super::draw_checker_board(
            self.bounding_rect(),
            self.clip_bounds(),
            6.0,
            drawing_context,
        );

        let size = self.bounding_rect().size;

        if self.color_gradient.points().is_empty() {
            drawing_context.push_rect_filled(&self.bounding_rect(), None);

            drawing_context.commit(
                self.clip_bounds(),
                Brush::Solid(ColorGradient::STUB_COLOR),
                CommandTexture::None,
                None,
            );
        } else {
            let first = self.color_gradient.points().first().unwrap();
            drawing_context.push_rect_multicolor(
                &Rect::new(0.0, 0.0, first.location() * size.x, size.y),
                [first.color(), first.color(), first.color(), first.color()],
            );

            for pair in self.color_gradient.points().windows(2) {
                let left = &pair[0];
                let right = &pair[1];

                let left_pos = left.location() * size.x;
                let right_pos = right.location() * size.x;
                let bounds = Rect::new(left_pos, 0.0, right_pos - left_pos, size.y);

                drawing_context.push_rect_multicolor(
                    &bounds,
                    [left.color(), right.color(), right.color(), left.color()],
                );
            }

            let last = self.color_gradient.points().last().unwrap();
            let last_pos = last.location() * size.x;
            drawing_context.push_rect_multicolor(
                &Rect::new(last_pos, 0.0, size.x - last_pos, size.y),
                [last.color(), last.color(), last.color(), last.color()],
            );

            drawing_context.commit(
                self.clip_bounds(),
                Brush::Solid(Color::WHITE),
                CommandTexture::None,
                None,
            );
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle && message.direction() == MessageDirection::ToWidget
        {
            if let Some(ColorGradientEditorMessage::Value(value)) = message.data() {
                self.color_gradient = value.clone();
            }
        }
    }
}

pub struct ColorGradientFieldBuilder {
    widget_builder: WidgetBuilder,
    color_gradient: ColorGradient,
}

impl ColorGradientFieldBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            color_gradient: Default::default(),
        }
    }

    pub fn with_color_gradient(mut self, gradient: ColorGradient) -> Self {
        self.color_gradient = gradient;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let field = ColorGradientField {
            widget: self.widget_builder.build(),
            color_gradient: self.color_gradient,
        };

        ctx.add_node(UiNode::new(field))
    }
}

#[derive(Clone)]
pub struct ColorGradientEditor {
    widget: Widget,
    gradient_field: Handle<UiNode>,
    selector_field: Handle<UiNode>,
    points_canvas: Handle<UiNode>,
}

define_widget_deref!(ColorGradientEditor);

impl Control for ColorGradientEditor {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle && message.direction() == MessageDirection::ToWidget
        {
            if let Some(ColorGradientEditorMessage::Value(value)) = message.data() {
                // Re-cast to inner field.
                ui.send_message(ColorGradientEditorMessage::value(
                    self.gradient_field,
                    MessageDirection::ToWidget,
                    value.clone(),
                ));
            }
        }

        if message.direction() == MessageDirection::FromWidget {
            if let Some(ColorPointMessage::Location(_)) = message.data() {
                let mut gradient = ColorGradient::new();

                for pt in ui
                    .node(self.points_canvas)
                    .children()
                    .iter()
                    .map(|c| ui.node(*c).query_component::<ColorPoint>().unwrap())
                {
                    gradient.add_point(GradientPoint::new(
                        pt.location,
                        if let Brush::Solid(color) = pt.foreground {
                            color
                        } else {
                            unreachable!()
                        },
                    ));
                }

                ui.send_message(ColorGradientEditorMessage::value(
                    self.handle,
                    MessageDirection::FromWidget,
                    gradient,
                ));
            }
        }
    }
}

pub struct ColorGradientEditorBuilder {
    widget_builder: WidgetBuilder,
    color_gradient: ColorGradient,
}

fn create_color_points(
    color_gradient: &ColorGradient,
    ctx: &mut BuildContext,
) -> Vec<Handle<UiNode>> {
    color_gradient
        .points()
        .iter()
        .map(|pt| {
            ColorPointBuilder::new(
                WidgetBuilder::new()
                    .with_cursor(Some(CursorIcon::EwResize))
                    .with_width(6.0)
                    .with_foreground(Brush::Solid(pt.color())),
            )
            .with_location(pt.location())
            .build(ctx)
        })
        .collect::<Vec<_>>()
}

impl ColorGradientEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            color_gradient: Default::default(),
        }
    }

    pub fn with_color_gradient(mut self, gradient: ColorGradient) -> Self {
        self.color_gradient = gradient;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let points_canvas = ColorPointsCanvasBuilder::new(
            WidgetBuilder::new()
                .with_height(10.0)
                .with_children(create_color_points(&self.color_gradient, ctx)),
        )
        .build(ctx);

        let gradient_field = ColorGradientFieldBuilder::new(
            WidgetBuilder::new()
                .with_height(20.0)
                .on_row(1)
                .on_column(0),
        )
        .with_color_gradient(self.color_gradient)
        .build(ctx);

        let selector_field = ColorFieldBuilder::new(
            WidgetBuilder::new()
                .on_row(2)
                .on_column(0)
                .with_height(18.0),
        )
        .build(ctx);

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(points_canvas)
                .with_child(selector_field)
                .with_child(gradient_field),
        )
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .build(ctx);

        let editor = ColorGradientEditor {
            widget: self.widget_builder.with_child(grid).build(),
            points_canvas,
            gradient_field,
            selector_field,
        };

        ctx.add_node(UiNode::new(editor))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColorPointMessage {
    Location(f32),
}

impl ColorPointMessage {
    define_constructor!(ColorPointMessage:Location => fn location(f32), layout: false);
}

#[derive(Clone)]
struct ColorPoint {
    widget: Widget,
    location: f32,
    dragging: bool,
}

define_widget_deref!(ColorPoint);

impl Control for ColorPoint {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn draw(&self, ctx: &mut DrawingContext) {
        let size = self.bounding_rect().size;

        ctx.push_triangle_filled([
            Vector2::new(0.0, 0.0),
            Vector2::new(size.x, 0.0),
            Vector2::new(size.x * 0.5, size.y),
        ]);

        ctx.commit(
            self.clip_bounds(),
            self.foreground(),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle {
            if message.direction() == MessageDirection::ToWidget {
                if let Some(msg) = message.data::<ColorPointMessage>() {
                    match msg {
                        ColorPointMessage::Location(location) => {
                            if *location != self.location {
                                self.location = *location;
                                self.invalidate_layout();
                                ui.send_message(message.reverse());
                            }
                        }
                    }
                }
            }

            if message.direction() == MessageDirection::FromWidget {
                if let Some(msg) = message.data::<WidgetMessage>() {
                    match msg {
                        WidgetMessage::MouseDown { button, .. } => {
                            if *button == MouseButton::Left {
                                ui.capture_mouse(self.handle);

                                self.dragging = true;
                            }
                        }
                        WidgetMessage::MouseUp { button, .. } => {
                            if *button == MouseButton::Left {
                                ui.release_mouse_capture();

                                self.dragging = false;

                                ui.send_message(ColorPointMessage::location(
                                    self.handle,
                                    MessageDirection::FromWidget,
                                    self.location,
                                ));
                            }
                        }
                        WidgetMessage::MouseMove { pos, .. } => {
                            if self.dragging {
                                let parent_canvas = ui.node(self.parent);

                                let cursor_x_local_to_parent =
                                    parent_canvas.screen_to_local(*pos).x;

                                self.location = (cursor_x_local_to_parent
                                    / parent_canvas.actual_local_size().x)
                                    .clamp(0.0, 1.0);

                                self.invalidate_layout();
                            }
                        }
                        _ => (),
                    }
                }
            }
        }
    }
}

struct ColorPointBuilder {
    widget_builder: WidgetBuilder,
    location: f32,
}

impl ColorPointBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            location: 0.0,
        }
    }

    pub fn with_location(mut self, location: f32) -> Self {
        self.location = location;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        ctx.add_node(UiNode::new(ColorPoint {
            widget: self.widget_builder.build(),
            location: self.location,
            dragging: false,
        }))
    }
}

#[derive(Clone)]
struct ColorPointsCanvas {
    widget: Widget,
}

define_widget_deref!(ColorPointsCanvas);

impl Control for ColorPointsCanvas {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        for &child in self.children() {
            let child_ref = ui.node(child);
            if let Some(color_point) = child_ref.query_component::<ColorPoint>() {
                let x_pos = final_size.x * color_point.location - child_ref.desired_size().x * 0.5;

                ui.arrange_node(
                    child,
                    &Rect::new(x_pos, 0.0, child_ref.desired_size().x, final_size.y),
                );
            }
        }

        final_size
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message)
    }
}

pub struct ColorPointsCanvasBuilder {
    widget_builder: WidgetBuilder,
}

impl ColorPointsCanvasBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        ctx.add_node(UiNode::new(ColorPointsCanvas {
            widget: self.widget_builder.build(),
        }))
    }
}
