use crate::absm::{
    node::AbsmNode, selectable::Selectable, selectable::SelectableMessage, transition,
};
use fyrox::{
    animation::machine::state::StateDefinition,
    core::{
        algebra::{Matrix3, Point2, Vector2},
        color::Color,
        math::{round_to_step, Rect},
        pool::Handle,
    },
    gui::{
        brush::Brush,
        define_constructor, define_widget_deref,
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

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Entry {
    pub node: Handle<UiNode>,
    pub initial_position: Vector2<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct DragContext {
    initial_cursor_position: Vector2<f32>,
    entries: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Mode {
    Normal,
    Drag {
        drag_context: DragContext,
    },
    CreateTransition {
        source: Handle<UiNode>,
        source_pos: Vector2<f32>,
        dest_pos: Vector2<f32>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum AbsmCanvasMessage {
    SwitchMode(Mode),
    CommitTransition {
        source: Handle<UiNode>,
        dest: Handle<UiNode>,
    },
    CommitDrag {
        entries: Vec<Entry>,
    },
    SelectionChanged(Vec<Handle<UiNode>>),
}

impl AbsmCanvasMessage {
    define_constructor!(AbsmCanvasMessage:SwitchMode => fn switch_mode(Mode), layout: false);
    define_constructor!(AbsmCanvasMessage:CommitTransition => fn commit_transition(source: Handle<UiNode>, dest: Handle<UiNode>), layout: false);
    define_constructor!(AbsmCanvasMessage:CommitDrag => fn commit_drag(entries: Vec<Entry>), layout: false);
    define_constructor!(AbsmCanvasMessage:SelectionChanged => fn selection_changed(Vec<Handle<UiNode>>), layout: false);
}

#[derive(Clone)]
pub struct AbsmCanvas {
    widget: Widget,
    selection: Vec<Handle<UiNode>>,
    view_position: Vector2<f32>,
    zoom: f32,
    initial_view_position: Vector2<f32>,
    click_position: Vector2<f32>,
    is_dragging_view: bool,
    mode: Mode,
}

define_widget_deref!(AbsmCanvas);

impl AbsmCanvas {
    pub fn point_to_local_space(&self, point: Vector2<f32>) -> Vector2<f32> {
        self.visual_transform()
            .try_inverse()
            .unwrap_or_default()
            .transform_point(&Point2::from(point))
            .coords
    }

    pub fn update_transform(&self, ui: &UserInterface) {
        let transform =
            Matrix3::new_translation(&-self.view_position) * Matrix3::new_scaling(self.zoom);

        ui.send_message(WidgetMessage::layout_transform(
            self.handle(),
            MessageDirection::ToWidget,
            transform,
        ));
    }

    fn make_drag_context(&self, ui: &UserInterface) -> DragContext {
        DragContext {
            initial_cursor_position: self.point_to_local_space(ui.cursor_position()),
            entries: self
                .selection
                .iter()
                .map(|n| Entry {
                    node: *n,
                    initial_position: ui.node(*n).actual_local_position(),
                })
                .collect(),
        }
    }

    fn set_selection(&mut self, new_selection: &[Handle<UiNode>], ui: &UserInterface) {
        if self.selection != new_selection {
            for &child in self
                .children()
                .iter()
                .filter(|n| ui.node(**n).query_component::<Selectable>().is_some())
            {
                ui.send_message(
                    SelectableMessage::select(
                        child,
                        MessageDirection::ToWidget,
                        new_selection.contains(&child),
                    )
                    .with_handled(true),
                );
            }

            self.selection = new_selection.to_vec();

            ui.send_message(AbsmCanvasMessage::selection_changed(
                self.handle(),
                MessageDirection::FromWidget,
                self.selection.clone(),
            ));

            // Make sure to update dragging context if we're in Drag mode.
            if let Mode::Drag { .. } = self.mode {
                self.mode = Mode::Drag {
                    drag_context: self.make_drag_context(ui),
                };
            }
        }
    }

    fn fetch_state_dest_node(&self, node: Handle<UiNode>, ui: &UserInterface) -> Handle<UiNode> {
        if node == self.handle() {
            self.find_by_criteria_up(ui, |n| {
                n.query_component::<AbsmNode<StateDefinition>>().is_some()
            })
        } else {
            ui.node(node).find_by_criteria_up(ui, |n| {
                n.query_component::<AbsmNode<StateDefinition>>().is_some()
            })
        }
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
        let size = 9999.0;

        let local_bounds = self
            .widget
            .bounding_rect()
            .inflate(size, size)
            .translate(Vector2::new(size * 0.5, size * 0.5));
        DrawingContext::push_rect_filled(ctx, &local_bounds, None);
        ctx.commit(
            self.clip_bounds(),
            self.widget.background(),
            CommandTexture::None,
            None,
        );
        let step_size = 50.0;

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
                1.0 / self.zoom,
            );
        }

        for nx in 0..=nw {
            let k = nx as f32 / (nw) as f32;
            let x = local_left_bottom.x + k * w;
            ctx.push_line(
                Vector2::new(x, local_left_bottom.y + step_size),
                Vector2::new(x, local_right_top.y - step_size),
                1.0 / self.zoom,
            );
        }

        ctx.commit(
            self.clip_bounds(),
            Brush::Solid(Color::opaque(60, 60, 60)),
            CommandTexture::None,
            None,
        );

        if let Mode::CreateTransition {
            source_pos,
            dest_pos,
            ..
        } = self.mode
        {
            transition::draw_transition(
                ctx,
                self.clip_bounds(),
                Brush::Solid(Color::WHITE),
                source_pos,
                dest_pos,
            );
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

        if let Some(SelectableMessage::Select(true)) = message.data() {
            if message.direction() == MessageDirection::FromWidget && !message.handled() {
                let selected_node = message.destination();

                let new_selection = if ui.keyboard_modifiers().control {
                    let mut selection = self.selection.clone();
                    selection.push(selected_node);
                    selection
                } else {
                    vec![selected_node]
                };

                self.set_selection(&new_selection, ui);
            }
        } else if let Some(WidgetMessage::MouseDown { pos, button }) = message.data() {
            if *button == MouseButton::Middle {
                self.is_dragging_view = true;
                self.click_position = *pos;
                self.initial_view_position = self.view_position;

                ui.capture_mouse(self.handle());
            } else if *button == MouseButton::Left {
                let dest_node_handle = self.fetch_state_dest_node(message.destination(), ui);

                match self.mode {
                    Mode::CreateTransition { source, .. } => {
                        if dest_node_handle.is_some() {
                            // Commit creation.
                            ui.send_message(AbsmCanvasMessage::commit_transition(
                                self.handle(),
                                MessageDirection::FromWidget,
                                source,
                                dest_node_handle,
                            ));
                        }

                        self.mode = Mode::Normal;
                    }
                    Mode::Normal => {
                        if dest_node_handle.is_some() {
                            self.mode = Mode::Drag {
                                drag_context: self.make_drag_context(ui),
                            }
                        } else {
                            self.set_selection(&[], ui);
                        }
                    }
                    _ => {}
                }
            }
        } else if let Some(WidgetMessage::MouseUp { button, pos }) = message.data() {
            if *button == MouseButton::Middle {
                self.is_dragging_view = false;

                ui.release_mouse_capture();
            } else if *button == MouseButton::Left {
                if let Mode::Drag { ref drag_context } = self.mode {
                    if self.screen_to_local(*pos) != drag_context.initial_cursor_position {
                        ui.send_message(AbsmCanvasMessage::commit_drag(
                            self.handle(),
                            MessageDirection::FromWidget,
                            drag_context.entries.clone(),
                        ));
                    }

                    self.mode = Mode::Normal;
                }
            }
        } else if let Some(WidgetMessage::MouseMove { pos, .. }) = message.data() {
            if self.is_dragging_view {
                self.view_position = self.initial_view_position + (*pos - self.click_position);
                self.update_transform(ui);
            }

            let local_cursor_position = self.screen_to_local(ui.cursor_position());

            match self.mode {
                Mode::Drag { ref drag_context } => {
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
                Mode::CreateTransition {
                    ref mut dest_pos, ..
                } => {
                    *dest_pos = local_cursor_position;
                }
                _ => (),
            }
        } else if let Some(WidgetMessage::MouseWheel { amount, pos }) = message.data() {
            let cursor_pos = (*pos - self.screen_position()).scale(self.zoom);

            self.zoom = (self.zoom + 0.1 * amount).clamp(0.2, 2.0);

            let new_cursor_pos = (*pos - self.screen_position()).scale(self.zoom);

            self.view_position -= (new_cursor_pos - cursor_pos).scale(self.zoom);

            self.update_transform(ui);
        } else if let Some(msg) = message.data::<AbsmCanvasMessage>() {
            if message.direction() == MessageDirection::ToWidget {
                match msg {
                    AbsmCanvasMessage::SwitchMode(mode) => {
                        // TODO: Check if other mode is active.
                        self.mode = mode.clone();
                    }
                    AbsmCanvasMessage::SelectionChanged(new_selection) => {
                        self.set_selection(new_selection, ui);
                    }
                    _ => (),
                }
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
            widget: self.widget_builder.with_clip_to_bounds(false).build(),
            selection: Default::default(),
            view_position: Default::default(),
            initial_view_position: Default::default(),
            click_position: Default::default(),
            is_dragging_view: false,
            zoom: 1.0,
            mode: Mode::Normal,
        };

        ctx.add_node(UiNode::new(canvas))
    }
}
