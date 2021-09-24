//! Extendable, retained mode, graphics API agnostic UI library.
//!
//! See examples here - <https://github.com/mrDIMAS/rusty-shooter/blob/master/src/menu.rs>

#![forbid(unsafe_code)]
#![allow(irrefutable_let_patterns)]
#![allow(clippy::float_cmp)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::from_over_into)]

#[macro_use]
extern crate lazy_static;

pub use rg3d_core as core;

pub mod border;
pub mod brush;
pub mod button;
pub mod canvas;
pub mod check_box;
pub mod color;
pub mod curve;
pub mod decorator;
pub mod dock;
pub mod draw;
pub mod dropdown_list;
pub mod expander;
pub mod file_browser;
pub mod formatted_text;
pub mod grid;
pub mod image;
pub mod inspector;
pub mod list_view;
pub mod menu;
pub mod message;
pub mod messagebox;
pub mod numeric;
pub mod popup;
pub mod progress_bar;
pub mod scroll_bar;
pub mod scroll_panel;
pub mod scroll_viewer;
pub mod stack_panel;
pub mod tab_control;
pub mod text;
pub mod text_box;
pub mod tree;
pub mod ttf;
pub mod utils;
pub mod vec;
pub mod vector_image;
pub mod widget;
pub mod window;
pub mod wrap_panel;

use crate::{
    brush::Brush,
    canvas::Canvas,
    core::{
        algebra::Vector2,
        color::Color,
        math::clampf,
        math::Rect,
        pool::{Handle, Pool},
        scope_profile, VecExtensions,
    },
    draw::{CommandTexture, Draw, DrawingContext},
    message::{
        ButtonState, CursorIcon, KeyboardModifiers, MessageDirection, MouseButton, OsEvent,
        PopupMessage, UiMessage, UiMessageData, WidgetMessage,
    },
    popup::Placement,
    ttf::{Font, SharedFont},
    widget::{Widget, WidgetBuilder},
};

use copypasta::ClipboardContext;
use std::any::Any;
use std::{
    cell::Cell,
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    ops::{Deref, DerefMut, Index, IndexMut},
    sync::{
        mpsc::{self, Receiver, Sender, TryRecvError},
        Arc, Mutex,
    },
};

// TODO: Make this part of UserInterface struct.
pub const COLOR_DARKEST: Color = Color::opaque(20, 20, 20);
pub const COLOR_DARKER: Color = Color::opaque(30, 30, 30);
pub const COLOR_DARK: Color = Color::opaque(40, 40, 40);
pub const COLOR_PRIMARY: Color = Color::opaque(50, 50, 50);
pub const COLOR_LIGHT: Color = Color::opaque(65, 65, 65);
pub const COLOR_LIGHTER: Color = Color::opaque(80, 80, 80);
pub const COLOR_LIGHTEST: Color = Color::opaque(95, 95, 95);
pub const COLOR_BRIGHT: Color = Color::opaque(130, 130, 130);
pub const COLOR_BRIGHT_BLUE: Color = Color::opaque(80, 118, 178);
pub const COLOR_TEXT: Color = Color::opaque(220, 220, 220);
pub const COLOR_FOREGROUND: Color = Color::WHITE;

pub const BRUSH_DARKEST: Brush = Brush::Solid(COLOR_DARKEST);
pub const BRUSH_DARKER: Brush = Brush::Solid(COLOR_DARKER);
pub const BRUSH_DARK: Brush = Brush::Solid(COLOR_DARK);
pub const BRUSH_PRIMARY: Brush = Brush::Solid(COLOR_PRIMARY);
pub const BRUSH_LIGHT: Brush = Brush::Solid(COLOR_LIGHT);
pub const BRUSH_LIGHTER: Brush = Brush::Solid(COLOR_LIGHTER);
pub const BRUSH_LIGHTEST: Brush = Brush::Solid(COLOR_LIGHTEST);
pub const BRUSH_BRIGHT: Brush = Brush::Solid(COLOR_BRIGHT);
pub const BRUSH_BRIGHT_BLUE: Brush = Brush::Solid(COLOR_BRIGHT_BLUE);
pub const BRUSH_TEXT: Brush = Brush::Solid(COLOR_TEXT);
pub const BRUSH_FOREGROUND: Brush = Brush::Solid(COLOR_FOREGROUND);

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

impl Default for Thickness {
    fn default() -> Self {
        Self::uniform(0.0)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

impl Thickness {
    pub fn zero() -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        }
    }

    pub fn uniform(v: f32) -> Self {
        Self {
            left: v,
            top: v,
            right: v,
            bottom: v,
        }
    }

    pub fn bottom(v: f32) -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: v,
        }
    }

    pub fn top(v: f32) -> Self {
        Self {
            left: 0.0,
            top: v,
            right: 0.0,
            bottom: 0.0,
        }
    }

    pub fn left(v: f32) -> Self {
        Self {
            left: v,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        }
    }

    pub fn right(v: f32) -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            right: v,
            bottom: 0.0,
        }
    }

    pub fn offset(&self) -> Vector2<f32> {
        Vector2::new(self.left, self.top)
    }

    /// Returns margin for each axis.
    pub fn axes_margin(&self) -> Vector2<f32> {
        Vector2::new(self.left + self.right, self.top + self.bottom)
    }
}

type NodeHandle = Handle<UiNode>;

pub struct NodeHandleMapping {
    hash_map: HashMap<NodeHandle, NodeHandle>,
}

impl Default for NodeHandleMapping {
    fn default() -> Self {
        Self {
            hash_map: Default::default(),
        }
    }
}

impl NodeHandleMapping {
    pub fn add_mapping(&mut self, old: Handle<UiNode>, new: Handle<UiNode>) {
        self.hash_map.insert(old, new);
    }

    pub fn resolve(&self, old: &mut Handle<UiNode>) {
        // None handles aren't mapped.
        if old.is_some() {
            *old = *self.hash_map.get(old).unwrap()
        }
    }

    pub fn resolve_cell(&self, old: &mut Cell<Handle<UiNode>>) {
        // None handles aren't mapped.
        if old.get().is_some() {
            old.set(*self.hash_map.get(&old.get()).unwrap())
        }
    }

    pub fn resolve_slice(&self, slice: &mut [Handle<UiNode>]) {
        for item in slice {
            self.resolve(item);
        }
    }
}

