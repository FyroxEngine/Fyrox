//! Extendable UI library.
//!
//! See examples here - https://github.com/mrDIMAS/rusty-shooter/blob/master/src/menu.rs

#![allow(irrefutable_let_patterns)]
#![allow(clippy::float_cmp)]

#[macro_use]
extern crate lazy_static;

pub use rg3d_core as core;

pub mod draw;
pub mod text;
pub mod border;
pub mod image;
pub mod canvas;
pub mod message;
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
pub mod brush;
pub mod node;
pub mod popup;
pub mod combobox;
pub mod items_control;
pub mod decorator;

use std::{
    collections::VecDeque,
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
    },
    canvas::Canvas,
    message::{
        UiMessage,
        UiMessageData,
    },
    style::Style,
    widget::Widget,
    ttf::Font,
    message::{OsEvent, ButtonState},
    brush::Brush,
    draw::CommandTexture,
    message::WidgetMessage,
    node::UINode,
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

/// Trait for all UI controls in engine.
///
/// Control must provide at least references (shared and mutable) to inner widget,
/// which means that any control must be based on widget struct.
/// Any other methods will be auto-implemented.
pub trait Control<M: 'static, C: 'static + Control<M, C>> {
    fn widget(&self) -> &Widget<M, C>;

    fn widget_mut(&mut self) -> &mut Widget<M, C>;

    fn measure_override(&self, ui: &UserInterface<M, C>, available_size: Vec2) -> Vec2 {
        self.widget().measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vec2) -> Vec2 {
        self.widget().arrange_override(ui, final_size)
    }

    fn arrange(&self, ui: &UserInterface<M, C>, final_rect: &Rect<f32>) {
        let widget = self.widget();

        if self.is_arrange_valid(ui) && widget.prev_arrange.get() == *final_rect {
            return;
        }

        if widget.visibility() {
            widget.prev_arrange.set(*final_rect);

            let margin_x = widget.margin().left + widget.margin().right;
            let margin_y = widget.margin().top + widget.margin().bottom;

            let mut origin_x = final_rect.x + widget.margin().left;
            let mut origin_y = final_rect.y + widget.margin().top;

            let mut size = Vec2 {
                x: (final_rect.w - margin_x).max(0.0),
                y: (final_rect.h - margin_y).max(0.0),
            };

            let size_without_margin = size;

            if widget.horizontal_alignment() != HorizontalAlignment::Stretch {
                size.x = size.x.min(widget.desired_size().x - margin_x);
            }
            if widget.vertical_alignment() != VerticalAlignment::Stretch {
                size.y = size.y.min(widget.desired_size().y - margin_y);
            }

            if widget.width() > 0.0 {
                size.x = widget.width();
            }
            if widget.height() > 0.0 {
                size.y = widget.height();
            }

            size = self.arrange_override(ui, size);

            if size.x > final_rect.w {
                size.x = final_rect.w;
            }
            if size.y > final_rect.h {
                size.y = final_rect.h;
            }

            match widget.horizontal_alignment() {
                HorizontalAlignment::Center | HorizontalAlignment::Stretch => {
                    origin_x += (size_without_margin.x - size.x) * 0.5;
                }
                HorizontalAlignment::Right => {
                    origin_x += size_without_margin.x - size.x;
                }
                _ => ()
            }

            match widget.vertical_alignment() {
                VerticalAlignment::Center | VerticalAlignment::Stretch => {
                    origin_y += (size_without_margin.y - size.y) * 0.5;
                }
                VerticalAlignment::Bottom => {
                    origin_y += size_without_margin.y - size.y;
                }
                _ => ()
            }

            widget.commit_arrange(Vec2::new(origin_x, origin_y), size);
        }
    }

    fn is_measure_valid(&self, ui: &UserInterface<M, C>) -> bool {
        let mut valid = self.widget().is_measure_valid() && self.widget().prev_global_visibility == self.widget().is_globally_visible();
        if valid {
            for child in self.widget().children() {
                valid &= ui.node(*child).is_measure_valid(ui);
                if !valid {
                    break;
                }
            }
        }
        valid
    }

