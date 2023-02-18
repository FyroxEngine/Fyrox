use crate::{
    absm::selection::{AbsmSelection, SelectedEntity},
    send_sync_message,
};
use fyrox::{
    animation::machine::{MachineLayer, PoseNode},
    core::{
        algebra::Vector2,
        color::Color,
        math::{Rect, TriangleDefinition},
        pool::Handle,
    },
    gui::{
        brush::Brush,
        define_constructor, define_widget_deref,
        draw::{CommandTexture, Draw, DrawingContext},
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        numeric::{NumericUpDownBuilder, NumericUpDownMessage},
        stack_panel::StackPanelBuilder,
        text::TextMessage,
        text_box::TextBoxBuilder,
        widget::{Widget, WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Control, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
        VerticalAlignment, BRUSH_DARK, BRUSH_LIGHT, BRUSH_LIGHTEST,
    },
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq)]
pub enum BlendSpaceFieldMessage {
    Points(Vec<Vector2<f32>>),
    Triangles(Vec<TriangleDefinition>),
    MinValues(Vector2<f32>),
    MaxValues(Vector2<f32>),
    SnapStep(Vector2<f32>),
}

impl BlendSpaceFieldMessage {
    define_constructor!(BlendSpaceFieldMessage:Points => fn points(Vec<Vector2<f32>>), layout: true);
    define_constructor!(BlendSpaceFieldMessage:Triangles => fn triangles(Vec<TriangleDefinition>), layout: false);
    define_constructor!(BlendSpaceFieldMessage:MinValues => fn min_values(Vector2<f32>), layout: false);
    define_constructor!(BlendSpaceFieldMessage:MaxValues => fn max_values(Vector2<f32>), layout: false);
    define_constructor!(BlendSpaceFieldMessage:SnapStep => fn snap_step(Vector2<f32>), layout: false);
}

#[derive(Clone)]
struct BlendSpaceField {
    widget: Widget,
    points: Vec<Handle<UiNode>>,
    min_values: Vector2<f32>,
    max_values: Vector2<f32>,
    snap_step: Vector2<f32>,
    point_positions: Vec<Vector2<f32>>,
    triangles: Vec<TriangleDefinition>,
    grid_brush: Brush,
}

define_widget_deref!(BlendSpaceField);

fn blend_to_local(
    p: Vector2<f32>,
    min: Vector2<f32>,
    max: Vector2<f32>,
    bounds: Rect<f32>,
) -> Vector2<f32> {
    let kx = (p.x - min.x) / (max.x - min.x);
    let ky = (p.y - min.y) / (max.y - min.y);
    bounds.position + Vector2::new(kx * bounds.w(), ky * bounds.h())
}

fn make_points<P: Iterator<Item = Vector2<f32>>>(
    points: P,
    ctx: &mut BuildContext,
) -> Vec<Handle<UiNode>> {
    points
        .map(|p| {
            BlendSpaceFieldPointBuilder::new(
                WidgetBuilder::new()
                    .with_foreground(Brush::Solid(Color::WHITE))
                    .with_desired_position(p),
            )
            .build(ctx)
        })
        .collect()
}

impl Control for BlendSpaceField {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn measure_override(&self, ui: &UserInterface, _available_size: Vector2<f32>) -> Vector2<f32> {
        let size_for_child = Vector2::new(f32::INFINITY, f32::INFINITY);

        for child_handle in self.widget.children() {
            ui.measure_node(*child_handle, size_for_child);
        }

        Vector2::default()
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        for &child_handle in self.widget.children() {
            let child = ui.node(child_handle);

            let position = blend_to_local(
                child.desired_local_position(),
                self.min_values,
                self.max_values,
                Rect::new(0.0, 0.0, final_size.x, final_size.y),
            ) - child.desired_size().scale(0.5);

            ui.arrange_node(
                child_handle,
                &Rect::new(
                    position.x,
                    position.y,
                    child.desired_size().x,
                    child.desired_size().y,
                ),
            );
        }

