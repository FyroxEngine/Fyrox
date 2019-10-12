#[macro_use]
pub mod builder;
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

use std::{
    collections::VecDeque,
    any::TypeId, rc::Rc,
    cell::RefCell,
};
use crate::{
    gui::{
        node::{UINode, UINodeKind},
        draw::{DrawingContext, CommandKind, CommandTexture},
        scroll_viewer::ScrollViewer,
        canvas::Canvas,
        event::{UIEvent, UIEventKind},
    },
    resource::{ttf::Font},
    utils::UnsafeCollectionView,
    ElementState, WindowEvent,
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
    fn measure_override(&self, self_handle: Handle<UINode>, ui: &UserInterface, available_size: Vec2) -> Vec2;
    fn arrange_override(&self, self_handle: Handle<UINode>, ui: &UserInterface, final_size: Vec2) -> Vec2;
}

trait Drawable {
    fn draw(&mut self, drawing_context: &mut DrawingContext, bounds: &Rect<f32>, color: Color);
}

pub type DeferredAction = dyn FnMut(&mut UserInterface);

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
    mouse_position: Vec2,
    deferred_actions: VecDeque<Box<DeferredAction>>,
    events: VecDeque<UIEvent>,
}

pub trait EventSource {
    fn emit_event(&mut self) -> Option<UIEvent>;
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
            mouse_position: Vec2::zero(),
            drawing_context: DrawingContext::new(),
            picked_node: Handle::NONE,
            prev_picked_node: Handle::NONE,
            deferred_actions: VecDeque::new(),
        };
        ui.root_canvas = ui.add_node(UINode::new(UINodeKind::Canvas(Canvas::new())));
        ui
    }

    #[inline]
    pub fn get_default_font(&self) -> Rc<RefCell<Font>> {
        self.default_font.clone()
    }

    pub fn add_node(&mut self, node: UINode) -> Handle<UINode> {
        let node_handle = self.nodes.spawn(node);
        if self.root_canvas.is_some() {
            self.link_nodes(node_handle, self.root_canvas);
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

    #[inline]
    pub fn begin_invoke(&mut self, action: Box<DeferredAction>) {
        self.deferred_actions.push_back(action)
    }

    /// Links specified child with specified parent.
    #[inline]
    pub fn link_nodes(&mut self, child_handle: Handle<UINode>, parent_handle: Handle<UINode>) {
        self.unlink_node(child_handle);
        let child = self.nodes.borrow_mut(child_handle);
        child.parent = parent_handle;
        let parent = self.nodes.borrow_mut(parent_handle);
        parent.children.push(child_handle);
    }

    /// Unlinks specified node from its parent, so node will become root.
    #[inline]
    pub fn unlink_node(&mut self, node_handle: Handle<UINode>) {
        let parent_handle;
        // Replace parent handle of child
        {
            let node = self.nodes.borrow_mut(node_handle);
            parent_handle = node.parent;
            node.parent = Handle::NONE;
        }
        // Remove child from parent's children list
        if parent_handle.is_some() {
            let parent = self.nodes.borrow_mut(parent_handle);
            if let Some(i) = parent.children.iter().position(|h| *h == node_handle) {
                parent.children.remove(i);
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

    fn default_measure_override(&self, handle: Handle<UINode>, available_size: Vec2) -> Vec2 {
        let mut size = Vec2::zero();

        let node = self.nodes.borrow(handle);
        for child_handle in node.children.iter() {
            self.measure(*child_handle, available_size);

            let child = self.nodes.borrow(*child_handle);
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

    fn measure(&self, node_handle: Handle<UINode>, available_size: Vec2) {
        let node = self.nodes.borrow(node_handle);
        let margin = Vec2 {
            x: node.margin.left + node.margin.right,
            y: node.margin.top + node.margin.bottom,
        };

        let size_for_child = Vec2 {
            x: {
                let w = if node.width.get() > 0.0 {
                    node.width.get()
                } else {
                    maxf(0.0, available_size.x - margin.x)
                };

                if w > node.max_size.x {
                    node.max_size.x
                } else if w < node.min_size.x {
                    node.min_size.x
                } else {
                    w
                }
            },
            y: {
                let h = if node.height.get() > 0.0 {
                    node.height.get()
                } else {
                    maxf(0.0, available_size.y - margin.y)
                };

                if h > node.max_size.y {
                    node.max_size.y
                } else if h < node.min_size.y {
                    node.min_size.y
                } else {
                    h
                }
            },
        };

        if node.visibility == Visibility::Visible {
            let mut desired_size = node.measure_override(node_handle, self, size_for_child);

            if !node.width.get().is_nan() {
                desired_size.x = node.width.get();
            }

            if desired_size.x > node.max_size.x {
                desired_size.x = node.max_size.x;
            } else if desired_size.x < node.min_size.x {
                desired_size.x = node.min_size.x;
            }

            if desired_size.y > node.max_size.y {
                desired_size.y = node.max_size.y;
            } else if desired_size.y < node.min_size.y {
                desired_size.y = node.min_size.y;
            }

            if !node.height.get().is_nan() {
                desired_size.y = node.height.get();
            }

            desired_size += margin;

            // Make sure that node won't go outside of available bounds.
            if desired_size.x > available_size.x {
                desired_size.x = available_size.x;
            }
            if desired_size.y > available_size.y {
                desired_size.y = available_size.y;
            }

            node.desired_size.set(desired_size);
        } else {
            node.desired_size.set(Vec2::make(0.0, 0.0));
        }

        node.measure_valid.set(true)
    }

    fn default_arrange_override(&self, handle: Handle<UINode>, final_size: Vec2) -> Vec2 {
        let final_rect = Rect::new(0.0, 0.0, final_size.x, final_size.y);

        let node = self.nodes.borrow(handle);
        for child_handle in node.children.iter() {
            self.arrange(*child_handle, &final_rect);
        }


        final_size
    }

    fn arrange(&self, node_handle: Handle<UINode>, final_rect: &Rect<f32>) {
        let node = self.nodes.borrow(node_handle);
        if node.visibility != Visibility::Visible {
            return;
        }

        let margin_x = node.margin.left + node.margin.right;
        let margin_y = node.margin.top + node.margin.bottom;

        let mut origin_x = final_rect.x + node.margin.left;
        let mut origin_y = final_rect.y + node.margin.top;

        let mut size = Vec2 {
            x: maxf(0.0, final_rect.w - margin_x),
            y: maxf(0.0, final_rect.h - margin_y),
        };

        let size_without_margin = size;

        if node.horizontal_alignment != HorizontalAlignment::Stretch {
            size.x = minf(size.x, node.desired_size.get().x - margin_x);
        }
        if node.vertical_alignment != VerticalAlignment::Stretch {
            size.y = minf(size.y, node.desired_size.get().y - margin_y);
        }

        if node.width.get() > 0.0 {
            size.x = node.width.get();
        }
        if node.height.get() > 0.0 {
            size.y = node.height.get();
        }

        size = node.arrange_override(node_handle, self, size);

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

        node.actual_size.set(size);
        node.actual_local_position.set(Vec2 { x: origin_x, y: origin_y });
        node.arrange_valid.set(true);
    }

    fn update_transform(&mut self, node_handle: Handle<UINode>) {
        let screen_position;
        let node = self.nodes.borrow(node_handle);
        let children = UnsafeCollectionView::from_slice(&node.children);
        if node.parent.is_some() {
            screen_position = node.actual_local_position.get() + self.nodes.borrow(node.parent).screen_position;
        } else {
            screen_position = node.actual_local_position.get();
        }

        self.nodes.borrow_mut(node_handle).screen_position = screen_position;

        // Continue on children
        for child_handle in children.iter() {
            self.update_transform(*child_handle);
        }
    }

    pub fn update(&mut self, screen_size: Vec2) {
        self.measure(self.root_canvas, screen_size);
        self.arrange(self.root_canvas, &Rect::new(0.0, 0.0, screen_size.x, screen_size.y));
        self.update_transform(self.root_canvas);

        // Do deferred actions. Some sort of simplest dispatcher.
        while let Some(mut action) = self.deferred_actions.pop_front() {
            action(self)
        }

        for i in 0..self.nodes.get_capacity() {
            let id = if let Some(node) = self.nodes.at(i) {
                node.get_kind_id()
            } else {
                continue;
            };

            let handle = self.nodes.handle_from_index(i);
            if id == TypeId::of::<ScrollViewer>() {
                ScrollViewer::update(handle, self);
            }
        }
    }

    fn draw_node(&mut self, node_handle: Handle<UINode>, nesting: u8) {
        let children;

        let node = self.nodes.borrow_mut(node_handle);
        if node.visibility != Visibility::Visible {
            return;
        }

        let start_index = self.drawing_context.get_commands().len();
        let bounds = node.get_screen_bounds();

        self.drawing_context.set_nesting(nesting);
        self.drawing_context.commit_clip_rect(&bounds.inflate(0.9, 0.9));

        node.kind.draw(&mut self.drawing_context, &bounds, node.color);

        children = UnsafeCollectionView::from_slice(&node.children);

        let end_index = self.drawing_context.get_commands().len();
        for i in start_index..end_index {
            node.command_indices.push(i);
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
            node.command_indices.clear();
        }

        let root_canvas = self.root_canvas;
        self.draw_node(root_canvas, 1);

        if self.visual_debug {
            self.drawing_context.set_nesting(0);

            let picked_bounds =
                if self.picked_node.is_some() {
                    Some(self.nodes.borrow(self.picked_node).get_screen_bounds())
                } else {
                    None
                };

            if let Some(picked_bounds) = picked_bounds {
                self.drawing_context.push_rect(&picked_bounds, 1.0, Color::white());
                self.drawing_context.commit(CommandKind::Geometry, CommandTexture::None);
            }
        }

        &self.drawing_context
    }

    fn is_node_clipped(&self, node_handle: Handle<UINode>, pt: Vec2) -> bool {
        let mut clipped = true;

        let node = self.nodes.borrow(node_handle);
        if node.visibility != Visibility::Visible {
            return clipped;
        }

        for command_index in node.command_indices.iter() {
            if let Some(command) = self.drawing_context.get_commands().get(*command_index) {
                if *command.get_kind() == CommandKind::Clip && self.drawing_context.is_command_contains_point(command, pt) {
                    clipped = false;
                    break;
                }
            }
        }

        // Point can be clipped by parent's clipping geometry.
        if !node.parent.is_none() && !clipped {
            clipped |= self.is_node_clipped(node.parent, pt);
        }

        clipped
    }

    fn is_node_contains_point(&self, node_handle: Handle<UINode>, pt: Vec2) -> bool {
        let node = self.nodes.borrow(node_handle);

        if node.visibility != Visibility::Visible {
            return false;
        }

        if !self.is_node_clipped(node_handle, pt) {
            for command_index in node.command_indices.iter() {
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
        let mut picked = Handle::NONE;
        let mut topmost_picked_level = 0;

        if self.is_node_contains_point(node_handle, pt) {
            picked = node_handle;
            topmost_picked_level = *level;
        }

        let node = self.nodes.borrow(node_handle);
        for child_handle in node.children.iter() {
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

        for child_handle in node.children.iter() {
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

        self.find_by_criteria_up(node.parent, func)
    }

    pub fn is_node_child_of(&self, node_handle: Handle<UINode>, root_handle: Handle<UINode>) -> bool {
        let root = self.nodes.borrow(root_handle);
        for child_handle in root.children.iter() {
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

    /// Searches a node by name up on tree starting from given root node.
    pub fn find_by_name_up(&self, node_handle: Handle<UINode>, name: &str) -> Handle<UINode> {
        self.find_by_criteria_up(node_handle, |node| node.name == name)
    }

    /// Searches a node by name down on tree starting from given root node.
    pub fn find_by_name_down(&self, node_handle: Handle<UINode>, name: &str) -> Handle<UINode> {
        self.find_by_criteria_down(node_handle, &|node| node.name == name)
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
            while let Some(mut response_event) = node.emit_event() {
                response_event.source = handle;
                self.events.push_back(response_event)
            }
        }

        let mut event = self.events.pop_front();

        // Pass event to all nodes first
        if let Some(ref mut event) = event {
            for i in 0..self.nodes.get_capacity() {
                let mut handler = if let Some(node) = self.nodes.at_mut(i) {
                    // Take...
                    node.event_handler.take()
                } else {
                    None
                };

                if let Some(ref mut handler) = handler {
                    // Call...
                    handler(self, self.nodes.handle_from_index(i), event);
                }

                if let Some(node) = self.nodes.at_mut(i) {
                    // Put back trick.
                    if let Some(handler) = handler {
                        node.event_handler.replace(handler);
                    }
                }
            }
        }

        event
    }

    pub fn get_node_kind_id(&self, handle: Handle<UINode>) -> TypeId {
        self.nodes.borrow(handle).get_kind_id()
    }

    pub fn process_input_event(&mut self, event: &WindowEvent) -> bool {
        let mut event_processed = false;

        match event {
            WindowEvent::MouseInput { button, state, .. } => {
                match state {
                    ElementState::Pressed => {
                        self.picked_node = self.hit_test(self.mouse_position);

                        if !self.picked_node.is_none() {
                            self.events.push_back(UIEvent {
                                handled: false,
                                kind: UIEventKind::MouseDown {
                                    pos: self.mouse_position,
                                    button: *button,
                                },
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
                                source: self.picked_node,
                            });
                            event_processed = true;
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = Vec2::make(position.x as f32, position.y as f32);
                self.picked_node = self.hit_test(self.mouse_position);

                // Fire mouse leave for previously picked node
                if self.picked_node != self.prev_picked_node {
                    let mut fire_mouse_leave = false;
                    if self.prev_picked_node.is_some() {
                        let prev_picked_node = self.nodes.borrow_mut(self.prev_picked_node);
                        if prev_picked_node.is_mouse_over {
                            prev_picked_node.is_mouse_over = false;
                            fire_mouse_leave = true;
                        }
                    }

                    if fire_mouse_leave {
                        self.events.push_back(UIEvent {
                            handled: false,
                            kind: UIEventKind::MouseLeave,
                            source: self.prev_picked_node,
                        });
                    }
                }

                if !self.picked_node.is_none() {
                    let mut fire_mouse_enter = false;
                    let picked_node = self.nodes.borrow_mut(self.picked_node);
                    if !picked_node.is_mouse_over {
                        picked_node.is_mouse_over = true;
                        fire_mouse_enter = true;
                    }

                    if fire_mouse_enter {
                        self.events.push_back(UIEvent {
                            handled: false,
                            kind: UIEventKind::MouseEnter,
                            source: self.picked_node,
                        });
                    }

                    // Fire mouse move
                    self.events.push_back(UIEvent {
                        handled: false,
                        kind: UIEventKind::MouseMove {
                            pos: self.mouse_position
                        },
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
                            source: self.picked_node,
                        });

                        event_processed = true;
                    }
                }
            }
            _ => ()
        }

        self.prev_picked_node = self.picked_node;

        event_processed
    }
}