    fn is_arrange_valid(&self, ui: &UserInterface<M, C>) -> bool {
        let mut valid = self.widget().is_arrange_valid() && self.widget().prev_global_visibility == self.widget().is_globally_visible();
        if valid {
            for child in self.widget().children() {
                valid &= ui.node(*child).is_arrange_valid(ui);
                if !valid {
                    break;
                }
            }
        }
        valid
    }

    fn measure(&self, ui: &UserInterface<M, C>, available_size: Vec2) {
        let widget = self.widget();

        if self.is_measure_valid(ui) && widget.prev_measure.get() == available_size {
            return;
        }

        if widget.visibility() {
            widget.prev_measure.set(available_size);

            let margin = Vec2 {
                x: widget.margin().left + widget.margin().right,
                y: widget.margin().top + widget.margin().bottom,
            };

            let size_for_child = Vec2 {
                x: {
                    let w = if widget.width() > 0.0 {
                        widget.width()
                    } else {
                        (available_size.x - margin.x).max(0.0)
                    };

                    if w > widget.max_size().x {
                        widget.max_size().x
                    } else if w < widget.min_size().x {
                        widget.min_size().x
                    } else {
                        w
                    }
                },
                y: {
                    let h = if widget.height() > 0.0 {
                        widget.height()
                    } else {
                        (available_size.y - margin.y).max(0.0)
                    };

                    if h > widget.max_size().y {
                        widget.max_size().y
                    } else if h < widget.min_size().y {
                        widget.min_size().y
                    } else {
                        h
                    }
                },
            };

            let mut desired_size = self.measure_override(ui, size_for_child);

            if !widget.width().is_nan() {
                desired_size.x = widget.width();
            }

            if desired_size.x > widget.max_size().x {
                desired_size.x = widget.max_size().x;
            } else if desired_size.x < widget.min_size().x {
                desired_size.x = widget.min_size().x;
            }

            if desired_size.y > widget.max_size().y {
                desired_size.y = widget.max_size().y;
            } else if desired_size.y < widget.min_size().y {
                desired_size.y = widget.min_size().y;
            }

            if !widget.height().is_nan() {
                desired_size.y = widget.height();
            }

            desired_size += margin;

            // Make sure that node won't go outside of available bounds.
            if desired_size.x > available_size.x {
                desired_size.x = available_size.x;
            }
            if desired_size.y > available_size.y {
                desired_size.y = available_size.y;
            }

            widget.commit_measure(desired_size);
        } else {
            widget.commit_measure(Vec2::new(0.0, 0.0));
        }
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
    fn handle_message(&mut self, _self_handle: Handle<UINode<M, C>>, _ui: &mut UserInterface<M, C>, _message: &mut UiMessage<M, C>) {}

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

    /// Called when a node is deleted from container thus giving a chance to remove dangling
    /// handles which may cause panic.
    fn remove_ref(&mut self, _handle: Handle<UINode<M, C>>) {}
}

pub struct UserInterface<M: 'static, C: 'static + Control<M, C>> {
    screen_size: Vec2,
    nodes: Pool<UINode<M, C>>,
    drawing_context: DrawingContext,
    visual_debug: bool,
    /// Every UI node will live on the window-sized canvas.
    root_canvas: Handle<UINode<M, C>>,
    picked_node: Handle<UINode<M, C>>,
    prev_picked_node: Handle<UINode<M, C>>,
    captured_node: Handle<UINode<M, C>>,
    keyboard_focus_node: Handle<UINode<M, C>>,
    cursor_position: Vec2,
    messages: VecDeque<UiMessage<M, C>>,
    stack: Vec<Handle<UINode<M, C>>>,
}

lazy_static! {
    static ref DEFAULT_FONT: Arc<Mutex<Font>> = {
        let font_bytes = std::include_bytes!("./built_in_font.ttf").to_vec();
        let font = Font::from_memory(font_bytes, 20.0, Font::default_char_set()).unwrap();
        Arc::new(Mutex::new(font))
    };
}

impl<M, C: 'static + Control<M, C>> Default for UserInterface<M, C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M, C: 'static + Control<M, C>> UserInterface<M, C> {
    pub fn new() -> UserInterface<M, C> {
        let mut ui = UserInterface {
            screen_size: Vec2::new(1000.0, 1000.0),
            messages: VecDeque::new(),
            visual_debug: false,
            captured_node: Handle::NONE,
            root_canvas: Handle::NONE,
            nodes: Pool::new(),
            cursor_position: Vec2::ZERO,
            drawing_context: DrawingContext::new(),
            picked_node: Handle::NONE,
            prev_picked_node: Handle::NONE,
            keyboard_focus_node: Handle::NONE,
            stack: Default::default(),
        };
        ui.root_canvas = ui.add_node(UINode::Canvas(Canvas::new(Widget::default())));
        ui
    }

