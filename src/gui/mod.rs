pub mod draw;

use crate::utils::pool::{Pool, Handle};
use crate::math::vec2::Vec2;
use glutin::{VirtualKeyCode, MouseButton};
use crate::gui::draw::{Color, DrawingContext, FormattedText, CommandKind, FormattedTextBuilder};
use crate::math::Rect;
use serde::export::PhantomData;
use crate::utils::rcpool::RcHandle;
use crate::resource::Resource;
use crate::resource::ttf::Font;

#[derive(Copy, Clone, PartialEq)]
pub enum HorizontalAlignment {
    Stretch,
    Left,
    Center,
    Right,
}

#[derive(Copy, Clone, PartialEq)]
pub enum VerticalAlignment {
    Stretch,
    Top,
    Center,
    Bottom,
}

#[derive(Copy, Clone, PartialEq)]
pub struct Thickness {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

impl Thickness {
    pub fn zero() -> Thickness {
        Thickness {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum Visibility {
    Visible,
    Collapsed,
    Hidden,
}

pub struct Text {
    need_update: bool,
    text: String,
    font: Handle<Font>,
    formatted_text: Option<FormattedText>,
}

impl Text {
    pub fn new(text: &str) -> Text {
        Text {
            text: String::from(text),
            need_update: true,
            formatted_text: Some(FormattedTextBuilder::new().build()),
            font: Handle::none(),
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.text.clear();
        self.text += text;
        self.need_update = true;
    }

    pub fn get_text(&self) -> &str {
        self.text.as_str()
    }

    pub fn set_font(&mut self, font: Handle<Font>) {
        self.font = font;
        self.need_update = true;
    }
}

pub struct Border {
    stroke_thickness: Thickness,
    stroke_color: Color,
}

pub struct Image {
    texture: RcHandle<Resource>
}

pub enum UINodeKind {
    Base,
    /// TODO
    Text(Text),
    /// TODO
    Border(Border),
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
    /// TODO Automatically arranges children by rows and columns
    Grid,
    /// TODO Allows user to directly set position and size of a node
    Canvas,
    /// TODO Allows user to scroll content
    ScrollContentPresenter,
    /// TODO
    SlideSelector,
    /// TODO
    CheckBox,
}

pub struct UINode {
    kind: UINodeKind,
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
    children: Vec<Handle<UINode>>,
    parent: Handle<UINode>,
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
        button: MouseButton,
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
        amount: u32,
    },
}

pub struct RoutedEvent {
    kind: RoutedEventKind,
    handled: bool,
}

pub struct UserInterface {
    nodes: Pool<UINode>,
    drawing_context: DrawingContext,

    /// Every UI node will live on the window-sized canvas.
    root_canvas: Handle<UINode>,
}

#[inline]
fn maxf(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

#[inline]
fn minf(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

struct UnsafeCollectionView<T> {
    items: *const T,
    len: usize,
}

impl<T> UnsafeCollectionView<T> {
    fn empty() -> UnsafeCollectionView<T> {
        UnsafeCollectionView {
            items: std::ptr::null(),
            len: 0,
        }
    }

    fn from_vec(vec: &Vec<T>) -> UnsafeCollectionView<T> {
        UnsafeCollectionView {
            items: vec.as_ptr(),
            len: vec.len(),
        }
    }

    fn iter(&self) -> CollectionViewIterator<T> {
        unsafe {
            CollectionViewIterator {
                current: self.items,
                end: self.items.offset(self.len as isize),
                marker: PhantomData,
            }
        }
    }
}

struct CollectionViewIterator<'a, T> {
    current: *const T,
    end: *const T,
    marker: PhantomData<&'a T>,
}

impl<'a, T> Iterator for CollectionViewIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        unsafe {
            if self.current != self.end {
                let value = self.current;
                self.current = self.current.offset(1);
                Some(&*value)
            } else {
                None
            }
        }
    }
}

impl UserInterface {
    pub fn new() -> UserInterface {
        let mut nodes = Pool::new();

        UserInterface {
            root_canvas: nodes.spawn(UINode::new(UINodeKind::Canvas)),
            nodes,
            drawing_context: DrawingContext::new(),
        }
    }

    pub fn add_node(&mut self, node: UINode) -> Handle<UINode> {
        let node_handle = self.nodes.spawn(node);
        if let Some(root) = self.nodes.borrow_mut(&self.root_canvas) {
            root.children.push(node_handle.clone());
        }
        node_handle
    }

