use crate::{
    border::BorderBuilder,
    core::{algebra::Vector2, math::Rect, pool::Handle},
    define_constructor,
    message::{ButtonState, MessageDirection, OsEvent, UiMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, NodeHandleMapping, RestrictionEntry, Thickness, UiNode, UserInterface,
    BRUSH_DARKER, BRUSH_LIGHTER,
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq)]
pub enum PopupMessage {
    Open,
    Close,
    Content(Handle<UiNode>),
    Placement(Placement),
    AdjustPosition,
}

impl PopupMessage {
    define_constructor!(PopupMessage:Open => fn open(), layout: false);
    define_constructor!(PopupMessage:Close => fn close(), layout: false);
    define_constructor!(PopupMessage:Content => fn content(Handle<UiNode>), layout: false);
    define_constructor!(PopupMessage:Placement => fn placement(Placement), layout: false);
    define_constructor!(PopupMessage:AdjustPosition => fn adjust_position(), layout: true);
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Placement {
    /// A popup should be placed relative to given widget at the left top corner of the widget screen bounds.
    /// Widget handle could be `NONE`, in this case the popup will be placed at the left top corner of the screen.
    LeftTop(Handle<UiNode>),

    /// A popup should be placed relative to given widget at the right top corner of the widget screen bounds.
    /// Widget handle could be `NONE`, in this case the popup will be placed at the right top corner of the screen.
    RightTop(Handle<UiNode>),

    /// A popup should be placed relative to given widget at the center of the widget screen bounds.
    /// Widget handle could be `NONE`, in this case the popup will be placed at the center of the screen.
    Center(Handle<UiNode>),

    /// A popup should be placed relative to given widget at the left bottom corner of the widget screen bounds.
    /// Widget handle could be `NONE`, in this case the popup will be placed at the left bottom corner of the screen.
    LeftBottom(Handle<UiNode>),

    /// A popup should be placed relative to given widget at the right bottom corner of the widget screen bounds.
    /// Widget handle could be `NONE`, in this case the popup will be placed at the right bottom corner of the screen.
    RightBottom(Handle<UiNode>),

    /// A popup should be placed at the cursor position. The widget handle could be either `NONE` or a handle of a
    /// widget that is directly behind the cursor.
    Cursor(Handle<UiNode>),

    /// A popup should be placed at given screen-space position.
    Position {
        /// Screen-space position.
        position: Vector2<f32>,

        /// A handle of the node that is located behind the given position. Could be `NONE` if there is nothing behind
        /// given position.
        target: Handle<UiNode>,
    },
}

#[derive(Clone)]
pub struct Popup {
    pub widget: Widget,
    pub placement: Placement,
    pub stays_open: bool,
    pub is_open: bool,
    pub content: Handle<UiNode>,
    pub body: Handle<UiNode>,
    pub smart_placement: bool,
}

crate::define_widget_deref!(Popup);

fn adjust_placement_position(
    node_screen_bounds: Rect<f32>,
    screen_size: Vector2<f32>,
) -> Vector2<f32> {
    let mut new_position = node_screen_bounds.position;
    let right_bottom = node_screen_bounds.right_bottom_corner();
    if right_bottom.x > screen_size.x {
        new_position.x -= right_bottom.x - screen_size.x;
    }
    if right_bottom.y > screen_size.y {
        new_position.y -= right_bottom.y - screen_size.y;
    }
    new_position
}

impl Popup {
    fn left_top_placement(&self, ui: &UserInterface, target: Handle<UiNode>) -> Vector2<f32> {
        ui.try_get_node(target)
            .map(|n| n.screen_position())
            .unwrap_or_default()
    }

    fn right_top_placement(&self, ui: &UserInterface, target: Handle<UiNode>) -> Vector2<f32> {
        ui.try_get_node(target)
            .map(|n| n.screen_position() + Vector2::new(n.actual_global_size().x, 0.0))
            .unwrap_or_else(|| {
                Vector2::new(ui.screen_size().x - self.widget.actual_global_size().x, 0.0)
            })
    }

    fn center_placement(&self, ui: &UserInterface, target: Handle<UiNode>) -> Vector2<f32> {
        ui.try_get_node(target)
            .map(|n| n.screen_position() + n.actual_global_size().scale(0.5))
            .unwrap_or_else(|| (ui.screen_size - self.widget.actual_global_size()).scale(0.5))
    }

    fn left_bottom_placement(&self, ui: &UserInterface, target: Handle<UiNode>) -> Vector2<f32> {
        ui.try_get_node(target)
            .map(|n| n.screen_position() + Vector2::new(0.0, n.actual_global_size().y))
            .unwrap_or_else(|| {
                Vector2::new(0.0, ui.screen_size().y - self.widget.actual_global_size().y)
            })
    }

