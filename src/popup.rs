use crate::{
    node::UINode,
    Control,
    UserInterface,
    widget::{
        Widget,
        WidgetBuilder,
    },
    message::{
        UiMessage,
        UiMessageData,
        PopupMessage,
        WidgetMessage,
        OsEvent,
        ButtonState
    },
    core::{
        pool::Handle,
        math::vec2::Vec2,
    },
    border::BorderBuilder,
    NodeHandleMapping,
};

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

pub struct Popup<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    placement: Placement,
    stays_open: bool,
    is_open: bool,
    content: Handle<UINode<M, C>>,
    body: Handle<UINode<M, C>>,
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for Popup<M, C> {
    fn widget(&self) -> &Widget<M, C> {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget<M, C> {
        &mut self.widget
    }

    fn raw_copy(&self) -> UINode<M, C> {
        UINode::Popup(Self {
            widget: self.widget.raw_copy(),
            placement: self.placement,
            stays_open: false,
            is_open: false,
            content: self.content,
            body: self.body,
        })
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        if let Some(content) = node_map.get(&self.content) {
            self.content = *content;
        }
        self.body = *node_map.get(&self.body).unwrap();
    }

    fn handle_message(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        match &message.data {
            UiMessageData::Popup(msg) if message.target == self_handle || message.source == self_handle => {
                match msg {
                    PopupMessage::Open => {
                        self.is_open = true;
                        self.widget.set_visibility(true);
                        if !self.stays_open {
                            ui.restrict_picking_to(self_handle);
                        }
                        self.widget
                            .outgoing_messages
                            .borrow_mut()
                            .push_back(
                                UiMessage::new(
                                    UiMessageData::Widget(
                                        WidgetMessage::TopMost)));
                        match self.placement {
                            Placement::LeftTop => {
                                self.widget
                                    .set_desired_local_position(Vec2::ZERO);
                            }
                            Placement::RightTop => {
                                let width = self.widget.actual_size().x;
                                let screen_width = ui.screen_size().x;
                                self.widget
                                    .set_desired_local_position(
                                        Vec2::new(screen_width - width, 0.0));
                            }
                            Placement::Center => {
                                let size = self.widget.actual_size();
                                let screen_size = ui.screen_size;
                                self.widget
                                    .set_desired_local_position(
                                        (screen_size - size).scale(0.5));
                            }
                            Placement::LeftBottom => {
                                let height = self.widget.actual_size().y;
                                let screen_height = ui.screen_size().y;
                                self.widget.
                                    set_desired_local_position(
                                        Vec2::new(0.0, screen_height - height));
                            }
                            Placement::RightBottom => {
                                let size = self.widget.actual_size();
                                let screen_size = ui.screen_size;
                                self.widget
                                    .set_desired_local_position(
                                        screen_size - size);
                            }
                            Placement::Cursor => {
                                self.widget
                                    .set_desired_local_position(
                                        ui.cursor_position())
                            }
                            Placement::Position(position) => {
                                self.widget
                                    .set_desired_local_position(
                                        position)
                            }
                        }
                    }
                    PopupMessage::Close => {
                        self.is_open = false;
                        self.widget.set_visibility(false);
                        if !self.stays_open {
                            ui.clear_picking_restriction();
                        }
                        if ui.captured_node() == self_handle {
                            ui.release_mouse_capture();
                        }
                    }
                    PopupMessage::Content(content) => {
                        if self.content.is_some() {
                            ui.remove_node(self.content);
                        }
                        self.content = *content;
                        ui.link_nodes(self.content, self.body);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn handle_os_event(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, event: &OsEvent) {
        if let OsEvent::MouseInput { state, .. } = event {
            if *state == ButtonState::Pressed {
                if ui.picking_restricted_node() == self_handle && self.is_open {
                    let pos = ui.cursor_position();
                    if !self.widget.screen_bounds().contains(pos.x, pos.y) && !self.stays_open {
                        self.close();
                    }
                }
            }
        }
    }
}

impl<M, C: 'static + Control<M, C>> Popup<M, C> {
    pub fn open(&mut self) {
        if !self.is_open {
            self.widget.invalidate_layout();
            self.widget
                .outgoing_messages
                .borrow_mut()
                .push_back(UiMessage::new(
                    UiMessageData::Popup(PopupMessage::Open)));
        }
    }

    pub fn close(&mut self) {
        if self.is_open {
            self.widget.invalidate_layout();
            self.widget
                .outgoing_messages
                .borrow_mut()
                .push_back(UiMessage::new(
                    UiMessageData::Popup(PopupMessage::Close)));
        }
    }

    pub fn set_placement(&mut self, placement: Placement) {
        if self.placement != placement {
            self.placement = placement;
            self.widget.invalidate_layout();
            self.widget
                .outgoing_messages
                .borrow_mut()
                .push_back(UiMessage::new(
                    UiMessageData::Popup(
                        PopupMessage::Placement(placement))));
        }
    }
}

pub struct PopupBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    placement: Placement,
    stays_open: bool,
    content: Handle<UINode<M, C>>,
}

impl<M, C: 'static + Control<M, C>> PopupBuilder<M, C> {
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

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> where Self: Sized {
        let body = BorderBuilder::new(WidgetBuilder::new()
            .with_child(self.content))
            .build(ui);

        let popup = Popup {
            widget: self.widget_builder
                .with_child(body)
                .with_visibility(false)
                .build(),
            placement: self.placement,
            stays_open: self.stays_open,
            is_open: false,
            content: self.content,
            body,
        };

        let handle = ui.add_node(UINode::Popup(popup));

        ui.flush_messages();

        handle
    }
}