    #[inline]
    pub fn get_node(&self, node_handle: &Handle<UINode>) -> Option<&UINode> {
        self.nodes.borrow(node_handle)
    }

    #[inline]
    pub fn get_node_mut(&mut self, node_handle: &Handle<UINode>) -> Option<&mut UINode> {
        self.nodes.borrow_mut(node_handle)
    }

    #[inline]
    pub fn get_drawing_context(&self) -> &DrawingContext {
        &self.drawing_context
    }

    #[inline]
    pub fn get_drawing_context_mut(&mut self) -> &mut DrawingContext {
        &mut self.drawing_context
    }

    /// Performs recursive kind-specific measurement of children nodes
    ///
    /// Returns desired size.
    fn measure_override(&mut self, node_kind: &UINodeKind, children: &UnsafeCollectionView<Handle<UINode>>, available_size: &Vec2) -> Vec2 {
        match node_kind {
            // TODO: Type-specific measure
            UINodeKind::Border(border) => {
                let margin_x = border.stroke_thickness.left + border.stroke_thickness.right;
                let margin_y = border.stroke_thickness.top + border.stroke_thickness.bottom;

                let size_for_child = Vec2::make(
                    available_size.x - margin_x,
                    available_size.y - margin_y,
                );
                let mut desired_size = Vec2::new();
                for child_handle in children.iter() {
                    self.measure(child_handle, &size_for_child);

                    if let Some(child) = self.nodes.borrow(child_handle) {
                        if child.desired_size.x > desired_size.x {
                            desired_size.x = child.desired_size.x;
                        }
                        if child.desired_size.y > desired_size.y {
                            desired_size.y = child.desired_size.y;
                        }
                    }
                }
                desired_size.x += margin_x;
                desired_size.y += margin_y;

                desired_size
            }
            UINodeKind::Canvas => {
                let size_for_child = Vec2::make(
                    std::f32::INFINITY,
                    std::f32::INFINITY,
                );

                for child_handle in children.iter() {
                    self.measure(child_handle, &size_for_child);
                }

                Vec2::new()
            }
            // Default measure
            _ => {
                let mut size = Vec2::new();

                for child_handle in children.iter() {
                    self.measure(child_handle, &available_size);

                    if let Some(child) = self.nodes.borrow(child_handle) {
                        if child.desired_size.x > size.x {
                            size.x = child.desired_size.x;
                        }
                        if child.desired_size.y > size.y {
                            size.y = child.desired_size.y;
                        }
                    }
                }

                size
            }
        }
    }

    fn measure(&mut self, node_handle: &Handle<UINode>, available_size: &Vec2) {
        let mut children: UnsafeCollectionView<Handle<UINode>> = UnsafeCollectionView::empty();
        let mut node_kind: *const UINodeKind = std::ptr::null();
        let mut size_for_child = Vec2::new();
        let mut margin = Vec2::new();


        match self.nodes.borrow_mut(&node_handle) {
            None => return,
            Some(node) => {
                margin = Vec2 {
                    x: node.margin.left + node.margin.right,
                    y: node.margin.top + node.margin.bottom,
                };

                size_for_child = Vec2 {
                    x: if node.width > 0.0 {
                        node.width
                    } else {
                        maxf(0.0, available_size.x - margin.x)
                    },
                    y: if node.height > 0.0 {
                        node.height
                    } else {
                        maxf(0.0, available_size.y - margin.y)
                    },
                };

                if size_for_child.x > node.max_size.x {
                    size_for_child.x = node.max_size.x;
                } else if size_for_child.x < node.min_size.x {
                    size_for_child.x = node.min_size.x;
                }

                if size_for_child.y > node.max_size.y {
                    size_for_child.y = node.max_size.y;
                } else if size_for_child.y < node.min_size.y {
                    size_for_child.y = node.min_size.y;
                }

                if node.visibility == Visibility::Visible {
                    // Remember immutable pointer to collection of children nodes on which we'll continue
                    // measure. It is one hundred percent safe to have immutable pointer to list of
                    // children handles, because we guarantee that children collection won't be modified
                    // during measure pass. Also this step *cannot* be performed in parallel so we don't
                    // have to bother about thread-safety here.
                    children = UnsafeCollectionView::from_vec(&node.children);
                    node_kind = &node.kind as *const UINodeKind;
                } else {
                    // We do not have any children so node want to collapse into point.
                    node.desired_size = Vec2::make(0.0, 0.0);
                }
            }
        }

        let desired_size = self.measure_override(unsafe { &*node_kind }, &children, &size_for_child);

        if let Some(node) = self.nodes.borrow_mut(&node_handle) {
            node.desired_size = desired_size;

            if !node.width.is_nan() {
                node.desired_size.x = node.width;
            }

            if node.desired_size.x > node.max_size.x {
                node.desired_size.x = node.max_size.x;
            } else if node.desired_size.x < node.min_size.x {
                node.desired_size.x = node.min_size.x;
            }

            if node.desired_size.y > node.max_size.y {
                node.desired_size.y = node.max_size.y;
            } else if node.desired_size.y < node.min_size.y {
                node.desired_size.y = node.min_size.y;
            }

            if !node.height.is_nan() {
                node.desired_size.y = node.height;
            }

            node.desired_size += margin;

            // Make sure that node won't go outside of available bounds.
            if node.desired_size.x > available_size.x {
                node.desired_size.x = available_size.x;
            }
            if node.desired_size.y > available_size.y {
                node.desired_size.y = available_size.y;
            }
        }
    }

