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
    scroll_viewer::ScrollViewerBuilder,
    widget::{
        Widget,
        WidgetBuilder,
    },
    Control,
    core::{
        pool::Handle,
        math::vec2::Vec2,
        color::Color,
    },
    message::{
        WidgetMessage,
        ButtonMessage,
        WindowMessage,
    },
    brush::{
        Brush,
        GradientPoint,
    },
    NodeHandleMapping,
};
use std::ops::{Deref, DerefMut};

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

impl<M: 'static, C: 'static + Control<M, C>> Deref for Window<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for Window<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for Window<M, C> {
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

    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.header = *node_map.get(&self.header).unwrap();
        self.minimize_button = *node_map.get(&self.minimize_button).unwrap();
        self.close_button = *node_map.get(&self.close_button).unwrap();
        self.scroll_viewer = *node_map.get(&self.scroll_viewer).unwrap();
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_routed_message(ui, message);

        match &message.data {
            UiMessageData::Widget(msg) => {
                if (message.destination == self.header || ui.node(self.header).has_descendant(message.destination, ui))
                    && message.destination != self.close_button && message.destination != self.minimize_button {
                    match msg {
                        WidgetMessage::MouseDown { pos, .. } => {
                            self.send_message(UiMessage {
                                data: UiMessageData::Widget(WidgetMessage::TopMost),
                                destination: self.handle,
                                handled: false,
                            });
                            ui.capture_mouse(self.header);
                            let initial_position = self.actual_local_position();
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
                        WidgetMessage::MouseMove { pos, .. } => {
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
                    if message.destination == self.minimize_button {
                        self.minimize(!self.minimized);
                    } else if message.destination == self.close_button {
                        self.close();
                    }
                }
            }
            UiMessageData::Window(msg) => {
                if message.destination == self.handle {
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
                                        scroll_viewer.set_visibility(!*minimized);
                                    }
                                }
                            }
                        }
                        WindowMessage::CanMinimize(value) => {
                            if self.can_minimize != *value {
                                self.can_minimize = *value;
                                self.widget.invalidate_layout();
                                if self.minimize_button.is_some() {
                                    ui.node_mut(self.minimize_button).set_visibility(*value);
                                }
                            }
                        }
                        WindowMessage::CanClose(value) => {
                            if self.can_close != *value {
                                self.can_close = *value;
                                self.widget.invalidate_layout();
                                if self.close_button.is_some() {
                                    ui.node_mut(self.close_button).set_visibility(*value);
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
        self.invalidate_layout();
        self.send_message(UiMessage {
            data: UiMessageData::Window(WindowMessage::Closed),
            destination: self.handle,
            handled: false,
        });
    }

    pub fn open(&mut self) {
        self.invalidate_layout();
        self.send_message(UiMessage {
            data: UiMessageData::Window(WindowMessage::Opened),
            destination: self.handle,
            handled: false,
        });
    }

    pub fn minimize(&mut self, state: bool) {
        self.invalidate_layout();
        self.send_message(UiMessage {
            data: UiMessageData::Window(WindowMessage::Minimized(state)),
            destination: self.handle,
            handled: false,
        });
    }

    pub fn set_can_close(&mut self, state: bool) {
        self.invalidate_layout();
        self.send_message(UiMessage {
            data: UiMessageData::Window(WindowMessage::CanClose(state)),
            destination: self.handle,
            handled: false,
        });
    }

    pub fn set_can_minimize(&mut self, state: bool) {
        self.invalidate_layout();
        self.send_message(UiMessage {
            data: UiMessageData::Window(WindowMessage::CanMinimize(state)),
            destination: self.handle,
            handled: false,
        });
    }
}

pub struct WindowBuilder<'a, M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    content: Handle<UINode<M, C>>,
    title: Option<WindowTitle<'a, M, C>>,
    can_close: bool,
    can_minimize: bool,
    open: bool,
    scroll_viewer: Option<Handle<UINode<M, C>>>,
    close_button: Option<Handle<UINode<M, C>>>,
    minimize_button: Option<Handle<UINode<M, C>>>,
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
            scroll_viewer: None,
            close_button: None,
            minimize_button: None,
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

    pub fn with_scroll_scroll_viewer(mut self, sv: Handle<UINode<M, C>>) -> Self {
        self.scroll_viewer = Some(sv);
        self
    }

    pub fn with_minimize_button(mut self, button: Handle<UINode<M, C>>) -> Self {
        self.minimize_button = Some(button);
        self
    }

    pub fn with_close_button(mut self, button: Handle<UINode<M, C>>) -> Self {
        self.close_button = Some(button);
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

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        let minimize_button;
        let close_button;

        let header = BorderBuilder::new(WidgetBuilder::new()
            .with_horizontal_alignment(HorizontalAlignment::Stretch)
            .with_height(30.0)
            .with_background(Brush::LinearGradient {
                from: Vec2::new(0.5, 0.0),
                to: Vec2::new(0.5, 1.0),
                stops: vec![
                    GradientPoint { stop: 0.0, color: Color::opaque(85, 85, 85) },
                    GradientPoint { stop: 0.5, color: Color::opaque(65, 65, 65) },
                    GradientPoint { stop: 1.0, color: Color::opaque(75, 75, 75) },
                ],
            })
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
                    minimize_button = self.minimize_button.unwrap_or_else(|| {
                        ButtonBuilder::new(WidgetBuilder::new()
                            .with_margin(Thickness::uniform(2.0)))
                            .with_text("_")
                            .build(ui)
                    });
                    ui.node_mut(minimize_button)
                        .set_visibility(self.can_minimize)
                        .set_width_mut(30.0)
                        .set_row(0)
                        .set_column(1);
                    minimize_button
                })
                .with_child({
                    close_button = self.close_button.unwrap_or_else(|| {
                        ButtonBuilder::new(WidgetBuilder::new()
                            .with_margin(Thickness::uniform(2.0)))
                            .with_text("X")
                            .build(ui)
                    });
                    ui.node_mut(close_button)
                        .set_width_mut(30.0)
                        .set_visibility(self.can_close)
                        .set_row(0)
                        .set_column(2);
                    close_button
                }))
                .add_column(Column::stretch())
                .add_column(Column::auto())
                .add_column(Column::auto())
                .add_row(Row::stretch())
                .build(ui))
            .on_row(0)
        ).build(ui);

        let scroll_viewer = self.scroll_viewer.unwrap_or_else(|| {
            ScrollViewerBuilder::new(WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0)))
                .build(ui)
        });

        if let UINode::ScrollViewer(sv) = ui.node_mut(scroll_viewer) {
            sv.set_content(self.content).set_row(1);
        }

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
                .build(ui.sender()),
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

        let handle = ui.add_node(UINode::Window(window));

        ui.flush_messages();

        handle
    }
}