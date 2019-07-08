pub mod draw;

use crate::utils::pool::Pool;
use crate::math::vec2::Vec2;
use glutin::{VirtualKeyCode, MouseButton};
use crate::gui::draw::{Color, DrawingContext};

pub struct UserInterface {
    nodes: Pool<UINode>,
    drawing_context: DrawingContext
}

#[derive(Copy, Clone)]
pub enum HorizontalAlignment {
    Stretch,
    Left,
    Center,
    Right,
}

#[derive(Copy, Clone)]
pub enum VerticalAlignment {
    Stretch,
    Top,
    Center,
    Bottom,
}

#[derive(Copy, Clone)]
pub struct Thickness {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

#[derive(Copy, Clone)]
pub enum Visibility {
    Visible,
    Collapsed,
    Hidden,
}

pub enum UINodeKind {
    Base,
    /// TODO
    Text,
    /// TODO
    Border,
    /// TODO
    Window,
    /// TODO
    Button,
    /// TODO
    ScrollBar,
    /// TODO
    ScrollViewer,
    /// TODO
    TextBox,
    /// TODO
    Image,
    /// TODO Automatically arranges children by rows and columns */
    Grid,
    /// TODO Allows user to directly set position and size of a node */
    Canvas,
    /// TODO Allows user to scroll content */
    ScrollContentPresenter,
    /// TODO
    SlideSelector,
    /// TODO
    CheckBox,
}

pub struct UINode {
    /// Desired position relative to parent node
    desired_local_position: Vec2,
    /// Explicit width for node or automatic if NaN (means value is undefined). Default is NaN
    width: f32,
    /// Explicit height for node or automatic if NaN (means value is undefined). Default is NaN
    height: f32,
    /// Screen position of the node
    screen_position: Vec2,
    /// Desired size of the node after Measure pass.
    desired_size: Vec2,
    /// Actual node local position after Arrange pass.
    actual_local_position: Vec2,
    /// Actual size of the node after Arrange pass.
    actual_size: Vec2,
    /// Minimum width and height
    min_size: Vec2,
    /// Maximum width and height
    max_size: Vec2,
    /// Overlay color of the node
    color: Color,
    /// Index of row to which this node belongs
    row: usize,
    /// Index of column to which this node belongs
    column: usize,
    /// Vertical alignment
    vertical_alignment: VerticalAlignment,
    /// Horizontal alignment
    horizontal_alignment: HorizontalAlignment,
    /// Margin (four sides)
    margin: Thickness,
    /// Current visibility state
    visibility: Visibility,
}

pub enum RoutedEventKind {
    MouseDown {
        pos: Vec2,
        button: MouseButton,
    },
    MouseMove {
        pos: Vec2
    },
    MouseUp {
        pos: Vec2,
        button: MouseButton
    },
    Text {
        symbol: char
    },
    KeyDown {
        code: VirtualKeyCode
    },
    KeyUp {
        code: VirtualKeyCode
    },
    MouseWheel {
        pos: Vec2,
        amount: u32
    }
}

pub struct RoutedEvent {
    kind: RoutedEventKind,
    handled: bool,
}

impl UserInterface {
    pub fn new() -> UserInterface {
        UserInterface {
            nodes: Pool::new(),
            drawing_context: DrawingContext::new(),
        }
    }

    pub fn get_drawing_context(&self) -> &DrawingContext {
        &self.drawing_context
    }

    pub fn get_drawing_context_mut(&mut self) -> &mut DrawingContext {
        &mut self.drawing_context
    }
}