    /// Performs recursive kind-specific arrangement of children nodes
    ///
    /// Returns actual size.
    fn arrange_override(&mut self, node_kind: &UINodeKind, children: &UnsafeCollectionView<Handle<UINode>>, final_size: &Vec2) -> Vec2 {
        match node_kind {
            // TODO: Kind-specific arrangement
            UINodeKind::Border(border) => {
                let rect_for_child = Rect::new(
                    border.stroke_thickness.left, border.stroke_thickness.top,
                    final_size.x - (border.stroke_thickness.right + border.stroke_thickness.left),
                    final_size.y - (border.stroke_thickness.bottom + border.stroke_thickness.top),
                );

                for child_handle in children.iter() {
                    self.arrange(child_handle, &rect_for_child);
                }

                *final_size
            }
            UINodeKind::Canvas => {
                for child_handle in children.iter() {
                    let mut final_rect = None;

                    if let Some(child) = self.nodes.borrow(&child_handle) {
                        final_rect = Some(Rect::new(
                            child.desired_local_position.x,
                            child.desired_local_position.y,
                            child.desired_size.x,
                            child.desired_size.y));
                    }

                    if let Some(rect) = final_rect {
                        self.arrange(child_handle, &rect);
                    }
                }

                *final_size
            }
            // Default arrangement
            _ => {
                let final_rect = Rect::new(0.0, 0.0, final_size.x, final_size.y);

                for child_handle in children.iter() {
                    self.arrange(child_handle, &final_rect);
                }

                *final_size
            }
        }
    }

    fn arrange(&mut self, node_handle: &Handle<UINode>, final_rect: &Rect<f32>) {
        let mut children: UnsafeCollectionView<Handle<UINode>> = UnsafeCollectionView::empty();

        let mut size = Vec2::new();
        let mut size_without_margin = Vec2::new();
        let mut origin_x = 0.0;
        let mut origin_y = 0.0;
        let mut node_kind: *const UINodeKind = std::ptr::null();

        match self.nodes.borrow_mut(node_handle) {
            None => return,
            Some(node) => {
                if node.visibility != Visibility::Visible {
                    return;
                }

                let margin_x = node.margin.left + node.margin.right;
                let margin_y = node.margin.top + node.margin.bottom;

                origin_x = final_rect.x + node.margin.left;
                origin_y = final_rect.y + node.margin.top;

                size = Vec2 {
                    x: maxf(0.0, final_rect.w - margin_x),
                    y: maxf(0.0, final_rect.h - margin_y),
                };

                size_without_margin = size;

                if node.horizontal_alignment != HorizontalAlignment::Stretch {
                    size.x = minf(size.x, node.desired_size.x - margin_x);
                }
                if node.vertical_alignment != VerticalAlignment::Stretch {
                    size.y = minf(size.y, node.desired_size.y - margin_y);
                }

                if node.width > 0.0 {
                    size.x = node.width;
                }
                if node.height > 0.0 {
                    size.y = node.height;
                }

                // Remember immutable pointer to collection of children nodes on which
                // we'll continue arrange recursively.
                children = UnsafeCollectionView::from_vec(&node.children);
                node_kind = &node.kind as *const UINodeKind;
            }
        }

        size = self.arrange_override(unsafe { &*node_kind }, &children, &size);

        if let Some(node) = self.nodes.borrow_mut(node_handle) {
            if size.x > final_rect.w {
                size.x = final_rect.w;
            }
            if size.y > final_rect.h {
                size.y = final_rect.h;
            }

            match node.horizontal_alignment {
                HorizontalAlignment::Center | HorizontalAlignment::Stretch => {
                    origin_x += (size_without_margin.x - size.x) * 0.5;
                }
                HorizontalAlignment::Right => {
                    origin_x += size_without_margin.x - size.x;
                }
                _ => ()
            }

            match node.vertical_alignment {
                VerticalAlignment::Center | VerticalAlignment::Stretch => {
                    origin_y += (size_without_margin.y - size.y) * 0.5;
                }
                VerticalAlignment::Bottom => {
                    origin_y += size_without_margin.y - size.y;
                }
                _ => ()
            }

            node.actual_size = size;
            node.actual_local_position = Vec2 { x: origin_x, y: origin_y };
        }
    }