        final_size
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.bounding_rect();

        // Draw background first.
        drawing_context.push_rect_filled(&bounds, None);
        drawing_context.commit(
            self.clip_bounds(),
            self.background(),
            CommandTexture::None,
            None,
        );

        // Draw grid.
        let dvalue = self.max_values - self.min_values;
        let nx = ((dvalue.x / self.snap_step.x) as usize).min(256);
        let ny = ((dvalue.y / self.snap_step.y) as usize).min(256);

        for xs in 0..=nx {
            let x = (xs as f32 / nx as f32) * bounds.w();
            drawing_context.push_line(Vector2::new(x, 0.0), Vector2::new(x, bounds.h()), 1.0);
        }

        for ys in 0..=ny {
            let y = (ys as f32 / ny as f32) * bounds.h();
            drawing_context.push_line(Vector2::new(0.0, y), Vector2::new(bounds.w(), y), 1.0);
        }

        drawing_context.commit(
            self.clip_bounds(),
            self.grid_brush.clone(),
            CommandTexture::None,
            None,
        );

        // Draw triangles.
        for triangle in self.triangles.iter() {
            let a = blend_to_local(
                self.point_positions[triangle[0] as usize],
                self.min_values,
                self.max_values,
                bounds,
            );
            let b = blend_to_local(
                self.point_positions[triangle[1] as usize],
                self.min_values,
                self.max_values,
                bounds,
            );
            let c = blend_to_local(
                self.point_positions[triangle[2] as usize],
                self.min_values,
                self.max_values,
                bounds,
            );

            for (begin, end) in [(a, b), (b, c), (c, a)] {
                drawing_context.push_line(begin, end, 2.0);
            }
        }

        drawing_context.commit(
            self.clip_bounds(),
            self.foreground.clone(),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle {
            if let Some(msg) = message.data::<BlendSpaceFieldMessage>() {
                match msg {
                    BlendSpaceFieldMessage::Points(points) => {
                        for &pt in self.points.iter() {
                            ui.send_message(WidgetMessage::remove(pt, MessageDirection::ToWidget));
                        }

                        let point_views = make_points(points.iter().cloned(), &mut ui.build_ctx());

                        for &new_pt in point_views.iter() {
                            ui.send_message(WidgetMessage::link(
                                new_pt,
                                MessageDirection::ToWidget,
                                self.handle,
                            ));
                        }

                        self.points = point_views;
                        self.point_positions = points.clone();
                    }
                    BlendSpaceFieldMessage::Triangles(triangles) => {
                        self.triangles = triangles.clone();
                    }
                    BlendSpaceFieldMessage::MinValues(min) => {
                        self.min_values = *min;
                    }
                    BlendSpaceFieldMessage::MaxValues(max) => {
                        self.max_values = *max;
                    }
                    BlendSpaceFieldMessage::SnapStep(snap_step) => {
                        self.snap_step = *snap_step;
                    }
                }
            }
        }
    }
}

struct BlendSpaceFieldBuilder {
    widget_builder: WidgetBuilder,
    min_values: Vector2<f32>,
    max_values: Vector2<f32>,
    snap_step: Vector2<f32>,
}

impl BlendSpaceFieldBuilder {
    fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            min_values: Default::default(),
            max_values: Default::default(),
            snap_step: Default::default(),
        }
    }

    fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let field = BlendSpaceField {
            widget: self.widget_builder.build(),
            points: Default::default(),
            min_values: self.min_values,
            max_values: self.max_values,
            snap_step: self.snap_step,
            point_positions: Default::default(),
            triangles: Default::default(),
            grid_brush: BRUSH_LIGHT,
        };

        ctx.add_node(UiNode::new(field))
    }
}

#[derive(Clone)]
struct BlendSpaceFieldPoint {
    widget: Widget,
}

define_widget_deref!(BlendSpaceFieldPoint);

