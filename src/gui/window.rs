use crate::{
    gui::{
        event::{
            UIEventKind,
            UIEvent
        },
        border::BorderBuilder,
        UINode,
        UserInterface,
        grid::{
            GridBuilder,
            Column,
            Row
        },
        HorizontalAlignment,
        text::TextBuilder,
        Thickness,
        button::ButtonBuilder,
        scroll_viewer::{
            ScrollViewerBuilder,
            ScrollViewer
        },
        widget::{
            Widget,
            WidgetBuilder
        },
        Visibility,
        bool_to_visibility,
        Control,
        ControlTemplate,
        UINodeContainer,
        Builder
    },
    core::{
        color::Color,
        pool::Handle,
        math::vec2::Vec2,
    },
};
use std::collections::HashMap;

/// Represents a widget looking as window in Windows - with title, minimize and close buttons.
/// It has scrollable region for content, content can be any desired node or even other window.
/// Window can be dragged by its title.
pub struct Window {
    widget: Widget,
    mouse_click_pos: Vec2,
    initial_position: Vec2,
    is_dragged: bool,
    minimized: bool,
    can_minimize: bool,
    can_close: bool,
    header: Handle<UINode>,
    minimize_button: Handle<UINode>,
    close_button: Handle<UINode>,
    scroll_viewer: Handle<UINode>,
}

impl Control for Window {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }

    fn raw_copy(&self) -> Box<dyn Control> {
        Box::new(Self {
            widget: *self.widget.raw_copy().downcast::<Widget>().unwrap_or_else(|_| panic!()),
            mouse_click_pos: self.mouse_click_pos,
            initial_position: self.initial_position,
            is_dragged: self.is_dragged,
            minimized: self.minimized,
            can_minimize: self.can_minimize,
            can_close: self.can_close,
            header: self.header,
            minimize_button: self.minimize_button,
            close_button: self.close_button,
            scroll_viewer: self.scroll_viewer
        })
    }

    fn resolve(&mut self, _: &ControlTemplate, node_map: &HashMap<Handle<UINode>, Handle<UINode>>) {
        self.header = *node_map.get(&self.header).unwrap();
        self.minimize_button = *node_map.get(&self.minimize_button).unwrap();
        self.close_button = *node_map.get(&self.close_button).unwrap();
        self.scroll_viewer = *node_map.get(&self.scroll_viewer).unwrap();
    }

    fn handle_event(&mut self, self_handle: Handle<UINode>, ui: &mut UserInterface, evt: &mut UIEvent) {
        if evt.source == self.header {
            match evt.kind {
                UIEventKind::MouseDown { pos, .. } => {
                    ui.capture_mouse(self.header);
                    let initial_position = self.widget().actual_local_position.get();
                    self.mouse_click_pos = pos;
                    self.initial_position = initial_position;
                    self.is_dragged = true;
                    evt.handled = true;
                }
                UIEventKind::MouseUp { .. } => {
                    ui.release_mouse_capture();
                    self.is_dragged = false;
                    evt.handled = true;
                }
                UIEventKind::MouseMove { pos, .. } => {
                    if self.is_dragged {
                        self.widget.set_desired_local_position(self.initial_position + pos - self.mouse_click_pos);
                    }
                    evt.handled = true;
                }
                _ => ()
            }
        }

        if evt.source == self.minimize_button {
            if let UIEventKind::Click = evt.kind {
                self.minimize(!self.minimized);
            }
        }

        if evt.source == self.close_button {
            if let UIEventKind::Click = evt.kind {
                self.close();
            }
        }

        if evt.source == self_handle || evt.target == self_handle {
            match evt.kind {
                UIEventKind::Opened => {
                    self.widget.set_visibility(Visibility::Visible);
                }
                UIEventKind::Closed => {
                    self.widget.set_visibility(Visibility::Collapsed);
                }
                UIEventKind::Minimized(minimized) => {
                    self.minimized = minimized;
                    let scroll_viewer = ui.node_mut(self.scroll_viewer).downcast_mut::<ScrollViewer>().unwrap();
                    let visibility = if !minimized { Visibility::Visible } else { Visibility::Collapsed };
                    scroll_viewer.widget_mut().set_visibility(visibility);
                }
                UIEventKind::CanMinimizeChanged(value) => {
                    self.can_minimize = value;
                    ui.node_mut(self.minimize_button)
                        .widget_mut()
                        .set_visibility(bool_to_visibility(value));
                }
                UIEventKind::CanCloseChanged(value) => {
                    self.can_close = value;
                    ui.node_mut(self.close_button)
                        .widget_mut()
                        .set_visibility(bool_to_visibility(value));
                }
                _ => ()
            }
        }
    }
}