    fn update_transform(&mut self, node_handle: &Handle<UINode>) {
        let mut children = UnsafeCollectionView::empty();

        let mut screen_position = Vec2::new();
        if let Some(node) = self.nodes.borrow(node_handle) {
            children = UnsafeCollectionView::from_vec(&node.children);
            if let Some(parent) = self.nodes.borrow(&node.parent) {
                screen_position = node.actual_local_position + parent.screen_position;
            } else {
                screen_position = node.actual_local_position;
            }
        }

        if let Some(node) = self.nodes.borrow_mut(node_handle) {
            node.screen_position = screen_position;
        }

        // Continue on children
        for child_handle in children.iter() {
            self.update_transform(child_handle);
        }
    }

    pub fn update(&mut self, screen_size: &Vec2) {
        let root_canvas_handle = self.root_canvas.clone();
        self.measure(&root_canvas_handle, screen_size);
        self.arrange(&root_canvas_handle, &Rect::new(0.0, 0.0, screen_size.x, screen_size.y));
        self.update_transform(&root_canvas_handle);
    }

    fn draw_node(&mut self, node_handle: &Handle<UINode>, font_cache: &Pool<Font>) {
        let mut children: UnsafeCollectionView<Handle<UINode>> = UnsafeCollectionView::empty();

        if let Some(node) = self.nodes.borrow_mut(node_handle) {
            match &mut node.kind {
                UINodeKind::Border(border) => {
                    let bounds = Rect::new(
                        node.screen_position.x,
                        node.screen_position.y,
                        node.actual_size.x,
                        node.actual_size.y);
                    self.drawing_context.push_rect_filled(&bounds, None, node.color);
                    self.drawing_context.push_rect_vary(&bounds, border.stroke_thickness, border.stroke_color);
                    self.drawing_context.commit(CommandKind::Geometry, 0);
                }
                UINodeKind::Image => {}
                UINodeKind::Text(text) => {
                    if text.need_update {
                        if let Some(font) = font_cache.borrow(&text.font) {
                            let formatted_text = FormattedTextBuilder::reuse(text.formatted_text.take().unwrap())
                                .with_size(node.actual_size)
                                .with_font(font)
                                .with_text(text.text.as_str())
                                .with_color(node.color)
                                .build();
                            text.formatted_text = Some(formatted_text);
                        }
                        text.need_update = true; // TODO
                    }
                    self.drawing_context.draw_text(node.screen_position, text.formatted_text.as_ref().unwrap());
                }
                _ => ()
            }

            children = UnsafeCollectionView::from_vec(&node.children);
        }

        // Continue on children
        for child_node in children.iter() {
            self.draw_node(child_node, font_cache);
        }
    }

    pub fn draw(&mut self, font_cache: &Pool<Font>) -> &DrawingContext {
        self.drawing_context.clear();

        let root_canvas = self.root_canvas.clone();
        self.draw_node(&root_canvas, font_cache);

        &self.drawing_context
    }
}

impl UINode {
    pub fn new(kind: UINodeKind) -> UINode {
        UINode {
            kind,
            desired_local_position: Vec2::new(),
            width: std::f32::NAN,
            height: std::f32::NAN,
            screen_position: Vec2::new(),
            desired_size: Vec2::new(),
            actual_local_position: Vec2::new(),
            actual_size: Vec2::new(),
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
        }
    }

    pub fn set_width(&mut self, width: f32) {
        self.width = width;
    }

    pub fn set_height(&mut self, height: f32) {
        self.height = height;
    }

    pub fn get_kind(&self) -> &UINodeKind {
        &self.kind
    }

    pub fn get_kind_mut(&mut self) -> &mut UINodeKind {
        &mut self.kind
    }
}