use rg3d_core::{
    color::Color,
    pool::Handle,
    math::vec2::Vec2,
};
use crate::gui::{
    event::{UIEvent, UIEventKind},
    border::BorderBuilder,
    node::{UINode, UINodeKind},
    builder::{CommonBuilderFields, GenericNodeBuilder},
    UserInterface,
    grid::{GridBuilder, Column, Row},
    HorizontalAlignment,
    text::TextBuilder,
    Thickness,
    button::ButtonBuilder,
    EventSource,
    scroll_viewer::ScrollViewerBuilder,
};

/// Represents a widget looking as window in Windows - with title, minimize and close buttons.
/// It has scrollable region for content, content can be any desired node or even other window.
/// Window can be dragged by its title.
pub struct Window {
    mouse_click_pos: Vec2,
    initial_position: Vec2,
    is_dragged: bool,
}

pub struct WindowBuilder<'a> {
    common: CommonBuilderFields,
    content: Handle<UINode>,
    title: Option<WindowTitle<'a>>,
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

impl<'a> Default for WindowBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> WindowBuilder<'a> {
    pub fn new() -> Self {
        Self {
            common: CommonBuilderFields::new(),
            content: Handle::NONE,
            title: None,
        }
    }

    impl_default_builder_methods!();

    pub fn with_content(mut self, content: Handle<UINode>) -> Self {
        self.content = content;
        self
    }

    pub fn with_title(mut self, title: WindowTitle<'a>) -> Self {
        self.title = Some(title);
        self
    }

    pub fn build(self, ui: &mut UserInterface) -> Handle<UINode> {
        let window = Window {
            mouse_click_pos: Vec2::zero(),
            initial_position: Vec2::zero(),
            is_dragged: false,
        };

        GenericNodeBuilder::new(UINodeKind::Window(window), self.common)
            .with_child(BorderBuilder::new()
                .with_color(Color::opaque(100, 100, 100))
                .with_child(GridBuilder::new()
                    .add_column(Column::stretch())
                    .add_row(Row::auto())
                    .add_row(Row::stretch())
                    .with_child(ScrollViewerBuilder::new()
                        .with_content(self.content)
                        .on_row(1)
                        .build(ui))
                    .with_child(BorderBuilder::new()
                        .with_color(Color::opaque(120, 120, 120))
                        .on_row(0)
                        .with_horizontal_alignment(HorizontalAlignment::Stretch)
                        .with_height(30.0)
                        .with_event_handler(Box::new(|ui, handle, evt| {
                            if evt.source == handle {
                                match evt.kind {
                                    UIEventKind::MouseDown { pos, .. } => {
                                        ui.capture_mouse(handle);
                                        let window_node = ui.borrow_by_criteria_up_mut(handle, |node| node.is_window());
                                        let initial_position = window_node.actual_local_position.get();
                                        let window = window_node.as_window_mut();
                                        window.mouse_click_pos = pos;
                                        window.initial_position = initial_position;
                                        window.is_dragged = true;
                                        evt.handled = true;
                                    }
                                    UIEventKind::MouseUp { .. } => {
                                        ui.release_mouse_capture();
                                        let window_node = ui.borrow_by_criteria_up_mut(handle, |node| node.is_window());
                                        window_node.as_window_mut().is_dragged = false;
                                        evt.handled = true;
                                    }
                                    UIEventKind::MouseMove { pos, .. } => {
                                        let window_node = ui.borrow_by_criteria_up_mut(handle, |node| node.is_window());
                                        let new_pos = if let UINodeKind::Window(window) = window_node.get_kind_mut() {
                                            if window.is_dragged {
                                                window.initial_position + pos - window.mouse_click_pos
                                            } else {
                                                return;
                                            }
                                        } else {
                                            return;
                                        };
                                        window_node.set_desired_local_position(new_pos);
                                        evt.handled = true;
                                    }
                                    _ => ()
                                }
                            }
                        }))
                        .with_child(GridBuilder::new()
                            .add_column(Column::stretch())
                            .add_column(Column::strict(30.0))
                            .add_column(Column::strict(30.0))
                            .add_row(Row::stretch())
                            .with_child({
                                match self.title {
                                    None => Handle::NONE,
                                    Some(window_title) => {
                                        match window_title {
                                            WindowTitle::Node(node) => node,
                                            WindowTitle::Text(text) => {
                                                TextBuilder::new()
                                                    .with_text(text)
                                                    .with_margin(Thickness::uniform(5.0))
                                                    .on_row(0)
                                                    .on_column(0)
                                                    .build(ui)
                                            }
                                        }
                                    }
                                }
                            })
                            .with_child(ButtonBuilder::new()
                                .on_row(0)
                                .on_column(1)
                                .with_margin(Thickness::uniform(2.0))
                                .with_text("_")
                                .build(ui))
                            .with_child(ButtonBuilder::new()
                                .on_row(0)
                                .on_column(2)
                                .with_margin(Thickness::uniform(2.0))
                                .with_text("X")
                                .build(ui))
                            .build(ui))
                        .build(ui))
                    .build(ui))
                .build(ui))
            .build(ui)
    }
}

impl EventSource for Window {
    fn emit_event(&mut self) -> Option<UIEvent> {
        None
    }
}