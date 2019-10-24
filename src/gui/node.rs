use rg3d_core::math::vec2::Vec2;
use crate::gui::{draw::DrawingContext, button::Button, text::Text, border::Border, scroll_bar::ScrollBar, scroll_viewer::ScrollViewer, image::Image, grid::Grid, scroll_content_presenter::ScrollContentPresenter, window::Window, Draw, Layout, UserInterface, canvas::Canvas, widget::{Widget, AsWidget}, list_box::ListBox, stack_panel::StackPanel, text_box::TextBox, Update};

/// UI node is a building block for all UI widgets. For example button could be a node with
/// this structure
///
/// Border
///    Text
///
/// or
///
/// Border
///    SomeOtherNode
///      Child1
///      Child2
///      ...
///      ChildN
///
///
/// Notes. Some fields wrapped into Cell's to be able to modify them while in measure/arrange
/// stage. This is required evil, I can't just unwrap all the recursive calls in measure/arrange.
pub enum UINode {
    Widget(Widget),
    Text(Text),
    Border(Border),
    Button(Button),
    ScrollBar(ScrollBar),
    ScrollViewer(ScrollViewer),
    Image(Image),
    Grid(Grid),
    Canvas(Canvas),
    ScrollContentPresenter(ScrollContentPresenter),
    Window(Window),
    ListBox(ListBox),
    StackPanel(StackPanel),
    TextBox(TextBox),
}

macro_rules! dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            UINode::Text(v) => v.$func($($args),*),
            UINode::Border(v) => v.$func($($args),*),
            UINode::Image(v) => v.$func($($args),*),
            UINode::Widget(v) => v.$func($($args),*),
            UINode::Button(v) => v.$func($($args),*),
            UINode::ScrollBar(v) => v.$func($($args),*),
            UINode::ScrollViewer(v) => v.$func($($args),*),
            UINode::Grid(v) => v.$func($($args),*),
            UINode::Canvas(v) => v.$func($($args),*),
            UINode::ScrollContentPresenter(v) => v.$func($($args),*),
            UINode::Window(v) => v.$func($($args),*),
            UINode::ListBox(v) => v.$func($($args),*),
            UINode::StackPanel(v) => v.$func($($args),*),
            UINode::TextBox(v) => v.$func($($args),*),
        }
    };
}

impl Draw for UINode {
    fn draw(&mut self, drawing_context: &mut DrawingContext) {
        dispatch!(self, draw, drawing_context)
    }
}

impl Update for UINode {
    fn update(&mut self, dt: f32) {
        dispatch!(self, update, dt)
    }
}

impl Layout for UINode {
    fn measure_override(&self, ui: &UserInterface, available_size: Vec2) -> Vec2 {
        dispatch!(self, measure_override, ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vec2) -> Vec2 {
        dispatch!(self, arrange_override, ui, final_size)
    }
}

impl AsWidget for UINode {
    fn widget(&self) -> &Widget {
        dispatch!(self, widget, )
    }

    fn widget_mut(&mut self) -> &mut Widget {
        dispatch!(self, widget_mut, )
    }
}

macro_rules! define_is_as {
    ($is:ident, $as_ref:ident, $as_mut:ident, $kind:ident, $result:ty) => {
        pub fn $is(&self) -> bool {
                match self {
                UINode::$kind(_) => true,
                _ => false
            }
        }

        pub fn $as_ref(&self) -> &$result {
            match self {
                UINode::$kind(ref val) => val,
                _ => panic!("Cast to {} failed!", stringify!($kind))
            }
        }

        pub fn $as_mut(&mut self) -> &mut $result {
            match self {
                UINode::$kind(ref mut val) => val,
                _ => panic!("Cast to {} failed!", stringify!($kind))
            }
        }
    }
}

impl UINode {
    define_is_as!(is_scroll_bar, as_scroll_bar, as_scroll_bar_mut, ScrollBar, ScrollBar);
    define_is_as!(is_text, as_text, as_text_mut, Text, Text);
    define_is_as!(is_border, as_border, as_border_mut, Border, Border);
    define_is_as!(is_button, as_button, as_button_mut, Button, Button);
    define_is_as!(is_scroll_viewer, as_scroll_viewer, as_scroll_viewer_mut, ScrollViewer, ScrollViewer);
    define_is_as!(is_image, as_image, as_image_mut, Image, Image);
    define_is_as!(is_canvas, as_canvas, as_canvas_mut, Canvas, Canvas);
    define_is_as!(is_scroll_content_presenter, as_scroll_content_presenter,
     as_scroll_content_presenter_mut, ScrollContentPresenter, ScrollContentPresenter);
    define_is_as!(is_window, as_window, as_window_mut, Window, Window);
    define_is_as!(is_list_box, as_list_box, as_list_box_mut, ListBox, ListBox);
    define_is_as!(is_stack_panel, as_stack_panel, as_stack_panel_mut, StackPanel, StackPanel);
    define_is_as!(is_text_box, as_text_box, as_text_box_mut, TextBox, TextBox);
}