use crate::fyrox::{
    core::{
        algebra::{Matrix3, Point2, Vector2},
        color::Color,
        math::Rect,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid_provider,
        visitor::prelude::*,
    },
    graph::BaseSceneGraph,
    gui::{
        brush::Brush,
        define_constructor, define_widget_deref,
        draw::{CommandTexture, Draw, DrawingContext},
        message::{MessageDirection, MouseButton, UiMessage},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, UiNode, UserInterface,
    },
};
use crate::{
    absm::{
        connection::{self, Connection},
        node::AbsmBaseNode,
        segment::SegmentMessage,
        selectable::{Selectable, SelectableMessage},
        socket::{Socket, SocketDirection, SocketMessage},
        transition::{self, TransitionView},
    },
    utils::fetch_node_screen_center_ui,
};
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq, Visit, Reflect, Default)]
pub(super) struct Entry {
    pub node: Handle<UiNode>,
    pub initial_position: Vector2<f32>,
}

#[derive(Debug, Clone, PartialEq, Visit, Reflect, Default)]
pub(super) struct DragContext {
    initial_cursor_position: Vector2<f32>,
    entries: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Visit, Reflect)]
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
    CreateConnection {
        source: Handle<UiNode>,
        source_pos: Vector2<f32>,
        dest_pos: Vector2<f32>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum AbsmCanvasMessage {
    SwitchMode(Mode),
    CommitTransition {
        source_node: Handle<UiNode>,
        dest_node: Handle<UiNode>,
    },
    CommitConnection {
        source_socket: Handle<UiNode>,
        dest_socket: Handle<UiNode>,
    },
    CommitDrag {
        entries: Vec<Entry>,
    },

    CommitTransitionToAllNodes {
        source_node: Handle<UiNode>,
        dest_nodes: Vec<Handle<UiNode>>,
    },

    SelectionChanged(Vec<Handle<UiNode>>),
    ForceSyncDependentObjects,
}

impl AbsmCanvasMessage {
    define_constructor!(AbsmCanvasMessage:SwitchMode => fn switch_mode(Mode), layout: false);
    define_constructor!(AbsmCanvasMessage:CommitTransition => fn commit_transition(source_node: Handle<UiNode>, dest_node: Handle<UiNode>), layout: false);
    define_constructor!(AbsmCanvasMessage:CommitConnection => fn commit_connection(source_socket: Handle<UiNode>, dest_socket: Handle<UiNode>), layout: false);
    define_constructor!(AbsmCanvasMessage:CommitDrag => fn commit_drag(entries: Vec<Entry>), layout: false);
    define_constructor!(AbsmCanvasMessage:CommitTransitionToAllNodes => fn commit_transition_to_all_nodes(source_node: Handle<UiNode>, dest_nodes: Vec<Handle<UiNode>>), layout: false);
    define_constructor!(AbsmCanvasMessage:SelectionChanged => fn selection_changed(Vec<Handle<UiNode>>), layout: false);
    define_constructor!(AbsmCanvasMessage:ForceSyncDependentObjects => fn force_sync_dependent_objects(), layout: true);
}

#[derive(Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct AbsmCanvas {
    widget: Widget,
    selection: Vec<Handle<UiNode>>,
    view_position: Vector2<f32>,
    zoom: f32,
    initial_view_position: Vector2<f32>,
    click_position: Vector2<f32>,
    is_dragging_view: bool,
    mode: Mode,
    // A handle to a node that was under the cursor at the moment of release of left mouse button.
    lmb_released_node: Cell<Handle<UiNode>>,
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

    fn fetch_dest_node_component<T>(
        &self,
        node_handle: Handle<UiNode>,
        ui: &UserInterface,
    ) -> Handle<UiNode>
    where
        T: 'static,
    {
        if ui
            .try_get(node_handle)
            .map_or(false, |n| n.has_component::<T>())
        {
            return node_handle;
        }

        if node_handle == self.handle() {
            self.find_by_criteria_up(ui, |n| n.has_component::<T>())
        } else {
            ui.node(node_handle)
                .find_by_criteria_up(ui, |n| n.has_component::<T>())
        }
    }

    fn sync_connections_ends(&self, moved_node: Handle<UiNode>, ui: &UserInterface, force: bool) {
        // Sync ends of each connection.
        for connection in self
            .children()
            .iter()
            .filter_map(|c| ui.node(*c).query_component::<Connection>())
        {
            if connection.source_node == moved_node || force {
                let source_pos = self
                    .screen_to_local(fetch_node_screen_center_ui(connection.segment.source, ui));
                ui.send_message(SegmentMessage::source_position(
                    connection.handle(),
                    MessageDirection::ToWidget,
                    source_pos,
                ));
            }

            if connection.dest_node == moved_node || force {
                let dest_pos =
                    self.screen_to_local(fetch_node_screen_center_ui(connection.segment.dest, ui));
                ui.send_message(SegmentMessage::dest_position(
                    connection.handle(),
                    MessageDirection::ToWidget,
                    dest_pos,
                ));
            }
        }
    }

    fn sync_transitions_ends(&self, moved_node: Handle<UiNode>, ui: &UserInterface, force: bool) {
        // Sync ends of each transition.
        // Check if any node has moved and sync ends accordingly.
        for transition in self
            .children()
            .iter()
            .filter_map(|c| ui.node(*c).query_component::<TransitionView>())
        {
            if force
                || moved_node == transition.segment.source
                || moved_node == transition.segment.dest
            {
                // Find other transitions sharing the same source and dest nodes (in both directions).
                for (i, transition_handle) in self
                    .children()
                    .iter()
                    .filter_map(|c| {
                        ui.node(*c)
                            .query_component::<TransitionView>()
                            .and_then(|t| {
                                if t.segment.source == transition.segment.source
                                    && t.segment.dest == transition.segment.dest
                                    || t.segment.source == transition.segment.dest
                                        && t.segment.dest == transition.segment.source
                                {
                                    Some(*c)
                                } else {
                                    None
                                }
                            })
                    })
                    .enumerate()
                {
                    if transition_handle == transition.handle() {
                        if let (Some(source_state), Some(dest_state)) = (
                            ui.try_get(transition.segment.source),
                            ui.try_get(transition.segment.dest),
                        ) {
                            let source_pos = source_state.center();
                            let dest_pos = dest_state.center();

                            let delta = dest_pos - source_pos;
                            let offset = Vector2::new(delta.y, -delta.x)
                                .normalize()
                                .scale(15.0 * i as f32);

                            ui.send_message(SegmentMessage::source_position(
                                transition.handle(),
                                MessageDirection::ToWidget,
                                source_pos + offset,
                            ));

                            ui.send_message(SegmentMessage::dest_position(
                                transition.handle(),
                                MessageDirection::ToWidget,
                                dest_pos + offset,
                            ));
                        }
                    }
                }
            }
        }
    }

    fn force_sync_dependent_objects(&self, ui: &UserInterface) {
        self.sync_transitions_ends(Handle::NONE, ui, true);
        self.sync_connections_ends(Handle::NONE, ui, true);
    }
}

