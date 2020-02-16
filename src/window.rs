use crate::{
    message::{
        UiMessageData,
        UiMessage,
    },
    border::BorderBuilder,
    UINode,
    UserInterface,
    grid::{
        GridBuilder,
        Column,
        Row,
    },
    HorizontalAlignment,
    text::TextBuilder,
    Thickness,
    button::ButtonBuilder,
    scroll_viewer::{
        ScrollViewerBuilder,
    },
    widget::{
        Widget,
        WidgetBuilder,
    },
    Control,
    ControlTemplate,
    UINodeContainer,
    Builder,
    core::{
        pool::Handle,
        math::vec2::Vec2,
    },
    message::{
        WidgetMessage,
        ButtonMessage,
        WindowMessage
    },
    NodeHandleMapping
};

/// Represents a widget looking as window in Windows - with title, minimize and close buttons.
/// It has scrollable region for content, content can be any desired node or even other window.
/// Window can be dragged by its title.
pub struct Window<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    mouse_click_pos: Vec2,
    initial_position: Vec2,
    is_dragged: bool,
    minimized: bool,
    can_minimize: bool,
    can_close: bool,
    header: Handle<UINode<M, C>>,
    minimize_button: Handle<UINode<M, C>>,
    close_button: Handle<UINode<M, C>>,
    scroll_viewer: Handle<UINode<M, C>>,
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for Window<M, C> {
    fn widget(&self) -> &Widget<M, C> {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget<M, C> {
        &mut self.widget
    }

    fn raw_copy(&self) -> UINode<M, C> {
        UINode::Window(Self {
            widget: self.widget.raw_copy(),
            mouse_click_pos: self.mouse_click_pos,
            initial_position: self.initial_position,
            is_dragged: self.is_dragged,
            minimized: self.minimized,
            can_minimize: self.can_minimize,
            can_close: self.can_close,
            header: self.header,
            minimize_button: self.minimize_button,
            close_button: self.close_button,
            scroll_viewer: self.scroll_viewer,
        })
    }

    fn resolve(&mut self, _: &ControlTemplate<M, C>, node_map: &NodeHandleMapping<M, C>) {
        self.header = *node_map.get(&self.header).unwrap();
        self.minimize_button = *node_map.get(&self.minimize_button).unwrap();
        self.close_button = *node_map.get(&self.close_button).unwrap();
        self.scroll_viewer = *node_map.get(&self.scroll_viewer).unwrap();
    }

    fn handle_message(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_message(self_handle, ui, message);

        match &message.data {
            UiMessageData::Widget(msg) => {
                if message.source == self.header {
                    match msg {
                        WidgetMessage::MouseDown { pos, .. } => {
                            ui.capture_mouse(self.header);
                            let initial_position = self.widget().actual_local_position();
                            self.mouse_click_pos = *pos;
                            self.initial_position = initial_position;
                            self.is_dragged = true;
                            message.handled = true;
                        }
                        WidgetMessage::MouseUp { .. } => {
                            ui.release_mouse_capture();
                            self.is_dragged = false;
                            message.handled = true;
                        }
                        WidgetMessage::MouseMove(pos) => {
                            if self.is_dragged {
                                self.widget.set_desired_local_position(self.initial_position + *pos - self.mouse_click_pos);
                            }
                            message.handled = true;
                        }
                        _ => ()
                    }
                }
            }
            UiMessageData::Button(msg) => {
                if let ButtonMessage::Click = msg {
                    if message.source == self.minimize_button {
                        self.minimize(!self.minimized);
                    } else if message.source == self.close_button {
                        self.close();
                    }
                }
            }
            UiMessageData::Window(msg) => {
                if message.source == self_handle || message.target == self_handle {
                    match msg {
                        WindowMessage::Opened => {
                            self.widget.set_visibility(true);
                        }
                        WindowMessage::Closed => {
                            self.widget.set_visibility(false);
                        }
                        WindowMessage::Minimized(minimized) => {
                            if self.minimized != *minimized {
                                self.minimized = *minimized;
                                self.widget.invalidate_layout();
                                if self.scroll_viewer.is_some() {
                                    if let UINode::ScrollViewer(scroll_viewer) = ui.node_mut(self.scroll_viewer) {
                                        scroll_viewer.widget_mut()
                                            .set_visibility(!*minimized);
                                    }
                                }
                            }
                        }
                        WindowMessage::CanMinimize(value) => {
                            if self.can_minimize != *value {
                                self.can_minimize = *value;
                                self.widget.invalidate_layout();
                                if self.minimize_button.is_some() {
                                    ui.node_mut(self.minimize_button)
                                        .widget_mut()
                                        .set_visibility(*value);
                                }
                            }
                        }
                        WindowMessage::CanClose(value) => {
                            if self.can_close != *value {
                                self.can_close = *value;
                                self.widget.invalidate_layout();
                                if self.close_button.is_some() {
                                    ui.node_mut(self.close_button)
                                        .widget_mut()
                                        .set_visibility(*value);
                                }
                            }
                        }
                    }
                }
            }
            _ => ()
        }
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        if self.header == handle {
            self.header = Handle::NONE;
        }
        if self.scroll_viewer == handle {
            self.scroll_viewer = Handle::NONE;
        }
        if self.close_button == handle {
            self.close_button = Handle::NONE;
        }
        if self.minimize_button == handle {
            self.minimize_button = Handle::NONE;
        }
    }
}

impl<M, C: 'static + Control<M, C>> Window<M, C> {
    pub fn new(
        widget: Widget<M, C>,
        header: Handle<UINode<M, C>>,
        minimize_button: Handle<UINode<M, C>>,
        close_button: Handle<UINode<M, C>>,
        scroll_viewer: Handle<UINode<M, C>>,
    ) -> Self {
        Self {
            widget,
            mouse_click_pos: Default::default(),
            initial_position: Default::default(),
            is_dragged: false,
            minimized: false,
            can_minimize: true,
            can_close: true,
            header,
            minimize_button,
            close_button,
            scroll_viewer,
        }
    }

    pub fn close(&mut self) {
        self.widget.invalidate_layout();
        self.widget
            .outgoing_messages
            .borrow_mut()
            .push_back(UiMessage::new(UiMessageData::Window(WindowMessage::Closed)));
    }

    pub fn open(&mut self) {
        self.widget.invalidate_layout();
        self.widget
            .outgoing_messages
            .borrow_mut()
            .push_back(UiMessage::new(UiMessageData::Window(WindowMessage::Opened)));
    }

    pub fn minimize(&mut self, state: bool) {
        self.widget.invalidate_layout();
        self.widget
            .outgoing_messages
            .borrow_mut()
            .push_back(UiMessage::new(UiMessageData::Window(WindowMessage::Minimized(state))));
    }

    pub fn set_can_close(&mut self, state: bool) {
        self.widget.invalidate_layout();
        self.widget
            .outgoing_messages
            .borrow_mut()
            .push_back(UiMessage::new(UiMessageData::Window(WindowMessage::CanClose(state))));
    }

    pub fn set_can_minimize(&mut self, state: bool) {
        self.widget.invalidate_layout();
        self.widget
            .outgoing_messages
            .borrow_mut()
            .push_back(UiMessage::new(UiMessageData::Window(WindowMessage::CanMinimize(state))));
    }
}

