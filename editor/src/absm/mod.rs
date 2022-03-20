use crate::{
    draw::DrawingContext, BuildContext, Color, UiMessage, UserInterface, VerticalAlignment,
    WidgetBuilder, WidgetMessage,
};
use fyrox::{
    core::{
        algebra::{Matrix3, Point2, Vector2},
        math::{round_to_step, Rect},
        pool::Handle,
    },
    gui::{
        border::BorderBuilder,
        brush::Brush,
        define_constructor, define_widget_deref,
        draw::{CommandTexture, Draw},
        message::{MessageDirection, MouseButton},
        text::TextBuilder,
        widget::Widget,
        window::{WindowBuilder, WindowTitle},
        Control, HorizontalAlignment, Thickness, UiNode,
    },
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

const NORMAL_BACKGROUND: Color = Color::opaque(60, 60, 60);
const SELECTED_BACKGROUND: Color = Color::opaque(80, 80, 80);

pub struct AbsmEditor {
    #[allow(dead_code)] // TODO
    window: Handle<UiNode>,
}

#[derive(Clone)]
pub struct AbsmStateNode {
    widget: Widget,
    background: Handle<UiNode>,
    selected: bool,
}

define_widget_deref!(AbsmStateNode);

#[derive(Debug, Clone, PartialEq)]
pub enum AbsmStateNodeMessage {
    Select(bool),
}

impl AbsmStateNodeMessage {
    define_constructor!(AbsmStateNodeMessage:Select => fn select(bool), layout: false);
}

impl Control for AbsmStateNode {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(WidgetMessage::MouseDown { button, .. }) = message.data() {
            if *button == MouseButton::Left || *button == MouseButton::Right {
                message.set_handled(true);

                ui.send_message(AbsmStateNodeMessage::select(
                    self.handle(),
                    MessageDirection::ToWidget,
                    true,
                ));
            }
        } else if let Some(AbsmStateNodeMessage::Select(state)) = message.data() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
                && self.selected != *state
            {
                self.selected = *state;

                ui.send_message(WidgetMessage::background(
                    self.background,
                    MessageDirection::ToWidget,
                    Brush::Solid(if self.selected {
                        SELECTED_BACKGROUND
                    } else {
                        NORMAL_BACKGROUND
                    }),
                ));
            }
        }
    }
}

pub struct AbsmStateNodeBuilder {
    widget_builder: WidgetBuilder,
}

impl AbsmStateNodeBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let background = BorderBuilder::new(
            WidgetBuilder::new()
                .with_foreground(Brush::Solid(SELECTED_BACKGROUND))
                .with_background(Brush::Solid(NORMAL_BACKGROUND))
                .with_child(
                    TextBuilder::new(
                        WidgetBuilder::new()
                            .with_vertical_alignment(VerticalAlignment::Center)
                            .with_horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .with_text("State")
                    .build(ctx),
                ),
        )
        .with_stroke_thickness(Thickness::uniform(4.0))
        .build(ctx);

        let node = AbsmStateNode {
            widget: self
                .widget_builder
                .with_min_size(Vector2::new(200.0, 100.0))
                .with_child(background)
                .build(),
            background,
            selected: false,
        };

        ctx.add_node(UiNode::new(node))
    }
}

#[derive(Clone)]
pub struct AbsmCanvas {
    widget: Widget,
    #[allow(dead_code)] // TODO
    selection_manager: Vec<Handle<UiNode>>,
    view_position: Vector2<f32>,
    initial_view_position: Vector2<f32>,
    click_position: Vector2<f32>,
    is_dragging: bool,
}

define_widget_deref!(AbsmCanvas);

impl AbsmCanvas {
    pub fn point_to_screen_space(&self, point: Vector2<f32>) -> Vector2<f32> {
        let screen_matrix = Matrix3::new_translation(&self.view_position);

        screen_matrix.transform_point(&Point2::from(point)).coords
    }

    /// Transforms a point to local space.
    pub fn point_to_local_space(&self, point: Vector2<f32>) -> Vector2<f32> {
        let mut p = point - self.screen_position();
        p.y = self.actual_size().y - p.y;

        let screen_matrix = Matrix3::new_translation(&-self.view_position);

        screen_matrix.transform_point(&Point2::from(p)).coords
    }
}

impl Control for AbsmCanvas {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn draw(&self, ctx: &mut DrawingContext) {
        let bounds = self.widget.screen_bounds();
        DrawingContext::push_rect_filled(ctx, &bounds, None);
        ctx.commit(
            self.clip_bounds(),
            self.widget.background(),
            CommandTexture::None,
            None,
        );

        let screen_bounds = self.screen_bounds();

        let step_size = 50.0 / 1.0; // self.zoom.clamp(0.001, 1000.0);