impl Window {
    pub fn new(
        widget: Widget,
        header: Handle<UINode>,
        minimize_button: Handle<UINode>,
        close_button: Handle<UINode>,
        scroll_viewer: Handle<UINode>,
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
            scroll_viewer
        }
    }

    pub fn close(&mut self) {
        self.widget
            .events
            .borrow_mut()
            .push_back(UIEvent::new(UIEventKind::Closed));
    }

    pub fn open(&mut self) {
        self.widget
            .events
            .borrow_mut()
            .push_back(UIEvent::new(UIEventKind::Opened));
    }

    pub fn minimize(&mut self, state: bool) {
        self.widget
            .events
            .borrow_mut()
            .push_back(UIEvent::new(UIEventKind::Minimized(state)));
    }

    pub fn can_close(&mut self, state: bool) {
        self.widget
            .events
            .borrow_mut()
            .push_back(UIEvent::new(UIEventKind::CanCloseChanged(state)));
    }

    pub fn can_minimize(&mut self, state: bool) {
        self.widget
            .events
            .borrow_mut()
            .push_back(UIEvent::new(UIEventKind::CanMinimizeChanged(state)));
    }
}

pub struct WindowBuilder<'a> {
    widget_builder: WidgetBuilder,
    content: Handle<UINode>,
    title: Option<WindowTitle<'a>>,
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
pub enum WindowTitle<'a> {
    Text(&'a str),
    Node(Handle<UINode>),
}

impl<'a> WindowBuilder<'a> {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            content: Handle::NONE,
            title: None,
            can_close: true,
            can_minimize: true,
            open: true,
        }
    }

    pub fn with_content(mut self, content: Handle<UINode>) -> Self {
        self.content = content;
        self
    }

    pub fn with_title(mut self, title: WindowTitle<'a>) -> Self {
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

impl Builder for WindowBuilder<'_> {
    fn build(self, ui: &mut dyn UINodeContainer) -> Handle<UINode> {
        let minimize_button;
        let close_button;

        let header = BorderBuilder::new(WidgetBuilder::new()
            .with_background(Color::opaque(120, 120, 120))
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
                        .with_visibility(if self.can_minimize { Visibility::Visible } else { Visibility::Collapsed })
                        .with_margin(Thickness::uniform(2.0)))
                        .with_text("_")
                        .build(ui);
                    minimize_button
                })
                .with_child({
                    close_button = ButtonBuilder::new(WidgetBuilder::new()
                        .on_row(0)
                        .on_column(2)
                        .with_visibility(if self.can_close { Visibility::Visible } else { Visibility::Collapsed })
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
                .with_visibility(if self.open { Visibility::Visible } else { Visibility::Collapsed })
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .with_child(GridBuilder::new(WidgetBuilder::new()
                        .with_child(scroll_viewer)
                        .with_child(header))
                        .add_column(Column::stretch())
                        .add_row(Row::auto())
                        .add_row(Row::stretch())
                        .build(ui))
                    .with_background(Color::opaque(100, 100, 100)))
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
            scroll_viewer
        };
        ui.add_node(Box::new(window))
    }
}