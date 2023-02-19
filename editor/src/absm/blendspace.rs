use crate::{
    absm::{
        command::blend::{
            AddBlendSpacePointCommand, RemoveBlendSpacePointCommand,
            SetBlendSpacePointPositionCommand,
        },
        selection::{AbsmSelection, SelectedEntity},
    },
    menu::create_menu_item,
    send_sync_message, Message,
};
use fyrox::{
    animation::machine::{
        node::blendspace::BlendSpacePoint, Machine, MachineLayer, Parameter, ParameterContainer,
        PoseNode,
    },
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
        menu::MenuItemMessage,
        message::{MessageDirection, MouseButton, UiMessage},
        popup::{Placement, PopupBuilder, PopupMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::{Widget, WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Control, Thickness, UiNode, UserInterface, BRUSH_DARK, BRUSH_LIGHT,
        BRUSH_LIGHTEST,
    },
};
use std::{
    any::{Any, TypeId},
    cell::Cell,
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

#[derive(Debug, Clone, PartialEq)]
pub enum BlendSpaceFieldMessage {
    Points(Vec<Vector2<f32>>),
    Triangles(Vec<TriangleDefinition>),
    MinValues(Vector2<f32>),
    MaxValues(Vector2<f32>),
    SnapStep(Vector2<f32>),
    SamplingPoint(Vector2<f32>),
    MovePoint {
        index: usize,
        position: Vector2<f32>,
    },
    AddPoint(Vector2<f32>),
    RemovePoint(usize),
}

impl BlendSpaceFieldMessage {
    define_constructor!(BlendSpaceFieldMessage:Points => fn points(Vec<Vector2<f32>>), layout: true);
    define_constructor!(BlendSpaceFieldMessage:Triangles => fn triangles(Vec<TriangleDefinition>), layout: false);
    define_constructor!(BlendSpaceFieldMessage:MinValues => fn min_values(Vector2<f32>), layout: false);
    define_constructor!(BlendSpaceFieldMessage:MaxValues => fn max_values(Vector2<f32>), layout: false);
    define_constructor!(BlendSpaceFieldMessage:SnapStep => fn snap_step(Vector2<f32>), layout: false);
    define_constructor!(BlendSpaceFieldMessage:SamplingPoint => fn sampling_point(Vector2<f32>), layout: false);
    define_constructor!(BlendSpaceFieldMessage:MovePoint  => fn move_point(index: usize, position: Vector2<f32>), layout: false);
    define_constructor!(BlendSpaceFieldMessage:AddPoint  => fn add_point(Vector2<f32>), layout: false);
    define_constructor!(BlendSpaceFieldMessage:RemovePoint  => fn remove_point(usize), layout: false);
}

#[derive(Clone)]
struct ContextMenu {
    menu: Handle<UiNode>,
    add_point: Handle<UiNode>,
    placement_target: Cell<Handle<UiNode>>,
    screen_position: Cell<Vector2<f32>>,
    remove_point: Handle<UiNode>,
}

#[derive(Clone)]
enum DragContext {
    SamplingPoint,
    Point { point: usize },
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
    sampling_point: Vector2<f32>,
    drag_context: Option<DragContext>,
    field_context_menu: ContextMenu,
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
    bounds.position + Vector2::new(kx * bounds.w(), bounds.h() - ky * bounds.h())
}

fn screen_to_blend(
    pos: Vector2<f32>,
    min: Vector2<f32>,
    max: Vector2<f32>,
    screen_bounds: Rect<f32>,
) -> Vector2<f32> {
    let rel = pos - screen_bounds.position;
    let kx = (rel.x / screen_bounds.w()).clamp(0.0, 1.0);
    let ky = 1.0 - (rel.y / screen_bounds.h()).clamp(0.0, 1.0);
    let bx = min.x + kx * (max.x - min.x);
    let by = min.y + ky * (max.y - min.y);
    Vector2::new(bx, by)
}

fn make_points<P: Iterator<Item = Vector2<f32>>>(
    points: P,
    context_menu: Handle<UiNode>,
    ctx: &mut BuildContext,
) -> Vec<Handle<UiNode>> {
    points
        .enumerate()
        .map(|(i, p)| {
            BlendSpaceFieldPointBuilder::new(
                WidgetBuilder::new()
                    .with_context_menu(context_menu)
                    .with_background(BRUSH_LIGHTEST)
                    .with_foreground(Brush::Solid(Color::WHITE))
                    .with_desired_position(p),
                i,
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

        // Draw sampling crosshair.
        let size = 14.0;
        let sampling_point = blend_to_local(
            self.sampling_point,
            self.min_values,
            self.max_values,
            bounds,
        );
        drawing_context.push_line(
            Vector2::new(sampling_point.x - size * 0.5, sampling_point.y),
            Vector2::new(sampling_point.x + size * 0.5, sampling_point.y),
            2.0,
        );
        drawing_context.push_line(
            Vector2::new(sampling_point.x, sampling_point.y - size * 0.5),
            Vector2::new(sampling_point.x, sampling_point.y + size * 0.5),
            2.0,
        );
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

                        let point_views = make_points(
                            points.iter().cloned(),
                            self.field_context_menu.menu,
                            &mut ui.build_ctx(),
                        );

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
                    BlendSpaceFieldMessage::SamplingPoint(sampling_point) => {
                        if message.direction == MessageDirection::ToWidget {
                            self.sampling_point = *sampling_point;
                            ui.send_message(message.reverse());
                        }
                    }
                    BlendSpaceFieldMessage::MovePoint { .. }
                    | BlendSpaceFieldMessage::AddPoint(_)
                    | BlendSpaceFieldMessage::RemovePoint(_) => {
                        // Do nothing
                    }
                }
            }
        }

        if let Some(msg) = message.data::<WidgetMessage>() {
            match msg {
                WidgetMessage::MouseDown { button, .. } => {
                    if *button == MouseButton::Left {
                        if let Some(pos) =
                            self.points.iter().position(|p| *p == message.destination())
                        {
                            self.drag_context = Some(DragContext::Point { point: pos });

                            ui.send_message(BlendSpaceFieldPointMessage::select(
                                self.points[pos],
                                MessageDirection::ToWidget,
                            ));
                        } else {
                            self.drag_context = Some(DragContext::SamplingPoint);
                        }

                        ui.capture_mouse(self.handle);
                    }
                }
                WidgetMessage::MouseUp { button, pos, .. } => {
                    if let Some(drag_context) = self.drag_context.take() {
                        if *button == MouseButton::Left {
                            if let DragContext::Point { point } = drag_context {
                                ui.send_message(BlendSpaceFieldMessage::move_point(
                                    self.handle,
                                    MessageDirection::ToWidget,
                                    point,
                                    screen_to_blend(
                                        *pos,
                                        self.min_values,
                                        self.max_values,
                                        self.screen_bounds(),
                                    ),
                                ));
                            }

                            ui.release_mouse_capture();
                        }
                    }
                }
                WidgetMessage::MouseMove { pos, .. } => {
                    if let Some(drag_context) = self.drag_context.as_ref() {
                        let blend_pos = screen_to_blend(
                            *pos,
                            self.min_values,
                            self.max_values,
                            self.screen_bounds(),
                        );
                        match drag_context {
                            DragContext::SamplingPoint => {
                                ui.send_message(BlendSpaceFieldMessage::sampling_point(
                                    self.handle,
                                    MessageDirection::ToWidget,
                                    blend_pos,
                                ));
                            }
                            DragContext::Point { point } => {
                                ui.send_message(WidgetMessage::desired_position(
                                    self.points[*point],
                                    MessageDirection::ToWidget,
                                    blend_pos,
                                ));
                            }
                        }
                    }
                }
                _ => (),
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.field_context_menu.menu {
                self.field_context_menu.placement_target.set(*target);

                ui.send_message(WidgetMessage::enabled(
                    self.field_context_menu.remove_point,
                    MessageDirection::ToWidget,
                    self.points.contains(target),
                ));

                self.field_context_menu
                    .screen_position
                    .set(ui.cursor_position());
            }
        } else if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.field_context_menu.add_point {
                let pos = screen_to_blend(
                    self.field_context_menu.screen_position.get(),
                    self.min_values,
                    self.max_values,
                    self.screen_bounds(),
                );
                ui.send_message(BlendSpaceFieldMessage::add_point(
                    self.handle,
                    MessageDirection::FromWidget,
                    pos,
                ));
            } else if message.destination() == self.field_context_menu.remove_point {
                if let Some(pos) = self
                    .points
                    .iter()
                    .position(|p| *p == self.field_context_menu.placement_target.get())
                {
                    ui.send_message(BlendSpaceFieldMessage::remove_point(
                        self.handle,
                        MessageDirection::FromWidget,
                        pos,
                    ));
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
        let add_point;
        let remove_point;
        let menu = PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
            .with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            add_point = create_menu_item("Add Point", vec![], ctx);
                            add_point
                        })
                        .with_child({
                            remove_point = create_menu_item("Remove Point", vec![], ctx);
                            remove_point
                        }),
                )
                .build(ctx),
            )
            .build(ctx);

        let field = BlendSpaceField {
            widget: self
                .widget_builder
                .with_clip_to_bounds(false)
                .with_preview_messages(true)
                .with_context_menu(menu)
                .build(),
            points: Default::default(),
            min_values: self.min_values,
            max_values: self.max_values,
            snap_step: self.snap_step,
            point_positions: Default::default(),
            triangles: Default::default(),
            grid_brush: BRUSH_LIGHT,
            sampling_point: Vector2::new(0.25, 0.5),
            drag_context: None,
            field_context_menu: ContextMenu {
                menu,
                add_point,
                placement_target: Default::default(),
                screen_position: Default::default(),
                remove_point,
            },
        };

        ctx.add_node(UiNode::new(field))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlendSpaceFieldPointMessage {
    Select,
}

impl BlendSpaceFieldPointMessage {
    define_constructor!(BlendSpaceFieldPointMessage:Select => fn select(), layout: false);
}

#[derive(Clone)]
struct BlendSpaceFieldPoint {
    widget: Widget,
    selected: bool,
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
            if self.selected {
                self.foreground()
            } else {
                self.background()
            },
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(BlendSpaceFieldPointMessage::Select) = message.data() {
            if message.destination() == self.handle {
                self.selected = true;
            }
        }
    }
}

struct BlendSpaceFieldPointBuilder {
    widget_builder: WidgetBuilder,
    index: usize,
}

impl BlendSpaceFieldPointBuilder {
    fn new(widget_builder: WidgetBuilder, index: usize) -> Self {
        Self {
            widget_builder,
            index,
        }
    }

    fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let point = BlendSpaceFieldPoint {
            widget: self
                .widget_builder
                .with_clip_to_bounds(false)
                .with_child(
                    TextBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::left(10.0))
                            .with_clip_to_bounds(false),
                    )
                    .with_text(format!("{:?}", self.index))
                    .build(ctx),
                )
                .with_width(10.0)
                .with_height(10.0)
                .build(),
            selected: false,
        };

        ctx.add_node(UiNode::new(point))
    }
}