/// Trait for all UI controls in library.
pub trait Control: 'static + Deref<Target = Widget> + DerefMut {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn clone_boxed(&self) -> Box<dyn Control>;

    fn resolve(&mut self, _node_map: &NodeHandleMapping) {}

    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        scope_profile!();

        self.deref().measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        scope_profile!();

        self.deref().arrange_override(ui, final_size)
    }

    fn draw(&self, _drawing_context: &mut DrawingContext) {}

    fn update(&mut self, _dt: f32) {}

    /// Performs event-specific actions. Must call widget.handle_message()!
    ///
    /// # Notes
    ///
    /// Do *not* try to borrow node by `self_handle` in UI - at this moment node has been moved
    /// out of pool and attempt of borrowing will cause panic! `self_handle` should be used only
    /// to check if event came from/for this node or to capture input on node.
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage);

    /// Used to react to a message (by producing another message) that was posted outside of current
    /// hierarchy. In other words this method is used when you need to "peek" a message before it'll
    /// be passed into bubbling router. Most common use case is to catch messages from popups: popup
    /// in 99.9% cases is a child of root canvas and it **won't** receive a message from a its *logical*
    /// parent during bubbling message routing. For example `preview_message` used in a dropdown list:
    /// dropdown list has two separate parts - a field with selected value and a popup for all possible
    /// options. Visual parent of the popup in this case is the root canvas, but logical parent is the
    /// dropdown list. Because of this fact, the field won't receive any messages from popup, to solve
    /// this we use `preview_message`. This method is much more restrictive - it does not allow you to
    /// modify a node and ui, you can either *request* changes by sending a message or use internal
    /// mutability (`Cell`, `RefCell`, etc).
    ///
    /// ## Important notes
    ///
    /// Due to performance reasons, you **must** set `.with_preview_messages(true)` in widget builder to
    /// force library to call `preview_message`!
    ///
    /// The order of execution of this method is undefined! There is no guarantee that it will be called
    /// hierarchically as widgets connected.
    fn preview_message(&self, _ui: &UserInterface, _message: &mut UiMessage) {
        // This method is optional.
    }

    /// Provides a way to respond to OS specific events. Can be useful to detect if a key or mouse
    /// button was pressed. This method significantly differs from `handle_message` because os events
    /// are not dispatched - they'll be passed to this method in any case.
    ///
    /// ## Important notes
    ///
    /// Due to performance reasons, you **must** set `.with_handle_os_messages(true)` in widget builder to
    /// force library to call `handle_os_event`!
    fn handle_os_event(
        &mut self,
        _self_handle: Handle<UiNode>,
        _ui: &mut UserInterface,
        _event: &OsEvent,
    ) {
    }

    /// Called when a node is deleted from container thus giving a chance to remove dangling
    /// handles which may cause panic.
    fn remove_ref(&mut self, _handle: Handle<UiNode>) {}
}

pub struct DragContext {
    is_dragging: bool,
    drag_node: Handle<UiNode>,
    click_pos: Vector2<f32>,
}

