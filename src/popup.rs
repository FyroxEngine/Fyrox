use crate::message::MessageDirection;
use crate::{
    border::BorderBuilder,
    core::{math::vec2::Vec2, pool::Handle},
    message::{ButtonState, OsEvent, PopupMessage, UiMessage, UiMessageData, WidgetMessage},
    node::UINode,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, NodeHandleMapping, UserInterface,
};
use std::ops::{Deref, DerefMut};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Placement {
    LeftTop,
    RightTop,
    Center,
    LeftBottom,
    RightBottom,
    Cursor,
    Position(Vec2),
}

#[derive(Clone)]
pub struct Popup<M: 'static + std::fmt::Debug + Clone + PartialEq, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    placement: Placement,
    stays_open: bool,
    is_open: bool,
    content: Handle<UINode<M, C>>,
    body: Handle<UINode<M, C>>,
}

impl<M: 'static + std::fmt::Debug + Clone + PartialEq, C: 'static + Control<M, C>> Deref
    for Popup<M, C>
{
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static + std::fmt::Debug + Clone + PartialEq, C: 'static + Control<M, C>> DerefMut
    for Popup<M, C>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static + std::fmt::Debug + Clone + PartialEq, C: 'static + Control<M, C>> Control<M, C>
    for Popup<M, C>
{
    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        if let Some(content) = node_map.get(&self.content) {
            self.content = *content;
        }
        self.body = *node_map.get(&self.body).unwrap();
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        match &message.data() {
            UiMessageData::Popup(msg) if message.destination() == self.handle() => match msg {
                PopupMessage::Open => {
                    if !self.is_open {
                        self.is_open = true;
                        ui.send_message(WidgetMessage::visibility(
                            self.handle(),
                            MessageDirection::ToWidget,
                            true,
                        ));
                        ui.push_picking_restriction(self.handle());
                        ui.send_message(WidgetMessage::topmost(
                            self.handle(),
                            MessageDirection::ToWidget,
                        ));
                        let position = match self.placement {
                            Placement::LeftTop => Vec2::ZERO,
                            Placement::RightTop => {
                                let width = self.widget.actual_size().x;
                                let screen_width = ui.screen_size().x;
                                Vec2::new(screen_width - width, 0.0)
                            }
                            Placement::Center => {
                                let size = self.widget.actual_size();
                                let screen_size = ui.screen_size;
                                (screen_size - size).scale(0.5)
                            }
                            Placement::LeftBottom => {
                                let height = self.widget.actual_size().y;
                                let screen_height = ui.screen_size().y;
                                Vec2::new(0.0, screen_height - height)
                            }
                            Placement::RightBottom => {
                                let size = self.widget.actual_size();
                                let screen_size = ui.screen_size;
                                screen_size - size
                            }
                            Placement::Cursor => ui.cursor_position(),
                            Placement::Position(position) => position,
                        };
                        ui.send_message(WidgetMessage::desired_position(
                            self.handle(),
                            MessageDirection::ToWidget,
                            position,
                        ));
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
                &PopupMessage::Placement(placement) => {
                    self.placement = placement;
                    self.invalidate_layout();
                }
            },
            _ => {}
        }
    }

    fn handle_os_event(
        &mut self,
        self_handle: Handle<UINode<M, C>>,
        ui: &mut UserInterface<M, C>,
        event: &OsEvent,
    ) {
        if let OsEvent::MouseInput { state, .. } = event {
            if *state == ButtonState::Pressed
                && ui.top_picking_restriction() == self_handle
                && self.is_open
            {
                let pos = ui.cursor_position();
                if !self.widget.screen_bounds().contains(pos.x, pos.y) && !self.stays_open {
                    ui.send_message(PopupMessage::close(
                        self.handle(),
                        MessageDirection::ToWidget,
                    ));
                }
            }
        }
    }
}

pub struct PopupBuilder<
    M: 'static + std::fmt::Debug + Clone + PartialEq,
    C: 'static + Control<M, C>,
> {
    widget_builder: WidgetBuilder<M, C>,
    placement: Placement,
    stays_open: bool,
    content: Handle<UINode<M, C>>,
}

impl<M: 'static + std::fmt::Debug + Clone + PartialEq, C: 'static + Control<M, C>>
    PopupBuilder<M, C>
{
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            placement: Placement::Cursor,
            stays_open: false,
            content: Default::default(),
        }
    }

    pub fn with_placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    pub fn stays_open(mut self, value: bool) -> Self {
        self.stays_open = value;
        self
    }

    pub fn with_content(mut self, content: Handle<UINode<M, C>>) -> Self {
        self.content = content;
        self
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let body = BorderBuilder::new(WidgetBuilder::new().with_child(self.content)).build(ctx);

        let popup = Popup {
            widget: self
                .widget_builder
                .with_child(body)
                .with_visibility(false)
                .build(),
            placement: self.placement,
            stays_open: self.stays_open,
            is_open: false,
            content: self.content,
            body,
        };

        ctx.add_node(UINode::Popup(popup))
    }
}