pub struct BlendSpaceEditor {
    pub window: Handle<UiNode>,
    field: Handle<UiNode>,
}

impl BlendSpaceEditor {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let field = BlendSpaceFieldBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(15.0))
                .with_foreground(BRUSH_LIGHTEST)
                .with_background(BRUSH_DARK),
        )
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(500.0).with_height(400.0))
            .open(false)
            .with_content(field)
            .with_title(WindowTitle::text("Blend Space Editor"))
            .build(ctx);

        Self { window, field }
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
        parameters: &ParameterContainer,
        layer: &MachineLayer,
        selection: &AbsmSelection,
        ui: &mut UserInterface,
    ) {
        if let Some(SelectedEntity::PoseNode(first)) = selection.entities.first() {
            if let PoseNode::BlendSpace(blend_space) = layer.node(*first) {
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

                if let Some(Parameter::SamplingPoint(pt)) =
                    parameters.get(blend_space.sampling_parameter())
                {
                    send_sync_message(
                        ui,
                        BlendSpaceFieldMessage::sampling_point(
                            self.field,
                            MessageDirection::ToWidget,
                            *pt,
                        ),
                    );
                }
            }
        }
    }

    pub fn handle_ui_message(
        &mut self,
        selection: &AbsmSelection,
        message: &UiMessage,
        sender: &Sender<Message>,
        machine: &mut Machine,
        is_preview_mode_active: bool,
    ) {
        if let Some(SelectedEntity::PoseNode(first)) = selection.entities.first() {
            if let Some(layer_index) = selection.layer {
                if let PoseNode::BlendSpace(blend_space) =
                    machine.layers()[layer_index].node(*first)
                {
                    if message.destination() == self.field {
                        if let Some(msg) = message.data::<BlendSpaceFieldMessage>() {
                            match *msg {
                                BlendSpaceFieldMessage::SamplingPoint(point) => {
                                    if is_preview_mode_active
                                        && message.direction() == MessageDirection::FromWidget
                                    {
                                        let param = blend_space.sampling_parameter().to_string();
                                        if let Some(Parameter::SamplingPoint(param)) =
                                            machine.parameters_mut().get_mut(&param)
                                        {
                                            *param = point;
                                        }
                                    }
                                }
                                BlendSpaceFieldMessage::MovePoint { index, position } => {
                                    sender
                                        .send(Message::do_scene_command(
                                            SetBlendSpacePointPositionCommand {
                                                node_handle: selection.absm_node_handle,
                                                handle: *first,
                                                layer_index,
                                                index,
                                                value: position,
                                            },
                                        ))
                                        .unwrap();
                                }
                                BlendSpaceFieldMessage::RemovePoint(index) => sender
                                    .send(Message::do_scene_command(RemoveBlendSpacePointCommand {
                                        scene_node_handle: selection.absm_node_handle,
                                        node_handle: *first,
                                        layer_index,
                                        point_index: index,
                                        point: None,
                                    }))
                                    .unwrap(),
                                BlendSpaceFieldMessage::AddPoint(pos) => sender
                                    .send(Message::do_scene_command(AddBlendSpacePointCommand {
                                        node_handle: selection.absm_node_handle,
                                        handle: *first,
                                        layer_index,
                                        value: Some(BlendSpacePoint {
                                            position: pos,
                                            pose_source: Default::default(),
                                        }),
                                    }))
                                    .unwrap(),
                                _ => (),
                            }
                        }
                    }
                }
            }
        }
    }
}
