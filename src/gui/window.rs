use crate::{
    utils::pool::Handle,
    gui::{
        border::BorderBuilder,
        node::{UINode, UINodeKind},
        builder::{CommonBuilderFields, GenericNodeBuilder},
        UserInterface,
        draw::Color,
        grid::{GridBuilder, Column, Row},
        HorizontalAlignment,
        event::{
            RoutedEventHandlerType,
            RoutedEventHandler,
            RoutedEventKind,
        },
        text::TextBuilder,
        Thickness,
        button::ButtonBuilder,
        scroll_content_presenter::ScrollContentPresenterBuilder,
        VerticalAlignment,
    },
    math::vec2::Vec2,
};

pub type ClosingEventHandler = dyn FnMut(&mut UserInterface, Handle<UINode>) -> bool;
pub type MinimizingEventHandler = dyn FnMut(&mut UserInterface, Handle<UINode>) -> bool;
pub type ClosedEventHandler = dyn FnMut(&mut UserInterface, Handle<UINode>);
pub type MinimizedEventHandler = dyn FnMut(&mut UserInterface, Handle<UINode>);

/// Represents a widget looking as window in Windows - with title, minimize and close buttons.
/// It has scrollable region for content, content can be any desired node or even other window.
/// Window can be dragged by its title.
pub struct Window {
    pub(in crate::gui) owner_handle: Handle<UINode>,
    mouse_click_pos: Vec2,
    initial_position: Vec2,
    is_dragged: bool,
    /// Invoked when user clicks on minimize button to decide should window be minimized.
    minimizing_handler: Option<Box<MinimizingEventHandler>>,
    /// Invoked when user clicks on close button to decide should window be closed.
    closing_handler: Option<Box<ClosingEventHandler>>,
    /// Invoked when window is closed.
    closed_handler: Option<Box<ClosedEventHandler>>,
    /// Invoked when windows is minimized.
    minimized_hanlder: Option<Box<MinimizedEventHandler>>
}

pub struct WindowBuilder<'a> {
    common: CommonBuilderFields,
    content: Handle<UINode>,
    title: Option<WindowTitle<'a>>,
    minimizing_handler: Option<Box<MinimizingEventHandler>>,
    closing_handler: Option<Box<ClosingEventHandler>>,
    closed_handler: Option<Box<ClosedEventHandler>>,
    minimized_handler: Option<Box<MinimizedEventHandler>>
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
    pub fn new() -> Self {
        Self {
            common: CommonBuilderFields::new(),
            content: Handle::none(),
            title: None,
            minimizing_handler: None,
            closing_handler: None,
            closed_handler: None,
            minimized_handler: None
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

    pub fn with_minimizing_handler(mut self, handler: Box<MinimizingEventHandler>) -> Self{
        self.minimizing_handler = Some(handler);
        self
    }

    pub fn with_minimized_handler(mut self, handler: Box<MinimizedEventHandler>) -> Self{
        self.minimized_handler = Some(handler);
        self
    }

    pub fn with_closing_handler(mut self, handler: Box<ClosingEventHandler>) -> Self{
        self.closing_handler = Some(handler);
        self
    }

    pub fn with_closed_handler(mut self, handler: Box<ClosedEventHandler>) -> Self{
        self.closed_handler = Some(handler);
        self
    }

    pub fn build(self, ui: &mut UserInterface) -> Handle<UINode> {
        let window = Window {
            owner_handle: Default::default(),
            mouse_click_pos: Vec2::zero(),
            initial_position: Vec2::zero(),
            is_dragged: false,
            minimizing_handler: None,
            closing_handler: None,
            closed_handler: None,
            minimized_hanlder: None
        };

        GenericNodeBuilder::new(UINodeKind::Window(window), self.common)
            .with_child(BorderBuilder::new()
                .with_color(Color::opaque(120, 120, 120))
                .with_child(GridBuilder::new()
                    .add_column(Column::stretch())
                    .add_row(Row::auto())
                    .add_row(Row::stretch())
                    .with_child(ScrollContentPresenterBuilder::new()
                        .with_content(self.content)
                        .on_row(1)
                        .build(ui))
                    .with_child(BorderBuilder::new()
                        .with_color(Color::opaque(120, 120, 120))
                        .on_row(0)
                        .with_horizontal_alignment(HorizontalAlignment::Stretch)
                        .with_height(30.0)
                        .with_handler(RoutedEventHandlerType::MouseDown, Box::new(|ui, handle, evt| {
                            ui.capture_mouse(handle);
                            if let RoutedEventKind::MouseDown { pos, .. } = evt.kind {
                                if let Some(window_node) = ui.borrow_by_criteria_up_mut(handle, |node| {
                                    if let UINodeKind::Window(_) = node.get_kind() { true } else { false }
                                }) {
                                    let initial_position = window_node.actual_local_position.get();
                                    if let UINodeKind::Window(window) = window_node.get_kind_mut() {
                                        window.mouse_click_pos = pos;
                                        window.initial_position = initial_position;
                                        window.is_dragged = true;
                                        evt.handled = true;
                                    }
                                }
                            }
                        }))
                        .with_handler(RoutedEventHandlerType::MouseUp, Box::new(|ui, handle, evt| {
                            ui.release_mouse_capture();
                            if let Some(window_node) = ui.borrow_by_criteria_up_mut(handle, |node| {
                                if let UINodeKind::Window(_) = node.get_kind() { true } else { false }
                            }) {
                                if let UINodeKind::Window(window) = window_node.get_kind_mut() {
                                    window.is_dragged = false;
                                    evt.handled = true;
                                }
                            }
                        }))
                        .with_handler(RoutedEventHandlerType::MouseMove, Box::new(|ui, handle, evt| {
                            if let RoutedEventKind::MouseMove { pos, .. } = evt.kind {
                                if let Some(window_node) = ui.borrow_by_criteria_up_mut(handle, |node| {
                                    if let UINodeKind::Window(_) = node.get_kind() { true } else { false }
                                }) {
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
                            }
                        }))
                        .with_child(GridBuilder::new()
                            .add_column(Column::stretch())
                            .add_column(Column::strict(30.0))
                            .add_column(Column::strict(30.0))
                            .add_row(Row::stretch())
                            .with_child(TextBuilder::new()
                                .with_text("Unnamed window")
                                .with_margin(Thickness::uniform(5.0))
                                .on_row(0)
                                .on_column(0)
                                .build(ui))
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