uuid_provider!(AbsmCanvas = "100b1c33-d017-4fe6-95e7-e1daf310ef27");

impl Control for AbsmCanvas {
    fn draw(&self, ctx: &mut DrawingContext) {
        let grid_size = 9999.0;

        let grid_bounds = self
            .widget
            .bounding_rect()
            .inflate(grid_size, grid_size)
            .translate(Vector2::new(grid_size * 0.5, grid_size * 0.5));
        ctx.push_rect_filled(&grid_bounds, None);
        ctx.commit(
            self.clip_bounds(),
            self.widget.background(),
            CommandTexture::None,
            None,
        );

        ctx.push_grid(self.zoom, Vector2::repeat(50.0), grid_bounds);
        ctx.commit(
            self.clip_bounds(),
            Brush::Solid(Color::opaque(60, 60, 60)),
            CommandTexture::None,
            None,
        );

        match &self.mode {
            Mode::CreateTransition {
                source_pos,
                dest_pos,
                ..
            } => {
                transition::draw_transition(
                    ctx,
                    self.clip_bounds(),
                    Brush::Solid(Color::WHITE),
                    *source_pos,
                    *dest_pos,
                );
            }
            Mode::CreateConnection {
                source_pos,
                dest_pos,
                ..
            } => {
                connection::draw_connection(
                    ctx,
                    *source_pos,
                    *dest_pos,
                    self.clip_bounds(),
                    Brush::Solid(Color::WHITE),
                );
            }

            _ => {}
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
            } else if *button == MouseButton::Left && !message.handled() {
                let dest_node_handle =
                    self.fetch_dest_node_component::<AbsmBaseNode>(message.destination(), ui);

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
                match self.mode {
                    Mode::Drag { ref drag_context } => {
                        if self.screen_to_local(*pos) != drag_context.initial_cursor_position {
                            ui.send_message(AbsmCanvasMessage::commit_drag(
                                self.handle(),
                                MessageDirection::FromWidget,
                                drag_context.entries.clone(),
                            ));
                        }

                        self.mode = Mode::Normal;
                    }
                    Mode::CreateConnection { source, .. } => {
                        let dest_socket_handle = self
                            .fetch_dest_node_component::<Socket>(self.lmb_released_node.get(), ui);

                        if dest_socket_handle.is_some() {
                            let source_socket_ref =
                                ui.node(source).query_component::<Socket>().unwrap();

                            let dest_socket_ref = ui
                                .node(dest_socket_handle)
                                .query_component::<Socket>()
                                .unwrap();

                            // Do not allow to create connections between sockets of the same node.
                            if dest_socket_ref.parent_node != source_socket_ref.parent_node
                                // Only allow to create connections either from Input -> Output, or
                                // Output -> Input. Input -> Input or Output -> Output is now
                                // allowed.
                                && dest_socket_ref.direction != source_socket_ref.direction
                            {
                                // Flip source and dest to always create "child -> parent" connections.
                                let (child, parent) = match dest_socket_ref.direction {
                                    SocketDirection::Input => (source, dest_socket_handle),
                                    SocketDirection::Output => (dest_socket_handle, source),
                                };

                                ui.send_message(AbsmCanvasMessage::commit_connection(
                                    self.handle(),
                                    MessageDirection::FromWidget,
                                    child,
                                    parent,
                                ));
                            }
                        }

                        self.mode = Mode::Normal;
                    }

                    _ => {}
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
                Mode::CreateConnection {
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
            if message.direction() == MessageDirection::ToWidget
                && message.destination() == self.handle()
            {
                match msg {
                    AbsmCanvasMessage::SwitchMode(mode) => {
                        // TODO: Check if other mode is active.
                        self.mode = mode.clone();
                    }
                    AbsmCanvasMessage::SelectionChanged(new_selection) => {
                        self.set_selection(new_selection, ui);
                    }
                    AbsmCanvasMessage::ForceSyncDependentObjects => {
                        self.force_sync_dependent_objects(ui);
                    }
                    _ => (),
                }
            }
        } else if let Some(SocketMessage::StartDragging) = message.data() {
            if message.direction() == MessageDirection::FromWidget {
                let socket_ref = ui
                    .node(message.destination())
                    .query_component::<Socket>()
                    .unwrap();

                ui.send_message(AbsmCanvasMessage::switch_mode(
                    self.handle(),
                    MessageDirection::ToWidget,
                    Mode::CreateConnection {
                        source: message.destination(),
                        source_pos: self.screen_to_local(socket_ref.screen_position()),
                        dest_pos: self.screen_to_local(ui.cursor_position()),
                    },
                ))
            }
        } else if let Some(WidgetMessage::DesiredPosition(_)) = message.data() {
            if ui
                .node(message.destination())
                .has_component::<AbsmBaseNode>()
            {
                let moved_node = message.destination();
                self.sync_connections_ends(moved_node, ui, false);
                self.sync_transitions_ends(moved_node, ui, false);
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(WidgetMessage::MouseUp { button, pos }) = message.data() {
            if *button == MouseButton::Left {
                self.lmb_released_node.set(ui.hit_test_unrestricted(*pos));
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
            widget: self
                .widget_builder
                .with_preview_messages(true)
                .with_clip_to_bounds(false)
                .build(),
            selection: Default::default(),
            view_position: Default::default(),
            initial_view_position: Default::default(),
            click_position: Default::default(),
            is_dragging_view: false,
            zoom: 1.0,
            mode: Mode::Normal,
            lmb_released_node: Default::default(),
        };

        ctx.add_node(UiNode::new(canvas))
    }
}
