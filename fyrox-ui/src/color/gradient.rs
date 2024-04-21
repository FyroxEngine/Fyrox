use crate::menu::ContextMenuBuilder;
use crate::{
    brush::Brush,
    color::{ColorFieldBuilder, ColorFieldMessage},
    core::{
        algebra::Vector2,
        color::Color,
        color_gradient::{ColorGradient, GradientPoint},
        math::Rect,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        visitor::prelude::*,
    },
    define_constructor, define_widget_deref,
    draw::{CommandTexture, Draw, DrawingContext},
    grid::{Column, GridBuilder, Row},
    menu::{MenuItemBuilder, MenuItemContent, MenuItemMessage},
    message::{CursorIcon, MessageDirection, MouseButton, UiMessage},
    popup::{Placement, PopupBuilder, PopupMessage},
    stack_panel::StackPanelBuilder,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, RcUiNodeHandle, UiNode, UserInterface,
};
use fyrox_graph::BaseSceneGraph;
use std::{
    cell::Cell,
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

#[derive(Default, Clone, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "50d00eb7-f30b-4973-8a36-03d6b8f007ec")]
pub struct ColorGradientField {
    widget: Widget,
    color_gradient: ColorGradient,
}

define_widget_deref!(ColorGradientField);

const SYNC_FLAG: u64 = 1;

impl Control for ColorGradientField {
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

#[derive(Default, Clone, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "82843d8b-1972-46e6-897c-9619b74059cc")]
pub struct ColorGradientEditor {
    widget: Widget,
    gradient_field: Handle<UiNode>,
    selector_field: Handle<UiNode>,
    points_canvas: Handle<UiNode>,
    context_menu: RcUiNodeHandle,
    point_context_menu: RcUiNodeHandle,
    add_point: Handle<UiNode>,
    remove_point: Handle<UiNode>,
    context_menu_target: Cell<Handle<UiNode>>,
    context_menu_open_position: Cell<Vector2<f32>>,
}

define_widget_deref!(ColorGradientEditor);

impl Control for ColorGradientEditor {
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

                for &point in ui.node(self.points_canvas).children() {
                    ui.send_message(WidgetMessage::remove(point, MessageDirection::ToWidget));
                }

                let points = create_color_points(
                    value,
                    self.point_context_menu.clone(),
                    &mut ui.build_ctx(),
                );

                for point in points {
                    ui.send_message(WidgetMessage::link(
                        point,
                        MessageDirection::ToWidget,
                        self.points_canvas,
                    ));
                }
            }
        }

        if message.direction() == MessageDirection::FromWidget {
            if let Some(ColorPointMessage::Location(_)) = message.data() {
                let gradient = self.fetch_gradient(Handle::NONE, ui);

                ui.send_message(ColorGradientEditorMessage::value(
                    self.handle,
                    MessageDirection::FromWidget,
                    gradient,
                ));
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.add_point {
                let mut gradient = self.fetch_gradient(Handle::NONE, ui);

                let location = (self
                    .screen_to_local(self.context_menu_open_position.get())
                    .x
                    / self.actual_local_size().x)
                    .clamp(0.0, 1.0);

                gradient.add_point(GradientPoint::new(location, Color::WHITE));

                ui.send_message(ColorGradientEditorMessage::value(
                    self.handle,
                    MessageDirection::FromWidget,
                    gradient,
                ));
            } else if message.destination() == self.remove_point
                && ui.try_get(self.context_menu_target.get()).is_some()
            {
                let gradient = self.fetch_gradient(self.context_menu_target.get(), ui);

                ui.send_message(ColorGradientEditorMessage::value(
                    self.handle,
                    MessageDirection::FromWidget,
                    gradient,
                ));
            }
        } else if let Some(ColorFieldMessage::Color(color)) = message.data() {
            if message.destination() == self.selector_field
                && message.direction() == MessageDirection::FromWidget
                && message.flags != SYNC_FLAG
            {
                let mut gradient = ColorGradient::new();

                for (handle, pt) in ui
                    .node(self.points_canvas)
                    .children()
                    .iter()
                    .map(|c| (*c, ui.node(*c).query_component::<ColorPoint>().unwrap()))
                {
                    gradient.add_point(GradientPoint::new(
                        pt.location,
                        if handle == self.context_menu_target.get() {
                            *color
                        } else {
                            pt.color()
                        },
                    ));
                }

                ui.send_message(ColorGradientEditorMessage::value(
                    self.handle,
                    MessageDirection::FromWidget,
                    gradient,
                ));
            }
        } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.context_menu.handle()
                || message.destination() == self.point_context_menu.handle()
            {
                self.context_menu_open_position.set(ui.cursor_position());
                self.context_menu_target.set(*target);

                if message.destination() == self.point_context_menu.handle() {
                    if let Some(point) = ui
                        .try_get(self.context_menu_target.get())
                        .and_then(|n| n.query_component::<ColorPoint>())
                    {
                        let mut msg = ColorFieldMessage::color(
                            self.selector_field,
                            MessageDirection::ToWidget,
                            point.color(),
                        );

                        msg.flags = SYNC_FLAG;

                        ui.send_message(msg)
                    }
                }
            }
        }
    }
}