pub struct WindowBuilder<'a, M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    content: Handle<UINode<M, C>>,
    title: Option<WindowTitle<'a, M, C>>,
    can_close: bool,
    can_minimize: bool,
    open: bool,
}

/// Window title can be either text or node.
///
/// If `Text` is used, then builder will automatically create Text node with specified text,
/// but with default font.
///
/// If you need more flexibility (i.e. put a picture near text) then `Node` option is for you:
/// it allows to put any UI node hierarchy you want to.
pub enum WindowTitle<'a, M: 'static, C: 'static + Control<M, C>> {
    Text(&'a str),
    Node(Handle<UINode<M, C>>),
}

impl<'a, M, C: 'static + Control<M, C>> WindowBuilder<'a, M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            content: Handle::NONE,
            title: None,
            can_close: true,
            can_minimize: true,
            open: true,
        }
    }

    pub fn with_content(mut self, content: Handle<UINode<M, C>>) -> Self {
        self.content = content;
        self
    }

    pub fn with_title(mut self, title: WindowTitle<'a, M, C>) -> Self {
        self.title = Some(title);
        self
    }

    pub fn can_close(mut self, can_close: bool) -> Self {
        self.can_close = can_close;
        self
    }

    pub fn can_minimize(mut self, can_minimize: bool) -> Self {
        self.can_minimize = can_minimize;
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }
}

impl<M, C: 'static + Control<M, C>> Builder<M, C> for WindowBuilder<'_, M, C> {
    fn build(self, ui: &mut dyn UINodeContainer<M, C>) -> Handle<UINode<M, C>> {
        let minimize_button;
        let close_button;

        let header = BorderBuilder::new(WidgetBuilder::new()
            .with_horizontal_alignment(HorizontalAlignment::Stretch)
            .with_height(30.0)
            .with_child(GridBuilder::new(WidgetBuilder::new()
                .with_child({
                    match self.title {
                        None => Handle::NONE,
                        Some(window_title) => {
                            match window_title {
                                WindowTitle::Node(node) => node,
                                WindowTitle::Text(text) => {
                                    TextBuilder::new(WidgetBuilder::new()
                                        .with_margin(Thickness::uniform(5.0))
                                        .on_row(0)
                                        .on_column(0))
                                        .with_text(text)
                                        .build(ui)
                                }
                            }
                        }
                    }
                })
                .with_child({
                    minimize_button = ButtonBuilder::new(WidgetBuilder::new()
                        .on_row(0)
                        .on_column(1)
                        .with_visibility(self.can_minimize)
                        .with_margin(Thickness::uniform(2.0)))
                        .with_text("_")
                        .build(ui);
                    minimize_button
                })
                .with_child({
                    close_button = ButtonBuilder::new(WidgetBuilder::new()
                        .on_row(0)
                        .on_column(2)
                        .with_visibility(self.can_close)
                        .with_margin(Thickness::uniform(2.0)))
                        .with_text("X")
                        .build(ui);
                    close_button
                }))
                .add_column(Column::stretch())
                .add_column(Column::strict(30.0))
                .add_column(Column::strict(30.0))
                .add_row(Row::stretch())
                .build(ui))
            .on_row(0)
        ).build(ui);

        let scroll_viewer = ScrollViewerBuilder::new(WidgetBuilder::new()
            .on_row(1)
            .with_margin(Thickness::uniform(1.0)))
            .with_content(self.content)
            .build(ui);

        let window = Window {
            widget: self.widget_builder
                .with_visibility(self.open)
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .with_child(GridBuilder::new(WidgetBuilder::new()
                        .with_child(scroll_viewer)
                        .with_child(header))
                        .add_column(Column::stretch())
                        .add_row(Row::auto())
                        .add_row(Row::stretch())
                        .build(ui)))
                    .build(ui))
                .build(),
            mouse_click_pos: Vec2::ZERO,
            initial_position: Vec2::ZERO,
            is_dragged: false,
            minimized: false,
            can_minimize: self.can_minimize,
            can_close: self.can_close,
            header,
            minimize_button,
            close_button,
            scroll_viewer,
        };
        ui.add_node(UINode::Window(window))
    }
}