        let mut local_left_bottom = self.point_to_local_space(screen_bounds.left_top_corner());
        local_left_bottom.x = round_to_step(local_left_bottom.x, step_size);
        local_left_bottom.y = round_to_step(local_left_bottom.y, step_size);

        let mut local_right_top = self.point_to_local_space(screen_bounds.right_bottom_corner());
        local_right_top.x = round_to_step(local_right_top.x, step_size);
        local_right_top.y = round_to_step(local_right_top.y, step_size);

        let w = (local_right_top.x - local_left_bottom.x).abs();
        let h = (local_right_top.y - local_left_bottom.y).abs();

        let nw = ((w / step_size).ceil()) as usize;
        let nh = ((h / step_size).ceil()) as usize;

        for ny in 0..=nh {
            let k = ny as f32 / (nh) as f32;
            let y = local_left_bottom.y - k * h;
            ctx.push_line(
                self.point_to_screen_space(Vector2::new(local_left_bottom.x - step_size, y)),
                self.point_to_screen_space(Vector2::new(local_right_top.x + step_size, y)),
                1.0,
            );
        }

        for nx in 0..=nw {
            let k = nx as f32 / (nw) as f32;
            let x = local_left_bottom.x + k * w;
            ctx.push_line(
                self.point_to_screen_space(Vector2::new(x, local_left_bottom.y + step_size)),
                self.point_to_screen_space(Vector2::new(x, local_right_top.y - step_size)),
                1.0,
            );
        }

        // Draw main axes.
        let vb = self.point_to_screen_space(Vector2::new(0.0, -10e6));
        let ve = self.point_to_screen_space(Vector2::new(0.0, 10e6));
        ctx.push_line(vb, ve, 2.0);

        let hb = self.point_to_screen_space(Vector2::new(-10e6, 0.0));
        let he = self.point_to_screen_space(Vector2::new(10e6, 0.0));
        ctx.push_line(hb, he, 2.0);

        ctx.commit(
            screen_bounds,
            Brush::Solid(Color::opaque(100, 100, 100)),
            CommandTexture::None,
            None,
        );
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
            ui.arrange_node(
                child_handle,
                &Rect::new(
                    self.view_position.x + child.desired_local_position().x,
                    self.view_position.y + child.desired_local_position().y,
                    child.desired_size().x,
                    child.desired_size().y,
                ),
            );
        }

        final_size
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(AbsmStateNodeMessage::Select(true)) = message.data() {
            for &child in self.children() {
                if message.destination() == child {
                    continue;
                }

                if ui.node(child).cast::<AbsmStateNode>().is_some() {
                    ui.send_message(AbsmStateNodeMessage::select(
                        child,
                        MessageDirection::ToWidget,
                        false,
                    ));
                }
            }
        } else if let Some(WidgetMessage::MouseDown { pos, button }) = message.data() {
            if *button == MouseButton::Middle {
                self.is_dragging = true;
                self.click_position = *pos;
                self.initial_view_position = self.view_position;
            }
        } else if let Some(WidgetMessage::MouseUp { button, .. }) = message.data() {
            if *button == MouseButton::Middle {
                self.is_dragging = false;
            }
        } else if let Some(WidgetMessage::MouseMove { pos, .. }) = message.data() {
            if self.is_dragging {
                self.view_position = self.initial_view_position + (*pos - self.click_position);
                self.invalidate_arrange();
            }
        }
    }
}

pub struct AbsmCanvasBuilder {
    widget_builder: WidgetBuilder,
}

impl AbsmCanvasBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let canvas = AbsmCanvas {
            widget: self.widget_builder.build(),
            selection_manager: Default::default(),
            view_position: Default::default(),
            initial_view_position: Default::default(),
            click_position: Default::default(),
            is_dragging: false,
        };

        ctx.add_node(UiNode::new(canvas))
    }
}

impl AbsmEditor {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(700.0).with_height(400.0))
            .with_content(
                AbsmCanvasBuilder::new(
                    WidgetBuilder::new()
                        .with_child(AbsmStateNodeBuilder::new(WidgetBuilder::new()).build(ctx))
                        .with_child(
                            AbsmStateNodeBuilder::new(
                                WidgetBuilder::new()
                                    .with_desired_position(Vector2::new(300.0, 200.0)),
                            )
                            .build(ctx),
                        )
                        .with_child(
                            AbsmStateNodeBuilder::new(
                                WidgetBuilder::new()
                                    .with_desired_position(Vector2::new(300.0, 400.0)),
                            )
                            .build(ctx),
                        ),
                )
                .build(ctx),
            )
            .open(false)
            .with_title(WindowTitle::text("ABSM Editor"))
            .build(ctx);

        Self { window }
    }
}