impl ColorGradientEditor {
    fn fetch_gradient(&self, exclude: Handle<UiNode>, ui: &UserInterface) -> ColorGradient {
        let mut gradient = ColorGradient::new();

        for pt in ui
            .node(self.points_canvas)
            .children()
            .iter()
            .filter(|c| **c != exclude)
            .map(|c| ui.node(*c).query_component::<ColorPoint>().unwrap())
        {
            gradient.add_point(GradientPoint::new(pt.location, pt.color()));
        }

        gradient
    }
}

pub struct ColorGradientEditorBuilder {
    widget_builder: WidgetBuilder,
    color_gradient: ColorGradient,
}

fn create_color_points(
    color_gradient: &ColorGradient,
    point_context_menu: RcUiNodeHandle,
    ctx: &mut BuildContext,
) -> Vec<Handle<UiNode>> {
    color_gradient
        .points()
        .iter()
        .map(|pt| {
            ColorPointBuilder::new(
                WidgetBuilder::new()
                    .with_context_menu(point_context_menu.clone())
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
        let add_point;
        let context_menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new()).with_content(
                StackPanelBuilder::new(WidgetBuilder::new().with_child({
                    add_point = MenuItemBuilder::new(WidgetBuilder::new())
                        .with_content(MenuItemContent::text("Add Point"))
                        .build(ctx);
                    add_point
                }))
                .build(ctx),
            ),
        )
        .build(ctx);
        let context_menu = RcUiNodeHandle::new(context_menu, ctx.sender());

        let selector_field;
        let remove_point;
        let point_context_menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_width(200.0)).with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            remove_point = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text("Remove Point"))
                                .build(ctx);
                            remove_point
                        })
                        .with_child({
                            selector_field =
                                ColorFieldBuilder::new(WidgetBuilder::new().with_height(18.0))
                                    .build(ctx);
                            selector_field
                        }),
                )
                .build(ctx),
            ),
        )
        .build(ctx);
        let point_context_menu = RcUiNodeHandle::new(point_context_menu, ctx.sender());

        let points_canvas = ColorPointsCanvasBuilder::new(
            WidgetBuilder::new()
                .with_height(10.0)
                .on_row(0)
                .on_column(0)
                .with_children(create_color_points(
                    &self.color_gradient,
                    point_context_menu.clone(),
                    ctx,
                )),
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

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(points_canvas)
                .with_child(gradient_field),
        )
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let editor = ColorGradientEditor {
            widget: self
                .widget_builder
                .with_preview_messages(true)
                .with_context_menu(context_menu.clone())
                .with_child(grid)
                .build(),
            points_canvas,
            gradient_field,
            selector_field,
            context_menu,
            point_context_menu,
            add_point,
            remove_point,
            context_menu_target: Cell::new(Default::default()),
            context_menu_open_position: Cell::new(Default::default()),
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

#[derive(Default, Clone, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "a493a603-3451-4005-8c80-559707729e70")]
pub struct ColorPoint {
    pub widget: Widget,
    pub location: f32,
    pub dragging: bool,
}

define_widget_deref!(ColorPoint);

impl Control for ColorPoint {
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

impl ColorPoint {
    fn color(&self) -> Color {
        if let Brush::Solid(color) = *self.foreground {
            color
        } else {
            unreachable!()
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

#[derive(Clone, Visit, Reflect, Debug, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "2608955a-4095-4fd1-af71-99bcdf2600f0")]
struct ColorPointsCanvas {
    widget: Widget,
}

define_widget_deref!(ColorPointsCanvas);

impl Control for ColorPointsCanvas {
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
