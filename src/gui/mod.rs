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

use glutin::{
    WindowEvent,
    ElementState,
    MouseScrollDelta,
};
use std::{
    collections::VecDeque,
    any::TypeId,
    rc::Rc,
    cell::RefCell
};
use crate::{
    gui::{
        node::{UINode, UINodeKind},
        draw::{
            DrawingContext,
            CommandKind,
            CommandTexture
        },
        scroll_viewer::ScrollViewer,
        event::{RoutedEvent, RoutedEventKind, RoutedEventHandlerType},
        canvas::Canvas,
    },
    resource::{ttf::Font},
    utils::UnsafeCollectionView,
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

trait Drawable {
    fn draw(&mut self, drawing_context: &mut DrawingContext, bounds: &Rect<f32>, color: Color);
}

impl Drawable for UINodeKind {
    fn draw(&mut self, drawing_context: &mut DrawingContext, bounds: &Rect<f32>, color: Color) {
        match self {
            UINodeKind::Text(text) => text.draw(drawing_context, bounds, color),
            UINodeKind::Border(border) => border.draw(drawing_context, bounds, color),
            _ => ()
        }
    }
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
    pub fn new(default_font: Rc<RefCell<Font>>) -> UserInterface {
        let mut ui = UserInterface {
            visual_debug: false,
            default_font,
            captured_node: Handle::none(),
            root_canvas: Handle::none(),
            nodes: Pool::new(),
            mouse_position: Vec2::zero(),
            drawing_context: DrawingContext::new(),
            picked_node: Handle::none(),
            prev_picked_node: Handle::none(),
            deferred_actions: VecDeque::new(),
        };
        ui.root_canvas = ui.add_node(UINode::new(UINodeKind::Canvas(Canvas::new())));
        ui
    }

    pub fn add_node(&mut self, node: UINode) -> Handle<UINode> {
        let node_handle = self.nodes.spawn(node);
        // Notify kind about owner. This is a bit hackish but it'll make a lot of things easier.
        if let Some(node) = self.nodes.borrow_mut(node_handle) {
            match node.get_kind_mut() {
                UINodeKind::ScrollBar(scroll_bar) => scroll_bar.owner_handle = node_handle,
                UINodeKind::Text(text) => text.owner_handle = node_handle,
                UINodeKind::Border(border) => border.owner_handle = node_handle,
                UINodeKind::Button(button) => button.owner_handle = node_handle,
                UINodeKind::ScrollViewer(scroll_viewer) => scroll_viewer.owner_handle = node_handle,
                UINodeKind::Image(image) => image.owner_handle = node_handle,
                UINodeKind::Grid(grid) => grid.owner_handle = node_handle,
                UINodeKind::Canvas(canvas) => canvas.owner_handle = node_handle,
                UINodeKind::ScrollContentPresenter(scp) => scp.owner_handle = node_handle,
                UINodeKind::Window(window) => window.owner_handle = node_handle,
                UINodeKind::User(user) => user.set_owner_handle(node_handle)
            }
        }
        self.link_nodes(node_handle, self.root_canvas);
        node_handle
    }

    pub fn capture_mouse(&mut self, node: Handle<UINode>) -> bool {
        if self.captured_node.is_none() && self.nodes.is_valid_handle(node) {
            self.captured_node = node;
            return true;
        }

        false
    }

    pub fn release_mouse_capture(&mut self) {
        self.captured_node = Handle::none();
    }

    pub fn begin_invoke(&mut self, action: Box<DeferredAction>) {
        self.deferred_actions.push_back(action)
    }

    /// Links specified child with specified parent.
    #[inline]
    pub fn link_nodes(&mut self, child_handle: Handle<UINode>, parent_handle: Handle<UINode>) {
        self.unlink_node(child_handle);
        if let Some(child) = self.nodes.borrow_mut(child_handle) {
            child.parent = parent_handle;
            if let Some(parent) = self.nodes.borrow_mut(parent_handle) {
                parent.children.push(child_handle);
            }
        }
    }

    /// Unlinks specified node from its parent, so node will become root.
    #[inline]
    pub fn unlink_node(&mut self, node_handle: Handle<UINode>) {
        let mut parent_handle: Handle<UINode> = Handle::none();
        // Replace parent handle of child
        if let Some(node) = self.nodes.borrow_mut(node_handle) {
            parent_handle = node.parent;
            node.parent = Handle::none();
        }
        // Remove child from parent's children list
        if let Some(parent) = self.nodes.borrow_mut(parent_handle) {
            if let Some(i) = parent.children.iter().position(|h| *h == node_handle) {
                parent.children.remove(i);
            }
        }
    }

    #[inline]
    pub fn get_node(&self, node_handle: Handle<UINode>) -> Option<&UINode> {
        self.nodes.borrow(node_handle)
    }

    #[inline]
    pub fn get_node_mut(&mut self, node_handle: Handle<UINode>) -> Option<&mut UINode> {
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

        if let Some(node) = self.nodes.borrow(handle) {
            for child_handle in node.children.iter() {
                self.measure(*child_handle, available_size);

                if let Some(child) = self.nodes.borrow(*child_handle) {
                    let child_desired_size = child.desired_size.get();
                    if child_desired_size.x > size.x {
                        size.x = child_desired_size.x;
                    }
                    if child_desired_size.y > size.y {
                        size.y = child_desired_size.y;
                    }
                }
            }
        }

        size
    }

    fn measure(&self, node_handle: Handle<UINode>, available_size: Vec2) {
        if let Some(node) = self.nodes.borrow(node_handle) {
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
                let mut desired_size = match &node.kind {
                    UINodeKind::Border(border) => border.measure_override(self, size_for_child),
                    UINodeKind::Canvas(canvas) => canvas.measure_override(self, size_for_child),
                    UINodeKind::Grid(grid) => grid.measure_override(self, size_for_child),
                    UINodeKind::ScrollContentPresenter(scp) => scp.measure_override(self, size_for_child),
                    UINodeKind::ScrollBar(scroll_bar) => scroll_bar.measure_override(self, size_for_child),
                    _ => self.default_measure_override(node_handle, size_for_child)
                };

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
    }

    fn default_arrange_override(&self, handle: Handle<UINode>, final_size: Vec2) -> Vec2 {
        let final_rect = Rect::new(0.0, 0.0, final_size.x, final_size.y);

        if let Some(node) = self.nodes.borrow(handle) {
            for child_handle in node.children.iter() {
                self.arrange(*child_handle, &final_rect);
            }
        }

        final_size
    }

    fn arrange(&self, node_handle: Handle<UINode>, final_rect: &Rect<f32>) {
        if let Some(node) = self.nodes.borrow(node_handle) {
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

            size = match &node.kind {
                UINodeKind::Border(border) => border.arrange_override(self, size),
                UINodeKind::Canvas(canvas) => canvas.arrange_override(self, size),
                UINodeKind::Grid(grid) => grid.arrange_override(self, size),
                UINodeKind::ScrollContentPresenter(scp) => scp.arrange_override(self, size),
                UINodeKind::ScrollBar(scroll_bar) => scroll_bar.arrange_override(self, size),
                _ => self.default_arrange_override(node_handle, size)
            };

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
    }

    fn update_transform(&mut self, node_handle: Handle<UINode>) {
        let mut children = UnsafeCollectionView::empty();

        let mut screen_position = Vec2::zero();
        if let Some(node) = self.nodes.borrow(node_handle) {
            children = UnsafeCollectionView::from_slice(&node.children);
            if let Some(parent) = self.nodes.borrow(node.parent) {
                screen_position = node.actual_local_position.get() + parent.screen_position;
            } else {
                screen_position = node.actual_local_position.get();
            }
        }

        if let Some(node) = self.nodes.borrow_mut(node_handle) {
            node.screen_position = screen_position;
        }

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
        let mut children: UnsafeCollectionView<Handle<UINode>> = UnsafeCollectionView::empty();

        if let Some(node) = self.nodes.borrow_mut(node_handle) {
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
        self.draw_node(root_canvas,  1);

        if self.visual_debug {
            self.drawing_context.set_nesting(0);

            let picked_bounds =
                if let Some(picked_node) = self.nodes.borrow(self.picked_node) {
                    Some(picked_node.get_screen_bounds())
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

        if let Some(node) = self.nodes.borrow(node_handle) {
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
        }

        clipped
    }

    fn is_node_contains_point(&self, node_handle: Handle<UINode>, pt: Vec2) -> bool {
        if let Some(node) = self.nodes.borrow(node_handle) {
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
        }

        false
    }

    fn pick_node(&self, node_handle: Handle<UINode>, pt: Vec2, level: &mut i32) -> Handle<UINode> {
        let mut picked = Handle::none();
        let mut topmost_picked_level = 0;

        if self.is_node_contains_point(node_handle, pt) {
            picked = node_handle;
            topmost_picked_level = *level;
        }

        if let Some(node) = self.nodes.borrow(node_handle) {
            for child_handle in node.children.iter() {
                *level += 1;
                let picked_child = self.pick_node(*child_handle, pt, level);
                if !picked_child.is_none() && *level > topmost_picked_level {
                    topmost_picked_level = *level;
                    picked = picked_child;
                }
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

    fn route_event(&mut self, node_handle: Handle<UINode>, event_type: RoutedEventHandlerType, event_args: &mut RoutedEvent) {
        let mut handler = None;
        let mut parent = Handle::none();
        let index = event_type as usize;

        if let Some(node) = self.nodes.borrow_mut(node_handle) {
            // Take event handler.
            handler = node.event_handlers[index].take();
            parent = node.parent;
        }

        // Execute event handler.
        if let Some(ref mut mouse_enter) = handler {
            mouse_enter(self, node_handle, event_args);
        }

        if let Some(node) = self.nodes.borrow_mut(node_handle) {
            // Put event handler back.
            node.event_handlers[index] = handler.take();
        }

        // Route event up on hierarchy (bubbling strategy) until is not handled.
        if !event_args.handled && !parent.is_none() {
            self.route_event(parent, event_type, event_args);
        }
    }

    /// Searches a node down on tree starting from give root that matches a criteria
    /// defined by a given func.
    pub fn find_by_criteria_down<Func>(&self, node_handle: Handle<UINode>, func: &Func) -> Handle<UINode>
        where Func: Fn(&UINode) -> bool {
        if let Some(node) = self.nodes.borrow(node_handle) {
            if func(node) {
                return node_handle;
            }

            for child_handle in node.children.iter() {
                let result = self.find_by_criteria_down(*child_handle, func);

                if result.is_some() {
                    return result;
                }
            }
        }
        Handle::none()
    }

    /// Searches a node up on tree starting from given root that matches a criteria
    /// defined by a given func.
    pub fn find_by_criteria_up<Func>(&self, node_handle: Handle<UINode>, func: Func) -> Handle<UINode>
        where Func: Fn(&UINode) -> bool {
        if let Some(node) = self.nodes.borrow(node_handle) {
            if func(node) {
                return node_handle;
            }

            return self.find_by_criteria_up(node.parent, func);
        }

        Handle::none()
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
    pub fn borrow_by_name_up(&self, start_node_handle: Handle<UINode>, name: &str) -> Option<&UINode> {
        self.nodes.borrow(self.find_by_name_up(start_node_handle, name))
    }

    /// Searches a node by name up on tree starting from given root node and tries to borrow it as mutable if exists.
    pub fn borrow_by_name_up_mut(&mut self, start_node_handle: Handle<UINode>, name: &str) -> Option<&mut UINode> {
        self.nodes.borrow_mut(self.find_by_name_up(start_node_handle, name))
    }

    /// Searches a node by name down on tree starting from given root node and tries to borrow it if exists.
    pub fn borrow_by_name_down(&self, start_node_handle: Handle<UINode>, name: &str) -> Option<&UINode> {
        self.nodes.borrow(self.find_by_name_down(start_node_handle, name))
    }

    /// Searches a node by name down on tree starting from given root node and tries to borrow it as mutable if exists.
    pub fn borrow_by_name_down_mut(&mut self, start_node_handle: Handle<UINode>, name: &str) -> Option<&mut UINode> {
        self.nodes.borrow_mut(self.find_by_name_down(start_node_handle, name))
    }

    pub fn borrow_by_criteria_up<Func>(&self, start_node_handle: Handle<UINode>, func: Func) -> Option<&UINode>
        where Func: Fn(&UINode) -> bool {
        self.nodes.borrow(self.find_by_criteria_up(start_node_handle, func))
    }

    pub fn borrow_by_criteria_up_mut<Func>(&mut self, start_node_handle: Handle<UINode>, func: Func) -> Option<&mut UINode>
        where Func: Fn(&UINode) -> bool {
        self.nodes.borrow_mut(self.find_by_criteria_up(start_node_handle, func))
    }

    pub fn get_node_kind_id(&self, handle: Handle<UINode>) -> TypeId {
        if let Some(node) = self.nodes.borrow(handle) {
            node.get_kind_id()
        } else {
            TypeId::of::<()>()
        }
    }

    pub fn process_event(&mut self, event: &glutin::WindowEvent) -> bool {
        let mut event_processed = false;

        if let WindowEvent::CursorMoved { position, .. } = event {
            self.mouse_position = Vec2::make(position.x as f32, position.y as f32);
            self.picked_node = self.hit_test(self.mouse_position);

            // Fire mouse leave for previously picked node
            if self.picked_node != self.prev_picked_node {
                let mut fire_mouse_leave = false;
                if let Some(prev_picked_node) = self.nodes.borrow_mut(self.prev_picked_node) {
                    if prev_picked_node.is_mouse_over {
                        prev_picked_node.is_mouse_over = false;
                        fire_mouse_leave = true;
                    }
                }

                if fire_mouse_leave {
                    let mut evt = RoutedEvent::new(RoutedEventKind::MouseLeave);
                    self.route_event(self.prev_picked_node, RoutedEventHandlerType::MouseLeave, &mut evt);
                }
            }

            if !self.picked_node.is_none() {
                let mut fire_mouse_enter = false;
                if let Some(picked_node) = self.nodes.borrow_mut(self.picked_node) {
                    if !picked_node.is_mouse_over {
                        picked_node.is_mouse_over = true;
                        fire_mouse_enter = true;
                    }
                }

                if fire_mouse_enter {
                    let mut evt = RoutedEvent::new(RoutedEventKind::MouseEnter);
                    self.route_event(self.picked_node, RoutedEventHandlerType::MouseEnter, &mut evt);
                }

                // Fire mouse move
                let mut evt = RoutedEvent::new(RoutedEventKind::MouseMove {
                    pos: self.mouse_position
                });
                self.route_event(self.picked_node, RoutedEventHandlerType::MouseMove, &mut evt);

                event_processed = true;
            }
        }

        if !self.picked_node.is_none() {
            match event {
                WindowEvent::MouseInput { button, state, .. } => {
                    match state {
                        ElementState::Pressed => {
                            let mut evt = RoutedEvent::new(RoutedEventKind::MouseDown {
                                pos: self.mouse_position,
                                button: *button,
                            });
                            self.route_event(self.picked_node, RoutedEventHandlerType::MouseDown, &mut evt);
                            event_processed = true;
                        }
                        ElementState::Released => {
                            let mut evt = RoutedEvent::new(RoutedEventKind::MouseUp {
                                pos: self.mouse_position,
                                button: *button,
                            });
                            self.route_event(self.picked_node, RoutedEventHandlerType::MouseUp, &mut evt);
                            event_processed = true;
                        }
                    }
                }

                WindowEvent::MouseWheel { delta, .. } => {
                    if let MouseScrollDelta::LineDelta(_, y) = delta {
                        let mut evt = RoutedEvent::new(RoutedEventKind::MouseWheel {
                            pos: self.mouse_position,
                            amount: *y,
                        });
                        self.route_event(self.picked_node, RoutedEventHandlerType::MouseWheel, &mut evt);
                        event_processed = true;
                    }
                }

                _ => ()
            }
        }

        self.prev_picked_node = self.picked_node;

        event_processed
    }
}

