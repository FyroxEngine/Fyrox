//! Extendable UI library.
//!
//! See examples here - https://github.com/mrDIMAS/rusty-shooter/blob/master/src/menu.rs

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate downcast_rs;

pub use rg3d_core as core;

pub mod draw;
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
pub mod check_box;
pub mod style;
pub mod tab_control;
pub mod ttf;

use std::{
    collections::{
        VecDeque,
        HashMap,
    },
    any::Any,
    sync::{
        Arc,
        Mutex,
    },
    rc::Rc,
};
use crate::{
    core::{
        color::Color,
        pool::{
            Pool,
            Handle,
        },
        math::{
            vec2::Vec2,
            Rect,
        },
    },
    draw::{
        DrawingContext,
        CommandKind,
        CommandTexture,
    },
    canvas::Canvas,
    event::{
        UIEvent,
        UIEventKind,
    },
    style::Style,
    widget::Widget,
    ttf::Font,
};
use crate::event::{OsEvent, ButtonState};

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
    pub fn zero() -> Self {
        Self { left: 0.0, top: 0.0, right: 0.0, bottom: 0.0 }
    }

    pub fn uniform(v: f32) -> Self {
        Self { left: v, top: v, right: v, bottom: v }
    }

    pub fn bottom(v: f32) -> Self {
        Self { left: 0.0, top: 0.0, right: 0.0, bottom: v }
    }

    pub fn top(v: f32) -> Self {
        Self { left: 0.0, top: v, right: 0.0, bottom: 0.0 }
    }

    pub fn left(v: f32) -> Self {
        Self { left: v, top: 0.0, right: 0.0, bottom: 0.0 }
    }

    pub fn right(v: f32) -> Self {
        Self { left: 0.0, top: 0.0, right: v, bottom: 0.0 }
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

/// Trait for all UI controls in engine.
///
/// Control must provide at least references (shared and mutable) to inner widget,
/// which means that any control must be based on widget struct.
/// Any other methods will be auto-implemented.
pub trait Control: downcast_rs::Downcast {
    fn widget(&self) -> &Widget;

    fn widget_mut(&mut self) -> &mut Widget;

    /// Creates raw copy of control
    fn raw_copy(&self) -> Box<dyn Control>;

    fn resolve(&mut self, template: &ControlTemplate, node_map: &HashMap<Handle<UINode>, Handle<UINode>>);

    fn measure_override(&self, ui: &UserInterface, available_size: Vec2) -> Vec2 {
        let mut size = Vec2::ZERO;

        for child_handle in self.widget().children.iter() {
            ui.node(*child_handle).measure(ui, available_size);

            let child = ui.node(*child_handle).widget();
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

        for child_handle in self.widget().children.iter() {
            ui.node(*child_handle).arrange(ui, &final_rect);
        }

        final_size
    }

    fn arrange(&self, ui: &UserInterface, final_rect: &Rect<f32>) {
        let widget = self.widget();
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

        size = self.arrange_override(ui, size);

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

    fn measure(&self, ui: &UserInterface, available_size: Vec2) {
        let widget = self.widget();
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
            let mut desired_size = self.measure_override(ui, size_for_child);

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

    fn draw(&self, _drawing_context: &mut DrawingContext) {}

    fn update(&mut self, _dt: f32) {}

    fn set_property(&mut self, _name: &str, _value: &dyn Any) {}

    fn get_property(&self, _name: &str) -> Option<&'_ dyn Any> {
        None
    }

    /// Performs event-specific actions.
    ///
    /// # Notes
    ///
    /// Do *not* try to borrow node by `self_handle` in UI - at this moment node has been moved
    /// out of pool and attempt of borrowing will cause panic! `self_handle` should be used only
    /// to check if event came from/for this node or to capture input on node.
    fn handle_event(&mut self, _self_handle: Handle<UINode>, _ui: &mut UserInterface, _evt: &mut UIEvent) {}

    fn apply_style(&mut self, style: Rc<Style>) {
        // Apply base style first.
        if let Some(base_style) = style.base_style() {
            self.apply_style(base_style);
        }

        // Remember last applied style.
        self.widget_mut().set_style(style.clone());

        // Then apply current.
        for setter in style.setters() {
            self.set_property(setter.name(), setter.value());
        }
    }
}

impl_downcast!(Control);

pub type UINode = Box<dyn Control>;

pub struct UserInterface {
    nodes: Pool<UINode>,
    drawing_context: DrawingContext,
    visual_debug: bool,
    /// Every UI node will live on the window-sized canvas.
    root_canvas: Handle<UINode>,
    picked_node: Handle<UINode>,
    prev_picked_node: Handle<UINode>,
    captured_node: Handle<UINode>,
    keyboard_focus_node: Handle<UINode>,
    mouse_position: Vec2,
    events: VecDeque<UIEvent>,
    stack: Vec<Handle<UINode>>,
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

lazy_static! {
    static ref DEFAULT_FONT: Arc<Mutex<Font>> = {
        let font_bytes = std::include_bytes!("./built_in_font.ttf").to_vec();
        let font = Font::from_memory(font_bytes, 20.0, Font::default_char_set()).unwrap();
        Arc::new(Mutex::new(font))
    };
}

impl UserInterface {
    pub fn new() -> UserInterface {
        let mut ui = UserInterface {
            events: VecDeque::new(),
            visual_debug: false,
            captured_node: Handle::NONE,
            root_canvas: Handle::NONE,
            nodes: Pool::new(),
            mouse_position: Vec2::ZERO,
            drawing_context: DrawingContext::new(),
            picked_node: Handle::NONE,
            prev_picked_node: Handle::NONE,
            keyboard_focus_node: Handle::NONE,
            stack: Default::default(),
        };
        ui.root_canvas = ui.add_node(Box::new(Canvas::new(Widget::default())));
        ui
    }

    #[inline]
    pub fn capture_mouse(&mut self, node: Handle<UINode>) -> bool {
        if self.captured_node.is_none() {
            self.captured_node = node;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn release_mouse_capture(&mut self) {
        self.captured_node = Handle::NONE;
    }

    #[inline]
    pub fn get_drawing_context(&self) -> &DrawingContext {
        &self.drawing_context
    }

    #[inline]
    pub fn get_drawing_context_mut(&mut self) -> &mut DrawingContext {
        &mut self.drawing_context
    }

    fn update_nodes(&mut self) {
        self.stack.clear();
        self.stack.push(self.root_canvas);
        while let Some(node_handle) = self.stack.pop() {
            let widget = self.nodes.borrow(node_handle).widget();
            for child_handle in widget.children.iter() {
                self.stack.push(*child_handle);
            }
            let (screen_position, parent_visibility) =
                if widget.parent.is_some() {
                    let parent_widget = self.nodes.borrow(widget.parent).widget();
                    (widget.actual_local_position.get() + parent_widget.screen_position, parent_widget.global_visibility)
                } else {
                    (widget.actual_local_position.get(), true)
                };
            let widget = self.nodes.borrow_mut(node_handle).widget_mut();
            widget.screen_position = screen_position;
            widget.global_visibility = widget.visibility == Visibility::Visible && parent_visibility;
        }
    }

    pub fn update(&mut self, screen_size: Vec2, dt: f32) {
        self.node(self.root_canvas)
            .measure(self, screen_size);
        self.node(self.root_canvas)
            .arrange(self, &Rect::new(0.0, 0.0, screen_size.x, screen_size.y));
        self.update_nodes();
        for node in self.nodes.iter_mut() {
            node.update(dt)
        }
    }

    fn draw_node(&mut self, node_handle: Handle<UINode>, nesting: u8) {
        let node = self.nodes.borrow(node_handle);
        let bounds = node.widget().get_screen_bounds();
        if !node.widget().global_visibility {
            return;
        }

        let start_index = self.drawing_context.get_commands().len();
        self.drawing_context.set_nesting(nesting);
        self.drawing_context.commit_clip_rect(&bounds.inflate(0.9, 0.9));

        node.draw(&mut self.drawing_context);

        let widget = node.widget();

        let end_index = self.drawing_context.get_commands().len();
        for i in start_index..end_index {
            widget.command_indices
                .borrow_mut()
                .push(i);
        }

        let children = unsafe { &(*(widget as *const Widget)).children };

        // Continue on children
        for child_node in children.iter() {
            self.draw_node(*child_node, nesting + 1);
        }

        self.drawing_context.revert_clip_geom();
    }

    pub fn draw(&mut self) -> &DrawingContext {
        self.drawing_context.clear();

        for node in self.nodes.iter_mut() {
            node.widget_mut()
                .command_indices
                .borrow_mut()
                .clear();
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
        if !widget.global_visibility {
            return clipped;
        }

        for command_index in widget.command_indices.borrow().iter() {
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

        if !widget.global_visibility {
            return false;
        }

        if !self.is_node_clipped(node_handle, pt) {
            for command_index in widget.command_indices.borrow().iter() {
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
        let widget = self.nodes.borrow(node_handle).widget();

        if !widget.is_hit_test_visible {
            return Handle::NONE;
        }

        let (mut picked, mut topmost_picked_level) =
            if self.is_node_contains_point(node_handle, pt) {
                (node_handle, *level)
            } else {
                (Handle::NONE, 0)
            };

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
        self.nodes.borrow(root_handle).widget().has_descendant(node_handle, self)
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

    /// Searches for a node up on tree that satisfies some criteria and then borrows
    /// shared reference.
    ///
    /// # Panics
    ///
    /// It will panic if there no node that satisfies given criteria.
    pub fn borrow_by_criteria_up<Func>(&self, start_node_handle: Handle<UINode>, func: Func) -> &UINode
        where Func: Fn(&UINode) -> bool {
        self.nodes.borrow(self.find_by_criteria_up(start_node_handle, func))
    }

    /// Searches for a node up on tree that satisfies some criteria and then borrows
    /// mutable reference.
    ///
    /// # Panics
    ///
    /// It will panic if there no node that satisfies given criteria.
    pub fn borrow_by_criteria_up_mut<Func>(&mut self, start_node_handle: Handle<UINode>, func: Func) -> &mut UINode
        where Func: Fn(&UINode) -> bool {
        self.nodes.borrow_mut(self.find_by_criteria_up(start_node_handle, func))
    }

    /// Pushes new UI event to the common queue. Could be useful to send an event
    /// to some specific node. Lets take a window for example, we can close or open
    /// it by just sending an appropriate event - something like this:
    ///
    /// ```no_run
    /// use rg3d::gui::event::{UIEvent, UIEventKind};
    ///
    /// ui.send_event(UIEvent::targeted(options_window, UIEventKind::Opened));
    /// ```
    pub fn send_event(&mut self, event: UIEvent) {
        self.events.push_back(event);
    }

    /// Extracts UI event one-by-one from common queue. Each extracted event will go to *all*
    /// available nodes first and only then will be moved outside of this method. This is one
    /// of most important methods which must be called each frame of your game loop, otherwise
    /// UI will not respond to any kind of events and simply speaking will just not work.
    pub fn poll_ui_event(&mut self) -> Option<UIEvent> {
        // Gather events from nodes.
        for (handle, node) in self.nodes.pair_iter_mut() {
            while let Some(mut response_event) = node.widget_mut().events.borrow_mut().pop_front() {
                response_event.source = handle;
                self.events.push_back(response_event)
            }
        }

        let mut event = self.events.pop_front();

        if let Some(ref mut event) = event {
            for i in 0..self.nodes.get_capacity() {
                if let Some(mut node) = self.nodes.take_at(i) {
                    node.handle_event(self.nodes.handle_from_index(i), self, event);

                    let old = self.nodes.replace_at(i, node);
                    assert!(old.is_none());
                }
            }
        }

        event
    }

    /// Translates raw window event into some specific UI event. This is one of the
    /// most important methods of UI. You must call it each time you received a message
    /// from a window.
    pub fn process_input_event(&mut self, event: &OsEvent) -> bool {
        let mut event_processed = false;

        match event {
            OsEvent::MouseInput { button, state, .. } => {
                match state {
                    ButtonState::Pressed => {
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
                    ButtonState::Released => {
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
            OsEvent::CursorMoved { position } => {
                self.mouse_position = *position;
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
            OsEvent::MouseWheel(_, y) => {
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
            OsEvent::KeyboardInput { button, state } => {
                if self.keyboard_focus_node.is_some() {
                    let event = UIEvent {
                        handled: false,
                        kind: match state {
                            ButtonState::Pressed => {
                                UIEventKind::KeyDown {
                                    code: *button,
                                }
                            }
                            ButtonState::Released => {
                                UIEventKind::KeyUp {
                                    code: *button,
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
            OsEvent::Character(unicode) => {
                if self.keyboard_focus_node.is_some() {
                    let event = UIEvent {
                        handled: false,
                        kind: UIEventKind::Text {
                            symbol: *unicode
                        },
                        target: Handle::NONE,
                        source: self.keyboard_focus_node,
                    };

                    self.events.push_back(event);

                    event_processed = true;
                }
            }
        }

        self.prev_picked_node = self.picked_node;

        event_processed
    }
}

pub fn bool_to_visibility(value: bool) -> Visibility {
    if value {
        Visibility::Visible
    } else {
        Visibility::Collapsed
    }
}

impl UINodeContainer for UserInterface {
    fn nodes(&self) -> &Pool<UINode> {
        &self.nodes
    }

    fn nodes_mut(&mut self) -> &mut Pool<UINode> {
        &mut self.nodes
    }

    fn root(&self) -> Handle<UINode> {
        self.root_canvas
    }

    fn add_node(&mut self, mut node: UINode) -> Handle<UINode> {
        let children = node.widget().children.clone();
        node.widget_mut().children.clear();
        let node_handle = self.nodes_mut().spawn(node);
        if self.root_canvas.is_some() {
            self.link_nodes(node_handle, self.root_canvas);
        }
        for child in children {
            self.link_nodes(child, node_handle)
        }
        node_handle
    }
}

pub struct ControlTemplate {
    nodes: Pool<UINode>
}

impl UINodeContainer for ControlTemplate {
    fn nodes(&self) -> &Pool<UINode> {
        &self.nodes
    }

    fn nodes_mut(&mut self) -> &mut Pool<UINode> {
        &mut self.nodes
    }

    fn root(&self) -> Handle<UINode> {
        for (handle, node) in self.nodes.pair_iter() {
            if node.widget().parent.is_none() {
                return handle;
            }
        }
        Handle::NONE
    }

    fn add_node(&mut self, mut node: UINode) -> Handle<UINode> {
        let children = node.widget().children.clone();
        node.widget_mut().children.clear();
        let node_handle = self.nodes_mut().spawn(node);
        for child in children {
            self.link_nodes(child, node_handle)
        }
        node_handle
    }
}

impl ControlTemplate {
    pub fn new() -> Self {
        Self {
            nodes: Default::default()
        }
    }

    pub fn instantiate(&self, container: &mut dyn UINodeContainer) -> Handle<UINode> {
        let mut map = HashMap::new();

        let root = self.instantiate_internal(self.root(), container, &mut map);

        // Resolve all instantiated nodes using template-to-ui node mapping.
        // This stage is required because some ui nodes may contain handles
        // to *template* nodes because of `raw_copy` method which does not
        // perform remapping.
        for node_handle in map.values() {
            container.nodes_mut()
                .borrow_mut(*node_handle)
                .resolve(self, &map);
        }

        root
    }

    fn instantiate_internal(&self, node_handle: Handle<UINode>, container: &mut dyn UINodeContainer, map: &mut HashMap<Handle<UINode>, Handle<UINode>>) -> Handle<UINode> {
        let node = self.nodes.borrow(node_handle);

        // Instantiate children first.
        let resolved_children = node.widget()
            .children
            .iter()
            .map(|c| self.instantiate_internal(*c, container, map))
            .collect();

        // Instantiate node.
        let mut copy = node.raw_copy();
        copy.widget_mut().children = resolved_children;
        let copy_handle = container.add_node(copy);
        map.insert(node_handle, copy_handle);
        copy_handle
    }
}

pub trait UINodeContainer {
    fn nodes(&self) -> &Pool<UINode>;

    fn nodes_mut(&mut self) -> &mut Pool<UINode>;

    fn root(&self) -> Handle<UINode>;

    fn add_node(&mut self, node: UINode) -> Handle<UINode>;

    #[inline]
    fn node(&self, node_handle: Handle<UINode>) -> &UINode {
        self.nodes()
            .borrow(node_handle)
    }

    #[inline]
    fn node_mut(&mut self, node_handle: Handle<UINode>) -> &mut UINode {
        self.nodes_mut()
            .borrow_mut(node_handle)
    }

    /// Links specified child with specified parent.
    #[inline]
    fn link_nodes(&mut self, child_handle: Handle<UINode>, parent_handle: Handle<UINode>) {
        assert_ne!(child_handle, parent_handle);
        self.unlink_node(child_handle);
        let child = self.nodes_mut()
            .borrow_mut(child_handle)
            .widget_mut();
        child.parent = parent_handle;
        let parent = self.nodes_mut()
            .borrow_mut(parent_handle)
            .widget_mut();
        parent.children.push(child_handle);
    }

    /// Unlinks specified node from its parent, so node will become root.
    #[inline]
    fn unlink_node(&mut self, node_handle: Handle<UINode>) {
        let parent_handle;
        // Replace parent handle of child
        let node = self.nodes_mut().borrow_mut(node_handle);
        parent_handle = node.widget().parent;
        node.widget_mut().parent = Handle::NONE;

        // Remove child from parent's children list
        if parent_handle.is_some() {
            let parent = self.nodes_mut().borrow_mut(parent_handle);
            if let Some(i) = parent.widget().children.iter().position(|h| *h == node_handle) {
                parent.widget_mut().children.remove(i);
            }
        }
    }
}

pub trait Builder {
    fn build(self, container: &mut dyn UINodeContainer) -> Handle<UINode> where Self: Sized;
}