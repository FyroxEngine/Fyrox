pub mod draw;
pub mod node;
pub mod text;
pub mod border;
pub mod image;
pub mod canvas;
pub mod event;
pub mod button;
pub mod scroll_bar;
pub mod scroll_content_presenter;
pub mod scroll_viewer;
pub mod grid;
pub mod window;
pub mod formatted_text;
pub mod widget;
pub mod list_box;
pub mod stack_panel;
pub mod text_box;

use std::{
    collections::VecDeque,
    rc::Rc,
    cell::RefCell,
};
use crate::{
    gui::{
        node::UINode,
        widget::AsWidget,
        draw::{DrawingContext, CommandKind, CommandTexture},
        canvas::Canvas,
        event::{
            UIEvent,
            UIEventKind,
            UIEventHandler
        }
    },
    resource::{ttf::Font},
    utils::UnsafeCollectionView,
    ElementState,
    WindowEvent,
    MouseScrollDelta,
};
use rg3d_core::{
    color::Color,
    pool::{Pool, Handle},
    math::{vec2::Vec2, Rect},
};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum HorizontalAlignment {
    Stretch,
    Left,
    Center,
    Right,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum VerticalAlignment {
    Stretch,
    Top,
    Center,
    Bottom,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Thickness {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
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

    pub fn uniform(v: f32) -> Thickness {
        Thickness {
            left: v,
            top: v,
            right: v,
            bottom: v,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Visibility {
    Visible,
    /// Collapses into a point so does not take space in layout and becomes invisible.
    Collapsed,
    /// Keeps space for node in layout and becomes invisible.
    Hidden,
}

trait Layout {
    fn measure_override(&self, ui: &UserInterface, available_size: Vec2) -> Vec2;
    fn arrange_override(&self, ui: &UserInterface, final_size: Vec2) -> Vec2;
}

trait Draw {
    fn draw(&mut self, drawing_context: &mut DrawingContext);
}

trait Update {
    fn update(&mut self, dt: f32);
}

pub struct UserInterface {
    nodes: Pool<UINode>,
    drawing_context: DrawingContext,
    default_font: Rc<RefCell<Font>>,
    visual_debug: bool,
    /// Every UI node will live on the window-sized canvas.
    root_canvas: Handle<UINode>,
    picked_node: Handle<UINode>,
    prev_picked_node: Handle<UINode>,
    captured_node: Handle<UINode>,
    keyboard_focus_node: Handle<UINode>,
    mouse_position: Vec2,
    events: VecDeque<UIEvent>,
    event_handlers: Vec<Option<Box<UIEventHandler>>>,
}

#[inline]
fn maxf(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}

#[inline]
fn minf(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}

impl UserInterface {
    pub(in crate) fn new() -> UserInterface {
        let font_bytes = std::include_bytes!("../built_in_font.ttf").to_vec();
        let font = Font::from_memory(font_bytes, 20.0, (0..255).collect()).unwrap();
        let font = Rc::new(RefCell::new(font));
        let mut ui = UserInterface {
            events: VecDeque::new(),
            visual_debug: false,
            default_font: font,
            captured_node: Handle::NONE,
            root_canvas: Handle::NONE,
            nodes: Pool::new(),
            mouse_position: Vec2::ZERO,
            drawing_context: DrawingContext::new(),
            picked_node: Handle::NONE,
            prev_picked_node: Handle::NONE,
            event_handlers: Vec::new(),
            keyboard_focus_node: Handle::NONE,
        };
        ui.root_canvas = ui.add_node(UINode::Canvas(Canvas::new()));
        ui
    }

    #[inline]
    pub fn get_default_font(&self) -> Rc<RefCell<Font>> {
        self.default_font.clone()
    }

    pub fn add_node(&mut self, mut node: UINode) -> Handle<UINode> {
        let children = node.widget().children.clone();
        node.widget_mut().children.clear();
        let node_handle = self.nodes.spawn(node);
        if self.root_canvas.is_some() {
            self.link_nodes(node_handle, self.root_canvas);
        }
        for child in children {
            self.link_nodes(child, node_handle)
        }
        node_handle
    }

    #[inline]
    pub fn capture_mouse(&mut self, node: Handle<UINode>) -> bool {
        if self.captured_node.is_none() && self.nodes.is_valid_handle(node) {
            self.captured_node = node;
            return true;
        }

        false
    }

    #[inline]
    pub fn release_mouse_capture(&mut self) {
        self.captured_node = Handle::NONE;
    }

    /// Links specified child with specified parent.
    #[inline]
    pub fn link_nodes(&mut self, child_handle: Handle<UINode>, parent_handle: Handle<UINode>) {
        self.unlink_node(child_handle);
        let child = self.nodes.borrow_mut(child_handle).widget_mut();
        child.parent = parent_handle;
        let parent = self.nodes.borrow_mut(parent_handle).widget_mut();
        parent.children.push(child_handle);
    }

    /// Unlinks specified node from its parent, so node will become root.
    #[inline]
    pub fn unlink_node(&mut self, node_handle: Handle<UINode>) {
        let parent_handle;
        // Replace parent handle of child
        let node = self.nodes.borrow_mut(node_handle);
        parent_handle = node.widget().parent;
        node.widget_mut().parent = Handle::NONE;

        // Remove child from parent's children list
        if parent_handle.is_some() {
            let parent = self.nodes.borrow_mut(parent_handle);
            if let Some(i) = parent.widget().children.iter().position(|h| *h == node_handle) {
                parent.widget_mut().children.remove(i);
            }
        }
    }

    #[inline]
    pub fn get_node(&self, node_handle: Handle<UINode>) -> &UINode {
        self.nodes.borrow(node_handle)
    }

    #[inline]
    pub fn get_node_mut(&mut self, node_handle: Handle<UINode>) -> &mut UINode {
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

    fn measure(&self, node_handle: Handle<UINode>, available_size: Vec2) {
        let node = self.nodes.borrow(node_handle);
        let widget = self.nodes.borrow(node_handle).widget();
        let margin = Vec2 {
            x: widget.margin.left + widget.margin.right,
            y: widget.margin.top + widget.margin.bottom,
        };

        let size_for_child = Vec2 {
            x: {
                let w = if widget.width.get() > 0.0 {
                    widget.width.get()
                } else {
                    maxf(0.0, available_size.x - margin.x)
                };

                if w > widget.max_size.x {
                    widget.max_size.x
                } else if w < widget.min_size.x {
                    widget.min_size.x
                } else {
                    w
                }
            },
            y: {
                let h = if widget.height.get() > 0.0 {
                    widget.height.get()
                } else {
                    maxf(0.0, available_size.y - margin.y)
                };

                if h > widget.max_size.y {
                    widget.max_size.y
                } else if h < widget.min_size.y {
                    widget.min_size.y
                } else {
                    h
                }
            },
        };

        if widget.visibility == Visibility::Visible {
            let mut desired_size = node.measure_override(self, size_for_child);

            if !widget.width.get().is_nan() {
                desired_size.x = widget.width.get();
            }

            if desired_size.x > widget.max_size.x {
                desired_size.x = widget.max_size.x;
            } else if desired_size.x < widget.min_size.x {
                desired_size.x = widget.min_size.x;
            }

            if desired_size.y > widget.max_size.y {
                desired_size.y = widget.max_size.y;
            } else if desired_size.y < widget.min_size.y {
                desired_size.y = widget.min_size.y;
            }

            if !widget.height.get().is_nan() {
                desired_size.y = widget.height.get();
            }

            desired_size += margin;

            // Make sure that node won't go outside of available bounds.
            if desired_size.x > available_size.x {
                desired_size.x = available_size.x;
            }
            if desired_size.y > available_size.y {
                desired_size.y = available_size.y;
            }

            widget.desired_size.set(desired_size);
        } else {
            widget.desired_size.set(Vec2::new(0.0, 0.0));
        }

        widget.measure_valid.set(true)
    }

    fn arrange(&self, node_handle: Handle<UINode>, final_rect: &Rect<f32>) {
        let node = self.nodes.borrow(node_handle);
        let widget = node.widget();
        if widget.visibility != Visibility::Visible {
            return;
        }

        let margin_x = widget.margin.left + widget.margin.right;
        let margin_y = widget.margin.top + widget.margin.bottom;

        let mut origin_x = final_rect.x + widget.margin.left;
        let mut origin_y = final_rect.y + widget.margin.top;

        let mut size = Vec2 {
            x: maxf(0.0, final_rect.w - margin_x),
            y: maxf(0.0, final_rect.h - margin_y),
        };

        let size_without_margin = size;

        if widget.horizontal_alignment != HorizontalAlignment::Stretch {
            size.x = minf(size.x, widget.desired_size.get().x - margin_x);
        }
        if widget.vertical_alignment != VerticalAlignment::Stretch {
            size.y = minf(size.y, widget.desired_size.get().y - margin_y);
        }

        if widget.width.get() > 0.0 {
            size.x = widget.width.get();
        }
        if widget.height.get() > 0.0 {
            size.y = widget.height.get();
        }

        size = node.arrange_override(self, size);

        if size.x > final_rect.w {
            size.x = final_rect.w;
        }
        if size.y > final_rect.h {
            size.y = final_rect.h;
        }

        match widget.horizontal_alignment {
            HorizontalAlignment::Center | HorizontalAlignment::Stretch => {
                origin_x += (size_without_margin.x - size.x) * 0.5;
            }
            HorizontalAlignment::Right => {
                origin_x += size_without_margin.x - size.x;
            }
            _ => ()
        }

        match widget.vertical_alignment {
            VerticalAlignment::Center | VerticalAlignment::Stretch => {
                origin_y += (size_without_margin.y - size.y) * 0.5;
            }
            VerticalAlignment::Bottom => {
                origin_y += size_without_margin.y - size.y;
            }
            _ => ()
        }

        widget.actual_size.set(size);
        widget.actual_local_position.set(Vec2 { x: origin_x, y: origin_y });
        widget.arrange_valid.set(true);
    }

    fn update_transform(&mut self, node_handle: Handle<UINode>) {
        let screen_position;
        let widget = self.nodes.borrow(node_handle).widget();
        let children = UnsafeCollectionView::from_slice(&widget.children);
        if widget.parent.is_some() {
            screen_position = widget.actual_local_position.get() + self.nodes.borrow(widget.parent).widget().screen_position;
        } else {
            screen_position = widget.actual_local_position.get();
        }

        self.nodes.borrow_mut(node_handle).widget_mut().screen_position = screen_position;

        // Continue on children
        for child_handle in children.iter() {
            self.update_transform(*child_handle);
        }
    }

    pub fn update(&mut self, screen_size: Vec2, dt: f32) {
        self.measure(self.root_canvas, screen_size);
        self.arrange(self.root_canvas, &Rect::new(0.0, 0.0, screen_size.x, screen_size.y));
        self.update_transform(self.root_canvas);
        for node in self.nodes.iter_mut() {
            node.update(dt)
        }
    }

    fn draw_node(&mut self, node_handle: Handle<UINode>, nesting: u8) {
        let children;

        let node = self.nodes.borrow_mut(node_handle);
        let bounds = node.widget().get_screen_bounds();
        if node.widget_mut().visibility != Visibility::Visible {
            return;
        }

        let start_index = self.drawing_context.get_commands().len();
        self.drawing_context.set_nesting(nesting);
        self.drawing_context.commit_clip_rect(&bounds.inflate(0.9, 0.9));

        node.draw(&mut self.drawing_context);

        let widget = node.widget_mut();
        children = UnsafeCollectionView::from_slice(&widget.children);

        let end_index = self.drawing_context.get_commands().len();
        for i in start_index..end_index {
            widget.command_indices.push(i);
        }

        // Continue on children
        for child_node in children.iter() {
            self.draw_node(*child_node, nesting + 1);
        }

        self.drawing_context.revert_clip_geom();
    }

    pub fn draw(&mut self) -> &DrawingContext {
        self.drawing_context.clear();

        for node in self.nodes.iter_mut() {
            node.widget_mut().command_indices.clear();
        }

        let root_canvas = self.root_canvas;
        self.draw_node(root_canvas, 1);

        if self.visual_debug {
            self.drawing_context.set_nesting(0);

            let picked_bounds =
                if self.picked_node.is_some() {
                    Some(self.nodes.borrow(self.picked_node).widget().get_screen_bounds())
                } else {
                    None
                };

            if let Some(picked_bounds) = picked_bounds {
                self.drawing_context.push_rect(&picked_bounds, 1.0, Color::WHITE);
                self.drawing_context.commit(CommandKind::Geometry, CommandTexture::None);
            }
        }

        &self.drawing_context
    }

    fn is_node_clipped(&self, node_handle: Handle<UINode>, pt: Vec2) -> bool {
        let mut clipped = true;

        let widget = self.nodes.borrow(node_handle).widget();
        if widget.visibility != Visibility::Visible {
            return clipped;
        }

        for command_index in widget.command_indices.iter() {
            if let Some(command) = self.drawing_context.get_commands().get(*command_index) {
                if *command.get_kind() == CommandKind::Clip && self.drawing_context.is_command_contains_point(command, pt) {
                    clipped = false;
                    break;
                }
            }
        }

        // Point can be clipped by parent's clipping geometry.
        if !widget.parent.is_none() && !clipped {
            clipped |= self.is_node_clipped(widget.parent, pt);
        }

        clipped
    }

    fn is_node_contains_point(&self, node_handle: Handle<UINode>, pt: Vec2) -> bool {
        let widget = self.nodes.borrow(node_handle).widget();

        if widget.visibility != Visibility::Visible {
            return false;
        }

        if !self.is_node_clipped(node_handle, pt) {
            for command_index in widget.command_indices.iter() {
                if let Some(command) = self.drawing_context.get_commands().get(*command_index) {
                    if *command.get_kind() == CommandKind::Geometry && self.drawing_context.is_command_contains_point(command, pt) {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn pick_node(&self, node_handle: Handle<UINode>, pt: Vec2, level: &mut i32) -> Handle<UINode> {
        let (mut picked, mut topmost_picked_level) =
            if self.is_node_contains_point(node_handle, pt) {
                (node_handle, *level)
            } else {
                (Handle::NONE, 0)
            };

        let widget = self.nodes.borrow(node_handle).widget();
        for child_handle in widget.children.iter() {
            *level += 1;
            let picked_child = self.pick_node(*child_handle, pt, level);
            if !picked_child.is_none() && *level > topmost_picked_level {
                topmost_picked_level = *level;
                picked = picked_child;
            }
        }

        picked
    }

    pub fn hit_test(&self, pt: Vec2) -> Handle<UINode> {
        if self.nodes.is_valid_handle(self.captured_node) {
            self.captured_node
        } else {
            let mut level = 0;
            self.pick_node(self.root_canvas, pt, &mut level)
        }
    }

    /// Searches a node down on tree starting from give root that matches a criteria
    /// defined by a given func.
    pub fn find_by_criteria_down<Func>(&self, node_handle: Handle<UINode>, func: &Func) -> Handle<UINode>
        where Func: Fn(&UINode) -> bool {
        let node = self.nodes.borrow(node_handle);

        if func(node) {
            return node_handle;
        }

        for child_handle in node.widget().children.iter() {
            let result = self.find_by_criteria_down(*child_handle, func);

            if result.is_some() {
                return result;
            }
        }

        Handle::NONE
    }

    /// Searches a node up on tree starting from given root that matches a criteria
    /// defined by a given func.
    pub fn find_by_criteria_up<Func>(&self, node_handle: Handle<UINode>, func: Func) -> Handle<UINode>
        where Func: Fn(&UINode) -> bool {
        let node = self.nodes.borrow(node_handle);

        if func(node) {
            return node_handle;
        }

        self.find_by_criteria_up(node.widget().parent, func)
    }

    /// Checks if specified node is a child of some other node on `root_handle`. This method
    /// is useful to understand if some event came from some node down by tree.
    pub fn is_node_child_of(&self, node_handle: Handle<UINode>, root_handle: Handle<UINode>) -> bool {
        for child_handle in self.nodes.borrow(root_handle).widget().children.iter() {
            if *child_handle == node_handle {
                return true;
            }

            let result = self.is_node_child_of(node_handle, *child_handle);
            if result {
                return true;
            }
        }
        false
    }

    /// Checks if specified node is a direct child of some other node on `root_handle`.
    pub fn is_node_direct_child_of(&self, node_handle: Handle<UINode>, root_handle: Handle<UINode>) -> bool {
        for child_handle in self.nodes.borrow(root_handle).widget().children.iter() {
            if *child_handle == node_handle {
                return true;
            }
        }
        false
    }

    /// Searches a node by name up on tree starting from given root node.
    pub fn find_by_name_up(&self, node_handle: Handle<UINode>, name: &str) -> Handle<UINode> {
        self.find_by_criteria_up(node_handle, |node| node.widget().name == name)
    }

    /// Searches a node by name down on tree starting from given root node.
    pub fn find_by_name_down(&self, node_handle: Handle<UINode>, name: &str) -> Handle<UINode> {
        self.find_by_criteria_down(node_handle, &|node| node.widget().name == name)
    }

    /// Searches a node by name up on tree starting from given root node and tries to borrow it if exists.
    pub fn borrow_by_name_up(&self, start_node_handle: Handle<UINode>, name: &str) -> &UINode {
        self.nodes.borrow(self.find_by_name_up(start_node_handle, name))
    }

    /// Searches a node by name up on tree starting from given root node and tries to borrow it as mutable if exists.
    pub fn borrow_by_name_up_mut(&mut self, start_node_handle: Handle<UINode>, name: &str) -> &mut UINode {
        self.nodes.borrow_mut(self.find_by_name_up(start_node_handle, name))
    }

    /// Searches a node by name down on tree starting from given root node and tries to borrow it if exists.
    pub fn borrow_by_name_down(&self, start_node_handle: Handle<UINode>, name: &str) -> &UINode {
        self.nodes.borrow(self.find_by_name_down(start_node_handle, name))
    }

    /// Searches a node by name down on tree starting from given root node and tries to borrow it as mutable if exists.
    pub fn borrow_by_name_down_mut(&mut self, start_node_handle: Handle<UINode>, name: &str) -> &mut UINode {
        self.nodes.borrow_mut(self.find_by_name_down(start_node_handle, name))
    }

    pub fn borrow_by_criteria_up<Func>(&self, start_node_handle: Handle<UINode>, func: Func) -> &UINode
        where Func: Fn(&UINode) -> bool {
        self.nodes.borrow(self.find_by_criteria_up(start_node_handle, func))
    }

    pub fn borrow_by_criteria_up_mut<Func>(&mut self, start_node_handle: Handle<UINode>, func: Func) -> &mut UINode
        where Func: Fn(&UINode) -> bool {
        self.nodes.borrow_mut(self.find_by_criteria_up(start_node_handle, func))
    }

    pub fn poll_ui_event(&mut self) -> Option<UIEvent> {
        // Gather events from nodes.
        for (handle, node) in self.nodes.pair_iter_mut() {
            while let Some(mut response_event) = node.widget_mut().events.borrow_mut().pop_front() {
                response_event.source = handle;
                self.events.push_back(response_event)
            }
        }

        let mut event = self.events.pop_front();

        // Dispatch events to nodes first. This is a bit tricky because we
        // need to pass self as mutable to event handler, but since multple
        // mutable borrow is not allowed we using take-call-putback trick.
        if let Some(ref mut event) = event {
            for i in 0..self.nodes.get_capacity() {
                // Take all event handlers.
                self.event_handlers.clear();
                if let Some(node) = self.nodes.at_mut(i) {
                    for event_handler in node.widget_mut().event_handlers.drain(..) {
                        self.event_handlers.push(Some(event_handler));
                    }
                } else {
                    continue;
                };

                // Iterate over and call one-by-one.
                for k in 0..self.event_handlers.len() {
                    let mut handler = self.event_handlers[k].take().unwrap();
                    handler(self, self.nodes.handle_from_index(i), event);
                    self.event_handlers[k].replace(handler);
                }

                // Transfer event handlers back to node if it exists. At this
                // point there is not guarantee that node still alive so we
                // doing additional checks.
                if let Some(node) = self.nodes.at_mut(i) {
                    for event_handler in self.event_handlers.drain(..) {
                        node.widget_mut().event_handlers.push(event_handler.unwrap());
                    }
                }
            }
        }

        event
    }

    pub fn process_input_event(&mut self, event: &WindowEvent) -> bool {
        let mut event_processed = false;

        match event {
            WindowEvent::MouseInput { button, state, .. } => {
                match state {
                    ElementState::Pressed => {
                        self.picked_node = self.hit_test(self.mouse_position);

                        self.keyboard_focus_node = self.picked_node;

                        if !self.picked_node.is_none() {
                            self.events.push_back(UIEvent {
                                handled: false,
                                kind: UIEventKind::MouseDown {
                                    pos: self.mouse_position,
                                    button: *button,
                                },
                                target: Handle::NONE,
                                source: self.picked_node,
                            });
                            event_processed = true;
                        }
                    }
                    ElementState::Released => {
                        if !self.picked_node.is_none() {
                            self.events.push_back(UIEvent {
                                handled: false,
                                kind: UIEventKind::MouseUp {
                                    pos: self.mouse_position,
                                    button: *button,
                                },
                                target: Handle::NONE,
                                source: self.picked_node,
                            });
                            event_processed = true;
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = Vec2::new(position.x as f32, position.y as f32);
                self.picked_node = self.hit_test(self.mouse_position);

                // Fire mouse leave for previously picked node
                if self.picked_node != self.prev_picked_node {
                    let mut fire_mouse_leave = false;
                    if self.prev_picked_node.is_some() {
                        let prev_picked_node = self.nodes.borrow_mut(self.prev_picked_node).widget_mut();
                        if prev_picked_node.is_mouse_over {
                            prev_picked_node.is_mouse_over = false;
                            fire_mouse_leave = true;
                        }
                    }

                    if fire_mouse_leave {
                        self.events.push_back(UIEvent {
                            handled: false,
                            kind: UIEventKind::MouseLeave,
                            target: Handle::NONE,
                            source: self.prev_picked_node,
                        });
                    }
                }

                if !self.picked_node.is_none() {
                    let mut fire_mouse_enter = false;
                    let picked_node = self.nodes.borrow_mut(self.picked_node).widget_mut();
                    if !picked_node.is_mouse_over {
                        picked_node.is_mouse_over = true;
                        fire_mouse_enter = true;
                    }

                    if fire_mouse_enter {
                        self.events.push_back(UIEvent {
                            handled: false,
                            kind: UIEventKind::MouseEnter,
                            target: Handle::NONE,
                            source: self.picked_node,
                        });
                    }

                    // Fire mouse move
                    self.events.push_back(UIEvent {
                        handled: false,
                        kind: UIEventKind::MouseMove {
                            pos: self.mouse_position
                        },
                        target: Handle::NONE,
                        source: self.picked_node,
                    });

                    event_processed = true;
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let MouseScrollDelta::LineDelta(_, y) = delta {
                    if !self.picked_node.is_none() {
                        self.events.push_back(UIEvent {
                            handled: false,
                            kind: UIEventKind::MouseWheel {
                                pos: self.mouse_position,
                                amount: *y,
                            },
                            target: Handle::NONE,
                            source: self.picked_node,
                        });

                        event_processed = true;
                    }
                }
            }
            WindowEvent::KeyboardInput { input, .. } => {
                if self.keyboard_focus_node.is_some() {
                    if let Some(keycode) = input.virtual_keycode {
                        let event = UIEvent {
                            handled: false,
                            kind: match input.state {
                                ElementState::Pressed => {
                                    UIEventKind::KeyDown {
                                        code: keycode,
                                    }
                                }
                                ElementState::Released => {
                                    UIEventKind::KeyUp {
                                        code: keycode,
                                    }
                                }
                            },
                            target: Handle::NONE,
                            source: self.keyboard_focus_node,
                        };

                        self.events.push_back(event);

                        event_processed = true;
                    }
                }
            },
            WindowEvent::ReceivedCharacter(unicode) => {
                if self.keyboard_focus_node.is_some() {
                    let event = UIEvent {
                        handled: false,
                        kind: UIEventKind::Text {
                            symbol: *unicode
                        },
                        target: Handle::NONE,
                        source: self.keyboard_focus_node
                    };

                    self.events.push_back(event);

                    event_processed = true;
                }
            }
            _ => ()
        }

        self.prev_picked_node = self.picked_node;

        event_processed
    }
}

