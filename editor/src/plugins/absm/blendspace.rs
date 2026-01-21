// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::fyrox::{
    core::{
        algebra::Vector2,
        color::Color,
        math::{Rect, TriangleDefinition},
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid_provider,
        visitor::prelude::*,
    },
    generic_animation::machine::{
        node::blendspace::BlendSpacePoint, node::PoseNode, parameter::Parameter,
        parameter::ParameterContainer, Machine, MachineLayer,
    },
    graph::{PrefabData, SceneGraph, SceneGraphNode},
    gui::{
        brush::Brush,
        define_widget_deref,
        draw::{CommandTexture, Draw, DrawingContext},
        grid::{Column, GridBuilder, Row},
        menu::MenuItemMessage,
        message::{CursorIcon, MessageDirection, MouseButton, UiMessage},
        popup::{Placement, PopupBuilder, PopupMessage},
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Control, HorizontalAlignment, RcUiNodeHandle, Thickness, UiNode,
        UserInterface, VerticalAlignment,
    },
};
use crate::plugins::absm::{
    command::blend::{
        AddBlendSpacePointCommand, RemoveBlendSpacePointCommand, SetBlendSpacePointPositionCommand,
    },
    selection::{AbsmSelection, SelectedEntity},
};
use crate::{menu::create_menu_item, message::MessageSender};

use fyrox::gui::menu::{ContextMenuBuilder, MenuItem};
use fyrox::gui::message::MessageData;
use fyrox::gui::style::resource::StyleResourceExt;
use fyrox::gui::style::Style;
use fyrox::gui::text::Text;
use fyrox::gui::window::{Window, WindowAlignment};
use std::{
    cell::Cell,
    fmt::{Debug, Formatter},
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

impl MessageData for BlendSpaceFieldMessage {
    fn need_perform_layout(&self) -> bool {
        matches!(self, Self::Points(_))
    }
}

#[derive(Clone, Visit, Reflect, Debug)]
struct ContextMenu {
    #[visit(skip)]
    #[reflect(hidden)]
    menu: RcUiNodeHandle,
    add_point: Handle<MenuItem>,
    placement_target: Cell<Handle<UiNode>>,
    screen_position: Cell<Vector2<f32>>,
    remove_point: Handle<MenuItem>,
}

#[derive(Clone)]
enum DragContext {
    SamplingPoint,
    Point { point: usize },
}

#[derive(Clone, Visit, Reflect, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
struct BlendSpaceField {
    widget: Widget,
    points: Vec<Handle<BlendSpaceFieldPoint>>,
    min_values: Vector2<f32>,
    max_values: Vector2<f32>,
    snap_step: Vector2<f32>,
    point_positions: Vec<Vector2<f32>>,
    triangles: Vec<TriangleDefinition>,
    grid_brush: Brush,
    sampling_point: Vector2<f32>,
    #[visit(skip)]
    #[reflect(hidden)]
    drag_context: Option<DragContext>,
    field_context_menu: ContextMenu,
}

impl Debug for BlendSpaceField {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "BlendSpaceField")
    }
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
    context_menu: RcUiNodeHandle,
    ctx: &mut BuildContext,
) -> Vec<Handle<BlendSpaceFieldPoint>> {
    points
        .enumerate()
        .map(|(i, p)| {
            BlendSpaceFieldPointBuilder::new(
                WidgetBuilder::new()
                    .with_context_menu(context_menu.clone())
                    .with_background(ctx.style.property(Style::BRUSH_LIGHTEST))
                    .with_foreground(ctx.style.property(Style::BRUSH_TEXT))
                    .with_desired_position(p),
                i,
            )
            .build(ctx)
        })
        .collect()
}

uuid_provider!(BlendSpaceField = "854a7c2d-3ccd-4331-95e1-956a3a035bd0");

