use crate::core::{
    color::Color,
    math::{
        vec2::Vec2,
        Rect,
    },
    pool::Handle,
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
    Update,
    Styleable,
    style::Style,
};
use std::{
    cell::{
        RefCell,
        Cell,
    },
    collections::VecDeque,
    any::Any,
    rc::Rc
};

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
    color: Color,
    /// Index of row to which this node belongs
    row: usize,
    /// Index of column to which this node belongs
    column: usize,
    /// Vertical alignment
    pub(in crate::gui) vertical_alignment: VerticalAlignment,
    /// Horizontal alignment
    pub(in crate::gui) horizontal_alignment: HorizontalAlignment,
    /// Margin (four sides)
    pub(in crate::gui) margin: Thickness,
    /// Current visibility state
    pub(in crate::gui) visibility: Visibility,
    pub(in crate::gui) global_visibility: bool,
    pub(in crate::gui) children: Vec<Handle<UINode>>,
    pub(in crate::gui) parent: Handle<UINode>,
    /// Indices of commands in command buffer emitted by the node.
    pub(in crate::gui) command_indices: Vec<usize>,
    pub(in crate::gui) is_mouse_over: bool,
    pub(in crate::gui) measure_valid: Cell<bool>,
    pub(in crate::gui) arrange_valid: Cell<bool>,
    pub(in crate::gui) event_handlers: Vec<Box<UIEventHandler>>,
    pub(in crate::gui) events: RefCell<VecDeque<UIEvent>>,
    pub(in crate::gui) is_hit_test_visible: bool,
    pub(in crate::gui) style: Option<Rc<Style>>,
}

impl Default for Widget {
    fn default() -> Self {
        WidgetBuilder::new().build()
    }
}

impl Update for Widget {
    fn update(&mut self, _dt: f32) {}
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
    pub const WIDTH: &'static str = "Width";
    pub const HEIGHT: &'static str = "Height";
    pub const VERTICAL_ALIGNMENT: &'static str = "VerticalAlignment";
    pub const HORIZONTAL_ALIGNMENT: &'static str = "HorizontalAlignment";
    pub const MARGIN: &'static str = "Margin";
    pub const ROW: &'static str = "Row";
    pub const COLUMN: &'static str = "Column";
    pub const COLOR: &'static str = "Color";
    pub const VISIBILITY: &'static str = "Visibility";
    pub const MIN_SIZE: &'static str = "MinSize";
    pub const MAX_SIZE: &'static str = "MaxSize";

    #[inline]
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    #[inline]
    pub fn color(&self) -> Color {
        self.color
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
    pub fn column(&self) -> usize {
        self.column
    }

    #[inline]
    pub fn row(&self) -> usize {
        self.row
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

    #[inline]
    pub fn set_style(&mut self, style: Rc<Style>) {
        self.style = Some(style);
    }

    #[allow(dead_code)] // TODO
    fn raw_copy(&self) -> Self {
        Self {
            name: self.name.clone(),
            width: self.width.clone(),
            height: self.height.clone(),
            min_size: self.min_size,
            max_size: self.max_size,
            color: self.color,
            row: self.row,
            column: self.column,
            vertical_alignment: self.vertical_alignment,
            horizontal_alignment: self.horizontal_alignment,
            margin: self.margin,
            visibility: self.visibility,
            ..Default::default()
        }
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
    is_hit_test_visible: bool,
    visibility: Visibility,
    style: Option<Rc<Style>>,
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
            is_hit_test_visible: true,
            visibility: Visibility::Visible,
            style: None,
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

    pub fn with_style(mut self, style: Rc<Style>) -> Self {
        self.style = Some(style);
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

    pub fn with_hit_test_visibility(mut self, state: bool) -> Self {
        self.is_hit_test_visible = state;
        self
    }

    pub fn with_visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn build(self) -> Widget {
        let mut widget = Widget {
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
            visibility: self.visibility,
            global_visibility: true,
            children: self.children,
            parent: Handle::NONE,
            command_indices: Vec::new(),
            is_mouse_over: false,
            measure_valid: Cell::new(false),
            arrange_valid: Cell::new(false),
            event_handlers: self.event_handlers,
            events: RefCell::new(VecDeque::new()),
            is_hit_test_visible: self.is_hit_test_visible,
            style: None,
        };

        if let Some(style) = self.style {
            widget.apply_style(style);
        }

        widget
    }
}

impl Styleable for Widget {
    fn set_property(&mut self, name: &str, value: &dyn Any) {
        match name {
            Self::HORIZONTAL_ALIGNMENT => if let Some(value) = value.downcast_ref() { self.horizontal_alignment = *value },
            Self::VERTICAL_ALIGNMENT => if let Some(value) = value.downcast_ref() { self.vertical_alignment = *value },
            Self::WIDTH => {
                if let Some(value) = value.downcast_ref() { self.width.set(*value) } else if let Some(value) = value.downcast_ref::<f64>() { self.width.set(*value as f32) }
            }
            Self::HEIGHT => {
                if let Some(value) = value.downcast_ref() { self.height.set(*value) } else if let Some(value) = value.downcast_ref::<f64>() { self.height.set(*value as f32) }
            }
            Self::MARGIN => if let Some(value) = value.downcast_ref() { self.margin = *value },
            Self::ROW => if let Some(value) = value.downcast_ref() { self.row = *value },
            Self::COLUMN => if let Some(value) = value.downcast_ref() { self.column = *value },
            Self::COLOR => if let Some(value) = value.downcast_ref() { self.color = *value },
            Self::VISIBILITY => if let Some(value) = value.downcast_ref() { self.visibility = *value },
            Self::MIN_SIZE => if let Some(value) = value.downcast_ref() { self.min_size = *value },
            Self::MAX_SIZE => if let Some(value) = value.downcast_ref() { self.max_size = *value },
            _ => ()
        }
    }

    fn get_property(&self, name: &str) -> Option<&'_ dyn Any> {
        match name {
            Self::HORIZONTAL_ALIGNMENT => Some(&self.horizontal_alignment),
            Self::VERTICAL_ALIGNMENT => Some(&self.vertical_alignment),
            Self::WIDTH => Some(&self.width),
            Self::HEIGHT => Some(&self.height),
            Self::MARGIN => Some(&self.margin),
            Self::ROW => Some(&self.row),
            Self::COLUMN => Some(&self.column),
            Self::VISIBILITY => Some(&self.visibility),
            Self::COLOR => Some(&self.color),
            Self::MIN_SIZE => Some(&self.min_size),
            Self::MAX_SIZE => Some(&self.max_size),
            _ => None,
        }
    }
}