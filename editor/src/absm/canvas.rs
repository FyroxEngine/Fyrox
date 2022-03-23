use crate::absm::node::{AbsmStateNode, AbsmStateNodeMessage};
use fyrox::core::algebra::Matrix3;
use fyrox::{
    core::{
        algebra::Vector2,
        color::Color,
        math::{round_to_step, Rect},
        pool::Handle,
    },
    gui::{
        brush::Brush,
        define_widget_deref,
        draw::{CommandTexture, Draw, DrawingContext},
        message::{MessageDirection, MouseButton, UiMessage},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, UiNode, UserInterface,
    },
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

#[derive(Clone)]
struct Entry {
    node: Handle<UiNode>,
    initial_position: Vector2<f32>,
}

#[derive(Clone)]
struct DragContext {
    initial_cursor_position: Vector2<f32>,
    entries: Vec<Entry>,
}

#[derive(Clone)]
pub struct AbsmCanvas {
    widget: Widget,
    #[allow(dead_code)] // TODO
    selection_manager: Vec<Handle<UiNode>>,
    view_position: Vector2<f32>,
    zoom: f32,
    initial_view_position: Vector2<f32>,
    click_position: Vector2<f32>,
    is_dragging_view: bool,
    drag_context: Option<DragContext>,
}

define_widget_deref!(AbsmCanvas);

impl AbsmCanvas {
    pub fn point_to_screen_space(&self, point: Vector2<f32>) -> Vector2<f32> {
        point.scale(self.zoom) + self.screen_position() + self.view_position
    }

    pub fn point_to_local_space(&self, point: Vector2<f32>) -> Vector2<f32> {
        (point - self.screen_position() - self.view_position).scale(1.0 / self.zoom)
    }

    pub fn view_point_to_local_space(&self, point: Vector2<f32>) -> Vector2<f32> {
        (point - self.view_position).scale(1.0 / self.zoom)
    }

    pub fn update_transform(&self, ui: &UserInterface) {
        let transform =
            Matrix3::new_translation(&self.view_position) * Matrix3::new_scaling(self.zoom);

        ui.send_message(WidgetMessage::layout_transform(
            self.handle(),
            MessageDirection::ToWidget,
            transform,
        ));
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
        let visual_transform = self
            .widget
            .visual_transform()
            .try_inverse()
            .unwrap_or_default();

        let local_bounds = self.widget.bounding_rect().transform(&visual_transform);
        DrawingContext::push_rect_filled(ctx, &local_bounds, None);
        ctx.commit(
            self.clip_bounds(),
            self.widget.background(),
            CommandTexture::None,
            None,
        );

        let step_size = 50.0 * self.zoom.clamp(0.001, 1000.0);

        let mut local_left_bottom = local_bounds.left_top_corner();
        local_left_bottom.x = round_to_step(local_left_bottom.x, step_size);
        local_left_bottom.y = round_to_step(local_left_bottom.y, step_size);

        let mut local_right_top = local_bounds.right_bottom_corner();
        local_right_top.x = round_to_step(local_right_top.x, step_size);
        local_right_top.y = round_to_step(local_right_top.y, step_size);

        let w = (local_right_top.x - local_left_bottom.x).abs();
        let h = (local_right_top.y - local_left_bottom.y).abs();

        let nw = ((w / step_size).ceil()) as usize;
        let nh = ((h / step_size).ceil()) as usize;

        for ny in 0..=nh {
            let k = ny as f32 / (nh) as f32;
            let y = local_left_bottom.y + k * h;
            ctx.push_line(
                Vector2::new(local_left_bottom.x - step_size, y),
                Vector2::new(local_right_top.x + step_size, y),
                1.0,
            );
        }

        for nx in 0..=nw {
            let k = nx as f32 / (nw) as f32;
            let x = local_left_bottom.x + k * w;
            ctx.push_line(
                Vector2::new(x, local_left_bottom.y + step_size),
                Vector2::new(x, local_right_top.y - step_size),
                1.0,
            );
        }

        ctx.commit(
            local_bounds,
            Brush::Solid(Color::opaque(60, 60, 60)),
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
                    child.desired_local_position().x,
                    child.desired_local_position().y,
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

            let selected_node = ui.node(message.destination());

            self.drag_context = Some(DragContext {
                initial_cursor_position: self.point_to_local_space(ui.cursor_position()),
                entries: vec![Entry {
                    node: message.destination(),
                    initial_position: selected_node.actual_local_position(),
                }],
            });
        } else if let Some(WidgetMessage::MouseDown { pos, button }) = message.data() {
            if *button == MouseButton::Middle {
                self.is_dragging_view = true;
                self.click_position = *pos;
                self.initial_view_position = self.view_position;

                ui.capture_mouse(self.handle());
            }
        } else if let Some(WidgetMessage::MouseUp { button, .. }) = message.data() {
            if *button == MouseButton::Middle {
                self.is_dragging_view = false;

                ui.release_mouse_capture();
            } else if *button == MouseButton::Left {
                self.drag_context = None;
            }
        } else if let Some(WidgetMessage::MouseMove { pos, .. }) = message.data() {
            if self.is_dragging_view {
                self.view_position = self.initial_view_position + (*pos - self.click_position);
                self.update_transform(ui);
            }

            if let Some(drag_context) = self.drag_context.as_ref() {
                for entry in drag_context.entries.iter() {
                    let local_cursor_pos = self.point_to_local_space(*pos);

                    let new_position = entry.initial_position
                        + (local_cursor_pos - drag_context.initial_cursor_position);

                    ui.send_message(WidgetMessage::desired_position(
                        entry.node,
                        MessageDirection::ToWidget,
                        new_position,
                    ));
                }
            }
        } else if let Some(WidgetMessage::MouseWheel { amount, pos }) = message.data() {
            let cursor_pos = (*pos - self.screen_position()).scale(self.zoom);

            self.zoom += 0.1 * amount;

            let new_cursor_pos = (*pos - self.screen_position()).scale(self.zoom);

            self.view_position -= (new_cursor_pos - cursor_pos).scale(self.zoom);

            self.update_transform(ui);
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
            is_dragging_view: false,
            zoom: 1.0,
            drag_context: None,
        };

        ctx.add_node(UiNode::new(canvas))
    }
}