impl Control for BlendSpaceField {
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
            &self.material,
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
            &self.material,
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
            self.foreground(),
            CommandTexture::None,
            &self.material,
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
            self.foreground(),
            CommandTexture::None,
            &self.material,
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
                            ui.send(pt, WidgetMessage::Remove);
                        }

                        let point_views = make_points(
                            points.iter().cloned(),
                            self.field_context_menu.menu.clone(),
                            &mut ui.build_ctx(),
                        );

                        for &new_pt in point_views.iter() {
                            ui.send(new_pt, WidgetMessage::LinkWith(self.handle));
                        }

                        self.points = point_views;
                        self.point_positions.clone_from(points);
                        self.invalidate_visual();
                    }
                    BlendSpaceFieldMessage::Triangles(triangles) => {
                        self.triangles.clone_from(triangles);
                        self.invalidate_visual();
                    }
                    BlendSpaceFieldMessage::MinValues(min) => {
                        self.min_values = *min;
                        self.invalidate_visual();
                    }
                    BlendSpaceFieldMessage::MaxValues(max) => {
                        self.max_values = *max;
                        self.invalidate_visual();
                    }
                    BlendSpaceFieldMessage::SnapStep(snap_step) => {
                        self.snap_step = *snap_step;
                        self.invalidate_visual();
                    }
                    BlendSpaceFieldMessage::SamplingPoint(sampling_point) => {
                        if message.is_for(self.handle) {
                            self.sampling_point = *sampling_point;
                            self.invalidate_visual();
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
                            self.points.iter().position(|p| message.destination() == *p)
                        {
                            self.drag_context = Some(DragContext::Point { point: pos });

                            ui.send(self.points[pos], BlendSpaceFieldPointMessage::Select);
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
                                ui.send(
                                    self.handle,
                                    BlendSpaceFieldMessage::MovePoint {
                                        index: point,
                                        position: screen_to_blend(
                                            *pos,
                                            self.min_values,
                                            self.max_values,
                                            self.screen_bounds(),
                                        ),
                                    },
                                );
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
                                ui.send(
                                    self.handle,
                                    BlendSpaceFieldMessage::SamplingPoint(blend_pos),
                                );
                            }
                            DragContext::Point { point } => {
                                ui.send(
                                    self.points[*point],
                                    WidgetMessage::DesiredPosition(blend_pos),
                                );
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
            if message.destination() == self.field_context_menu.menu.handle() {
                self.field_context_menu.placement_target.set(*target);

                ui.send(
                    self.field_context_menu.remove_point,
                    WidgetMessage::Enabled(self.points.contains(&target.to_variant())),
                );

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
                ui.post(self.handle, BlendSpaceFieldMessage::AddPoint(pos));
            } else if message.destination() == self.field_context_menu.remove_point {
                if let Some(pos) = self
                    .points
                    .iter()
                    .position(|p| self.field_context_menu.placement_target.get() == *p)
                {
                    ui.post(self.handle, BlendSpaceFieldMessage::RemovePoint(pos));
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
    pub const ADD_POINT: Uuid = uuid!("4482aa34-35ca-4432-978a-720b4e4375a3");
    pub const REMOVE_POINT: Uuid = uuid!("f84cdb71-090a-463c-919d-b01c1a39ed38");

    fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            min_values: Default::default(),
            max_values: Default::default(),
            snap_step: Default::default(),
        }
    }

    fn build(self, ctx: &mut BuildContext) -> Handle<BlendSpaceField> {
        let add_point;
        let remove_point;
        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
                .with_content(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_child({
                                add_point =
                                    create_menu_item("Add Point", Self::ADD_POINT, vec![], ctx);
                                add_point
                            })
                            .with_child({
                                remove_point = create_menu_item(
                                    "Remove Point",
                                    Self::REMOVE_POINT,
                                    vec![],
                                    ctx,
                                );
                                remove_point
                            }),
                    )
                    .build(ctx),
                )
                .with_restrict_picking(false),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        let field = BlendSpaceField {
            widget: self
                .widget_builder
                .with_clip_to_bounds(false)
                .with_preview_messages(true)
                .with_context_menu(menu.clone())
                .build(ctx),
            points: Default::default(),
            min_values: self.min_values,
            max_values: self.max_values,
            snap_step: self.snap_step,
            point_positions: Default::default(),
            triangles: Default::default(),
            grid_brush: ctx.style.get_or_default(Style::BRUSH_LIGHT),
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

        ctx.add_node(UiNode::new(field)).to_variant()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlendSpaceFieldPointMessage {
    Select,
}
impl MessageData for BlendSpaceFieldPointMessage {}

#[derive(Clone, Visit, Reflect, Debug, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
struct BlendSpaceFieldPoint {
    widget: Widget,
    selected: bool,
}

define_widget_deref!(BlendSpaceFieldPoint);

uuid_provider!(BlendSpaceFieldPoint = "22c215c1-ff23-4a64-9aa7-640b5014a78b");

impl Control for BlendSpaceFieldPoint {
    fn draw(&self, drawing_context: &mut DrawingContext) {
        drawing_context.push_circle_filled(
            Vector2::new(*self.width * 0.5, *self.height * 0.5),
            (*self.width + *self.height) * 0.25,
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
            &self.material,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(BlendSpaceFieldPointMessage::Select) = message.data() {
            if message.destination() == self.handle {
                self.selected = true;
                self.invalidate_visual();
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

    fn build(self, ctx: &mut BuildContext) -> Handle<BlendSpaceFieldPoint> {
        let point = BlendSpaceFieldPoint {
            widget: self
                .widget_builder
                .with_cursor(Some(CursorIcon::Grab))
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
                .build(ctx),
            selected: false,
        };

        ctx.add_node(UiNode::new(point)).to_variant()
    }
}

pub struct BlendSpaceEditor {
    pub window: Handle<Window>,
    min_x: Handle<Text>,
    max_x: Handle<Text>,
    min_y: Handle<Text>,
    max_y: Handle<Text>,
    x_axis_name: Handle<Text>,
    y_axis_name: Handle<Text>,
    field: Handle<BlendSpaceField>,
}

impl BlendSpaceEditor {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let field = BlendSpaceFieldBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .on_column(1)
                .with_margin(Thickness::uniform(15.0))
                .with_foreground(ctx.style.property(Style::BRUSH_LIGHTEST))
                .with_background(ctx.style.property(Style::BRUSH_DARK)),
        )
        .build(ctx);

        let min_x;
        let max_x;
        let min_y;
        let max_y;
        let x_axis_name;
        let y_axis_name;
        let content = GridBuilder::new(
            WidgetBuilder::new().with_child(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .on_row(1)
                        .on_column(0)
                        .with_child(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .with_child({
                                        max_y = TextBuilder::new(
                                            WidgetBuilder::new()
                                                .on_row(0)
                                                .with_margin(Thickness::uniform(1.0))
                                                .with_height(22.0)
                                                .with_vertical_alignment(VerticalAlignment::Top),
                                        )
                                        .build(ctx);
                                        max_y
                                    })
                                    .with_child({
                                        y_axis_name = TextBuilder::new(
                                            WidgetBuilder::new()
                                                .with_height(22.0)
                                                .on_row(1)
                                                .with_margin(Thickness::uniform(1.0))
                                                .with_vertical_alignment(VerticalAlignment::Center),
                                        )
                                        .with_vertical_text_alignment(VerticalAlignment::Center)
                                        .build(ctx);
                                        y_axis_name
                                    })
                                    .with_child({
                                        min_y = TextBuilder::new(
                                            WidgetBuilder::new()
                                                .with_height(22.0)
                                                .on_row(2)
                                                .with_margin(Thickness::uniform(1.0))
                                                .with_vertical_alignment(VerticalAlignment::Bottom),
                                        )
                                        .build(ctx);
                                        min_y
                                    }),
                            )
                            .add_row(Row::stretch())
                            .add_row(Row::stretch())
                            .add_row(Row::stretch())
                            .add_column(Column::strict(24.0))
                            .build(ctx),
                        )
                        .with_child(field)
                        .with_child(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(1)
                                    .on_column(1)
                                    .with_child({
                                        min_x = TextBuilder::new(
                                            WidgetBuilder::new()
                                                .on_column(0)
                                                .with_margin(Thickness::uniform(1.0))
                                                .with_width(50.0)
                                                .with_horizontal_alignment(
                                                    HorizontalAlignment::Left,
                                                ),
                                        )
                                        .build(ctx);
                                        min_x
                                    })
                                    .with_child({
                                        x_axis_name = TextBuilder::new(
                                            WidgetBuilder::new()
                                                .on_column(1)
                                                .with_margin(Thickness::uniform(1.0))
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
                                        max_x = TextBuilder::new(
                                            WidgetBuilder::new()
                                                .on_column(2)
                                                .with_margin(Thickness::uniform(1.0))
                                                .with_width(50.0)
                                                .with_horizontal_alignment(
                                                    HorizontalAlignment::Right,
                                                ),
                                        )
                                        .with_horizontal_text_alignment(HorizontalAlignment::Right)
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
        .add_row(Row::strict(24.0))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(500.0).with_height(400.0))
            .open(false)
            .with_content(content)
            .with_title(WindowTitle::text("Blend Space Editor"))
            .with_tab_label("Blend Space")
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
        ui.send(
            self.window,
            WindowMessage::Open {
                alignment: WindowAlignment::Center,
                modal: true,
                focus_content: true,
            },
        );
    }

    pub fn sync_to_model<P, G, N>(
        &mut self,
        parameters: &ParameterContainer,
        layer: &MachineLayer<Handle<N>>,
        selection: &AbsmSelection<N>,
        ui: &mut UserInterface,
    ) where
        P: PrefabData<Graph = G>,
        G: SceneGraph<Node = N, Prefab = P>,
        N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
    {
        if let Some(SelectedEntity::PoseNode(first)) = selection.entities.first() {
            if let PoseNode::BlendSpace(blend_space) = layer.node(*first) {
                let sync_text = |destination: Handle<Text>, text: String| {
                    ui.send_sync(destination, TextMessage::Text(text));
                };

                sync_text(self.min_x, blend_space.min_values().x.to_string());
                sync_text(self.max_x, blend_space.max_values().x.to_string());
                sync_text(self.min_y, blend_space.min_values().y.to_string());
                sync_text(self.max_y, blend_space.max_values().y.to_string());
                sync_text(self.x_axis_name, blend_space.x_axis_name().to_string());
                sync_text(self.y_axis_name, blend_space.y_axis_name().to_string());

                ui.send_sync(
                    self.field,
                    BlendSpaceFieldMessage::MinValues(blend_space.min_values()),
                );
                ui.send_sync(
                    self.field,
                    BlendSpaceFieldMessage::MaxValues(blend_space.max_values()),
                );
                ui.send_sync(
                    self.field,
                    BlendSpaceFieldMessage::SnapStep(blend_space.snap_step()),
                );
                ui.send_sync(
                    self.field,
                    BlendSpaceFieldMessage::Points(
                        blend_space.points().iter().map(|p| p.position).collect(),
                    ),
                );
                ui.send_sync(
                    self.field,
                    BlendSpaceFieldMessage::Triangles(blend_space.triangles().to_vec()),
                );

                if let Some(Parameter::SamplingPoint(pt)) =
                    parameters.get(blend_space.sampling_parameter())
                {
                    ui.send_sync(self.field, BlendSpaceFieldMessage::SamplingPoint(*pt));
                }
            }
        }
    }

    pub fn handle_ui_message<P, G, N>(
        &mut self,
        selection: &AbsmSelection<N>,
        message: &UiMessage,
        sender: &MessageSender,
        machine: &mut Machine<Handle<N>>,
        is_preview_mode_active: bool,
    ) where
        P: PrefabData<Graph = G>,
        G: SceneGraph<Node = N, Prefab = P>,
        N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
    {
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
                                    sender.do_command(SetBlendSpacePointPositionCommand {
                                        node_handle: selection.absm_node_handle,
                                        handle: *first,
                                        layer_index,
                                        index,
                                        value: position,
                                    });
                                }
                                BlendSpaceFieldMessage::RemovePoint(index) => {
                                    sender.do_command(RemoveBlendSpacePointCommand {
                                        scene_node_handle: selection.absm_node_handle,
                                        node_handle: *first,
                                        layer_index,
                                        point_index: index,
                                        point: None,
                                    })
                                }
                                BlendSpaceFieldMessage::AddPoint(pos) => {
                                    sender.do_command(AddBlendSpacePointCommand {
                                        node_handle: selection.absm_node_handle,
                                        handle: *first,
                                        layer_index,
                                        value: Some(BlendSpacePoint {
                                            position: pos,
                                            pose_source: Default::default(),
                                        }),
                                    })
                                }
                                _ => (),
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::plugins::absm::blendspace::{BlendSpaceFieldBuilder, BlendSpaceFieldPointBuilder};
    use fyrox::{gui::test::test_widget_deletion, gui::widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| {
            BlendSpaceFieldPointBuilder::new(WidgetBuilder::new(), 0).build(ctx)
        });
        test_widget_deletion(|ctx| BlendSpaceFieldBuilder::new(WidgetBuilder::new()).build(ctx));
    }
}