impl Default for DragContext {
    fn default() -> Self {
        Self {
            is_dragging: false,
            drag_node: Default::default(),
            click_pos: Vector2::new(0.0, 0.0),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MouseState {
    left: ButtonState,
    right: ButtonState,
    middle: ButtonState,
    // TODO Add rest of buttons
}

impl Default for MouseState {
    fn default() -> Self {
        Self {
            left: ButtonState::Released,
            right: ButtonState::Released,
            middle: ButtonState::Released,
        }
    }
}

pub struct UiNode(pub Box<dyn Control>);

impl Deref for UiNode {
    type Target = dyn Control;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl DerefMut for UiNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

impl UiNode {
    pub fn new<T: Control>(widget: T) -> Self {
        Self(Box::new(widget))
    }

    pub fn cast<T: Control>(&self) -> Option<&T> {
        self.0.as_any().downcast_ref::<T>()
    }

    pub fn cast_mut<T: Control>(&mut self) -> Option<&mut T> {
        self.0.as_any_mut().downcast_mut::<T>()
    }
}

pub struct BuildContext<'a> {
    ui: &'a mut UserInterface,
}

impl<'a> BuildContext<'a> {
    pub fn add_node(&mut self, node: UiNode) -> Handle<UiNode> {
        self.ui.add_node(node)
    }

    pub fn link(&mut self, child: Handle<UiNode>, parent: Handle<UiNode>) {
        self.ui.link_nodes_internal(child, parent, false)
    }

    pub fn copy(&mut self, node: Handle<UiNode>) -> Handle<UiNode> {
        self.ui.copy_node(node)
    }
}

impl<'a> Index<Handle<UiNode>> for BuildContext<'a> {
    type Output = UiNode;

    fn index(&self, index: Handle<UiNode>) -> &Self::Output {
        &self.ui.nodes[index]
    }
}

impl<'a> IndexMut<Handle<UiNode>> for BuildContext<'a> {
    fn index_mut(&mut self, index: Handle<UiNode>) -> &mut Self::Output {
        &mut self.ui.nodes[index]
    }
}

#[derive(Copy, Clone)]
pub struct RestrictionEntry {
    /// Handle to UI node to which picking must be restricted to.
    pub handle: Handle<UiNode>,

    /// A flag that tells UI to stop iterating over picking stack.
    /// There are two use cases: chain of menus (popups) and set of modal windows. In case of
    /// menus you need to restrict picking to an entire chain, but leave possibility to select
    /// any menu in the chain. In case of multiple modal windows you need to restrict picking
    /// individually per window, not allowing to pick anything behind modal window, but still
    /// save restrictions in the entire chain of modal windows so if topmost closes, restriction
    /// will be on previous one and so on.
    pub stop: bool,
}

struct TooltipEntry {
    tooltip: Handle<UiNode>,
    /// Time remaining until this entry should disappear (in seconds).
    time: f32,
    /// Maximum time that it should be kept for
    /// This is stored here as well, because when hovering
    /// over the tooltip, we don't know the time it should stay for and
    /// so we use this to refresh the timer.
    max_time: f32,
}
impl TooltipEntry {
    fn new(tooltip: Handle<UiNode>, time: f32) -> TooltipEntry {
        Self {
            tooltip,
            time,
            max_time: time,
        }
    }

    fn decrease(&mut self, amount: f32) {
        self.time -= amount;
    }

    fn should_display(&self) -> bool {
        self.time > 0.0
    }
}

pub struct UserInterface {
    screen_size: Vector2<f32>,
    nodes: Pool<UiNode>,
    drawing_context: DrawingContext,
    visual_debug: bool,
    root_canvas: Handle<UiNode>,
    picked_node: Handle<UiNode>,
    prev_picked_node: Handle<UiNode>,
    captured_node: Handle<UiNode>,
    keyboard_focus_node: Handle<UiNode>,
    cursor_position: Vector2<f32>,
    receiver: Receiver<UiMessage>,
    sender: Sender<UiMessage>,
    stack: Vec<Handle<UiNode>>,
    picking_stack: Vec<RestrictionEntry>,
    bubble_queue: VecDeque<Handle<UiNode>>,
    drag_context: DragContext,
    mouse_state: MouseState,
    keyboard_modifiers: KeyboardModifiers,
    cursor_icon: CursorIcon,
    visible_tooltips: Vec<TooltipEntry>,
    preview_set: HashSet<Handle<UiNode>>,
    clipboard: Option<ClipboardContext>,
}

lazy_static! {
    pub static ref DEFAULT_FONT: SharedFont = {
        let font_bytes = std::include_bytes!("./built_in_font.ttf").to_vec();
        let font = Font::from_memory(font_bytes, 16.0, Font::default_char_set()).unwrap();
        Arc::new(Mutex::new(font)).into()
    };
}

fn draw_node(
    nodes: &Pool<UiNode>,
    node_handle: Handle<UiNode>,
    drawing_context: &mut DrawingContext,
) {
    scope_profile!();

    let node = &nodes[node_handle];
    if !node.is_globally_visible() {
        return;
    }

    // Crawl up on tree and check if current bounds are intersects with every screen bound
    // of parents chain. This is needed because some control can move their children outside of
    // their bounds (like scroll viewer, etc.) and single intersection test of parent bounds with
    // current bounds is not enough.
    let bounds = node.screen_bounds();
    let mut parent = node.parent();
    while parent.is_some() {
        let parent_node = nodes.borrow(parent);
        if !parent_node.screen_bounds().intersects(bounds) {
            return;
        }
        parent = parent_node.parent();
    }

    let start_index = drawing_context.get_commands().len();

    drawing_context.push_opacity(if is_node_enabled(nodes, node_handle) {
        node.opacity()
    } else {
        0.4
    });

    node.draw(drawing_context);

    let end_index = drawing_context.get_commands().len();
    for i in start_index..end_index {
        node.command_indices.borrow_mut().push(i);
    }

    // Continue on children
    for &child_node in node.children().iter() {
        // Do not continue render of top-most nodes - they'll be rendered in separate pass.
        if !nodes[child_node].is_draw_on_top() {
            draw_node(nodes, child_node, drawing_context);
        }
    }

    drawing_context.pop_opacity();
}

fn is_node_enabled(nodes: &Pool<UiNode>, handle: Handle<UiNode>) -> bool {
    let root_node = &nodes[handle];
    let mut enabled = root_node.enabled();
    let mut parent = root_node.parent();
    while parent.is_some() {
        let node = &nodes[parent];
        if !node.enabled() {
            enabled = false;
            break;
        }
        parent = node.parent();
    }
    enabled
}

impl UserInterface {
    pub fn new(screen_size: Vector2<f32>) -> UserInterface {
        let (sender, receiver) = mpsc::channel();
        let mut ui = UserInterface {
            screen_size,
            sender,
            receiver,
            visual_debug: false,
            captured_node: Handle::NONE,
            root_canvas: Handle::NONE,
            nodes: Pool::new(),
            cursor_position: Vector2::new(0.0, 0.0),
            drawing_context: DrawingContext::new(),
            picked_node: Handle::NONE,
            prev_picked_node: Handle::NONE,
            keyboard_focus_node: Handle::NONE,
            stack: Default::default(),
            picking_stack: Default::default(),
            bubble_queue: Default::default(),
            drag_context: Default::default(),
            mouse_state: Default::default(),
            keyboard_modifiers: Default::default(),
            cursor_icon: Default::default(),
            visible_tooltips: Default::default(),
            preview_set: Default::default(),
            clipboard: ClipboardContext::new().ok(),
        };
        ui.root_canvas = ui.add_node(UiNode::new(Canvas::new(WidgetBuilder::new().build())));
        ui
    }

    pub fn keyboard_modifiers(&self) -> KeyboardModifiers {
        self.keyboard_modifiers
    }

    pub fn build_ctx(&mut self) -> BuildContext<'_> {
        BuildContext { ui: self }
    }

    #[inline]
    pub fn capture_mouse(&mut self, node: Handle<UiNode>) -> bool {
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

    pub fn is_node_enabled(&self, handle: Handle<UiNode>) -> bool {
        is_node_enabled(&self.nodes, handle)
    }

    fn update_visibility(&mut self) {
        scope_profile!();

        self.stack.clear();
        self.stack.push(self.root_canvas);
        while let Some(node_handle) = self.stack.pop() {
            let (widget, parent) = self
                .nodes
                .try_borrow_dependant_mut(node_handle, |n| n.parent());

            let widget = widget.unwrap();

            self.stack.extend_from_slice(widget.children());

            let visibility = if let Some(parent) = parent {
                widget.visibility() && parent.is_globally_visible()
            } else {
                widget.visibility()
            };

            widget.set_global_visibility(visibility);
        }
    }

    fn update_transform(&mut self) {
        scope_profile!();

        self.stack.clear();
        self.stack.push(self.root_canvas);
        while let Some(node_handle) = self.stack.pop() {
            let (widget, parent) = self
                .nodes
                .try_borrow_dependant_mut(node_handle, |n| n.parent());

            let widget = widget.unwrap();

            if widget.is_globally_visible() {
                self.stack.extend_from_slice(widget.children());

                let screen_position = if let Some(parent) = parent {
                    widget.actual_local_position() + parent.screen_position()
                } else {
                    widget.actual_local_position()
                };

                widget.screen_position = screen_position;
            }
        }
    }

    pub fn screen_size(&self) -> Vector2<f32> {
        self.screen_size
    }

    pub fn update(&mut self, screen_size: Vector2<f32>, dt: f32) {
        scope_profile!();

        self.screen_size = screen_size;
        self.update_visibility();

        for n in self.nodes.iter() {
            if !n.is_globally_visible() && n.prev_global_visibility == n.is_globally_visible() {
                n.commit_measure(Vector2::default());
                n.commit_arrange(Vector2::new(0.0, 0.0), Vector2::default());
            }
        }

        self.measure_node(self.root_canvas, screen_size);
        self.arrange_node(
            self.root_canvas,
            &Rect::new(0.0, 0.0, screen_size.x, screen_size.y),
        );
        self.update_transform();

        for node in self.nodes.iter_mut() {
            node.update(dt)
        }

        self.update_tooltips(dt);

        if !self.drag_context.is_dragging {
            // Try to fetch new cursor icon starting from current picked node. Traverse
            // tree up until cursor with different value is found.
            self.cursor_icon = CursorIcon::default();
            let mut handle = self.picked_node;
            while handle.is_some() {
                let node = &self.nodes[handle];
                if let Some(cursor) = node.cursor() {
                    self.cursor_icon = cursor;
                    break;
                }
                handle = node.parent();
            }
        }
    }

    pub fn cursor(&self) -> CursorIcon {
        self.cursor_icon
    }

    pub fn draw(&mut self) -> &DrawingContext {
        scope_profile!();

        self.calculate_clip_bounds(
            self.root_canvas,
            Rect::new(0.0, 0.0, self.screen_size.x, self.screen_size.y),
        );
        self.drawing_context.clear();

        for node in self.nodes.iter_mut() {
            node.command_indices.borrow_mut().clear();
        }

        // Draw everything except top-most nodes.
        draw_node(&self.nodes, self.root_canvas, &mut self.drawing_context);

        // Render top-most nodes in separate pass.
        // TODO: This may give weird results because of invalid nesting.
        self.stack.clear();
        self.stack.push(self.root());
        while let Some(node_handle) = self.stack.pop() {
            let node = &self.nodes[node_handle];
            if node.is_draw_on_top() {
                draw_node(&self.nodes, node_handle, &mut self.drawing_context);
            }
            for &child in node.children() {
                self.stack.push(child);
            }
        }

        // Debug info rendered on top of other.
        if self.visual_debug {
            if self.picked_node.is_some() {
                let bounds = self.nodes.borrow(self.picked_node).screen_bounds();
                self.drawing_context.push_rect(&bounds, 1.0);
                self.drawing_context.commit(
                    bounds,
                    Brush::Solid(Color::WHITE),
                    CommandTexture::None,
                    None,
                );
            }

            if self.keyboard_focus_node.is_some() {
                let bounds = self.nodes.borrow(self.keyboard_focus_node).screen_bounds();
                self.drawing_context.push_rect(&bounds, 1.0);
                self.drawing_context.commit(
                    bounds,
                    Brush::Solid(Color::GREEN),
                    CommandTexture::None,
                    None,
                );
            }
        }

        &self.drawing_context
    }

    pub fn clipboard(&self) -> Option<&ClipboardContext> {
        self.clipboard.as_ref()
    }

    pub fn clipboard_mut(&mut self) -> Option<&mut ClipboardContext> {
        self.clipboard.as_mut()
    }

    pub fn arrange_node(&self, handle: Handle<UiNode>, final_rect: &Rect<f32>) -> bool {
        scope_profile!();

        let node = self.node(handle);

        if node.is_arrange_valid_with_descendants(self) && node.prev_arrange.get() == *final_rect {
            return false;
        }

        if node.visibility() {
            node.prev_arrange.set(*final_rect);

            let margin = node.margin().axes_margin();

            let mut size = Vector2::new(
                (final_rect.w() - margin.x).max(0.0),
                (final_rect.h() - margin.y).max(0.0),
            );

            let available_size = size;

            if node.horizontal_alignment() != HorizontalAlignment::Stretch {
                size.x = size.x.min(node.desired_size().x - margin.x);
            }
            if node.vertical_alignment() != VerticalAlignment::Stretch {
                size.y = size.y.min(node.desired_size().y - margin.y);
            }

            if node.width() > 0.0 {
                size.x = node.width();
            }
            if node.height() > 0.0 {
                size.y = node.height();
            }

            size = node.arrange_override(self, size);

            size.x = size.x.min(final_rect.w());
            size.y = size.y.min(final_rect.h());

            let mut origin = final_rect.position + node.margin().offset();

            match node.horizontal_alignment() {
                HorizontalAlignment::Center | HorizontalAlignment::Stretch => {
                    origin.x += (available_size.x - size.x) * 0.5;
                }
                HorizontalAlignment::Right => origin.x += available_size.x - size.x,
                _ => (),
            }

            match node.vertical_alignment() {
                VerticalAlignment::Center | VerticalAlignment::Stretch => {
                    origin.y += (available_size.y - size.y) * 0.5;
                }
                VerticalAlignment::Bottom => origin.y += available_size.y - size.y,
                _ => (),
            }

            node.commit_arrange(origin, size);
        }

        true
    }

    pub fn measure_node(&self, handle: Handle<UiNode>, available_size: Vector2<f32>) -> bool {
        scope_profile!();

        let node = self.node(handle);

        if node.is_measure_valid_with_descendants(self) && node.prev_measure.get() == available_size
        {
            return false;
        }

        if node.visibility() {
            node.prev_measure.set(available_size);

            let axes_margin = node.margin().axes_margin();
            let mut inner_size = available_size - axes_margin;
            inner_size.x = inner_size.x.max(0.0);
            inner_size.y = inner_size.y.max(0.0);

            let mut size = Vector2::new(
                if node.width() > 0.0 {
                    node.width()
                } else {
                    inner_size.x
                },
                if node.height() > 0.0 {
                    node.height()
                } else {
                    inner_size.y
                },
            );

            size.x = clampf(size.x, node.min_size().x, node.max_size().x);
            size.y = clampf(size.y, node.min_size().y, node.max_size().y);

            let mut desired_size = node.measure_override(self, size);

            if !node.width().is_nan() {
                desired_size.x = node.width();
            }
            if !node.height().is_nan() {
                desired_size.y = node.height();
            }

            desired_size.x = clampf(desired_size.x, node.min_size().x, node.max_size().x);
            desired_size.y = clampf(desired_size.y, node.min_size().y, node.max_size().y);

            desired_size += axes_margin;

            desired_size.x = desired_size.x.min(available_size.x);
            desired_size.y = desired_size.y.min(available_size.y);

            node.commit_measure(desired_size);
        } else {
            node.commit_measure(Vector2::new(0.0, 0.0));
        }

        true
    }

    fn is_node_clipped(&self, node_handle: Handle<UiNode>, pt: Vector2<f32>) -> bool {
        scope_profile!();

        let mut clipped = true;

        let widget = self.nodes.borrow(node_handle);

        if widget.is_globally_visible() {
            clipped = !widget.screen_bounds().contains(pt);

            if !clipped {
                for command_index in widget.command_indices.borrow().iter() {
                    if let Some(command) = self.drawing_context.get_commands().get(*command_index) {
                        if let Some(geometry) = command.clipping_geometry.as_ref() {
                            if geometry.is_contains_point(pt) {
                                clipped = false;
                                break;
                            }
                        }
                    }
                }
            }

            // Point can be clipped by parent's clipping geometry.
            if !widget.parent().is_none() && !clipped {
                clipped |= self.is_node_clipped(widget.parent(), pt);
            }
        }

        clipped
    }

    fn is_node_contains_point(&self, node_handle: Handle<UiNode>, pt: Vector2<f32>) -> bool {
        scope_profile!();

        let widget = self.nodes.borrow(node_handle);

        if !widget.is_globally_visible() {
            return false;
        }

        if !self.is_node_clipped(node_handle, pt) {
            for command_index in widget.command_indices.borrow().iter() {
                if let Some(command) = self.drawing_context.get_commands().get(*command_index) {
                    if self.drawing_context.is_command_contains_point(command, pt) {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn pick_node(
        &self,
        node_handle: Handle<UiNode>,
        pt: Vector2<f32>,
        level: &mut i32,
    ) -> Handle<UiNode> {
        scope_profile!();

        let widget = self.nodes.borrow(node_handle);

        if !widget.is_hit_test_visible()
            || !widget.enabled()
            || !widget.screen_bounds().intersects(Rect {
                position: Default::default(),
                size: self.screen_size,
            })
        {
            return Handle::NONE;
        }

        let (mut picked, mut topmost_picked_level) = if self.is_node_contains_point(node_handle, pt)
        {
            (node_handle, *level)
        } else {
            (Handle::NONE, 0)
        };

        for child_handle in widget.children() {
            *level += 1;
            let picked_child = self.pick_node(*child_handle, pt, level);
            if picked_child.is_some() && *level > topmost_picked_level {
                topmost_picked_level = *level;
                picked = picked_child;
            }
        }

        picked
    }

    pub fn cursor_position(&self) -> Vector2<f32> {
        self.cursor_position
    }

    pub fn hit_test(&self, pt: Vector2<f32>) -> Handle<UiNode> {
        scope_profile!();

        if self.nodes.is_valid_handle(self.captured_node) {
            self.captured_node
        } else if self.picking_stack.is_empty() {
            // We're not restricted to any node, just start from root.
            let mut level = 0;
            self.pick_node(self.root_canvas, pt, &mut level)
        } else {
            // We have some picking restriction chain.
            // Go over picking stack and try each entry. This will help with picking
            // in a series of popups, especially in menus where may be many open popups
            // at the same time.
            for root in self.picking_stack.iter().rev() {
                if self.nodes.is_valid_handle(root.handle) {
                    let mut level = 0;
                    let picked = self.pick_node(root.handle, pt, &mut level);
                    if picked.is_some() {
                        return picked;
                    }
                }
                if root.stop {
                    break;
                }
            }
            Handle::NONE
        }
    }

    /// Searches a node down on tree starting from give root that matches a criteria
    /// defined by a given func.
    pub fn find_by_criteria_down<Func>(
        &self,
        node_handle: Handle<UiNode>,
        func: &Func,
    ) -> Handle<UiNode>
    where
        Func: Fn(&UiNode) -> bool,
    {
        let node = self.nodes.borrow(node_handle);

        if func(node) {
            return node_handle;
        }

        for child_handle in node.children() {
            let result = self.find_by_criteria_down(*child_handle, func);

            if result.is_some() {
                return result;
            }
        }

        Handle::NONE
    }

    /// Searches a node up on tree starting from given root that matches a criteria
    /// defined by a given func.
    pub fn find_by_criteria_up<Func>(
        &self,
        node_handle: Handle<UiNode>,
        func: Func,
    ) -> Handle<UiNode>
    where
        Func: Fn(&UiNode) -> bool,
    {
        let node = self.nodes.borrow(node_handle);

        if func(node) {
            return node_handle;
        }

        if node.parent().is_some() {
            self.find_by_criteria_up(node.parent(), func)
        } else {
            Handle::NONE
        }
    }

    /// Checks if specified node is a child of some other node on `root_handle`. This method
    /// is useful to understand if some event came from some node down by tree.
    pub fn is_node_child_of(
        &self,
        node_handle: Handle<UiNode>,
        root_handle: Handle<UiNode>,
    ) -> bool {
        self.nodes
            .borrow(root_handle)
            .has_descendant(node_handle, self)
    }

    /// Recursively calculates clipping bounds for every node.
    fn calculate_clip_bounds(&self, node: Handle<UiNode>, parent_bounds: Rect<f32>) {
        let node = &self.nodes[node];
        node.clip_bounds
            .set(node.screen_bounds().clip_by(parent_bounds));
        for &child in node.children() {
            self.calculate_clip_bounds(child, node.clip_bounds.get());
        }
    }

    /// Checks if specified node is a direct child of some other node on `root_handle`.
    pub fn is_node_direct_child_of(
        &self,
        node_handle: Handle<UiNode>,
        root_handle: Handle<UiNode>,
    ) -> bool {
        for child_handle in self.nodes.borrow(root_handle).children() {
            if *child_handle == node_handle {
                return true;
            }
        }
        false
    }

    /// Searches a node by name up on tree starting from given root node.
    pub fn find_by_name_up(&self, node_handle: Handle<UiNode>, name: &str) -> Handle<UiNode> {
        self.find_by_criteria_up(node_handle, |node| node.name() == name)
    }

    /// Searches a node by name down on tree starting from given root node.
    pub fn find_by_name_down(&self, node_handle: Handle<UiNode>, name: &str) -> Handle<UiNode> {
        self.find_by_criteria_down(node_handle, &|node| node.name() == name)
    }

    /// Searches a node by name up on tree starting from given root node and tries to borrow it if exists.
    pub fn borrow_by_name_up(&self, start_node_handle: Handle<UiNode>, name: &str) -> &UiNode {
        self.nodes
            .borrow(self.find_by_name_up(start_node_handle, name))
    }

    /// Searches a node by name down on tree starting from given root node and tries to borrow it if exists.
    pub fn borrow_by_name_down(&self, start_node_handle: Handle<UiNode>, name: &str) -> &UiNode {
        self.nodes
            .borrow(self.find_by_name_down(start_node_handle, name))
    }

    /// Searches for a node up on tree that satisfies some criteria and then borrows
    /// shared reference.
    ///
    /// # Panics
    ///
    /// It will panic if there no node that satisfies given criteria.
    pub fn borrow_by_criteria_up<Func>(
        &self,
        start_node_handle: Handle<UiNode>,
        func: Func,
    ) -> &UiNode
    where
        Func: Fn(&UiNode) -> bool,
    {
        self.nodes
            .borrow(self.find_by_criteria_up(start_node_handle, func))
    }

    pub fn try_borrow_by_criteria_up<Func>(
        &self,
        start_node_handle: Handle<UiNode>,
        func: Func,
    ) -> Option<&UiNode>
    where
        Func: Fn(&UiNode) -> bool,
    {
        self.nodes
            .try_borrow(self.find_by_criteria_up(start_node_handle, func))
    }

    pub fn try_borrow_by_type_up<T>(
        &self,
        node_handle: Handle<UiNode>,
    ) -> Option<(Handle<UiNode>, &T)>
    where
        T: Control,
    {
        let node = self.nodes.borrow(node_handle);

        let casted = node.cast::<T>();
        if let Some(casted) = casted {
            return Some((node_handle, casted));
        }

        if node.parent().is_some() {
            self.try_borrow_by_type_up(node.parent())
        } else {
            None
        }
    }

    /// Returns instance of message sender which can be used to push messages into queue
    /// from other threads.
    pub fn sender(&self) -> Sender<UiMessage> {
        self.sender.clone()
    }

    pub fn send_message(&self, message: UiMessage) {
        self.sender.send(message).unwrap()
    }

    // Puts node at the end of children list of a parent node.
    //
    // # Notes
    //
    // Node will be topmost *only* on same hierarchy level! So if you have a floating
    // window (for example) and a window embedded into some other control (yes this is
    // possible) then floating window won't be the topmost.
    fn make_topmost(&mut self, node: Handle<UiNode>) {
        let parent = self.node(node).parent();
        if parent.is_some() {
            let parent = &mut self.nodes[parent];
            parent.remove_child(node);
            parent.add_child(node, false);
        }
    }

    fn bubble_message(&mut self, message: &mut UiMessage) {
        scope_profile!();

        // Dispatch event using bubble strategy. Bubble routing means that message will go
        // from specified destination up on tree to tree root.
        // Gather chain of nodes from source to root.
        self.bubble_queue.clear();
        self.bubble_queue.push_back(message.destination());
        let mut parent = self.nodes[message.destination()].parent();
        while parent.is_some() && self.nodes.is_valid_handle(parent) {
            self.bubble_queue.push_back(parent);
            parent = self.nodes[parent].parent();
        }

        while let Some(handle) = self.bubble_queue.pop_front() {
            let (ticket, mut node) = self.nodes.take_reserve(handle);
            node.handle_routed_message(self, message);
            self.nodes.put_back(ticket, node);
        }
    }

    /// Extracts UI event one-by-one from common queue. Each extracted event will go to *all*
    /// available nodes first and only then will be moved outside of this method. This is one
    /// of most important methods which must be called each frame of your game loop, otherwise
    /// UI will not respond to any kind of events and simply speaking will just not work.
    pub fn poll_message(&mut self) -> Option<UiMessage> {
        match self.receiver.try_recv() {
            Ok(mut message) => {
                // Destination node may be destroyed at the time we receive message,
                // we have to discard such messages.
                if !self.nodes.is_valid_handle(message.destination()) {
                    return None;
                }

                if message.need_perform_layout() {
                    self.update(self.screen_size, 0.0);
                }

                for &handle in self.preview_set.iter() {
                    if self.nodes.is_valid_handle(handle) {
                        self.nodes[handle].preview_message(self, &mut message);
                    }
                }

                self.bubble_message(&mut message);

                if let UiMessageData::Widget(msg) = &message.data() {
                    match msg {
                        WidgetMessage::ZIndex(_) => {
                            // Keep order of children of a parent node of a node that changed z-index
                            // the same as z-index of children.
                            let parent = self.node(message.destination()).parent();
                            if parent.is_some() {
                                self.stack.clear();
                                for child in self.nodes.borrow(parent).children() {
                                    self.stack.push(*child);
                                }

                                let nodes = &mut self.nodes;
                                self.stack.sort_by(|a, b| {
                                    let z_a = nodes.borrow(*a).z_index();
                                    let z_b = nodes.borrow(*b).z_index();
                                    z_a.cmp(&z_b)
                                });

                                let parent = self.nodes.borrow_mut(parent);
                                parent.clear_children();
                                for child in self.stack.iter() {
                                    parent.add_child(*child, false);
                                }
                            }
                        }
                        WidgetMessage::TopMost => {
                            if message.destination().is_some() {
                                self.make_topmost(message.destination());
                            }
                        }
                        WidgetMessage::Unlink => {
                            if message.destination().is_some() {
                                self.unlink_node(message.destination());

                                let node = &self.nodes[message.destination()];
                                let new_position = node.screen_position;
                                self.send_message(WidgetMessage::desired_position(
                                    message.destination(),
                                    MessageDirection::ToWidget,
                                    new_position,
                                ));
                            }
                        }
                        &WidgetMessage::LinkWith(parent) => {
                            if message.destination().is_some() {
                                self.link_nodes_internal(message.destination(), parent, false);
                            }
                        }
                        &WidgetMessage::LinkWithReverse(parent) => {
                            if message.destination().is_some() {
                                self.link_nodes_internal(message.destination(), parent, true);
                            }
                        }
                        WidgetMessage::Remove => {
                            if message.destination().is_some() {
                                self.remove_node(message.destination());
                            }
                        }
                        WidgetMessage::Center => {
                            if message.destination().is_some() {
                                let node = self.node(message.destination());
                                let size = node.actual_size();
                                let parent = node.parent();
                                let parent_size = if parent.is_some() {
                                    if parent == self.root_canvas {
                                        self.screen_size
                                    } else {
                                        self.node(parent).actual_size()
                                    }
                                } else {
                                    self.screen_size
                                };

                                self.send_message(WidgetMessage::desired_position(
                                    message.destination(),
                                    MessageDirection::ToWidget,
                                    (parent_size - size).scale(0.5),
                                ));
                            }
                        }
                        WidgetMessage::MouseDown { button, .. } => {
                            if *button == MouseButton::Right {
                                if let Some(picked) = self.nodes.try_borrow(self.picked_node) {
                                    // Get the context menu from the current node or aa parent node
                                    let (context_menu, target) = if picked.context_menu().is_some()
                                    {
                                        (picked.context_menu(), self.picked_node)
                                    } else {
                                        let parent_handle = picked.find_by_criteria_up(self, |n| {
                                            n.context_menu().is_some()
                                        });

                                        if let Some(parent) = self.nodes.try_borrow(parent_handle) {
                                            (parent.context_menu(), parent_handle)
                                        } else {
                                            (Handle::NONE, Handle::NONE)
                                        }
                                    };

                                    // Display context menu
                                    if context_menu.is_some() {
                                        self.send_message(PopupMessage::placement(
                                            context_menu,
                                            MessageDirection::ToWidget,
                                            Placement::Cursor(target),
                                        ));
                                        self.send_message(PopupMessage::open(
                                            context_menu,
                                            MessageDirection::ToWidget,
                                        ));
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }

                Some(message)
            }
            Err(e) => match e {
                TryRecvError::Empty => None,
                TryRecvError::Disconnected => unreachable!(),
            },
        }
    }

    /// Adds a tooltip if it doesn't already exist.
    /// If it already exists then it refreshes the timer to `time`
    /// Returns the instance that was added or already existed
    fn add_tooltip(&mut self, tooltip: Handle<UiNode>, time: f32) -> &mut TooltipEntry {
        let index = if let Some(index) = self
            .visible_tooltips
            .iter()
            .position(|e| e.tooltip == tooltip)
        {
            self.visible_tooltips[index].time = time;
            index
        } else {
            self.send_message(WidgetMessage::visibility(
                tooltip,
                MessageDirection::ToWidget,
                true,
            ));
            self.send_message(WidgetMessage::topmost(tooltip, MessageDirection::ToWidget));
            self.send_message(WidgetMessage::desired_position(
                tooltip,
                MessageDirection::ToWidget,
                self.cursor_position(),
            ));
            self.visible_tooltips.push(TooltipEntry::new(tooltip, time));

            // Index of the entry we just added
            self.visible_tooltips.len() - 1
        };

        &mut self.visible_tooltips[index]
    }

    /// Find any tooltips that are being hovered and activate them.
    /// As well, update their time.
    fn update_tooltips(&mut self, dt: f32) {
        // Decrease all tooltip times, removing those which have had their timer run out
        let sender = &self.sender;
        self.visible_tooltips
            .retain_mut(|entry: &mut TooltipEntry| {
                entry.decrease(dt);
                if entry.should_display() {
                    true
                } else {
                    // This uses sender directly since we're currently mutably borrowing
                    // visible_tooltips
                    sender
                        .send(WidgetMessage::visibility(
                            entry.tooltip,
                            MessageDirection::ToWidget,
                            false,
                        ))
                        .unwrap();
                    false
                }
            });

        // Check for hovering over a widget with a tooltip, or hovering over a tooltip.
        let mut handle = self.picked_node;
        while let Some(node) = self.nodes.try_borrow(handle) {
            // Get the parent to avoid the problem with having a immutable access here and a
            // mutable access later
            let parent = node.parent();

            if node.tooltip().is_some() {
                // They have a tooltip, we stop here and use that.
                let tooltip = node.tooltip();
                let tooltip_time = node.tooltip_time();
                let entry = self.add_tooltip(tooltip, tooltip_time);
                // Update max time, just in case it was changed.
                // We can't do this in the case where we're hovering over the tooltip, though.
                entry.max_time = tooltip_time;
                break;
            } else if let Some(entry) = self
                .visible_tooltips
                .iter_mut()
                .find(|e| e.tooltip == handle)
            {
                // The current node was a tooltip.
                // We refresh the timer back to the stored max time.
                entry.time = entry.max_time;
                break;
            }

            handle = parent;
        }
    }

    pub fn captured_node(&self) -> Handle<UiNode> {
        self.captured_node
    }

    /// Translates raw window event into some specific UI message. This is one of the
    /// most important methods of UI. You must call it each time you received a message
    /// from a window.
    pub fn process_os_event(&mut self, event: &OsEvent) -> bool {
        let mut event_processed = false;

        match event {
            &OsEvent::MouseInput { button, state, .. } => {
                match button {
                    MouseButton::Left => self.mouse_state.left = state,
                    MouseButton::Right => self.mouse_state.right = state,
                    MouseButton::Middle => self.mouse_state.middle = state,
                    _ => {}
                }

                match state {
                    ButtonState::Pressed => {
                        self.picked_node = self.hit_test(self.cursor_position);

                        // Try to find draggable node in hierarchy starting from picked node.
                        if self.picked_node.is_some() {
                            self.stack.clear();
                            self.stack.push(self.picked_node);
                            while let Some(handle) = self.stack.pop() {
                                let node = &self.nodes[handle];
                                if node.is_drag_allowed() {
                                    self.drag_context.drag_node = handle;
                                    self.stack.clear();
                                    break;
                                } else if node.parent().is_some() {
                                    self.stack.push(node.parent());
                                }
                            }
                            self.drag_context.click_pos = self.cursor_position;
                        }

                        if self.keyboard_focus_node != self.picked_node {
                            if self.keyboard_focus_node.is_some() {
                                self.send_message(WidgetMessage::lost_focus(
                                    self.keyboard_focus_node,
                                    MessageDirection::FromWidget,
                                ));
                            }

                            self.keyboard_focus_node = self.picked_node;

                            if self.keyboard_focus_node.is_some() {
                                self.send_message(WidgetMessage::got_focus(
                                    self.keyboard_focus_node,
                                    MessageDirection::FromWidget,
                                ));
                            }
                        }

                        if self.picked_node.is_some() {
                            self.send_message(WidgetMessage::mouse_down(
                                self.picked_node,
                                MessageDirection::FromWidget,
                                self.cursor_position,
                                button,
                            ));
                            event_processed = true;
                        }
                    }
                    ButtonState::Released => {
                        if self.picked_node.is_some() {
                            if self.drag_context.is_dragging {
                                self.drag_context.is_dragging = false;
                                self.cursor_icon = CursorIcon::Default;

                                // Try to find node with drop allowed in hierarchy starting from picked node.
                                self.stack.clear();
                                self.stack.push(self.picked_node);
                                while let Some(handle) = self.stack.pop() {
                                    let node = &self.nodes[handle];
                                    if node.is_drop_allowed() {
                                        self.send_message(WidgetMessage::drop(
                                            handle,
                                            MessageDirection::FromWidget,
                                            self.drag_context.drag_node,
                                        ));
                                        self.stack.clear();
                                        break;
                                    } else if node.parent().is_some() {
                                        self.stack.push(node.parent());
                                    }
                                }
                            }
                            self.drag_context.drag_node = Handle::NONE;

                            self.send_message(WidgetMessage::mouse_up(
                                self.picked_node,
                                MessageDirection::FromWidget,
                                self.cursor_position,
                                button,
                            ));
                            event_processed = true;
                        }
                    }
                }
            }
            OsEvent::CursorMoved { position } => {
                self.cursor_position = *position;
                self.picked_node = self.hit_test(self.cursor_position);

                if !self.drag_context.is_dragging
                    && self.mouse_state.left == ButtonState::Pressed
                    && self.picked_node.is_some()
                    && self.drag_context.drag_node.is_some()
                    && (self.drag_context.click_pos - *position).norm() > 5.0
                {
                    self.drag_context.is_dragging = true;

                    self.send_message(WidgetMessage::drag_started(
                        self.picked_node,
                        MessageDirection::FromWidget,
                        self.drag_context.drag_node,
                    ));

                    self.cursor_icon = CursorIcon::Crosshair;
                }

                // Fire mouse leave for previously picked node
                if self.picked_node != self.prev_picked_node && self.prev_picked_node.is_some() {
                    let prev_picked_node = self.nodes.borrow_mut(self.prev_picked_node);
                    if prev_picked_node.is_mouse_directly_over {
                        prev_picked_node.is_mouse_directly_over = false;
                        self.send_message(WidgetMessage::mouse_leave(
                            self.prev_picked_node,
                            MessageDirection::FromWidget,
                        ));
                    }
                }

                if self.picked_node.is_some() {
                    let picked_node = self.nodes.borrow_mut(self.picked_node);
                    if !picked_node.is_mouse_directly_over {
                        picked_node.is_mouse_directly_over = true;
                        self.send_message(WidgetMessage::mouse_enter(
                            self.picked_node,
                            MessageDirection::FromWidget,
                        ));
                    }

                    // Fire mouse move
                    self.send_message(WidgetMessage::mouse_move(
                        self.picked_node,
                        MessageDirection::FromWidget,
                        self.cursor_position,
                        self.mouse_state,
                    ));

                    if self.drag_context.is_dragging {
                        self.send_message(WidgetMessage::drag_over(
                            self.picked_node,
                            MessageDirection::FromWidget,
                            self.drag_context.drag_node,
                        ));
                    }

                    event_processed = true;
                }
            }
            OsEvent::MouseWheel(_, y) => {
                if self.picked_node.is_some() {
                    self.send_message(WidgetMessage::mouse_wheel(
                        self.picked_node,
                        MessageDirection::FromWidget,
                        self.cursor_position,
                        *y,
                    ));

                    event_processed = true;
                }
            }
            OsEvent::KeyboardInput { button, state } => {
                if self.keyboard_focus_node.is_some() {
                    self.send_message(match state {
                        ButtonState::Pressed => WidgetMessage::key_down(
                            self.keyboard_focus_node,
                            MessageDirection::FromWidget,
                            *button,
                        ),
                        ButtonState::Released => WidgetMessage::key_up(
                            self.keyboard_focus_node,
                            MessageDirection::FromWidget,
                            *button,
                        ),
                    });

                    event_processed = true;
                }
            }
            OsEvent::Character(unicode) => {
                if self.keyboard_focus_node.is_some() {
                    self.send_message(WidgetMessage::text(
                        self.keyboard_focus_node,
                        MessageDirection::FromWidget,
                        *unicode,
                    ));

                    event_processed = true;
                }
            }
            &OsEvent::KeyboardModifiers(modifiers) => {
                // TODO: Is message needed for focused node?
                self.keyboard_modifiers = modifiers;
            }
        }

        self.prev_picked_node = self.picked_node;

        for i in 0..self.nodes.get_capacity() {
            let handle = self.nodes.handle_from_index(i);

            if let Some(node_ref) = self.nodes.try_borrow(handle) {
                if node_ref.handle_os_events {
                    let (ticket, mut node) = self.nodes.take_reserve(handle);

                    node.handle_os_event(handle, self, event);

                    self.nodes.put_back(ticket, node);
                }
            }
        }

        event_processed
    }

    pub fn nodes(&self) -> &Pool<UiNode> {
        &self.nodes
    }

    pub fn root(&self) -> Handle<UiNode> {
        self.root_canvas
    }

    pub fn add_node(&mut self, mut node: UiNode) -> Handle<UiNode> {
        let children = node.children().to_vec();
        node.clear_children();
        let node_handle = self.nodes.spawn(node);
        if self.root_canvas.is_some() {
            self.link_nodes_internal(node_handle, self.root_canvas, false);
        }
        for child in children {
            self.link_nodes_internal(child, node_handle, false)
        }
        let node = self.nodes[node_handle].deref_mut();
        if node.preview_messages {
            self.preview_set.insert(node_handle);
        }
        node.handle = node_handle;
        node_handle
    }

    pub fn push_picking_restriction(&mut self, restriction: RestrictionEntry) {
        if let Some(top) = self.top_picking_restriction() {
            assert_ne!(top.handle, restriction.handle);
        }
        self.picking_stack.push(restriction);
    }

    pub fn remove_picking_restriction(&mut self, node: Handle<UiNode>) {
        if let Some(pos) = self.picking_stack.iter().position(|h| h.handle == node) {
            self.picking_stack.remove(pos);
        }
    }

    pub fn picking_restriction_stack(&self) -> &[RestrictionEntry] {
        &self.picking_stack
    }

    /// Removes all picking restrictions.
    pub fn drop_picking_restrictions(&mut self) {
        self.picking_stack.clear();
    }

    pub fn top_picking_restriction(&self) -> Option<RestrictionEntry> {
        self.picking_stack.last().cloned()
    }

    /// Use WidgetMessage::remove(...) to remove node.
    fn remove_node(&mut self, node: Handle<UiNode>) {
        self.unlink_node_internal(node);

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
            self.remove_picking_restriction(handle);

            for child in self.nodes().borrow(handle).children().iter() {
                stack.push(*child);
            }
            self.nodes.free(handle);
        }

        self.preview_set.remove(&node);

        for node in self.nodes.iter_mut() {
            for removed_node in removed_nodes.iter() {
                node.remove_ref(*removed_node);
            }
        }
    }

    /// Links specified child with specified parent.
    #[inline]
    fn link_nodes_internal(
        &mut self,
        child_handle: Handle<UiNode>,
        parent_handle: Handle<UiNode>,
        in_front: bool,
    ) {
        assert_ne!(child_handle, parent_handle);
        self.unlink_node_internal(child_handle);
        self.nodes[child_handle].set_parent(parent_handle);
        self.nodes[parent_handle].add_child(child_handle, in_front);
    }

    /// Unlinks specified node from its parent, so node will become root.
    #[inline]
    fn unlink_node_internal(&mut self, node_handle: Handle<UiNode>) {
        // Replace parent handle of child
        let node = self.nodes.borrow_mut(node_handle);
        let parent_handle = node.parent();
        if parent_handle.is_some() {
            node.set_parent(Handle::NONE);

            // Remove child from parent's children list
            self.nodes[parent_handle].remove_child(node_handle);
        }
    }

    /// Unlinks specified node from its parent and attaches back to root canvas.
    ///
    /// Use [WidgetMessage::remove](enum.WidgetMessage.html#method.remove) to unlink
    /// a node at runtime!
    #[inline]
    fn unlink_node(&mut self, node_handle: Handle<UiNode>) {
        self.unlink_node_internal(node_handle);
        self.link_nodes_internal(node_handle, self.root_canvas, false);
    }

    #[inline]
    pub fn node(&self, node_handle: Handle<UiNode>) -> &UiNode {
        self.nodes.borrow(node_handle)
    }

    #[inline]
    pub fn try_get_node(&self, node_handle: Handle<UiNode>) -> Option<&UiNode> {
        self.nodes.try_borrow(node_handle)
    }

    pub fn copy_node(&mut self, node: Handle<UiNode>) -> Handle<UiNode> {
        let mut map = NodeHandleMapping::default();

        let root = self.copy_node_recursive(node, &mut map);

        for &node_handle in map.hash_map.values() {
            self.nodes[node_handle].resolve(&map);
        }

        root
    }

    fn copy_node_recursive(
        &mut self,
        node_handle: Handle<UiNode>,
        map: &mut NodeHandleMapping,
    ) -> Handle<UiNode> {
        let node = self.nodes.borrow(node_handle);
        let mut cloned = UiNode(node.clone_boxed());

        let mut cloned_children = Vec::new();
        for child in node.children().to_vec() {
            cloned_children.push(self.copy_node_recursive(child, map));
        }

        cloned.set_children(cloned_children);
        let copy_handle = self.add_node(cloned);
        map.add_mapping(node_handle, copy_handle);
        copy_handle
    }
}

#[cfg(test)]
mod test {
    use crate::{
        border::BorderBuilder,
        core::algebra::Vector2,
        message::{MessageDirection, WidgetMessage},
        widget::WidgetBuilder,
        UserInterface,
    };

    #[test]
    fn center() {
        let screen_size = Vector2::new(1000.0, 1000.0);
        let widget_size = Vector2::new(100.0, 100.0);
        let mut ui = UserInterface::new(screen_size);
        let widget = BorderBuilder::new(
            WidgetBuilder::new()
                .with_width(widget_size.x)
                .with_height(widget_size.y),
        )
        .build(&mut ui.build_ctx());
        ui.update(screen_size, 0.0); // Make sure layout was calculated.
        ui.send_message(WidgetMessage::center(widget, MessageDirection::ToWidget));
        while let Some(_) = ui.poll_message() {}
        ui.update(screen_size, 0.0);
        let expected_position = (screen_size - widget_size).scale(0.5);
        let actual_position = ui.node(widget).actual_local_position();
        assert_eq!(actual_position, expected_position);
    }
}