    #[inline]
    pub fn capture_mouse(&mut self, node: Handle<UINode<M, C>>) -> bool {
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

    fn update_visibility(&mut self) {
        self.stack.clear();
        self.stack.push(self.root_canvas);
        while let Some(node_handle) = self.stack.pop() {
            let widget = self.nodes.borrow(node_handle).widget();
            for child_handle in widget.children() {
                self.stack.push(*child_handle);
            }
            let parent_visibility =
                if widget.parent().is_some() {
                    self.node(widget.parent())
                        .widget()
                        .is_globally_visible()
                } else {
                    true
                };
            let widget = self.nodes.borrow_mut(node_handle).widget_mut();
            widget.set_global_visibility(widget.visibility() && parent_visibility);
        }
    }

    fn update_transform(&mut self) {
        self.stack.clear();
        self.stack.push(self.root_canvas);
        while let Some(node_handle) = self.stack.pop() {
            let widget = self.nodes.borrow(node_handle).widget();
            for child_handle in widget.children() {
                self.stack.push(*child_handle);
            }
            let screen_position =
                if widget.parent().is_some() {
                    widget.actual_local_position() + self.nodes.borrow(widget.parent()).widget().screen_position
                } else {
                    widget.actual_local_position()
                };
            self.nodes.borrow_mut(node_handle).widget_mut().screen_position = screen_position;
        }
    }

    pub fn screen_size(&self) -> Vec2 {
        self.screen_size
    }

    pub fn update(&mut self, screen_size: Vec2, dt: f32) {
        self.screen_size = screen_size;
        self.update_visibility();

        for n in self.nodes.iter() {
            if !n.widget().is_globally_visible() && n.widget().prev_global_visibility == n.widget().is_globally_visible() {
                n.widget().commit_measure(Vec2::ZERO);
                n.widget().commit_arrange(Vec2::ZERO, Vec2::ZERO);
            }
        }

        self.node(self.root_canvas)
            .measure(self, screen_size);
        self.node(self.root_canvas)
            .arrange(self, &Rect::new(0.0, 0.0, screen_size.x, screen_size.y));
        self.update_transform();

        for node in self.nodes.iter_mut() {
            node.update(dt)
        }
    }

    fn draw_node(&mut self, node_handle: Handle<UINode<M, C>>, nesting: u8) {
        let node = self.nodes.borrow(node_handle);
        let bounds = node.widget().screen_bounds();
        let parent = node.widget().parent();
        if parent.is_some() {
            if !self.nodes.borrow(parent).widget().screen_bounds().intersects(bounds) {
                return;
            }
        }
        if !node.widget().is_globally_visible() {
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

        let children = unsafe { (*(widget as *const Widget<M, C>)).children() };

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
                    Some(self.nodes.borrow(self.picked_node).widget().screen_bounds())
                } else {
                    None
                };

            if let Some(picked_bounds) = picked_bounds {
                self.drawing_context.push_rect(&picked_bounds, 1.0);
                self.drawing_context.commit(CommandKind::Geometry, Brush::Solid(Color::WHITE), CommandTexture::None);
            }
        }

        &self.drawing_context
    }