    fn right_bottom_placement(&self, ui: &UserInterface, target: Handle<UiNode>) -> Vector2<f32> {
        ui.try_get_node(target)
            .map(|n| n.screen_position() + n.actual_global_size())
            .unwrap_or_else(|| ui.screen_size - self.widget.actual_global_size())
    }
}

impl Control for Popup {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        node_map.resolve(&mut self.content);
        node_map.resolve(&mut self.body);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<PopupMessage>() {
            if message.destination() == self.handle() {
                match msg {
                    PopupMessage::Open => {
                        if !self.is_open {
                            self.is_open = true;
                            ui.send_message(WidgetMessage::visibility(
                                self.handle(),
                                MessageDirection::ToWidget,
                                true,
                            ));
                            ui.push_picking_restriction(RestrictionEntry {
                                handle: self.handle(),
                                stop: false,
                            });
                            ui.send_message(WidgetMessage::topmost(
                                self.handle(),
                                MessageDirection::ToWidget,
                            ));
                            let position = match self.placement {
                                Placement::LeftTop(target) => self.left_top_placement(ui, target),
                                Placement::RightTop(target) => self.right_top_placement(ui, target),
                                Placement::Center(target) => self.center_placement(ui, target),
                                Placement::LeftBottom(target) => {
                                    self.left_bottom_placement(ui, target)
                                }
                                Placement::RightBottom(target) => {
                                    self.right_bottom_placement(ui, target)
                                }
                                Placement::Cursor(_) => ui.cursor_position(),
                                Placement::Position { position, .. } => position,
                            };

                            ui.send_message(WidgetMessage::desired_position(
                                self.handle(),
                                MessageDirection::ToWidget,
                                ui.screen_to_root_canvas_space(position),
                            ));
                            if self.smart_placement {
                                ui.send_message(PopupMessage::adjust_position(
                                    self.handle,
                                    MessageDirection::ToWidget,
                                ));
                            }
                        }
                    }
                    PopupMessage::Close => {
                        if self.is_open {
                            self.is_open = false;
                            ui.send_message(WidgetMessage::visibility(
                                self.handle(),
                                MessageDirection::ToWidget,
                                false,
                            ));
                            ui.remove_picking_restriction(self.handle());
                            if ui.captured_node() == self.handle() {
                                ui.release_mouse_capture();
                            }
                        }
                    }
                    PopupMessage::Content(content) => {
                        if self.content.is_some() {
                            ui.send_message(WidgetMessage::remove(
                                self.content,
                                MessageDirection::ToWidget,
                            ));
                        }
                        self.content = *content;

                        ui.send_message(WidgetMessage::link(
                            self.content,
                            MessageDirection::ToWidget,
                            self.body,
                        ));
                    }
                    PopupMessage::Placement(placement) => {
                        self.placement = *placement;
                        self.invalidate_layout();
                    }
                    PopupMessage::AdjustPosition => {
                        let new_position =
                            adjust_placement_position(self.screen_bounds(), ui.screen_size());

                        if new_position != self.screen_position() {
                            ui.send_message(WidgetMessage::desired_position(
                                self.handle,
                                MessageDirection::ToWidget,
                                ui.screen_to_root_canvas_space(new_position),
                            ));
                        }
                    }
                }
            }
        }
    }

    fn handle_os_event(
        &mut self,
        self_handle: Handle<UiNode>,
        ui: &mut UserInterface,
        event: &OsEvent,
    ) {
        if let OsEvent::MouseInput { state, .. } = event {
            if let Some(top_restriction) = ui.top_picking_restriction() {
                if *state == ButtonState::Pressed
                    && top_restriction.handle == self_handle
                    && self.is_open
                {
                    let pos = ui.cursor_position();
                    if !self.widget.screen_bounds().contains(pos) && !self.stays_open {
                        ui.send_message(PopupMessage::close(
                            self.handle(),
                            MessageDirection::ToWidget,
                        ));
                    }
                }
            }
        }
    }
}

pub struct PopupBuilder {
    widget_builder: WidgetBuilder,
    placement: Placement,
    stays_open: bool,
    content: Handle<UiNode>,
    smart_placement: bool,
}

impl PopupBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            placement: Placement::Cursor(Default::default()),
            stays_open: false,
            content: Default::default(),
            smart_placement: true,
        }
    }

    pub fn with_placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    pub fn with_smart_placement(mut self, smart_placement: bool) -> Self {
        self.smart_placement = smart_placement;
        self
    }

    pub fn stays_open(mut self, value: bool) -> Self {
        self.stays_open = value;
        self
    }

    pub fn with_content(mut self, content: Handle<UiNode>) -> Self {
        self.content = content;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let body = BorderBuilder::new(
            WidgetBuilder::new()
                .with_background(BRUSH_DARKER)
                .with_foreground(BRUSH_LIGHTER)
                .with_child(self.content),
        )
        .with_stroke_thickness(Thickness::uniform(1.0))
        .build(ctx);

        let popup = Popup {
            widget: self
                .widget_builder
                .with_child(body)
                .with_visibility(false)
                .with_handle_os_events(true)
                .build(),
            placement: self.placement,
            stays_open: self.stays_open,
            is_open: false,
            content: self.content,
            smart_placement: self.smart_placement,
            body,
        };

        ctx.add_node(UiNode::new(popup))
    }
}
