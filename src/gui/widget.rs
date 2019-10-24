use std::{
    cell::Cell,
    collections::VecDeque,
};
use rg3d_core::{
    color::Color,
    math::vec2::Vec2,
    pool::Handle,
    math::Rect,
};
use crate::gui::{
    VerticalAlignment,
    HorizontalAlignment,
    Thickness,
    Visibility,
    UserInterface,
    Layout,
    Draw,
    node::UINode,
    event::{UIEventHandler, UIEvent},
    draw::DrawingContext,
    Update
};
use std::cell::RefCell;

pub trait AsWidget {
    fn widget(&self) -> &Widget;
    fn widget_mut(&mut self) -> &mut Widget;
}

pub struct Widget {
    pub(in crate::gui) name: String,
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
    pub(in crate::gui) measure_valid: Cell<bool>,
    pub(in crate::gui) arrange_valid: Cell<bool>,
    pub(in crate::gui) event_handlers: Vec<Box<UIEventHandler>>,
    pub(in crate::gui) events: RefCell<VecDeque<UIEvent>>,
}

impl Default for Widget {
    fn default() -> Self {
        WidgetBuilder::new().build()
    }
}

impl Update for Widget {
    fn update(&mut self, _dt: f32) {

    }
}

impl Layout for Widget {
    fn measure_override(&self, ui: &UserInterface, available_size: Vec2) -> Vec2 {
        let mut size = Vec2::ZERO;

        for child_handle in self.children.iter() {
            ui.measure(*child_handle, available_size);

            let child = ui.get_node(*child_handle).widget();
            let child_desired_size = child.desired_size.get();
            if child_desired_size.x > size.x {
                size.x = child_desired_size.x;
            }
            if child_desired_size.y > size.y {
                size.y = child_desired_size.y;
            }
        }

        size
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vec2) -> Vec2 {
        let final_rect = Rect::new(0.0, 0.0, final_size.x, final_size.y);

        for child_handle in self.children.iter() {
            ui.arrange(*child_handle, &final_rect);
        }

        final_size
    }
}

impl AsWidget for Widget {
    fn widget(&self) -> &Widget {
        self
    }

    fn widget_mut(&mut self) -> &mut Widget {
        self
    }
}

impl Draw for Widget {
    fn draw(&mut self, _: &mut DrawingContext) {
        // Nothing to do.
    }
}

impl Widget {
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
    pub fn set_vertical_alignment(&mut self, valign: VerticalAlignment) {
        self.vertical_alignment = valign;
    }

    #[inline]
    pub fn set_horizontal_alignment(&mut self, halign: HorizontalAlignment) {
        self.horizontal_alignment = halign;
    }

    #[inline]
    pub fn get_screen_bounds(&self) -> Rect<f32> {
        Rect::new(self.screen_position.x, self.screen_position.y, self.actual_size.get().x, self.actual_size.get().y)
    }

    #[inline]
    pub fn set_visibility(&mut self, visibility: Visibility) {
        self.visibility = visibility;
    }

    #[inline]
    pub fn get_visibility(&self) -> Visibility {
        self.visibility
    }
}

pub struct WidgetBuilder {
    name: Option<String>,
    width: Option<f32>,
    height: Option<f32>,
    desired_position: Option<Vec2>,
    vertical_alignment: Option<VerticalAlignment>,
    horizontal_alignment: Option<HorizontalAlignment>,
    max_size: Option<Vec2>,
    min_size: Option<Vec2>,
    color: Option<Color>,
    row: Option<usize>,
    column: Option<usize>,
    margin: Option<Thickness>,
    children: Vec<Handle<UINode>>,
    event_handlers: Vec<Box<UIEventHandler>>,
}

impl Default for WidgetBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetBuilder {
    pub fn new() -> Self {
        Self {
            name: None,
            width: None,
            height: None,
            vertical_alignment: None,
            horizontal_alignment: None,
            max_size: None,
            min_size: None,
            color: None,
            row: None,
            column: None,
            margin: None,
            desired_position: None,
            children: Vec::new(),
            event_handlers: Vec::new(),
        }
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn with_height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    pub fn with_vertical_alignment(mut self, valign: VerticalAlignment) -> Self {
        self.vertical_alignment = Some(valign);
        self
    }

    pub fn with_horizontal_alignment(mut self, halign: HorizontalAlignment) -> Self {
        self.horizontal_alignment = Some(halign);
        self
    }

    pub fn with_max_size(mut self, max_size: Vec2) -> Self {
        self.max_size = Some(max_size);
        self
    }

    pub fn with_min_size(mut self, min_size: Vec2) -> Self {
        self.min_size = Some(min_size);
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn on_row(mut self, row: usize) -> Self {
        self.row = Some(row);
        self
    }

    pub fn on_column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }

    pub fn with_margin(mut self, margin: Thickness) -> Self {
        self.margin = Some(margin);
        self
    }

    pub fn with_desired_position(mut self, desired_position: Vec2) -> Self {
        self.desired_position = Some(desired_position);
        self
    }

    pub fn with_child(mut self, handle: Handle<UINode>) -> Self {
        if handle.is_some() {
            self.children.push(handle);
        }
        self
    }

    pub fn with_children(mut self, children: &[Handle<UINode>]) -> Self {
        for child in children {
            self.children.push(*child)
        }
        self
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = Some(String::from(name));
        self
    }

    pub fn with_event_handler(mut self, handler: Box<UIEventHandler>) -> Self {
        self.event_handlers.push(handler);
        self
    }

    pub fn build(self) -> Widget {
        Widget {
            name: self.name.unwrap_or_default(),
            desired_local_position: Cell::new(self.desired_position.unwrap_or(Vec2::ZERO)),
            width: Cell::new(self.width.unwrap_or(std::f32::NAN)),
            height: Cell::new(self.height.unwrap_or(std::f32::NAN)),
            screen_position: Vec2::ZERO,
            desired_size: Cell::new(Vec2::ZERO),
            actual_local_position: Cell::new(Vec2::ZERO),
            actual_size: Cell::new(Vec2::ZERO),
            min_size: self.min_size.unwrap_or(Vec2::ZERO),
            max_size: self.max_size.unwrap_or_else(|| Vec2::new(std::f32::INFINITY, std::f32::INFINITY)),
            color: self.color.unwrap_or(Color::WHITE),
            row: self.row.unwrap_or(0),
            column: self.column.unwrap_or(0),
            vertical_alignment: self.vertical_alignment.unwrap_or(VerticalAlignment::Stretch),
            horizontal_alignment: self.horizontal_alignment.unwrap_or(HorizontalAlignment::Stretch),
            margin: self.margin.unwrap_or_else(Thickness::zero),
            visibility: Visibility::Visible,
            children: self.children,
            parent: Handle::NONE,
            command_indices: Vec::new(),
            is_mouse_over: false,
            measure_valid: Cell::new(false),
            arrange_valid: Cell::new(false),
            event_handlers: self.event_handlers,
            events: RefCell::new(VecDeque::new()),
        }
    }
}