    fn is_node_clipped(&self, node_handle: Handle<UINode<M, C>>, pt: Vec2) -> bool {
        let mut clipped = true;

        let widget = self.nodes.borrow(node_handle).widget();
        if !widget.is_globally_visible() {
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
        if !widget.parent().is_none() && !clipped {
            clipped |= self.is_node_clipped(widget.parent(), pt);
        }

        clipped
    }

    fn is_node_contains_point(&self, node_handle: Handle<UINode<M, C>>, pt: Vec2) -> bool {
        let widget = self.nodes.borrow(node_handle).widget();

        if !widget.is_globally_visible() {
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

    fn pick_node(&self, node_handle: Handle<UINode<M, C>>, pt: Vec2, level: &mut i32) -> Handle<UINode<M, C>> {
        let widget = self.nodes.borrow(node_handle).widget();

        if !widget.is_hit_test_visible() {
            return Handle::NONE;
        }

        let (mut picked, mut topmost_picked_level) =
            if self.is_node_contains_point(node_handle, pt) {
                (node_handle, *level)
            } else {
                (Handle::NONE, 0)
            };

        for child_handle in widget.children() {
            *level += 1;
            let picked_child = self.pick_node(*child_handle, pt, level);
            if !picked_child.is_none() && *level > topmost_picked_level {
                topmost_picked_level = *level;
                picked = picked_child;
            }
        }

        picked
    }

    pub fn cursor_position(&self) -> Vec2 {
        self.cursor_position
    }

    pub fn hit_test(&self, pt: Vec2) -> Handle<UINode<M, C>> {
        if self.nodes.is_valid_handle(self.captured_node) {
            self.captured_node
        } else {
            let mut level = 0;
            self.pick_node(self.root_canvas, pt, &mut level)
        }
    }

    /// Searches a node down on tree starting from give root that matches a criteria
    /// defined by a given func.
    pub fn find_by_criteria_down<Func>(&self, node_handle: Handle<UINode<M, C>>, func: &Func) -> Handle<UINode<M, C>>
        where Func: Fn(&UINode<M, C>) -> bool {
        let node = self.nodes.borrow(node_handle);

        if func(node) {
            return node_handle;
        }

        for child_handle in node.widget().children() {
            let result = self.find_by_criteria_down(*child_handle, func);

            if result.is_some() {
                return result;
            }
        }

        Handle::NONE
    }

    /// Searches a node up on tree starting from given root that matches a criteria
    /// defined by a given func.
    pub fn find_by_criteria_up<Func>(&self, node_handle: Handle<UINode<M, C>>, func: Func) -> Handle<UINode<M, C>>
        where Func: Fn(&UINode<M, C>) -> bool {
        let node = self.nodes.borrow(node_handle);

        if func(node) {
            return node_handle;
        }

        self.find_by_criteria_up(node.widget().parent(), func)
    }

    /// Checks if specified node is a child of some other node on `root_handle`. This method
    /// is useful to understand if some event came from some node down by tree.
    pub fn is_node_child_of(&self, node_handle: Handle<UINode<M, C>>, root_handle: Handle<UINode<M, C>>) -> bool {
        self.nodes.borrow(root_handle).widget().has_descendant(node_handle, self)
    }

    /// Checks if specified node is a direct child of some other node on `root_handle`.
    pub fn is_node_direct_child_of(&self, node_handle: Handle<UINode<M, C>>, root_handle: Handle<UINode<M, C>>) -> bool {
        for child_handle in self.nodes.borrow(root_handle).widget().children() {
            if *child_handle == node_handle {
                return true;
            }
        }
        false
    }

    /// Searches a node by name up on tree starting from given root node.
    pub fn find_by_name_up(&self, node_handle: Handle<UINode<M, C>>, name: &str) -> Handle<UINode<M, C>> {
        self.find_by_criteria_up(node_handle, |node| node.widget().name() == name)
    }

    /// Searches a node by name down on tree starting from given root node.
    pub fn find_by_name_down(&self, node_handle: Handle<UINode<M, C>>, name: &str) -> Handle<UINode<M, C>> {
        self.find_by_criteria_down(node_handle, &|node| node.widget().name() == name)
    }

    /// Searches a node by name up on tree starting from given root node and tries to borrow it if exists.
    pub fn borrow_by_name_up(&self, start_node_handle: Handle<UINode<M, C>>, name: &str) -> &UINode<M, C> {
        self.nodes.borrow(self.find_by_name_up(start_node_handle, name))
    }

    /// Searches a node by name up on tree starting from given root node and tries to borrow it as mutable if exists.
    pub fn borrow_by_name_up_mut(&mut self, start_node_handle: Handle<UINode<M, C>>, name: &str) -> &mut UINode<M, C> {
        self.nodes.borrow_mut(self.find_by_name_up(start_node_handle, name))
    }

    /// Searches a node by name down on tree starting from given root node and tries to borrow it if exists.
    pub fn borrow_by_name_down(&self, start_node_handle: Handle<UINode<M, C>>, name: &str) -> &UINode<M, C> {
        self.nodes.borrow(self.find_by_name_down(start_node_handle, name))
    }

    /// Searches a node by name down on tree starting from given root node and tries to borrow it as mutable if exists.
    pub fn borrow_by_name_down_mut(&mut self, start_node_handle: Handle<UINode<M, C>>, name: &str) -> &mut UINode<M, C> {
        self.nodes.borrow_mut(self.find_by_name_down(start_node_handle, name))
    }

    /// Searches for a node up on tree that satisfies some criteria and then borrows
    /// shared reference.
    ///
    /// # Panics
    ///
    /// It will panic if there no node that satisfies given criteria.
    pub fn borrow_by_criteria_up<Func>(&self, start_node_handle: Handle<UINode<M, C>>, func: Func) -> &UINode<M, C>
        where Func: Fn(&UINode<M, C>) -> bool {
        self.nodes.borrow(self.find_by_criteria_up(start_node_handle, func))
    }

    /// Searches for a node up on tree that satisfies some criteria and then borrows
    /// mutable reference.
    ///
    /// # Panics
    ///
    /// It will panic if there no node that satisfies given criteria.
    pub fn borrow_by_criteria_up_mut<Func>(&mut self, start_node_handle: Handle<UINode<M, C>>, func: Func) -> &mut UINode<M, C>
        where Func: Fn(&UINode<M, C>) -> bool {
        self.nodes.borrow_mut(self.find_by_criteria_up(start_node_handle, func))
    }

    /// Pushes new UI event to the common queue. Could be useful to send a message
    /// to some specific node in deferred manner.
    pub fn post_message(&mut self, message: UiMessage<M, C>) {
        self.messages.push_back(message);
    }

    /// Puts node at the end of children list of a parent node.
    ///
    /// # Notes
    ///
    /// Node will be topmost *only* on same hierarchy level! So if you have a floating
    /// window (for example) and a window embedded into some other control (yes this is
    /// possible) then floating window won't be the topmost.
    pub fn make_topmost(&mut self, node: Handle<UINode<M, C>>) {
        let parent = self.node(node).widget().parent();
        if parent.is_some() {
            let parent = self.node_mut(parent).widget_mut();
            parent.remove_child(node);
            parent.add_child(node);
        }
    }

    /// Extracts UI event one-by-one from common queue. Each extracted event will go to *all*
    /// available nodes first and only then will be moved outside of this method. This is one
    /// of most important methods which must be called each frame of your game loop, otherwise
    /// UI will not respond to any kind of events and simply speaking will just not work.
    pub fn poll_ui_event(&mut self) -> Option<UiMessage<M, C>> {
        // Gather events from nodes.
        for (handle, node) in self.nodes.pair_iter_mut() {
            while let Some(mut outgoing_message) = node.widget_mut().outgoing_messages.borrow_mut().pop_front() {
                outgoing_message.source = handle;
                self.messages.push_back(outgoing_message)
            }
        }

        let mut event = self.messages.pop_front();

        if let Some(ref mut message) = event {
            for i in 0..self.nodes.get_capacity() {
                let handle = self.nodes.handle_from_index(i);

                if self.nodes.is_valid_handle(handle) {
                    let (ticket, mut node) = self.nodes.take_reserve(handle);

                    node.handle_message(handle, self, message);

                    self.nodes.put_back(ticket, node);
                }
            }

            if let UiMessageData::Widget(msg) = &message.data {
                match msg {
                    // Keep order of children of a parent node of a node that changed z-index
                    // the same as z-index of children.
                    WidgetMessage::ZIndex(_) => {
                        let parent = self.node(message.source).widget().parent();
                        if parent.is_some() {
                            self.stack.clear();
                            for child in self.nodes.borrow(parent).widget().children() {
                                self.stack.push(*child);
                            }

                            let nodes = &mut self.nodes;
                            self.stack.sort_by(|a, b| {
                                let z_a = nodes.borrow(*a).widget().z_index();
                                let z_b = nodes.borrow(*b).widget().z_index();
                                z_a.cmp(&z_b)
                            });

                            let parent = self.nodes.borrow_mut(parent).widget_mut();
                            parent.clear_children();
                            for child in self.stack.iter() {
                                parent.add_child(*child);
                            }
                        }
                    }
                    WidgetMessage::TopMost => {
                        if message.target.is_some() {
                            self.make_topmost(message.target);
                        } else if message.source.is_some() {
                            self.make_topmost(message.source);
                        }
                    }
                    _ => {}
                }
            }
        }

        event
    }

    pub fn captured_node(&self) -> Handle<UINode<M, C>> {
        self.captured_node
    }

    pub fn flush_messages(&mut self) {
        while let Some(_) = self.poll_ui_event() {}
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
                        self.picked_node = self.hit_test(self.cursor_position);

                        self.keyboard_focus_node = self.picked_node;

                        if !self.picked_node.is_none() {
                            self.messages.push_back(UiMessage {
                                handled: false,
                                data: UiMessageData::Widget(WidgetMessage::MouseDown {
                                    pos: self.cursor_position,
                                    button: *button,
                                }),
                                target: Handle::NONE,
                                source: self.picked_node,
                            });
                            event_processed = true;
                        }
                    }
                    ButtonState::Released => {
                        if !self.picked_node.is_none() {
                            self.messages.push_back(UiMessage {
                                handled: false,
                                data: UiMessageData::Widget(WidgetMessage::MouseUp {
                                    pos: self.cursor_position,
                                    button: *button,
                                }),
                                target: Handle::NONE,
                                source: self.picked_node,
                            });
                            event_processed = true;
                        }
                    }
                }
            }
            OsEvent::CursorMoved { position } => {
                self.cursor_position = *position;
                self.picked_node = self.hit_test(self.cursor_position);

                // Fire mouse leave for previously picked node
                if self.picked_node != self.prev_picked_node && self.prev_picked_node.is_some() {
                    let prev_picked_node = self.nodes.borrow_mut(self.prev_picked_node).widget_mut();
                    if prev_picked_node.is_mouse_directly_over {
                        prev_picked_node.is_mouse_directly_over = false;
                        self.messages.push_back(UiMessage {
                            handled: false,
                            data: UiMessageData::Widget(WidgetMessage::MouseLeave),
                            target: Handle::NONE,
                            source: self.prev_picked_node,
                        });
                    }
                }

                if !self.picked_node.is_none() {
                    let picked_node = self.nodes.borrow_mut(self.picked_node).widget_mut();
                    if !picked_node.is_mouse_directly_over {
                        picked_node.is_mouse_directly_over = true;
                        self.messages.push_back(UiMessage {
                            handled: false,
                            data: UiMessageData::Widget(WidgetMessage::MouseEnter),
                            target: Handle::NONE,
                            source: self.picked_node,
                        });
                    }

                    // Fire mouse move
                    self.messages.push_back(UiMessage {
                        handled: false,
                        data: UiMessageData::Widget(WidgetMessage::MouseMove(self.cursor_position)),
                        target: Handle::NONE,
                        source: self.picked_node,
                    });

                    event_processed = true;
                }
            }
            OsEvent::MouseWheel(_, y) => {
                if !self.picked_node.is_none() {
                    self.messages.push_back(UiMessage {
                        handled: false,
                        data: UiMessageData::Widget(WidgetMessage::MouseWheel {
                            pos: self.cursor_position,
                            amount: *y,
                        }),
                        target: Handle::NONE,
                        source: self.picked_node,
                    });

                    event_processed = true;
                }
            }
            OsEvent::KeyboardInput { button, state } => {
                if self.keyboard_focus_node.is_some() {
                    let event = UiMessage {
                        handled: false,
                        data: match state {
                            ButtonState::Pressed => {
                                UiMessageData::Widget(WidgetMessage::KeyDown(*button))
                            }
                            ButtonState::Released => {
                                UiMessageData::Widget(WidgetMessage::KeyUp(*button))
                            }
                        },
                        target: Handle::NONE,
                        source: self.keyboard_focus_node,
                    };

                    self.messages.push_back(event);

                    event_processed = true;
                }
            }
            OsEvent::Character(unicode) => {
                if self.keyboard_focus_node.is_some() {
                    let event = UiMessage {
                        handled: false,
                        data: UiMessageData::Widget(WidgetMessage::Text(*unicode)),
                        target: Handle::NONE,
                        source: self.keyboard_focus_node,
                    };

                    self.messages.push_back(event);

                    event_processed = true;
                }
            }
        }