impl Control for BlendSpaceFieldPoint {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        drawing_context.push_circle(
            Vector2::new(self.width * 0.5, self.height * 0.5),
            (self.width + self.height) * 0.25,
            16,
            Color::WHITE,
        );
        drawing_context.commit(
            self.clip_bounds(),
            self.foreground(),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message)
    }
}

struct BlendSpaceFieldPointBuilder {
    widget_builder: WidgetBuilder,
}

impl BlendSpaceFieldPointBuilder {
    fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }

    fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let point = BlendSpaceFieldPoint {
            widget: self
                .widget_builder
                .with_width(10.0)
                .with_height(10.0)
                .build(),
        };

        ctx.add_node(UiNode::new(point))
    }
}

pub struct BlendSpaceEditor {
    pub window: Handle<UiNode>,
    min_x: Handle<UiNode>,
    max_x: Handle<UiNode>,
    min_y: Handle<UiNode>,
    max_y: Handle<UiNode>,
    x_axis_name: Handle<UiNode>,
    y_axis_name: Handle<UiNode>,
    field: Handle<UiNode>,
}

impl BlendSpaceEditor {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let min_x;
        let max_x;
        let min_y;
        let max_y;
        let x_axis_name;
        let y_axis_name;
        let field;
        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    StackPanelBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
                        .with_orientation(Orientation::Horizontal)
                        .build(ctx),
                )
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .on_row(1)
                            .on_column(0)
                            .with_child(
                                GridBuilder::new(
                                    WidgetBuilder::new()
                                        .with_child({
                                            max_y = NumericUpDownBuilder::new(
                                                WidgetBuilder::new()
                                                    .on_row(0)
                                                    .with_height(22.0)
                                                    .with_vertical_alignment(
                                                        VerticalAlignment::Top,
                                                    ),
                                            )
                                            .with_value(0.0f32)
                                            .build(ctx);
                                            max_y
                                        })
                                        .with_child({
                                            y_axis_name = TextBoxBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_height(22.0)
                                                    .on_row(1)
                                                    .with_vertical_alignment(
                                                        VerticalAlignment::Center,
                                                    ),
                                            )
                                            .with_vertical_text_alignment(VerticalAlignment::Center)
                                            .build(ctx);
                                            y_axis_name
                                        })
                                        .with_child({
                                            min_y = NumericUpDownBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_height(22.0)
                                                    .on_row(2)
                                                    .with_vertical_alignment(
                                                        VerticalAlignment::Bottom,
                                                    ),
                                            )
                                            .with_value(0.0f32)
                                            .build(ctx);
                                            min_y
                                        }),
                                )
                                .add_row(Row::stretch())
                                .add_row(Row::stretch())
                                .add_row(Row::stretch())
                                .add_column(Column::strict(50.0))
                                .build(ctx),
                            )
                            .with_child({
                                field = BlendSpaceFieldBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::uniform(1.0))
                                        .on_row(0)
                                        .on_column(1)
                                        .with_foreground(BRUSH_LIGHTEST)
                                        .with_background(BRUSH_DARK),
                                )
                                .build(ctx);
                                field
                            })
                            .with_child(
                                GridBuilder::new(
                                    WidgetBuilder::new()
                                        .on_row(1)
                                        .on_column(1)
                                        .with_child({
                                            min_x = NumericUpDownBuilder::new(
                                                WidgetBuilder::new()
                                                    .on_column(0)
                                                    .with_width(50.0)
                                                    .with_horizontal_alignment(
                                                        HorizontalAlignment::Left,
                                                    ),
                                            )
                                            .with_value(0.0f32)
                                            .build(ctx);
                                            min_x
                                        })
                                        .with_child({
                                            x_axis_name = TextBoxBuilder::new(
                                                WidgetBuilder::new()
                                                    .on_column(1)
                                                    .with_width(50.0)
                                                    .with_horizontal_alignment(
                                                        HorizontalAlignment::Center,
                                                    ),
                                            )
                                            .with_vertical_text_alignment(VerticalAlignment::Center)
                                            .build(ctx);
                                            x_axis_name
                                        })
                                        .with_child({
                                            max_x = NumericUpDownBuilder::new(
                                                WidgetBuilder::new()
                                                    .on_column(2)
                                                    .with_width(50.0)
                                                    .with_horizontal_alignment(
                                                        HorizontalAlignment::Right,
                                                    ),
                                            )
                                            .with_value(0.0f32)
                                            .build(ctx);
                                            max_x
                                        }),
                                )
                                .add_column(Column::stretch())
                                .add_column(Row::stretch())
                                .add_column(Row::stretch())
                                .add_row(Column::strict(22.0))
                                .build(ctx),
                            ),
                    )
                    .add_row(Row::stretch())
                    .add_row(Row::auto())
                    .add_column(Column::auto())
                    .add_column(Column::stretch())
                    .build(ctx),
                ),
        )
        .add_row(Row::strict(22.0))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(400.0).with_height(300.0))
            .open(false)
            .with_content(content)
            .with_title(WindowTitle::text("Blend Space Editor"))
            .build(ctx);

        Self {
            window,
            min_x,
            max_x,
            min_y,
            max_y,
            x_axis_name,
            y_axis_name,
            field,
        }
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));
    }

    pub fn sync_to_model(
        &mut self,
        layer: &MachineLayer,
        selection: &AbsmSelection,
        ui: &mut UserInterface,
    ) {
        if let Some(SelectedEntity::PoseNode(first)) = selection.entities.first() {
            if let PoseNode::BlendSpace(blend_space) = layer.node(*first) {
                send_sync_message(
                    ui,
                    NumericUpDownMessage::value(
                        self.min_x,
                        MessageDirection::ToWidget,
                        blend_space.min_values().x,
                    ),
                );
                send_sync_message(
                    ui,
                    NumericUpDownMessage::value(
                        self.max_x,
                        MessageDirection::ToWidget,
                        blend_space.max_values().x,
                    ),
                );
                send_sync_message(
                    ui,
                    NumericUpDownMessage::value(
                        self.min_y,
                        MessageDirection::ToWidget,
                        blend_space.min_values().y,
                    ),
                );
                send_sync_message(
                    ui,
                    NumericUpDownMessage::value(
                        self.max_y,
                        MessageDirection::ToWidget,
                        blend_space.max_values().y,
                    ),
                );
                send_sync_message(
                    ui,
                    TextMessage::text(
                        self.x_axis_name,
                        MessageDirection::ToWidget,
                        blend_space.x_axis_name().to_string(),
                    ),
                );
                send_sync_message(
                    ui,
                    TextMessage::text(
                        self.y_axis_name,
                        MessageDirection::ToWidget,
                        blend_space.y_axis_name().to_string(),
                    ),
                );

                send_sync_message(
                    ui,
                    BlendSpaceFieldMessage::min_values(
                        self.field,
                        MessageDirection::ToWidget,
                        blend_space.min_values(),
                    ),
                );
                send_sync_message(
                    ui,
                    BlendSpaceFieldMessage::max_values(
                        self.field,
                        MessageDirection::ToWidget,
                        blend_space.max_values(),
                    ),
                );
                send_sync_message(
                    ui,
                    BlendSpaceFieldMessage::snap_step(
                        self.field,
                        MessageDirection::ToWidget,
                        blend_space.snap_step(),
                    ),
                );
                send_sync_message(
                    ui,
                    BlendSpaceFieldMessage::points(
                        self.field,
                        MessageDirection::ToWidget,
                        blend_space.points().iter().map(|p| p.position).collect(),
                    ),
                );
                send_sync_message(
                    ui,
                    BlendSpaceFieldMessage::triangles(
                        self.field,
                        MessageDirection::ToWidget,
                        blend_space.triangles().to_vec(),
                    ),
                );
            }
        }
    }
}
