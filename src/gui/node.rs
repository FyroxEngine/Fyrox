use std::{
    cell::Cell,
    any::{Any, TypeId},
};
use crate::{
    math::{vec2::Vec2, Rect},
    utils::pool::Handle,
    gui::{
        button::Button,
        Canvas,
        text::Text,
        draw::Color,
        VerticalAlignment,
        HorizontalAlignment,
        Thickness,
        Visibility,
        border::Border,
        scroll_bar::ScrollBar,
        scroll_viewer::ScrollViewer,
        image::Image,
        grid::Grid,
        scroll_content_presenter::ScrollContentPresenter,
        event::{
            RoutedEventHandlerType,
            RoutedEventHandler,
            RoutedEventHandlerList,
        },
        window::Window,
    },
};

pub enum UINodeKind {
    Text(Text),
    Border(Border),
    Button(Button),
    ScrollBar(ScrollBar),
    ScrollViewer(ScrollViewer),
    Image(Image),
    /// Automatically arranges children by rows and columns
    Grid(Grid),
    /// Allows user to directly set position and size of a node
    Canvas(Canvas),
    /// Allows user to scroll content
    ScrollContentPresenter(ScrollContentPresenter),
    Window(Window),
}

/// Notes. Some fields wrapped into Cell's to be able to modify them while in measure/arrange
/// stage. This is required evil, I can't just unwrap all the recursive calls in measure/arrange.
pub struct UINode {
    pub(in crate::gui) name: String,
    pub(in crate::gui) kind: UINodeKind,
    /// Desired position relative to parent node
    pub(in crate::gui) desired_local_position: Cell<Vec2>,
    /// Explicit width for node or automatic if NaN (means value is undefined). Default is NaN
    pub(in crate::gui) width: Cell<f32>,
    /// Explicit height for node or automatic if NaN (means value is undefined). Default is NaN
    pub(in crate::gui) height: Cell<f32>,
    /// Screen position of the node
    pub(in crate::gui) screen_position: Vec2,
    /// Desired size of the node after Measure pass.
    pub(in crate::gui) desired_size: Cell<Vec2>,
    /// Actual node local position after Arrange pass.
    pub(in crate::gui) actual_local_position: Cell<Vec2>,
    /// Actual size of the node after Arrange pass.
    pub(in crate::gui) actual_size: Cell<Vec2>,
    /// Minimum width and height
    pub(in crate::gui) min_size: Vec2,
    /// Maximum width and height
    pub(in crate::gui) max_size: Vec2,
    /// Overlay color of the node
    pub(in crate::gui) color: Color,
    /// Index of row to which this node belongs
    pub(in crate::gui) row: usize,
    /// Index of column to which this node belongs
    pub(in crate::gui) column: usize,
    /// Vertical alignment
    pub(in crate::gui) vertical_alignment: VerticalAlignment,
    /// Horizontal alignment
    pub(in crate::gui) horizontal_alignment: HorizontalAlignment,
    /// Margin (four sides)
    pub(in crate::gui) margin: Thickness,
    /// Current visibility state
    pub(in crate::gui) visibility: Visibility,
    pub(in crate::gui) children: Vec<Handle<UINode>>,
    pub(in crate::gui) parent: Handle<UINode>,
    /// Indices of commands in command buffer emitted by the node.
    pub(in crate::gui) command_indices: Vec<usize>,
    pub(in crate::gui) is_mouse_over: bool,
    pub(in crate::gui) event_handlers: RoutedEventHandlerList,
    pub(in crate::gui) measure_valid: Cell<bool>,
    pub(in crate::gui) arrange_valid: Cell<bool>,
}

impl UINode {
    pub fn new(kind: UINodeKind) -> UINode {
        UINode {
            kind,
            name: String::new(),
            desired_local_position: Cell::new(Vec2::new()),
            width: Cell::new(std::f32::NAN),
            height: Cell::new(std::f32::NAN),
            screen_position: Vec2::new(),
            desired_size: Cell::new(Vec2::new()),
            actual_local_position: Cell::new(Vec2::new()),
            actual_size: Cell::new(Vec2::new()),
            min_size: Vec2::make(0.0, 0.0),
            max_size: Vec2::make(std::f32::INFINITY, std::f32::INFINITY),
            color: Color::white(),
            row: 0,
            column: 0,
            vertical_alignment: VerticalAlignment::Stretch,
            horizontal_alignment: HorizontalAlignment::Stretch,
            margin: Thickness::zero(),
            visibility: Visibility::Visible,
            children: Vec::new(),
            parent: Handle::none(),
            command_indices: Vec::new(),
            event_handlers: Default::default(),
            is_mouse_over: false,
            measure_valid: Cell::new(false),
            arrange_valid: Cell::new(false),
        }
    }

    #[inline]
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    #[inline]
    pub fn set_width(&mut self, width: f32) {
        self.width.set(width);
    }

    #[inline]
    pub fn set_height(&mut self, height: f32) {
        self.height.set(height);
    }

    #[inline]
    pub fn set_desired_local_position(&self, pos: Vec2) {
        self.desired_local_position.set(pos);
    }

    #[inline]
    pub fn get_kind(&self) -> &UINodeKind {
        &self.kind
    }

    #[inline]
    pub fn set_vertical_alignment(&mut self, valign: VerticalAlignment) {
        self.vertical_alignment = valign;
    }

    #[inline]
    pub fn set_horizontal_alignment(&mut self, halign: HorizontalAlignment) {
        self.horizontal_alignment = halign;
    }

    #[inline]
    pub fn get_kind_mut(&mut self) -> &mut UINodeKind {
        &mut self.kind
    }

    #[inline]
    pub fn get_screen_bounds(&self) -> Rect<f32> {
        Rect::new(self.screen_position.x, self.screen_position.y, self.actual_size.get().x, self.actual_size.get().y)
    }

    #[inline]
    pub fn set_handler(&mut self, handler_type: RoutedEventHandlerType, handler: Box<RoutedEventHandler>) {
        self.event_handlers[handler_type as usize] = Some(handler);
    }

    #[inline]
    pub fn set_visibility(&mut self, visibility: Visibility) {
        self.visibility = visibility;
    }

    #[inline]
    pub fn get_visibility(&self) -> Visibility {
        self.visibility
    }

    #[inline]
    pub fn get_kind_id(&self) -> TypeId {
        match &self.kind {
            UINodeKind::ScrollBar(scroll_bar) => scroll_bar.type_id(),
            UINodeKind::Text(text) => text.type_id(),
            UINodeKind::Border(border) => border.type_id(),
            UINodeKind::Button(button) => button.type_id(),
            UINodeKind::ScrollViewer(scroll_viewer) => scroll_viewer.type_id(),
            UINodeKind::Image(image) => image.type_id(),
            UINodeKind::Grid(grid) => grid.type_id(),
            UINodeKind::Canvas(canvas) => canvas.type_id(),
            UINodeKind::ScrollContentPresenter(scp) => scp.type_id(),
            UINodeKind::Window(window) => window.type_id()
        }
    }
}