        self.prev_picked_node = self.picked_node;

        event_processed
    }

    pub fn nodes(&self) -> &Pool<UINode<M, C>> {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut Pool<UINode<M, C>> {
        &mut self.nodes
    }

    pub fn root(&self) -> Handle<UINode<M, C>> {
        self.root_canvas
    }

    pub fn add_node(&mut self, mut node: UINode<M, C>) -> Handle<UINode<M, C>> {
        let children = node.widget().children().to_vec();
        node.widget_mut().clear_children();
        let node_handle = self.nodes_mut().spawn(node);
        if self.root_canvas.is_some() {
            self.link_nodes(node_handle, self.root_canvas);
        }
        for child in children {
            self.link_nodes(child, node_handle)
        }
        node_handle
    }

    pub fn remove_node(&mut self, node: Handle<UINode<M, C>>) {
        self.unlink_node(node);

        let mut removed_nodes = Vec::new();
        let mut stack = vec![node];
        while let Some(handle) = stack.pop() {
            removed_nodes.push(handle);

            if self.prev_picked_node == handle {
                self.prev_picked_node = Handle::NONE;
            }
            if self.picked_node == handle {
                self.picked_node = Handle::NONE;
            }
            if self.captured_node == handle {
                self.captured_node = Handle::NONE;
            }
            if self.keyboard_focus_node == handle {
                self.keyboard_focus_node = Handle::NONE;
            }

            for child in self.nodes().borrow(handle).widget().children().iter() {
                stack.push(*child);
            }
            self.nodes_mut().free(handle);
        }

        for node in self.nodes_mut().iter_mut() {
            for removed_node in removed_nodes.iter() {
                node.remove_ref(*removed_node);
            }
        }
    }

    /// Links specified child with specified parent.
    #[inline]
    pub fn link_nodes(&mut self, child_handle: Handle<UINode<M, C>>, parent_handle: Handle<UINode<M, C>>) {
        assert_ne!(child_handle, parent_handle);
        self.unlink_node(child_handle);
        let child = self.nodes_mut()
            .borrow_mut(child_handle)
            .widget_mut();
        child.set_parent(parent_handle);
        let parent = self.nodes_mut()
            .borrow_mut(parent_handle)
            .widget_mut();
        parent.add_child(child_handle);
    }

    /// Unlinks specified node from its parent, so node will become root.
    #[inline]
    pub fn unlink_node(&mut self, node_handle: Handle<UINode<M, C>>) {
        let parent_handle;
        // Replace parent handle of child
        let node = self.nodes_mut().borrow_mut(node_handle);
        parent_handle = node.widget().parent();
        node.widget_mut().set_parent(Handle::NONE);

        // Remove child from parent's children list
        if parent_handle.is_some() {
            self.node_mut(parent_handle)
                .widget_mut()
                .remove_child(node_handle);
        }
    }

    #[inline]
    pub fn node(&self, node_handle: Handle<UINode<M, C>>) -> &UINode<M, C> {
        self.nodes()
            .borrow(node_handle)
    }

    #[inline]
    pub fn node_mut(&mut self, node_handle: Handle<UINode<M, C>>) -> &mut UINode<M, C> {
        self.nodes_mut()
            .borrow_mut(node_handle)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        widget::{WidgetBuilder, Widget},
        grid::{GridBuilder, Row, Column},
        window::{WindowBuilder, WindowTitle},
        Thickness,
        Builder,
        UserInterface,
        Control,
        button::ButtonBuilder,
        node::UINode,
        core::math::vec2::Vec2,
    };

    pub struct StubUiMessage {}

    pub struct StubUiNode {}

    impl Control<StubUiMessage, StubUiNode> for StubUiNode {
        fn widget(&self) -> &Widget<StubUiMessage, StubUiNode> {
            unimplemented!()
        }

        fn widget_mut(&mut self) -> &mut Widget<StubUiMessage, StubUiNode> {
            unimplemented!()
        }
    }

    #[test]
    fn perf_test() {
        let mut ui = UserInterface::<StubUiMessage, StubUiNode>::new();

        GridBuilder::new(WidgetBuilder::new()
            .with_width(1000.0)
            .with_height(1000.0)
            .with_child(WindowBuilder::new(WidgetBuilder::new()
                .on_row(1)
                .on_column(1))
                .can_minimize(false)
                .can_close(false)
                .with_title(WindowTitle::Text("Test"))
                .with_content(GridBuilder::new(WidgetBuilder::new()
                    .with_margin(Thickness::uniform(20.0))
                    .with_child({
                        ButtonBuilder::new(WidgetBuilder::new()
                            .on_column(0)
                            .on_row(0)
                            .with_margin(Thickness::uniform(4.0)))
                            .with_text("New Game")
                            .build(&mut ui)
                    })
                    .with_child({
                        ButtonBuilder::new(WidgetBuilder::new()
                            .on_column(0)
                            .on_row(1)
                            .with_margin(Thickness::uniform(4.0)))
                            .with_text("Save Game")
                            .build(&mut ui)
                    })
                    .with_child({
                        ButtonBuilder::new(WidgetBuilder::new()
                            .on_column(0)
                            .on_row(2)
                            .with_margin(Thickness::uniform(4.0)))
                            .with_text("Load Game")
                            .build(&mut ui)
                    })
                    .with_child({
                        ButtonBuilder::new(WidgetBuilder::new()
                            .on_column(0)
                            .on_row(3)
                            .with_margin(Thickness::uniform(4.0)))
                            .with_text("Settings")
                            .build(&mut ui)
                    })
                    .with_child({
                        ButtonBuilder::new(WidgetBuilder::new()
                            .on_column(0)
                            .on_row(4)
                            .with_margin(Thickness::uniform(4.0)))
                            .with_text("Quit")
                            .build(&mut ui)
                    }))
                    .add_column(Column::stretch())
                    .add_row(Row::strict(75.0))
                    .add_row(Row::strict(75.0))
                    .add_row(Row::strict(75.0))
                    .add_row(Row::strict(75.0))
                    .add_row(Row::strict(75.0))
                    .build(&mut ui))
                .build(&mut ui)))
            .add_row(Row::stretch())
            .add_row(Row::strict(500.0))
            .add_row(Row::stretch())
            .add_column(Column::stretch())
            .add_column(Column::strict(400.0))
            .add_column(Column::stretch())
            .build(&mut ui);

        ui.update(Vec2::new(1000.0, 1000.0), 0.016);
    }
}