//! Extendable, retained mode, graphics API agnostic UI library with lots (35+) of built-in widgets, HiDPI support,
//! rich layout system and many more.
//!
//! ## Basic Concepts
//!
//! FyroxUI is quite complex UI library and before using it, you should understand basic concepts of it. Especially,
//! if you're got used to immediate-mode UIs.
//!
//! ### Stateful
//!
//! **Stateful UI* means that we can create and destroy widgets when we need to, it is the opposite approach of
//! **immediate-mode** or **stateless UIs** when you don't have long-lasting state for your widgets
//! (usually stateless UI hold its state only for one or few frames).
//!
//! Stateful UI is much more powerful and flexible, it allows you to have complex layout system without having to
//! create hacks to create complex layout as you'd do in immediate-mode UIs. It is also much faster in terms of
//! performance. Stateful UI is a must for complex user interfaces that requires rich layout and high performance.
//!
//! ### Node-based architecture
//!
//! Every user interface could be represented as a set of small blocks that have hierarchical bonding between each
//! other. For example a button could be represented using two parts: a background and a foreground. Usually the background
//! is just a simple rectangle (either a vector or bitmap), and a foreground is a text. The text (the foreground widget)
//! is a child object of the rectangle (the background widget). These two widgets forms another, more complex widget that
//! we call button.
//!
//! Such approach allows us to modify the look of the button as we wish, we can create a button with image background,
//! or with any vector image, or even other widgets. The foreground can be anything too, it can also contain its own
//! complex hierarchy, like a pair of an icon with a text and so on.
//!
//! ### Composition
//!
//! Every widget in the engine uses composition to build more complex widgets. All widgets (and respective builders) contains
//! `Widget` instance inside, it provides basic functionality the widget such as layout information, hierarchy, default
//! foreground and background brushes (their usage depends on derived widget), render and layout transform and so on.
//!
//! ### Message passing
//!
//! The engine uses message passing mechanism for UI logic. What does that mean? Let's see at the button from the
//! previous section and imagine we want to change its text. To do that we need to explicitly "tell" the button's text
//! widget to change its content to something new. This is done by sending a message to the widget.
//!
//! There is no "classic" callbacks to handle various types of messages, which may come from widgets. Instead, you should write
//! your own message dispatcher where you'll handle all messages. Why so? At first - decoupling, in this case business logic
//! is decoupled from the UI. You just receive messages one-by-one and do specific logic. The next reason is that any
//! callback would require context capturing which could be somewhat restrictive - since you need to share context with the
//! UI, it would force you to wrap it in `Rc<RefCell<..>>`/`Arc<Mutex<..>>`.
//!
//! ### Message routing strategies
//!
//! Message passing mechanism works in pair with various routing strategies that allows you to define how the message
//! will "travel" across the tree of nodes.
//!
//! 1. Bubble - a message starts its way from a widget and goes up on hierarchy until it reaches the root node of the hierarchy.
//! Nodes that lies outside that path won't receive the message. This is the most important message routing strategy, that
//! is used for **every** node by default.
//! 2. Direct - a message passed directly to every node that are capable to handle it. There is actual routing in this
//! case. Direct routing is used in rare cases when you need to catch a message outside its normal "bubble" route. It is **off**
//! by default for every widget, but can be enabled on per-widget instance basis.
//!
//! ## Widgets Overview
//!
//! The following subsections explains how to use every widget built into FyroxUI. We will order them by primary function to
//! help introduce them to new users.
//!
//! ### Containers
//!
//! The Container widgets primary purpose is to contain other widgets. They are mostly used as a tool to layout the UI in
//! visually different ways.
//!
//! * [`crate::stack_panel::StackPanel`]: The Stack Panel arranges widgets in a linear fashion, either vertically or horizontally
//! depending on how it's setup.
//! * [`crate::wrap_panel::WrapPanel`]: The Wrap Panel arranges widgets in a linear fashion but if it overflows the widgets are
//! continued adjacent to the first line. Can arrange widgets either vertically or horizontally depending on how it's setup.
//! * [`crate::grid::Grid`]: The Grid arranges widgets into rows and columns with given size constraints.
//! * [`crate::canvas::Canvas`]: The Canvas arranges widgets at their desired positions; it has infinite size and does not restrict
//! their children widgets position and size.
//! * [`crate::window::Window`]: The Window holds other widgets in a panel that can be configured at setup to be move-able,
//! expanded and contracted via user input, exited, and have a displayed label. The window has a title bar to assist with these
//! features.
//! * [`crate::messagebox::MessageBox`]: The Message Box is a Window that has been streamlined to show standard confirmation/information
//! dialogues, for example, closing a document with unsaved changes. It has a title, some text, and a fixed set of buttons (Yes, No,
//! Cancel in different combinations).
//! * [`crate::menu::Menu`]: The Menu is a root container for Menu Items, an example could be a menu strip with File, Edit, View, etc
//! items.
//! * [`crate::popup::Popup`]: The Popup is a panel that locks input to its content while it is open. A simple example of it could be a
//! context menu.
//! * [`crate::scroll_viewer::ScrollViewer`]: The ScrollViewer is a wrapper for Scroll Panel that adds two scroll bars to it.
//! * [`crate::scroll_panel::ScrollPanel`]: The Scroll Panel is a panel that allows you apply some offset to children widgets. It
//! is used to create "scrollable" area in conjunction with the Scroll Viewer.
//! * [`crate::expander::Expander`]: The Expander handles hiding and showing multiple panels of widgets in an according style UI element.
//! Multiple panels can be shown or hidden at any time based on user input.
//! * [`crate::tab_control::TabControl`]: The Tab Control handles hiding several panels of widgets, only showing the one that the user
//! has selected.
//! * [`crate::dock::DockingManager`]: The Docking manager allows you to dock windows and hold them in-place.
//! * [`crate::tree::Tree`]: The Tree allows you to create views for hierarchical data.
//! * [`crate::screen::Screen`]: The Screen widgets always has its bounds match the current screen size
//! thus making it possible to create widget hierarchy that always fits the screen bounds.
//!
//!
//! ### Visual
//!
//! The Visual widgets primary purpose is to provide the user feedback generally without the user directly interacting with them.
//!
//! * [`crate::text::Text`]: The Text widget is used to display a string to the user.
//! * [`crate::image::Image`]: The Image widget is used to display a pixel image to the user.
//! * [`crate::vector_image::VectorImage`]: The Vector Image is used to render vector instructions as a graphical element.
//! * [`crate::rect::RectEditor`]: The Rect allows you to specify numeric values for X, Y, Width, and Height of a rectangle.
//! * [`crate::progress_bar::ProgressBar`]: The Progress Bar shows a bar whose fill state can be adjusted to indicate visually how full
//! something is, for example how close to 100% is a loading process.
//! * [`crate::decorator::Decorator`]: The Decorator is used to style any widget. It has support for different styles depending on various
//! events like mouse hover or click.
//! * [`crate::border::Border`]: The Border widget is used in conjunction with the Decorator widget to provide configurable boarders to
//! any widget for styling purposes.
//!
//! ### Controls
//!
//! Control widgets primary purpose is to provide users with intractable UI elements to control some aspect of the program.
//!
//! * [`crate::border::Border`]: The Button provides a press-able control that can contain other UI elements, for example a Text
//! or Image Widget.
//! * [`crate::check_box::CheckBox`]: The Check Box is a toggle-able control that can contain other UI elements, for example a Text
//! or Image Widget.
//! * [`crate::text_box::TextBox`]: The Text Box is a control that allows the editing of text.
//! * [`crate::scroll_bar::ScrollBar`]: The Scroll Bar provides a scroll bar like control that can be used on it's own as a data input or with
//! certain other widgets to provide content scrolling capabilities.
//! * [`crate::numeric::NumericUpDown`]: The Numeric Field provides the ability to adjust a number via increment and decrement buttons or direct
//! input. The number can be constrained to remain inside a specific range or have a specific step.
//! * [`crate::range::RangeEditor`]: The Range allows the user to edit a numeric range - specify its begin and end values.
//! * [`crate::list_view::ListView`]: The List View provides a control where users can select from a list of items.
//! * [`crate::dropdown_list::DropdownList`]: The Drop-down List is a control which shows the currently selected item and provides a drop-down
//! list to select an item.
//! * [`crate::file_browser::FileBrowser`]: The File Browser is a tree view of the file system allowing the user to select a file or folder.
//! * [`crate::curve::CurveEditor`]: The CurveEditor allows editing parametric curves - adding points, and setting up transitions (constant,
//! linear, cubic) between them.
//! * [`crate::inspector::Inspector`]: The Inspector automatically creates and handles the input of UI elements based on a populated Inspector
//! Context given to it allowing the user to adjust values of a variety of models without manually creating UI's for each type.
//!
//! ## Examples
//!
//! A simple usage example could be the following code:
//!
//! ```rust
//! use fyrox_ui::{
//!     button::{ButtonBuilder, ButtonMessage},
//!     core::algebra::Vector2,
//!     widget::WidgetBuilder,
//!     UserInterface,
//! };
//!
//! // Create the UI first.
//! let mut ui = UserInterface::new(Vector2::new(1024.0, 768.0));
//!
//! // Add some widgets.
//! let button = ButtonBuilder::new(WidgetBuilder::new())
//!     .with_text("Click Me!")
//!     .build(&mut ui.build_ctx());
//!
//! // Poll the messages coming from the widgets and react to them.
//! while let Some(message) = ui.poll_message() {
//!     if let Some(ButtonMessage::Click) = message.data() {
//!         if message.destination() == button {
//!             println!("The button was clicked!");
//!         }
//!     }
//! }
//! ```
//!
//! **Important**: This example **does not** include any drawing or OS event processing! It is because this
//! crate is OS- and GAPI-agnostic and do not create native OS windows and cannot draw anything on screen.
//! For more specific examples, please see `examples` of the crate.

#![forbid(unsafe_code)]
#![allow(irrefutable_let_patterns)]
#![allow(clippy::float_cmp)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::from_over_into)]
#![allow(clippy::new_without_default)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]

pub use copypasta;
pub use fyrox_core as core;
use message::TouchPhase;

pub mod absm;
mod alignment;
pub mod animation;
pub mod bit;
pub mod border;
pub mod brush;
mod build;
pub mod button;
pub mod canvas;
pub mod check_box;
pub mod color;
mod control;
pub mod curve;
pub mod decorator;
pub mod dock;
pub mod draw;
pub mod dropdown_list;
pub mod dropdown_menu;
pub mod expander;
pub mod file_browser;
pub mod font;
pub mod formatted_text;
pub mod grid;
pub mod image;
pub mod inspector;
pub mod key;
pub mod list_view;
pub mod loader;
pub mod matrix2;
pub mod menu;
pub mod message;
pub mod messagebox;
pub mod navigation;
pub mod nine_patch;
mod node;
pub mod numeric;
pub mod path;
pub mod popup;
pub mod progress_bar;
pub mod range;
pub mod rect;
pub mod screen;
pub mod scroll_bar;
pub mod scroll_panel;
pub mod scroll_viewer;
pub mod searchbar;
pub mod selector;
pub mod stack_panel;
pub mod tab_control;
pub mod text;
pub mod text_box;
mod thickness;
pub mod tree;
pub mod utils;
pub mod uuid;
pub mod vec;
pub mod vector_image;
pub mod widget;
pub mod window;
pub mod wrap_panel;

use crate::{
    brush::Brush,
    canvas::Canvas,
    constructor::WidgetConstructorContainer,
    container::WidgetContainer,
    core::{
        algebra::{Matrix3, Vector2},
        color::Color,
        math::Rect,
        pool::{Handle, Pool},
        reflect::prelude::*,
        scope_profile,
        uuid::uuid,
        visitor::prelude::*,
    },
    core::{parking_lot::Mutex, pool::Ticket, uuid::Uuid, uuid_provider, TypeUuidProvider},
    draw::{CommandTexture, Draw, DrawingContext},
    font::FontResource,
    font::BUILT_IN_FONT,
    message::{
        ButtonState, CursorIcon, KeyboardModifiers, MessageDirection, MouseButton, OsEvent,
        UiMessage,
    },
    popup::{Placement, PopupMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
};
use copypasta::ClipboardContext;
use fxhash::{FxHashMap, FxHashSet};
use fyrox_resource::{
    io::FsResourceIo, io::ResourceIo, manager::ResourceManager, untyped::UntypedResource, Resource,
    ResourceData,
};
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::ops::{Deref, Index, IndexMut};
use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
    collections::{btree_set::BTreeSet, hash_map::Entry, VecDeque},
    error::Error,
    fmt::{Debug, Formatter},
    ops::DerefMut,
    path::Path,
    sync::{
        mpsc::{self, Receiver, Sender, TryRecvError},
        Arc,
    },
};
use strum_macros::{AsRefStr, EnumString, VariantNames};

pub use alignment::*;
pub use build::*;
pub use control::*;
use fyrox_core::futures::future::join_all;
use fyrox_core::log::Log;
use fyrox_graph::{
    AbstractSceneGraph, AbstractSceneNode, BaseSceneGraph, NodeHandleMap, NodeMapping, PrefabData,
    SceneGraph, SceneGraphNode,
};
pub use node::*;
pub use thickness::*;

pub use fyrox_animation as generic_animation;
use fyrox_core::pool::ErasedHandle;

// TODO: Make this part of UserInterface struct.
pub const COLOR_COAL_BLACK: Color = Color::opaque(10, 10, 10);
pub const COLOR_DARKEST: Color = Color::opaque(20, 20, 20);
pub const COLOR_DARKER: Color = Color::opaque(30, 30, 30);
pub const COLOR_DARK: Color = Color::opaque(40, 40, 40);
pub const COLOR_PRIMARY: Color = Color::opaque(50, 50, 50);
pub const COLOR_LIGHT: Color = Color::opaque(70, 70, 70);
pub const COLOR_LIGHTER: Color = Color::opaque(85, 85, 85);
pub const COLOR_LIGHTEST: Color = Color::opaque(100, 100, 100);
pub const COLOR_BRIGHT: Color = Color::opaque(130, 130, 130);
pub const COLOR_BRIGHTEST: Color = Color::opaque(160, 160, 160);
pub const COLOR_BRIGHT_BLUE: Color = Color::opaque(80, 118, 178);
pub const COLOR_DIM_BLUE: Color = Color::opaque(66, 99, 149);
pub const COLOR_TEXT: Color = Color::opaque(220, 220, 220);
pub const COLOR_FOREGROUND: Color = Color::WHITE;

pub const BRUSH_COAL_BLACK: Brush = Brush::Solid(COLOR_COAL_BLACK);
pub const BRUSH_DARKEST: Brush = Brush::Solid(COLOR_DARKEST);
pub const BRUSH_DARKER: Brush = Brush::Solid(COLOR_DARKER);
pub const BRUSH_DARK: Brush = Brush::Solid(COLOR_DARK);
pub const BRUSH_PRIMARY: Brush = Brush::Solid(COLOR_PRIMARY);
pub const BRUSH_LIGHT: Brush = Brush::Solid(COLOR_LIGHT);
pub const BRUSH_LIGHTER: Brush = Brush::Solid(COLOR_LIGHTER);
pub const BRUSH_LIGHTEST: Brush = Brush::Solid(COLOR_LIGHTEST);
pub const BRUSH_BRIGHT: Brush = Brush::Solid(COLOR_BRIGHT);
pub const BRUSH_BRIGHTEST: Brush = Brush::Solid(COLOR_BRIGHTEST);
pub const BRUSH_BRIGHT_BLUE: Brush = Brush::Solid(COLOR_BRIGHT_BLUE);
pub const BRUSH_DIM_BLUE: Brush = Brush::Solid(COLOR_DIM_BLUE);
pub const BRUSH_TEXT: Brush = Brush::Solid(COLOR_TEXT);
pub const BRUSH_FOREGROUND: Brush = Brush::Solid(COLOR_FOREGROUND);

#[derive(Default, Reflect, Debug)]
pub(crate) struct RcUiNodeHandleInner {
    handle: Handle<UiNode>,
    #[reflect(hidden)]
    sender: Option<Sender<UiMessage>>,
}

impl Visit for RcUiNodeHandleInner {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.handle.visit(name, visitor)?;

        if visitor.is_reading() {
            self.sender = Some(
                visitor
                    .blackboard
                    .get::<Sender<UiMessage>>()
                    .expect("Ui message sender must be provided for correct deserialization!")
                    .clone(),
            );
        }

        Ok(())
    }
}

impl Drop for RcUiNodeHandleInner {
    fn drop(&mut self) {
        if let Some(sender) = self.sender.as_ref() {
            let _ = sender.send(WidgetMessage::remove(
                self.handle,
                MessageDirection::ToWidget,
            ));
        } else {
            Log::warn(format!(
                "There's no message sender for shared handle {}. The object \
            won't be destroyed.",
                self.handle
            ))
        }
    }
}

/// Reference counted handle to a widget. It is used to automatically destroy the widget it points
/// to when the reference counter reaches zero. It's main usage in the library is to store handles
/// to context menus, that could be shared across multiple widgets.
#[derive(Clone, Default, Visit, Reflect, TypeUuidProvider)]
#[type_uuid(id = "9111a53b-05dc-4c75-aab1-71d5b1c93311")]
pub struct RcUiNodeHandle(Arc<Mutex<RcUiNodeHandleInner>>);

impl Debug for RcUiNodeHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let handle = self.0.lock().handle;

        writeln!(
            f,
            "RcUiNodeHandle - {}:{} with {} uses",
            handle.index(),
            handle.generation(),
            Arc::strong_count(&self.0)
        )
    }
}

impl PartialEq for RcUiNodeHandle {
    fn eq(&self, other: &Self) -> bool {
        let a = self.0.lock().handle;
        let b = other.0.lock().handle;
        a == b
    }
}

impl RcUiNodeHandle {
    /// Creates a new reference counted widget handle.
    #[inline]
    pub fn new(handle: Handle<UiNode>, sender: Sender<UiMessage>) -> Self {
        Self(Arc::new(Mutex::new(RcUiNodeHandleInner {
            handle,
            sender: Some(sender),
        })))
    }

    /// Returns the inner handle.
    #[inline]
    pub fn handle(&self) -> Handle<UiNode> {
        self.0.lock().handle
    }
}

/// Orientation of something.
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Visit,
    Reflect,
    Default,
    Serialize,
    Deserialize,
    AsRefStr,
    EnumString,
    VariantNames,
)]
pub enum Orientation {
    /// Vertical orientation. This is default value.
    #[default]
    Vertical,
    /// Horizontal orientation.
    Horizontal,
}

uuid_provider!(Orientation = "1c6ad1b0-3f4c-48be-87dd-6929cb3577bf");

#[derive(Default, Clone)]
pub struct NodeStatistics(pub FxHashMap<&'static str, isize>);

impl NodeStatistics {
    pub fn new(ui: &UserInterface) -> NodeStatistics {
        let mut statistics = Self::default();
        for node in ui.nodes.iter() {
            statistics
                .0
                .entry(BaseControl::type_name(&*node.0))
                .and_modify(|counter| *counter += 1)
                .or_insert(1);
        }
        statistics
    }

    fn unite_type_names(&self, prev_stats: &NodeStatistics) -> BTreeSet<&'static str> {
        let mut union = BTreeSet::default();
        for stats in [self, prev_stats] {
            for &type_name in stats.0.keys() {
                union.insert(type_name);
            }
        }
        union
    }

    fn count_of(&self, type_name: &str) -> isize {
        self.0.get(type_name).cloned().unwrap_or_default()
    }

    pub fn print_diff(&self, prev_stats: &NodeStatistics, show_unchanged: bool) {
        println!("**** Diff UI Node Statistics ****");
        for type_name in self.unite_type_names(prev_stats) {
            let count = self.count_of(type_name);
            let prev_count = prev_stats.count_of(type_name);
            let delta = count - prev_count;
            if delta != 0 || show_unchanged {
                println!("{}: \x1b[93m{}\x1b[0m", type_name, delta);
            }
        }
    }

    pub fn print_changed(&self, prev_stats: &NodeStatistics) {
        println!("**** Changed UI Node Statistics ****");
        for type_name in self.unite_type_names(prev_stats) {
            let count = self.count_of(type_name);
            let prev_count = prev_stats.count_of(type_name);
            if count - prev_count != 0 {
                println!("{}: \x1b[93m{}\x1b[0m", type_name, count);
            }
        }
    }
}

#[derive(Visit, Reflect, Debug, Clone)]
pub struct DragContext {
    pub is_dragging: bool,
    pub drag_node: Handle<UiNode>,
    pub click_pos: Vector2<f32>,
    pub drag_preview: Handle<UiNode>,
}

impl Default for DragContext {
    fn default() -> Self {
        Self {
            is_dragging: false,
            drag_node: Default::default(),
            click_pos: Vector2::new(0.0, 0.0),
            drag_preview: Default::default(),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Visit, Reflect)]
pub struct MouseState {
    pub left: ButtonState,
    pub right: ButtonState,
    pub middle: ButtonState,
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

#[derive(Copy, Clone, Visit, Reflect, Debug, Default)]
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

#[derive(Clone, Debug)]
struct TooltipEntry {
    tooltip: RcUiNodeHandle,
    /// Time remaining until this entry should disappear (in seconds).
    time: f32,
    /// Maximum time that it should be kept for
    /// This is stored here as well, because when hovering
    /// over the tooltip, we don't know the time it should stay for and
    /// so we use this to refresh the timer.
    max_time: f32,
}
impl TooltipEntry {
    fn new(tooltip: RcUiNodeHandle, time: f32) -> TooltipEntry {
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

#[derive(Debug)]
pub enum LayoutEvent {
    MeasurementInvalidated(Handle<UiNode>),
    ArrangementInvalidated(Handle<UiNode>),
    VisibilityChanged(Handle<UiNode>),
}

#[derive(Clone, Debug, Visit, Reflect, Default)]
struct DoubleClickEntry {
    timer: f32,
    click_count: u32,
}

struct Clipboard(Option<RefCell<ClipboardContext>>);

impl Debug for Clipboard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Clipboard")
    }
}

#[derive(Default, Debug, Clone)]
struct WidgetMethodsRegistry {
    preview_message: FxHashSet<Handle<UiNode>>,
    on_update: FxHashSet<Handle<UiNode>>,
    handle_os_event: FxHashSet<Handle<UiNode>>,
}

impl WidgetMethodsRegistry {
    fn register<T: Control + ?Sized>(&mut self, node: &T) {
        let node_handle = node.handle();

        if node.preview_messages && !self.preview_message.insert(node_handle) {
            Log::warn(format!(
                "Widget {node_handle} `preview_message` method is already registered!"
            ));
        }
        if node.handle_os_events && !self.handle_os_event.insert(node_handle) {
            Log::warn(format!(
                "Widget {node_handle} `handle_os_event` method is already registered!"
            ));
        }
        if node.need_update && !self.on_update.insert(node_handle) {
            Log::warn(format!(
                "Widget {node_handle} `on_update` method is already registered!"
            ));
        }
    }

    fn unregister<T: Control + ?Sized>(&mut self, node: &T) {
        let node_handle = node.handle();

        self.preview_message.remove(&node_handle);
        self.on_update.remove(&node_handle);
        self.handle_os_event.remove(&node_handle);
    }
}

/// A set of switches that allows you to disable a particular step of UI update pipeline.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct UiUpdateSwitches {
    /// A set of nodes that will be updated, everything else won't be updated.
    pub node_overrides: Option<FxHashSet<Handle<UiNode>>>,
}

#[derive(Reflect, Debug)]
pub struct UserInterface {
    screen_size: Vector2<f32>,
    nodes: Pool<UiNode, WidgetContainer>,
    #[reflect(hidden)]
    drawing_context: DrawingContext,
    visual_debug: bool,
    root_canvas: Handle<UiNode>,
    picked_node: Handle<UiNode>,
    prev_picked_node: Handle<UiNode>,
    captured_node: Handle<UiNode>,
    keyboard_focus_node: Handle<UiNode>,
    cursor_position: Vector2<f32>,
    #[reflect(hidden)]
    receiver: Receiver<UiMessage>,
    #[reflect(hidden)]
    sender: Sender<UiMessage>,
    stack: Vec<Handle<UiNode>>,
    picking_stack: Vec<RestrictionEntry>,
    #[reflect(hidden)]
    bubble_queue: VecDeque<Handle<UiNode>>,
    drag_context: DragContext,
    mouse_state: MouseState,
    keyboard_modifiers: KeyboardModifiers,
    cursor_icon: CursorIcon,
    #[reflect(hidden)]
    active_tooltip: Option<TooltipEntry>,
    #[reflect(hidden)]
    methods_registry: WidgetMethodsRegistry,
    #[reflect(hidden)]
    clipboard: Clipboard,
    #[reflect(hidden)]
    layout_events_receiver: Receiver<LayoutEvent>,
    #[reflect(hidden)]
    layout_events_sender: Sender<LayoutEvent>,
    need_update_global_transform: bool,
    #[reflect(hidden)]
    pub default_font: FontResource,
    #[reflect(hidden)]
    double_click_entries: FxHashMap<MouseButton, DoubleClickEntry>,
    pub double_click_time_slice: f32,
}

impl Visit for UserInterface {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        if region.is_reading() {
            self.nodes.clear();
            self.root_canvas = Handle::NONE;
            self.methods_registry = Default::default();
        }

        self.screen_size.visit("ScreenSize", &mut region)?;
        self.nodes.visit("Nodes", &mut region)?;
        self.visual_debug.visit("VisualDebug", &mut region)?;
        self.root_canvas.visit("RootCanvas", &mut region)?;
        self.picked_node.visit("PickedNode", &mut region)?;
        self.prev_picked_node.visit("PrevPickedNode", &mut region)?;
        self.captured_node.visit("CapturedNode", &mut region)?;
        self.keyboard_focus_node
            .visit("KeyboardFocusNode", &mut region)?;
        self.cursor_position.visit("CursorPosition", &mut region)?;
        self.picking_stack.visit("PickingStack", &mut region)?;
        self.drag_context.visit("DragContext", &mut region)?;
        self.mouse_state.visit("MouseState", &mut region)?;
        self.keyboard_modifiers
            .visit("KeyboardModifiers", &mut region)?;
        self.cursor_icon.visit("CursorIcon", &mut region)?;
        self.double_click_time_slice
            .visit("DoubleClickTimeSlice", &mut region)?;

        if region.is_reading() {
            for node in self.nodes.iter() {
                self.methods_registry.register(node.deref());
            }
        }

        Ok(())
    }
}

impl Clone for UserInterface {
    fn clone(&self) -> Self {
        let (sender, receiver) = mpsc::channel();
        let (layout_events_sender, layout_events_receiver) = mpsc::channel();
        let mut nodes = Pool::new();
        for (handle, node) in self.nodes.pair_iter() {
            let mut clone = node.clone_boxed();
            clone.layout_events_sender = Some(layout_events_sender.clone());
            nodes.spawn_at_handle(handle, UiNode(clone)).unwrap();
        }

        Self {
            screen_size: self.screen_size,
            nodes,
            drawing_context: self.drawing_context.clone(),
            visual_debug: self.visual_debug,
            root_canvas: self.root_canvas,
            picked_node: self.picked_node,
            prev_picked_node: self.prev_picked_node,
            captured_node: self.captured_node,
            keyboard_focus_node: self.keyboard_focus_node,
            cursor_position: self.cursor_position,
            receiver,
            sender,
            stack: self.stack.clone(),
            picking_stack: self.picking_stack.clone(),
            bubble_queue: self.bubble_queue.clone(),
            drag_context: self.drag_context.clone(),
            mouse_state: self.mouse_state,
            keyboard_modifiers: self.keyboard_modifiers,
            cursor_icon: self.cursor_icon,
            active_tooltip: self.active_tooltip.clone(),
            methods_registry: self.methods_registry.clone(),
            clipboard: Clipboard(ClipboardContext::new().ok().map(RefCell::new)),
            layout_events_receiver,
            layout_events_sender,
            need_update_global_transform: self.need_update_global_transform,
            default_font: self.default_font.clone(),
            double_click_entries: self.double_click_entries.clone(),
            double_click_time_slice: self.double_click_time_slice,
        }
    }
}

impl Default for UserInterface {
    fn default() -> Self {
        Self::new(Vector2::new(100.0, 100.0))
    }
}

#[derive(Default)]
pub struct UiContainer {
    pool: Pool<UserInterface>,
}

impl UiContainer {
    /// Creates a new user interface container.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new user interface container with the given user interface.
    pub fn new_with_ui(ui: UserInterface) -> Self {
        let mut pool = Pool::new();
        let _ = pool.spawn(ui);
        Self { pool }
    }

    /// Returns a reference to the first user interface in the container. Panics, if the container
    /// is empty.
    pub fn first(&self) -> &UserInterface {
        self.pool
            .first_ref()
            .expect("The container must have at least one user interface.")
    }

    /// Returns a reference to the first user interface in the container. Panics, if the container
    /// is empty.
    pub fn first_mut(&mut self) -> &mut UserInterface {
        self.pool
            .first_mut()
            .expect("The container must have at least one user interface.")
    }

    /// Return true if given handle is valid and "points" to "alive" user interface.
    pub fn is_valid_handle(&self, handle: Handle<UserInterface>) -> bool {
        self.pool.is_valid_handle(handle)
    }

    /// Returns pair iterator which yields (handle, user_interface_ref) pairs.
    pub fn pair_iter(&self) -> impl Iterator<Item = (Handle<UserInterface>, &UserInterface)> {
        self.pool.pair_iter()
    }

    /// Returns pair iterator which yields (handle, user_interface_ref) pairs.
    pub fn pair_iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (Handle<UserInterface>, &mut UserInterface)> {
        self.pool.pair_iter_mut()
    }

    /// Tries to borrow a user interface using its handle.
    pub fn try_get(&self, handle: Handle<UserInterface>) -> Option<&UserInterface> {
        self.pool.try_borrow(handle)
    }

    /// Tries to borrow a user interface using its handle.
    pub fn try_get_mut(&mut self, handle: Handle<UserInterface>) -> Option<&mut UserInterface> {
        self.pool.try_borrow_mut(handle)
    }

    /// Creates new iterator over user interfaces in container.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &UserInterface> {
        self.pool.iter()
    }

    /// Creates new mutable iterator over user interfaces in container.
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut UserInterface> {
        self.pool.iter_mut()
    }

    /// Adds a new user interface into container.
    #[inline]
    pub fn add(&mut self, scene: UserInterface) -> Handle<UserInterface> {
        self.pool.spawn(scene)
    }

    /// Removes all user interfaces from container.
    #[inline]
    pub fn clear(&mut self) {
        self.pool.clear()
    }

    /// Removes the given user interface from container. The user interface will be destroyed
    /// immediately.
    #[inline]
    pub fn remove(&mut self, handle: Handle<UserInterface>) {
        self.pool.free(handle);
    }

    /// Takes a user interface from the container and transfers ownership to caller. You must either
    /// put the user interface back using ticket or call `forget_ticket` to make memory used by the
    /// user interface vacant again.
    pub fn take_reserve(
        &mut self,
        handle: Handle<UserInterface>,
    ) -> (Ticket<UserInterface>, UserInterface) {
        self.pool.take_reserve(handle)
    }

    /// Puts a user interface back to the container using its ticket.
    pub fn put_back(
        &mut self,
        ticket: Ticket<UserInterface>,
        scene: UserInterface,
    ) -> Handle<UserInterface> {
        self.pool.put_back(ticket, scene)
    }

    /// Forgets ticket of a user interface, making place at which ticket points, vacant again.
    pub fn forget_ticket(&mut self, ticket: Ticket<UserInterface>) {
        self.pool.forget_ticket(ticket)
    }
}

impl Index<Handle<UserInterface>> for UiContainer {
    type Output = UserInterface;

    #[inline]
    fn index(&self, index: Handle<UserInterface>) -> &Self::Output {
        &self.pool[index]
    }
}

impl IndexMut<Handle<UserInterface>> for UiContainer {
    #[inline]
    fn index_mut(&mut self, index: Handle<UserInterface>) -> &mut Self::Output {
        &mut self.pool[index]
    }
}

fn is_on_screen(node: &UiNode, nodes: &Pool<UiNode, WidgetContainer>) -> bool {
    // Crawl up on tree and check if current bounds are intersects with every screen bound
    // of parents chain. This is needed because some control can move their children outside of
    // their bounds (like scroll viewer, etc.) and single intersection test of parent bounds with
    // current bounds is not enough.
    let bounds = node.clip_bounds();
    let mut parent = node.parent();
    while parent.is_some() {
        let parent_node = nodes.borrow(parent);
        if !parent_node.clip_bounds().intersects(bounds) {
            return false;
        }
        parent = parent_node.parent();
    }
    true
}

fn draw_node(
    nodes: &Pool<UiNode, WidgetContainer>,
    node_handle: Handle<UiNode>,
    drawing_context: &mut DrawingContext,
) {
    scope_profile!();

    let node = &nodes[node_handle];
    if !node.is_globally_visible() {
        return;
    }

    if !is_on_screen(node, nodes) {
        return;
    }

    let pushed = if !is_node_enabled(nodes, node_handle) {
        drawing_context.push_opacity(0.4);
        true
    } else if let Some(opacity) = node.opacity() {
        drawing_context.push_opacity(opacity);
        true
    } else {
        false
    };

    drawing_context.transform_stack.push(node.visual_transform);

    // Draw
    {
        let start_index = drawing_context.get_commands().len();
        node.draw(drawing_context);
        let end_index = drawing_context.get_commands().len();
        node.command_indices
            .borrow_mut()
            .extend(start_index..end_index);
    }

    // Continue on children
    for &child_node in node.children().iter() {
        // Do not continue render of top-most nodes - they'll be rendered in separate pass.
        if !nodes[child_node].is_draw_on_top() {
            draw_node(nodes, child_node, drawing_context);
        }
    }

    // Post draw.
    {
        let start_index = drawing_context.get_commands().len();
        node.post_draw(drawing_context);
        let end_index = drawing_context.get_commands().len();
        node.command_indices
            .borrow_mut()
            .extend(start_index..end_index);
    }

    drawing_context.transform_stack.pop();

    if pushed {
        drawing_context.pop_opacity();
    }
}

fn is_node_enabled(nodes: &Pool<UiNode, WidgetContainer>, handle: Handle<UiNode>) -> bool {
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

#[derive(Debug)]
pub struct SubGraph {
    pub root: (Ticket<UiNode>, UiNode),

    pub descendants: Vec<(Ticket<UiNode>, UiNode)>,

    pub parent: Handle<UiNode>,
}

fn remap_handles(old_new_mapping: &NodeHandleMap<UiNode>, ui: &mut UserInterface) {
    // Iterate over instantiated nodes and remap handles.
    for (_, &new_node_handle) in old_new_mapping.inner().iter() {
        old_new_mapping.remap_handles(
            &mut ui.nodes[new_node_handle],
            &[TypeId::of::<UntypedResource>()],
        );
    }
}

impl UserInterface {
    pub fn new(screen_size: Vector2<f32>) -> UserInterface {
        let (sender, receiver) = mpsc::channel();
        Self::new_with_channel(sender, receiver, screen_size)
    }

    pub fn new_with_channel(
        sender: Sender<UiMessage>,
        receiver: Receiver<UiMessage>,
        screen_size: Vector2<f32>,
    ) -> UserInterface {
        let (layout_events_sender, layout_events_receiver) = mpsc::channel();
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
            active_tooltip: Default::default(),
            methods_registry: Default::default(),
            clipboard: Clipboard(ClipboardContext::new().ok().map(RefCell::new)),
            layout_events_receiver,
            layout_events_sender,
            need_update_global_transform: Default::default(),
            default_font: BUILT_IN_FONT.clone(),
            double_click_entries: Default::default(),
            double_click_time_slice: 0.5, // 500 ms is standard in most operating systems.
        };
        ui.root_canvas = ui.add_node(UiNode::new(Canvas {
            widget: WidgetBuilder::new().build(),
        }));
        ui.keyboard_focus_node = ui.root_canvas;
        ui
    }

    pub fn keyboard_modifiers(&self) -> KeyboardModifiers {
        self.keyboard_modifiers
    }

    pub fn build_ctx(&mut self) -> BuildContext<'_> {
        self.into()
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

    fn update_global_visibility(&mut self, from: Handle<UiNode>) {
        scope_profile!();

        self.stack.clear();
        self.stack.push(from);
        while let Some(node_handle) = self.stack.pop() {
            let (widget, parent) = self
                .nodes
                .try_borrow_dependant_mut(node_handle, |n| n.parent());

            if let Some(widget) = widget {
                self.stack.extend_from_slice(widget.children());

                let visibility = if let Some(parent) = parent {
                    widget.visibility() && parent.is_globally_visible()
                } else {
                    widget.visibility()
                };

                if widget.prev_global_visibility != visibility {
                    let _ = self
                        .layout_events_sender
                        .send(LayoutEvent::MeasurementInvalidated(node_handle));
                    let _ = self
                        .layout_events_sender
                        .send(LayoutEvent::ArrangementInvalidated(node_handle));
                }

                widget.set_global_visibility(visibility);
            }
        }
    }

    fn update_visual_transform(&mut self, from: Handle<UiNode>) {
        scope_profile!();

        self.stack.clear();
        self.stack.push(from);
        while let Some(node_handle) = self.stack.pop() {
            let (widget, parent) = self
                .nodes
                .try_borrow_dependant_mut(node_handle, |n| n.parent());

            let widget = widget.unwrap();

            if widget.is_globally_visible() {
                self.stack.extend_from_slice(widget.children());

                let mut layout_transform = widget.layout_transform;

                layout_transform[6] = widget.actual_local_position().x;
                layout_transform[7] = widget.actual_local_position().y;

                let visual_transform = if let Some(parent) = parent {
                    parent.visual_transform * widget.render_transform * layout_transform
                } else {
                    widget.render_transform * layout_transform
                };

                widget.visual_transform = visual_transform;
            }
        }
    }

    pub fn screen_size(&self) -> Vector2<f32> {
        self.screen_size
    }

    pub fn set_screen_size(&mut self, screen_size: Vector2<f32>) {
        self.screen_size = screen_size;
    }

    fn handle_layout_events(&mut self) {
        fn invalidate_recursive_up(
            nodes: &Pool<UiNode, WidgetContainer>,
            node: Handle<UiNode>,
            callback: fn(&UiNode),
        ) {
            if let Some(node_ref) = nodes.try_borrow(node) {
                (callback)(node_ref);
                if node_ref.parent().is_some() {
                    invalidate_recursive_up(nodes, node_ref.parent(), callback);
                }
            }
        }

        while let Ok(layout_event) = self.layout_events_receiver.try_recv() {
            match layout_event {
                LayoutEvent::MeasurementInvalidated(node) => {
                    invalidate_recursive_up(&self.nodes, node, |node_ref| {
                        node_ref.measure_valid.set(false)
                    });
                }
                LayoutEvent::ArrangementInvalidated(node) => {
                    invalidate_recursive_up(&self.nodes, node, |node_ref| {
                        node_ref.arrange_valid.set(false)
                    });
                    self.need_update_global_transform = true;
                }
                LayoutEvent::VisibilityChanged(node) => {
                    self.update_global_visibility(node);
                }
            }
        }
    }

    pub fn invalidate_layout(&mut self) {
        for node in self.nodes.iter_mut() {
            node.invalidate_layout();
        }
    }

    pub fn update_layout(&mut self, screen_size: Vector2<f32>) {
        self.screen_size = screen_size;

        self.handle_layout_events();

        self.measure_node(self.root_canvas, screen_size);
        let arrangement_changed = self.arrange_node(
            self.root_canvas,
            &Rect::new(0.0, 0.0, screen_size.x, screen_size.y),
        );

        if self.need_update_global_transform {
            self.update_visual_transform(self.root_canvas);
            self.need_update_global_transform = false;
        }

        if arrangement_changed {
            self.calculate_clip_bounds(
                self.root_canvas,
                Rect::new(0.0, 0.0, self.screen_size.x, self.screen_size.y),
            );
        }
    }

    pub fn update(&mut self, screen_size: Vector2<f32>, dt: f32, switches: &UiUpdateSwitches) {
        for entry in self.double_click_entries.values_mut() {
            entry.timer -= dt;
        }

        self.update_layout(screen_size);

        if let Some(node_overrides) = switches.node_overrides.as_ref() {
            for &handle in node_overrides.iter() {
                let (ticket, mut node) = self.nodes.take_reserve(handle);
                node.update(dt, self);
                self.nodes.put_back(ticket, node);
            }
        } else {
            let update_subs = std::mem::take(&mut self.methods_registry.on_update);
            for &handle in update_subs.iter() {
                let (ticket, mut node) = self.nodes.take_reserve(handle);
                node.update(dt, self);
                self.nodes.put_back(ticket, node);
            }
            self.methods_registry.on_update = update_subs;
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

        self.drawing_context.clear();

        for node in self.nodes.iter_mut() {
            node.command_indices.get_mut().clear();
        }

        // Draw everything except top-most nodes.
        draw_node(&self.nodes, self.root_canvas, &mut self.drawing_context);

        // Render top-most nodes in separate pass.
        // TODO: This may give weird results because of invalid nesting.
        self.stack.clear();
        self.stack.push(self.root());
        while let Some(node_handle) = self.stack.pop() {
            let node = &self.nodes[node_handle];

            if !is_on_screen(node, &self.nodes) {
                continue;
            }

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

        if let Some(keyboard_focus_node) = self.nodes.try_borrow(self.keyboard_focus_node) {
            if keyboard_focus_node.global_visibility && keyboard_focus_node.accepts_input {
                let bounds = keyboard_focus_node.screen_bounds().inflate(1.0, 1.0);
                self.drawing_context.push_rounded_rect(&bounds, 1.0, 2.0, 6);
                self.drawing_context.commit(
                    bounds,
                    Brush::Solid(COLOR_BRIGHT_BLUE),
                    CommandTexture::None,
                    None,
                );
            }
        }

        &self.drawing_context
    }

    pub fn clipboard(&self) -> Option<Ref<ClipboardContext>> {
        self.clipboard.0.as_ref().map(|v| v.borrow())
    }

    pub fn clipboard_mut(&self) -> Option<RefMut<ClipboardContext>> {
        self.clipboard.0.as_ref().map(|v| v.borrow_mut())
    }

    pub fn arrange_node(&self, handle: Handle<UiNode>, final_rect: &Rect<f32>) -> bool {
        scope_profile!();

        let node = self.node(handle);

        if node.is_arrange_valid() && node.prev_arrange.get() == *final_rect {
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

            size = transform_size(size, &node.layout_transform);

            if !node.ignore_layout_rounding {
                size.x = size.x.ceil();
                size.y = size.y.ceil();
            }

            size = node.arrange_override(self, size);

            size.x = size.x.min(final_rect.w());
            size.y = size.y.min(final_rect.h());

            let transformed_rect =
                Rect::new(0.0, 0.0, size.x, size.y).transform(&node.layout_transform);

            size = transformed_rect.size;

            let mut origin =
                final_rect.position - transformed_rect.position + node.margin().offset();

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

            if !node.ignore_layout_rounding {
                origin.x = origin.x.floor();
                origin.y = origin.y.floor();
            }

            node.commit_arrange(origin, size);
        }

        true
    }

    pub fn measure_node(&self, handle: Handle<UiNode>, available_size: Vector2<f32>) -> bool {
        scope_profile!();

        let node = self.node(handle);

        if node.is_measure_valid() && node.prev_measure.get() == available_size {
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

            size = transform_size(size, &node.layout_transform);

            size.x = size.x.clamp(node.min_size().x, node.max_size().x);
            size.y = size.y.clamp(node.min_size().y, node.max_size().y);

            let mut desired_size = node.measure_override(self, size);

            desired_size = Rect::new(0.0, 0.0, desired_size.x, desired_size.y)
                .transform(&node.layout_transform)
                .size;

            if !node.width().is_nan() {
                desired_size.x = node.width();
            }
            if !node.height().is_nan() {
                desired_size.y = node.height();
            }

            desired_size.x = desired_size.x.clamp(node.min_size().x, node.max_size().x);
            desired_size.y = desired_size.y.clamp(node.min_size().y, node.max_size().y);

            desired_size += axes_margin;

            if node.ignore_layout_rounding {
                desired_size.x = desired_size.x.min(available_size.x);
                desired_size.y = desired_size.y.min(available_size.y);
            } else {
                desired_size.x = desired_size.x.min(available_size.x).ceil();
                desired_size.y = desired_size.y.min(available_size.y).ceil();
            }

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
            clipped = !widget.clip_bounds().contains(pt);

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
            || !widget.clip_bounds().intersects(Rect {
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

    pub fn hit_test_unrestricted(&self, pt: Vector2<f32>) -> Handle<UiNode> {
        // We're not restricted to any node, just start from root.
        let mut level = 0;
        self.pick_node(self.root_canvas, pt, &mut level)
    }

    pub fn hit_test(&self, pt: Vector2<f32>) -> Handle<UiNode> {
        scope_profile!();

        if self.nodes.is_valid_handle(self.captured_node) {
            self.captured_node
        } else if self.picking_stack.is_empty() {
            self.hit_test_unrestricted(pt)
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

        let screen_bounds = if *node.clip_to_bounds {
            node.screen_bounds()
        } else {
            Rect::new(0.0, 0.0, self.screen_size.x, self.screen_size.y)
        };

        node.clip_bounds.set(
            screen_bounds
                .clip_by(parent_bounds)
                .unwrap_or(screen_bounds),
        );

        for &child in node.children() {
            self.calculate_clip_bounds(child, node.clip_bounds.get());
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

    fn make_lowermost(&mut self, node: Handle<UiNode>) {
        let parent = self.node(node).parent();
        if parent.is_some() {
            let parent = &mut self.nodes[parent];
            parent.remove_child(node);
            parent.add_child(node, true);
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
                // we have skip processing of such messages.
                if !self.nodes.is_valid_handle(message.destination()) {
                    return Some(message);
                }

                if message.need_perform_layout() {
                    self.update_layout(self.screen_size);
                }

                for &handle in self.methods_registry.preview_message.iter() {
                    if let Some(node_ref) = self.nodes.try_borrow(handle) {
                        node_ref.preview_message(self, &mut message);
                    }
                }

                self.bubble_message(&mut message);

                if let Some(msg) = message.data::<WidgetMessage>() {
                    match msg {
                        WidgetMessage::ZIndex(_) => {
                            // Keep order of children of a parent node of a node that changed z-index
                            // the same as z-index of children.
                            if let Some(parent) =
                                self.try_get(message.destination()).map(|n| n.parent())
                            {
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
                        WidgetMessage::Focus => {
                            if self.nodes.is_valid_handle(message.destination())
                                && message.direction() == MessageDirection::ToWidget
                            {
                                self.request_focus(message.destination());
                            }
                        }
                        WidgetMessage::Unfocus => {
                            if self.nodes.is_valid_handle(message.destination())
                                && message.direction() == MessageDirection::ToWidget
                            {
                                self.request_focus(self.root_canvas);
                            }
                        }
                        WidgetMessage::Topmost => {
                            if self.nodes.is_valid_handle(message.destination()) {
                                self.make_topmost(message.destination());
                            }
                        }
                        WidgetMessage::Lowermost => {
                            if self.nodes.is_valid_handle(message.destination()) {
                                self.make_lowermost(message.destination());
                            }
                        }
                        WidgetMessage::Unlink => {
                            if self.nodes.is_valid_handle(message.destination()) {
                                self.unlink_node(message.destination());

                                let node = &self.nodes[message.destination()];
                                let new_position = node.screen_position();
                                self.send_message(WidgetMessage::desired_position(
                                    message.destination(),
                                    MessageDirection::ToWidget,
                                    new_position,
                                ));
                            }
                        }
                        &WidgetMessage::LinkWith(parent) => {
                            if self.nodes.is_valid_handle(message.destination())
                                && self.nodes.is_valid_handle(parent)
                            {
                                self.link_nodes(message.destination(), parent, false);
                            }
                        }
                        &WidgetMessage::LinkWithReverse(parent) => {
                            if self.nodes.is_valid_handle(message.destination())
                                && self.nodes.is_valid_handle(parent)
                            {
                                self.link_nodes(message.destination(), parent, true);
                            }
                        }
                        WidgetMessage::Remove => {
                            if self.nodes.is_valid_handle(message.destination()) {
                                self.remove_node(message.destination());
                            }
                        }
                        WidgetMessage::ContextMenu(context_menu) => {
                            if self.nodes.is_valid_handle(message.destination()) {
                                let node = self.nodes.borrow_mut(message.destination());
                                node.set_context_menu(context_menu.clone());
                            }
                        }
                        WidgetMessage::Tooltip(tooltip) => {
                            if self.nodes.is_valid_handle(message.destination()) {
                                let node = self.nodes.borrow_mut(message.destination());
                                node.set_tooltip(tooltip.clone());
                            }
                        }
                        WidgetMessage::Center => {
                            if self.nodes.is_valid_handle(message.destination()) {
                                let node = self.node(message.destination());
                                let size = node.actual_initial_size();
                                let parent = node.parent();
                                let parent_size = if parent.is_some() {
                                    self.node(parent).actual_initial_size()
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
                        WidgetMessage::RenderTransform(_) => {
                            if self.nodes.is_valid_handle(message.destination()) {
                                self.update_visual_transform(message.destination());
                            }
                        }
                        WidgetMessage::AdjustPositionToFit => {
                            if self.nodes.is_valid_handle(message.destination()) {
                                let node = self.node(message.destination());
                                let mut position = node.actual_local_position();
                                let size = node.actual_initial_size();
                                let parent = node.parent();
                                let parent_size = if parent.is_some() {
                                    self.node(parent).actual_initial_size()
                                } else {
                                    self.screen_size
                                };

                                if position.x < 0.0 {
                                    position.x = 0.0;
                                }
                                if position.x + size.x > parent_size.x {
                                    position.x -= (position.x + size.x) - parent_size.x;
                                }
                                if position.y < 0.0 {
                                    position.y = 0.0;
                                }
                                if position.y + size.y > parent_size.y {
                                    position.y -= (position.y + size.y) - parent_size.y;
                                }

                                self.send_message(WidgetMessage::desired_position(
                                    message.destination(),
                                    MessageDirection::ToWidget,
                                    position,
                                ));
                            }
                        }
                        WidgetMessage::Align {
                            relative_to,
                            horizontal_alignment,
                            vertical_alignment,
                            margin,
                        } => {
                            if let (Some(node), Some(relative_node)) = (
                                self.try_get(message.destination()),
                                self.try_get(*relative_to),
                            ) {
                                // Calculate new anchor point in screen coordinate system.
                                let relative_node_screen_size = relative_node.screen_bounds().size;
                                let relative_node_screen_position = relative_node.screen_position();
                                let node_screen_size = node.screen_bounds().size;

                                let mut screen_anchor_point = Vector2::default();
                                match horizontal_alignment {
                                    HorizontalAlignment::Stretch => {
                                        // Do nothing.
                                    }
                                    HorizontalAlignment::Left => {
                                        screen_anchor_point.x =
                                            relative_node_screen_position.x + margin.left;
                                    }
                                    HorizontalAlignment::Center => {
                                        screen_anchor_point.x = relative_node_screen_position.x
                                            + (relative_node_screen_size.x
                                                + node_screen_size.x
                                                + margin.left
                                                + margin.right)
                                                * 0.5;
                                    }
                                    HorizontalAlignment::Right => {
                                        screen_anchor_point.x = relative_node_screen_position.x
                                            + relative_node_screen_size.x
                                            - node_screen_size.x
                                            - margin.right;
                                    }
                                }

                                match vertical_alignment {
                                    VerticalAlignment::Stretch => {
                                        // Do nothing.
                                    }
                                    VerticalAlignment::Top => {
                                        screen_anchor_point.y =
                                            relative_node_screen_position.y + margin.top;
                                    }
                                    VerticalAlignment::Center => {
                                        screen_anchor_point.y = relative_node_screen_position.y
                                            + (relative_node_screen_size.y
                                                + node_screen_size.y
                                                + margin.top
                                                + margin.bottom)
                                                * 0.5;
                                    }
                                    VerticalAlignment::Bottom => {
                                        screen_anchor_point.y = relative_node_screen_position.y
                                            + (relative_node_screen_size.y
                                                - node_screen_size.y
                                                - margin.bottom);
                                    }
                                }

                                if let Some(parent) = self.try_get(node.parent()) {
                                    // Transform screen anchor point into the local coordinate system
                                    // of the parent node.
                                    let local_anchor_point =
                                        parent.screen_to_local(screen_anchor_point);
                                    self.send_message(WidgetMessage::desired_position(
                                        message.destination(),
                                        MessageDirection::ToWidget,
                                        local_anchor_point,
                                    ));
                                }
                            }
                        }
                        WidgetMessage::MouseDown { button, .. } => {
                            if *button == MouseButton::Right {
                                if let Some(picked) = self.nodes.try_borrow(self.picked_node) {
                                    // Get the context menu from the current node or a parent node
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
                                            (None, Handle::NONE)
                                        }
                                    };

                                    // Display context menu
                                    if let Some(context_menu) = context_menu {
                                        self.send_message(PopupMessage::placement(
                                            context_menu.handle(),
                                            MessageDirection::ToWidget,
                                            Placement::Cursor(target),
                                        ));
                                        self.send_message(PopupMessage::open(
                                            context_menu.handle(),
                                            MessageDirection::ToWidget,
                                        ));
                                        // Send Event messages to the widget that was clicked on,
                                        // not to the widget that has the context menu.
                                        self.send_message(PopupMessage::owner(
                                            context_menu.handle(),
                                            MessageDirection::ToWidget,
                                            self.picked_node,
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

    pub fn screen_to_root_canvas_space(&self, position: Vector2<f32>) -> Vector2<f32> {
        self.node(self.root()).screen_to_local(position)
    }

    fn show_tooltip(&self, tooltip: RcUiNodeHandle) {
        self.send_message(WidgetMessage::visibility(
            tooltip.handle(),
            MessageDirection::ToWidget,
            true,
        ));
        self.send_message(WidgetMessage::topmost(
            tooltip.handle(),
            MessageDirection::ToWidget,
        ));
        self.send_message(WidgetMessage::desired_position(
            tooltip.handle(),
            MessageDirection::ToWidget,
            self.screen_to_root_canvas_space(self.cursor_position() + Vector2::new(0.0, 16.0)),
        ));
        self.send_message(WidgetMessage::adjust_position_to_fit(
            tooltip.handle(),
            MessageDirection::ToWidget,
        ));
    }

    fn replace_or_update_tooltip(&mut self, tooltip: RcUiNodeHandle, time: f32) {
        if let Some(entry) = self.active_tooltip.as_mut() {
            if entry.tooltip == tooltip {
                // Keep current visible.
                entry.time = time;
            } else {
                let old_tooltip = entry.tooltip.clone();

                entry.tooltip = tooltip.clone();
                self.show_tooltip(tooltip);

                // Hide previous.
                self.send_message(WidgetMessage::visibility(
                    old_tooltip.handle(),
                    MessageDirection::ToWidget,
                    false,
                ));
            }
        } else {
            self.show_tooltip(tooltip.clone());
            self.active_tooltip = Some(TooltipEntry::new(tooltip, time));
        }
    }

    /// Find any tooltips that are being hovered and activate them.
    /// As well, update their time.
    fn update_tooltips(&mut self, dt: f32) {
        let sender = &self.sender;
        if let Some(entry) = self.active_tooltip.as_mut() {
            entry.decrease(dt);
            if !entry.should_display() {
                // This uses sender directly since we're currently mutably borrowing
                // visible_tooltips
                sender
                    .send(WidgetMessage::visibility(
                        entry.tooltip.handle(),
                        MessageDirection::ToWidget,
                        false,
                    ))
                    .unwrap();

                self.active_tooltip = None;
            }
        }

        // Check for hovering over a widget with a tooltip, or hovering over a tooltip.
        let mut handle = self.picked_node;
        while let Some(node) = self.nodes.try_borrow(handle) {
            // Get the parent to avoid the problem with having a immutable access here and a
            // mutable access later
            let parent = node.parent();

            if let Some(tooltip) = node.tooltip() {
                // They have a tooltip, we stop here and use that.
                let tooltip_time = node.tooltip_time();
                self.replace_or_update_tooltip(tooltip, tooltip_time);
                break;
            } else if let Some(entry) = self.active_tooltip.as_mut() {
                if entry.tooltip.handle() == handle {
                    // The current node was a tooltip.
                    // We refresh the timer back to the stored max time.
                    entry.time = entry.max_time;
                    break;
                }
            }

            handle = parent;
        }
    }

    pub fn captured_node(&self) -> Handle<UiNode> {
        self.captured_node
    }

    // Tries to set new picked node (a node under the cursor) and returns `true` if the node was
    // changed.
    fn try_set_picked_node(&mut self, node: Handle<UiNode>) -> bool {
        if self.picked_node != node {
            self.picked_node = node;
            self.reset_double_click_entries();
            true
        } else {
            false
        }
    }

    fn reset_double_click_entries(&mut self) {
        for entry in self.double_click_entries.values_mut() {
            entry.timer = self.double_click_time_slice;
            entry.click_count = 0;
        }
    }

    fn request_focus(&mut self, new_focused: Handle<UiNode>) {
        if self.keyboard_focus_node != new_focused {
            if self.keyboard_focus_node.is_some() {
                self.send_message(WidgetMessage::unfocus(
                    self.keyboard_focus_node,
                    MessageDirection::FromWidget,
                ));
            }

            self.keyboard_focus_node = new_focused;

            if self.keyboard_focus_node.is_some() {
                self.send_message(WidgetMessage::focus(
                    self.keyboard_focus_node,
                    MessageDirection::FromWidget,
                ));
            }
        }
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
                        let picked_changed =
                            self.try_set_picked_node(self.hit_test(self.cursor_position));

                        let mut emit_double_click = false;
                        if !picked_changed {
                            match self.double_click_entries.entry(button) {
                                Entry::Occupied(e) => {
                                    let entry = e.into_mut();
                                    if entry.timer > 0.0 {
                                        entry.click_count += 1;
                                        if entry.click_count >= 2 {
                                            entry.click_count = 0;
                                            entry.timer = self.double_click_time_slice;
                                            emit_double_click = true;
                                        }
                                    } else {
                                        entry.timer = self.double_click_time_slice;
                                        entry.click_count = 1;
                                    }
                                }
                                Entry::Vacant(entry) => {
                                    // A button was clicked for the first time, no double click
                                    // in this case.
                                    entry.insert(DoubleClickEntry {
                                        timer: self.double_click_time_slice,
                                        click_count: 1,
                                    });
                                }
                            }
                        }

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

                        self.request_focus(self.picked_node);

                        if self.picked_node.is_some() {
                            self.send_message(WidgetMessage::mouse_down(
                                self.picked_node,
                                MessageDirection::FromWidget,
                                self.cursor_position,
                                button,
                            ));
                            event_processed = true;
                        }

                        // Make sure double click will be emitted after mouse down event.
                        if emit_double_click {
                            self.send_message(WidgetMessage::double_click(
                                self.picked_node,
                                MessageDirection::FromWidget,
                                button,
                            ));
                        }
                    }
                    ButtonState::Released => {
                        if self.picked_node.is_some() {
                            self.send_message(WidgetMessage::mouse_up(
                                self.picked_node,
                                MessageDirection::FromWidget,
                                self.cursor_position,
                                button,
                            ));

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
                            if self.nodes.is_valid_handle(self.drag_context.drag_preview) {
                                self.remove_node(self.drag_context.drag_preview);
                                self.drag_context.drag_preview = Default::default();
                            }

                            event_processed = true;
                        }
                    }
                }
            }
            OsEvent::CursorMoved { position } => {
                self.cursor_position = *position;
                self.try_set_picked_node(self.hit_test(self.cursor_position));

                if !self.drag_context.is_dragging
                    && self.mouse_state.left == ButtonState::Pressed
                    && self.picked_node.is_some()
                    && self.drag_context.drag_node.is_some()
                    && (self.drag_context.click_pos - *position).norm() > 5.0
                {
                    self.drag_context.drag_preview =
                        self.copy_node_with_limit(self.drag_context.drag_node, Some(30));
                    self.nodes[self.drag_context.drag_preview].set_opacity(Some(0.5));

                    // Make preview nodes invisible for hit test.
                    let mut stack = vec![self.drag_context.drag_preview];
                    while let Some(handle) = stack.pop() {
                        let preview_node = &mut self.nodes[handle];
                        preview_node.hit_test_visibility.set_value_silent(false);
                        stack.extend_from_slice(preview_node.children());
                    }

                    self.drag_context.is_dragging = true;

                    self.send_message(WidgetMessage::drag_started(
                        self.picked_node,
                        MessageDirection::FromWidget,
                        self.drag_context.drag_node,
                    ));

                    self.cursor_icon = CursorIcon::Crosshair;
                }

                if self.drag_context.is_dragging
                    && self.nodes.is_valid_handle(self.drag_context.drag_preview)
                {
                    self.send_message(WidgetMessage::desired_position(
                        self.drag_context.drag_preview,
                        MessageDirection::ToWidget,
                        *position,
                    ));
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
            OsEvent::KeyboardInput {
                button,
                state,
                text,
            } => {
                if let Some(keyboard_focus_node) = self.try_get(self.keyboard_focus_node) {
                    if keyboard_focus_node.is_globally_visible() {
                        match state {
                            ButtonState::Pressed => {
                                self.send_message(WidgetMessage::key_down(
                                    self.keyboard_focus_node,
                                    MessageDirection::FromWidget,
                                    *button,
                                ));

                                if !text.is_empty() {
                                    self.send_message(WidgetMessage::text(
                                        self.keyboard_focus_node,
                                        MessageDirection::FromWidget,
                                        text.clone(),
                                    ));
                                }
                            }
                            ButtonState::Released => self.send_message(WidgetMessage::key_up(
                                self.keyboard_focus_node,
                                MessageDirection::FromWidget,
                                *button,
                            )),
                        }

                        event_processed = true;
                    }
                }
            }
            &OsEvent::KeyboardModifiers(modifiers) => {
                // TODO: Is message needed for focused node?
                self.keyboard_modifiers = modifiers;
            }
            OsEvent::Touch {
                phase,
                location,
                force,
                id,
            } => match phase {
                TouchPhase::Started => {
                    self.cursor_position = *location;
                    let picked_changed =
                        self.try_set_picked_node(self.hit_test(self.cursor_position));

                    let mut emit_double_tap = false;
                    if !picked_changed {
                        match self.double_click_entries.entry(MouseButton::Left) {
                            Entry::Occupied(e) => {
                                let entry = e.into_mut();
                                if entry.timer > 0.0 {
                                    entry.click_count += 1;
                                    if entry.click_count >= 2 {
                                        entry.click_count = 0;
                                        entry.timer = self.double_click_time_slice;
                                        emit_double_tap = true;
                                    }
                                } else {
                                    entry.timer = self.double_click_time_slice;
                                    entry.click_count = 1;
                                }
                            }
                            Entry::Vacant(entry) => {
                                // A button was clicked for the first time, no double click
                                // in this case.
                                entry.insert(DoubleClickEntry {
                                    timer: self.double_click_time_slice,
                                    click_count: 1,
                                });
                            }
                        }
                    }

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

                    self.request_focus(self.picked_node);

                    if self.picked_node.is_some() {
                        self.send_message(WidgetMessage::touch_started(
                            self.picked_node,
                            MessageDirection::FromWidget,
                            self.cursor_position,
                            *force,
                            *id,
                        ));
                        event_processed = true;
                    }

                    // Make sure double click will be emitted after mouse down event.
                    if emit_double_tap {
                        self.send_message(WidgetMessage::double_tap(
                            self.picked_node,
                            MessageDirection::FromWidget,
                            *location,
                            *force,
                            *id,
                        ));
                    }
                }
                TouchPhase::Moved => {
                    self.cursor_position = *location;
                    self.try_set_picked_node(self.hit_test(self.cursor_position));

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

                    self.request_focus(self.picked_node);

                    if self.picked_node.is_some() {
                        self.send_message(WidgetMessage::touch_moved(
                            self.picked_node,
                            MessageDirection::FromWidget,
                            self.cursor_position,
                            *force,
                            *id,
                        ));
                        event_processed = true;
                    }
                }
                TouchPhase::Ended => {
                    if self.picked_node.is_some() {
                        self.send_message(WidgetMessage::touch_ended(
                            self.picked_node,
                            MessageDirection::FromWidget,
                            self.cursor_position,
                            *id,
                        ));

                        if self.drag_context.is_dragging {
                            self.drag_context.is_dragging = false;

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
                        if self.nodes.is_valid_handle(self.drag_context.drag_preview) {
                            self.remove_node(self.drag_context.drag_preview);
                            self.drag_context.drag_preview = Default::default();
                        }

                        event_processed = true;
                    }
                }
                TouchPhase::Cancelled => {
                    if self.picked_node.is_some() {
                        self.send_message(WidgetMessage::touch_cancelled(
                            self.picked_node,
                            MessageDirection::FromWidget,
                            self.cursor_position,
                            *id,
                        ));

                        if self.drag_context.is_dragging {
                            self.drag_context.is_dragging = false;
                            self.cursor_icon = CursorIcon::Default;
                            self.stack.clear();
                        }
                        self.drag_context.drag_node = Handle::NONE;
                        if self.nodes.is_valid_handle(self.drag_context.drag_preview) {
                            self.remove_node(self.drag_context.drag_preview);
                            self.drag_context.drag_preview = Default::default();
                        }

                        event_processed = true;
                    }
                }
            },
        }

        self.prev_picked_node = self.picked_node;

        let on_os_event_subs = std::mem::take(&mut self.methods_registry.handle_os_event);

        for &handle in on_os_event_subs.iter() {
            let (ticket, mut node) = self.nodes.take_reserve(handle);
            node.handle_os_event(handle, self, event);
            self.nodes.put_back(ticket, node);
        }

        self.methods_registry.handle_os_event = on_os_event_subs;

        event_processed
    }

    pub fn nodes(&self) -> &Pool<UiNode, WidgetContainer> {
        &self.nodes
    }

    pub fn root(&self) -> Handle<UiNode> {
        self.root_canvas
    }

    /// Extracts a widget from the user interface and reserves its handle. It is used to temporarily take
    /// ownership over the widget, and then put the widget back using the returned ticket. Extracted
    /// widget is detached from its parent!
    #[inline]
    pub fn take_reserve(&mut self, handle: Handle<UiNode>) -> (Ticket<UiNode>, UiNode) {
        self.isolate_node(handle);
        self.nodes.take_reserve(handle)
    }

    /// Puts the widget back by the given ticket. Attaches it back to the root canvas of the user interface.
    #[inline]
    pub fn put_back(&mut self, ticket: Ticket<UiNode>, node: UiNode) -> Handle<UiNode> {
        let handle = self.nodes.put_back(ticket, node);
        self.link_nodes(handle, self.root_canvas, false);
        handle
    }

    /// Makes a widget handle vacant again.
    #[inline]
    pub fn forget_ticket(&mut self, ticket: Ticket<UiNode>, node: UiNode) -> UiNode {
        self.nodes.forget_ticket(ticket);
        node
    }

    /// Extracts sub-graph starting from the given widget. All handles to extracted widgets
    /// becomes reserved and will be marked as "occupied", an attempt to borrow a widget
    /// at such handle will result in panic!. Please note that root widget will be
    /// detached from its parent!
    #[inline]
    pub fn take_reserve_sub_graph(&mut self, root: Handle<UiNode>) -> SubGraph {
        // Take out descendants first.
        let mut descendants = Vec::new();
        let root_ref = &mut self.nodes[root];
        let mut stack = root_ref.children().to_vec();
        let parent = root_ref.parent;
        while let Some(handle) = stack.pop() {
            stack.extend_from_slice(self.nodes[handle].children());
            descendants.push(self.nodes.take_reserve(handle));
        }

        SubGraph {
            // Root must be extracted with detachment from its parent (if any).
            root: self.take_reserve(root),
            descendants,
            parent,
        }
    }

    /// Puts previously extracted sub-graph into the user interface. Handles to widgets will become valid
    /// again. After that you probably want to re-link returned handle with its previous parent.
    #[inline]
    pub fn put_sub_graph_back(&mut self, sub_graph: SubGraph) -> Handle<UiNode> {
        for (ticket, node) in sub_graph.descendants {
            self.nodes.put_back(ticket, node);
        }

        let (ticket, node) = sub_graph.root;
        let root_handle = self.put_back(ticket, node);

        self.link_nodes(root_handle, sub_graph.parent, false);

        root_handle
    }

    /// Forgets the entire sub-graph making handles to widgets invalid.
    #[inline]
    pub fn forget_sub_graph(&mut self, sub_graph: SubGraph) {
        for (ticket, _) in sub_graph.descendants {
            self.nodes.forget_ticket(ticket);
        }
        let (ticket, _) = sub_graph.root;
        self.nodes.forget_ticket(ticket);
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

    pub fn drag_context(&self) -> &DragContext {
        &self.drag_context
    }

    /// Links the specified child widget with the specified parent widget.
    #[inline]
    pub fn link_nodes(
        &mut self,
        child_handle: Handle<UiNode>,
        parent_handle: Handle<UiNode>,
        in_front: bool,
    ) {
        assert_ne!(child_handle, parent_handle);
        self.isolate_node(child_handle);
        self.nodes[child_handle].set_parent(parent_handle);
        self.nodes[parent_handle].add_child(child_handle, in_front);

        // Sort by Z index. This uses stable sort, so every child node with the same z index will
        // remain on its position.
        let mbc = self.nodes.begin_multi_borrow();
        if let Ok(mut parent) = mbc.try_get_mut(parent_handle) {
            parent
                .children
                .sort_by_key(|handle| mbc.try_get(*handle).map(|c| *c.z_index).unwrap_or_default());
        };
    }

    #[inline]
    pub fn node_mut(&mut self, node_handle: Handle<UiNode>) -> &mut UiNode {
        self.nodes.borrow_mut(node_handle)
    }

    #[inline]
    pub fn try_get_node_mut(&mut self, node_handle: Handle<UiNode>) -> Option<&mut UiNode> {
        self.nodes.try_borrow_mut(node_handle)
    }

    pub fn copy_node(&mut self, node: Handle<UiNode>) -> Handle<UiNode> {
        let mut old_new_mapping = NodeHandleMap::default();

        let root = self.copy_node_recursive(node, &mut old_new_mapping);

        remap_handles(&old_new_mapping, self);

        root
    }

    #[allow(clippy::unnecessary_to_owned)] // False positive
    fn copy_node_recursive(
        &mut self,
        node_handle: Handle<UiNode>,
        old_new_mapping: &mut NodeHandleMap<UiNode>,
    ) -> Handle<UiNode> {
        let node = self.nodes.borrow(node_handle);
        let mut cloned = UiNode(node.clone_boxed());
        cloned.id = Uuid::new_v4();

        let mut cloned_children = Vec::new();
        for child in node.children().to_vec() {
            cloned_children.push(self.copy_node_recursive(child, old_new_mapping));
        }

        cloned.set_children(cloned_children);
        let copy_handle = self.add_node(cloned);
        old_new_mapping.insert(node_handle, copy_handle);
        copy_handle
    }

    pub fn copy_node_to<Post>(
        &self,
        node: Handle<UiNode>,
        dest: &mut UserInterface,
        post_process_callback: &mut Post,
    ) -> (Handle<UiNode>, NodeHandleMap<UiNode>)
    where
        Post: FnMut(Handle<UiNode>, Handle<UiNode>, &mut UiNode),
    {
        let mut old_new_mapping = NodeHandleMap::default();

        let root =
            self.copy_node_to_recursive(node, dest, &mut old_new_mapping, post_process_callback);

        remap_handles(&old_new_mapping, dest);

        (root, old_new_mapping)
    }

    fn copy_node_to_recursive<Post>(
        &self,
        node_handle: Handle<UiNode>,
        dest: &mut UserInterface,
        old_new_mapping: &mut NodeHandleMap<UiNode>,
        post_process_callback: &mut Post,
    ) -> Handle<UiNode>
    where
        Post: FnMut(Handle<UiNode>, Handle<UiNode>, &mut UiNode),
    {
        let node = self.nodes.borrow(node_handle);
        let children = node.children.clone();

        let mut cloned = UiNode(node.clone_boxed());
        cloned.children.clear();
        cloned.parent = Handle::NONE;
        cloned.id = Uuid::new_v4();
        let cloned_node_handle = dest.add_node(cloned);

        for child in children {
            let cloned_child_node_handle =
                self.copy_node_to_recursive(child, dest, old_new_mapping, post_process_callback);
            dest.link_nodes(cloned_child_node_handle, cloned_node_handle, false);
        }

        old_new_mapping.insert(node_handle, cloned_node_handle);

        post_process_callback(
            cloned_node_handle,
            node_handle,
            dest.try_get_node_mut(cloned_node_handle).unwrap(),
        );

        cloned_node_handle
    }

    pub fn copy_node_with_limit(
        &mut self,
        node: Handle<UiNode>,
        limit: Option<usize>,
    ) -> Handle<UiNode> {
        let mut old_new_mapping = NodeHandleMap::default();
        let mut counter = 0;

        let root =
            self.copy_node_recursive_with_limit(node, &mut old_new_mapping, limit, &mut counter);

        remap_handles(&old_new_mapping, self);

        root
    }

    #[allow(clippy::unnecessary_to_owned)] // False positive
    fn copy_node_recursive_with_limit(
        &mut self,
        node_handle: Handle<UiNode>,
        old_new_mapping: &mut NodeHandleMap<UiNode>,
        limit: Option<usize>,
        counter: &mut usize,
    ) -> Handle<UiNode> {
        if let Some(limit) = limit {
            if *counter >= limit {
                return Default::default();
            }
        }

        let node = self.nodes.borrow(node_handle);
        let mut cloned = UiNode(node.clone_boxed());
        cloned.id = Uuid::new_v4();

        let mut cloned_children = Vec::new();
        for child in node.children().to_vec() {
            let cloned_child =
                self.copy_node_recursive_with_limit(child, old_new_mapping, limit, counter);
            if cloned_child.is_some() {
                cloned_children.push(cloned_child);
            } else {
                break;
            }
        }

        cloned.set_children(cloned_children);
        let copy_handle = self.add_node(cloned);
        old_new_mapping.insert(node_handle, copy_handle);

        *counter += 1;

        copy_handle
    }

    pub fn save(&mut self, path: &Path) -> Result<Visitor, VisitError> {
        let mut visitor = Visitor::new();
        self.visit("Ui", &mut visitor)?;
        visitor.save_binary(path)?;
        Ok(visitor)
    }

    #[allow(clippy::arc_with_non_send_sync)]
    pub async fn load_from_file<P: AsRef<Path>>(
        path: P,
        resource_manager: ResourceManager,
    ) -> Result<Self, VisitError> {
        Self::load_from_file_ex(
            path,
            Arc::new(WidgetConstructorContainer::new()),
            resource_manager,
            &FsResourceIo,
        )
        .await
    }

    fn restore_dynamic_node_data(&mut self) {
        for (handle, widget) in self.nodes.pair_iter_mut() {
            widget.handle = handle;
            widget.layout_events_sender = Some(self.layout_events_sender.clone());
            widget.invalidate_layout();
        }
    }

    pub fn resolve(&mut self) {
        self.restore_dynamic_node_data();
        self.restore_original_handles_and_inherit_properties(&[], |_, _| {});
        self.update_visual_transform(self.root_canvas);
        self.update_global_visibility(self.root_canvas);
        let instances = self.restore_integrity(|model, model_data, handle, dest_graph| {
            model_data.copy_node_to(handle, dest_graph, &mut |_, original_handle, node| {
                node.set_inheritance_data(original_handle, model.clone());
            })
        });
        self.remap_handles(&instances);
    }

    /// Collects all resources used by the user interface. It uses reflection to "scan" the contents
    /// of the user interface, so if some fields marked with `#[reflect(hidden)]` attribute, then
    /// such field will be ignored!
    pub fn collect_used_resources(&self) -> FxHashSet<UntypedResource> {
        let mut collection = FxHashSet::default();
        fyrox_resource::collect_used_resources(self, &mut collection);
        collection
    }

    #[allow(clippy::arc_with_non_send_sync)]
    pub async fn load_from_file_ex<P: AsRef<Path>>(
        path: P,
        constructors: Arc<WidgetConstructorContainer>,
        resource_manager: ResourceManager,
        io: &dyn ResourceIo,
    ) -> Result<Self, VisitError> {
        let mut ui = {
            let mut visitor = Visitor::load_from_memory(&io.load_file(path.as_ref()).await?)?;
            let (sender, receiver) = mpsc::channel();
            visitor.blackboard.register(constructors);
            visitor.blackboard.register(Arc::new(sender.clone()));
            visitor.blackboard.register(Arc::new(resource_manager));
            let mut ui =
                UserInterface::new_with_channel(sender, receiver, Vector2::new(100.0, 100.0));
            ui.visit("Ui", &mut visitor)?;
            ui
        };

        Log::info("UserInterface - Collecting resources used by the scene...");

        let used_resources = ui.collect_used_resources();

        let used_resources_count = used_resources.len();

        Log::info(format!(
            "UserInterface - {} resources collected. Waiting them to load...",
            used_resources_count
        ));

        // Wait everything.
        join_all(used_resources.into_iter()).await;

        ui.resolve();

        Ok(ui)
    }
}

impl PrefabData for UserInterface {
    type Graph = Self;

    #[inline]
    fn graph(&self) -> &Self::Graph {
        self
    }

    #[inline]
    fn mapping(&self) -> NodeMapping {
        NodeMapping::UseHandles
    }
}

impl AbstractSceneGraph for UserInterface {
    fn try_get_node_untyped(&self, handle: ErasedHandle) -> Option<&dyn AbstractSceneNode> {
        self.nodes
            .try_borrow(handle.into())
            .map(|n| n as &dyn AbstractSceneNode)
    }

    fn try_get_node_untyped_mut(
        &mut self,
        handle: ErasedHandle,
    ) -> Option<&mut dyn AbstractSceneNode> {
        self.nodes
            .try_borrow_mut(handle.into())
            .map(|n| n as &mut dyn AbstractSceneNode)
    }
}

impl BaseSceneGraph for UserInterface {
    type Prefab = Self;
    type Node = UiNode;

    #[inline]
    fn root(&self) -> Handle<Self::Node> {
        self.root_canvas
    }

    #[inline]
    fn set_root(&mut self, root: Handle<Self::Node>) {
        self.root_canvas = root;
    }

    #[inline]
    fn try_get(&self, handle: Handle<Self::Node>) -> Option<&Self::Node> {
        self.nodes.try_borrow(handle)
    }

    #[inline]
    fn try_get_mut(&mut self, handle: Handle<Self::Node>) -> Option<&mut Self::Node> {
        self.nodes.try_borrow_mut(handle)
    }

    #[inline]
    fn is_valid_handle(&self, handle: Handle<Self::Node>) -> bool {
        self.nodes.is_valid_handle(handle)
    }

    #[inline]
    fn add_node(&mut self, mut node: Self::Node) -> Handle<Self::Node> {
        let children = node.children().to_vec();
        node.clear_children();
        let node_handle = self.nodes.spawn(node);
        if self.root_canvas.is_some() {
            self.link_nodes(node_handle, self.root_canvas, false);
        }
        for child in children {
            self.link_nodes(child, node_handle, false)
        }
        let node = self.nodes[node_handle].deref_mut();
        node.layout_events_sender = Some(self.layout_events_sender.clone());
        node.handle = node_handle;
        self.methods_registry.register(node);
        node.invalidate_layout();
        self.layout_events_sender
            .send(LayoutEvent::VisibilityChanged(node_handle))
            .unwrap();
        node_handle
    }

    #[inline]
    fn remove_node(&mut self, node: Handle<Self::Node>) {
        self.isolate_node(node);

        let sender = self.sender.clone();
        let mut stack = vec![node];
        while let Some(handle) = stack.pop() {
            if self.prev_picked_node == handle {
                self.prev_picked_node = Handle::NONE;
            }
            if self.picked_node == handle {
                self.try_set_picked_node(Handle::NONE);
            }
            if self.captured_node == handle {
                self.captured_node = Handle::NONE;
            }
            if self.keyboard_focus_node == handle {
                self.keyboard_focus_node = Handle::NONE;
            }
            self.remove_picking_restriction(handle);

            let node_ref = self.nodes.borrow(handle);
            stack.extend_from_slice(node_ref.children());

            // Notify node that it is about to be deleted so it will have a chance to remove
            // other widgets (like popups).
            node_ref.on_remove(&sender);

            self.methods_registry.unregister(node_ref.deref());
            self.nodes.free(handle);
        }
    }

    #[inline]
    fn link_nodes(&mut self, child: Handle<Self::Node>, parent: Handle<Self::Node>) {
        self.link_nodes(child, parent, false)
    }

    #[inline]
    fn unlink_node(&mut self, node_handle: Handle<Self::Node>) {
        self.isolate_node(node_handle);
        self.link_nodes(node_handle, self.root_canvas, false);
    }

    #[inline]
    fn isolate_node(&mut self, node_handle: Handle<Self::Node>) {
        let node = self.nodes.borrow_mut(node_handle);
        let parent_handle = node.parent();
        if parent_handle.is_some() {
            node.set_parent(Handle::NONE);

            // Remove child from parent's children list
            self.nodes[parent_handle].remove_child(node_handle);
        }
    }
}

impl SceneGraph for UserInterface {
    #[inline]
    fn pair_iter(&self) -> impl Iterator<Item = (Handle<Self::Node>, &Self::Node)> {
        self.nodes.pair_iter()
    }

    #[inline]
    fn linear_iter(&self) -> impl Iterator<Item = &Self::Node> {
        self.nodes.iter()
    }

    #[inline]
    fn linear_iter_mut(&mut self) -> impl Iterator<Item = &mut Self::Node> {
        self.nodes.iter_mut()
    }
}

pub trait UserInterfaceResourceExtension {
    fn instantiate(&self, ui: &mut UserInterface) -> (Handle<UiNode>, NodeHandleMap<UiNode>);
}

impl UserInterfaceResourceExtension for Resource<UserInterface> {
    fn instantiate(&self, ui: &mut UserInterface) -> (Handle<UiNode>, NodeHandleMap<UiNode>) {
        let resource = self.clone();
        let mut data = self.state();
        let data = data.data().expect("The resource must be loaded!");

        let (root, mapping) =
            data.copy_node_to(data.root_canvas, ui, &mut |_, original_handle, node| {
                node.set_inheritance_data(original_handle, resource.clone());
            });

        // Explicitly mark as root node.
        ui.node_mut(root).is_resource_instance_root = true;

        (root, mapping)
    }
}

fn is_approx_zero(v: f32) -> bool {
    v.abs() <= 10.0 * f32::EPSILON
}

fn are_close(value1: f32, value2: f32) -> bool {
    //in case they are Infinities (then epsilon check does not work)
    if value1 == value2 {
        return true;
    }
    // This computes (|value1-value2| / (|value1| + |value2| + 10.0)) < DBL_EPSILON
    let eps = (value1.abs() + value2.abs() + 10.0) * f32::EPSILON;
    let delta = value1 - value2;
    (-eps < delta) && (eps > delta)
}

fn greater_than_or_close(value1: f32, value2: f32) -> bool {
    (value1 > value2) || are_close(value1, value2)
}

fn less_than_or_close(value1: f32, value2: f32) -> bool {
    (value1 < value2) || are_close(value1, value2)
}

/// Calculates a new size for the rect after transforming it with the given matrix. Basically it
/// finds a new rectangle that can contain the rotated rectangle.
///
/// # Origin
///
/// Original code was taken from WPF source code (FindMaximalAreaLocalSpaceRect) and ported to Rust.
/// It handles a lot of edge cases that could occur due to the fact that the UI uses a lot of
/// special floating-point constants like Infinity or NaN. If there would be no such values, simple
/// `rect.transform(&matrix).size` could be used.
fn transform_size(transform_space_bounds: Vector2<f32>, matrix: &Matrix3<f32>) -> Vector2<f32> {
    // X (width) and Y (height) constraints for axis-aligned bounding box in dest. space
    let mut x_constr: f32 = transform_space_bounds.x;
    let mut y_constr: f32 = transform_space_bounds.y;

    //if either of the sizes is 0, return 0,0 to avoid doing math on an empty rect (bug 963569)
    if is_approx_zero(x_constr) || is_approx_zero(y_constr) {
        return Vector2::new(0.0, 0.0);
    }

    let x_constr_infinite = x_constr.is_infinite();
    let y_constr_infinite = y_constr.is_infinite();

    if x_constr_infinite && y_constr_infinite {
        return Vector2::new(f32::INFINITY, f32::INFINITY);
    } else if x_constr_infinite
    //assume square for one-dimensional constraint
    {
        x_constr = y_constr;
    } else if y_constr_infinite {
        y_constr = x_constr;
    }

    // We only deal with nonsingular matrices here. The nonsingular matrix is the one
    // that has inverse (determinant != 0).
    if !matrix.is_invertible() {
        return Vector2::new(0.0, 0.0);
    }

    let a = matrix[(0, 0)];
    let b = matrix[(0, 1)];
    let c = matrix[(1, 0)];
    let d = matrix[(1, 1)];

    // Result width and height (in child/local space)
    let mut w;
    let mut h;

    // because we are dealing with nonsingular transform matrices,
    // we have (b==0 || c==0) XOR (a==0 || d==0)

    if is_approx_zero(b) || is_approx_zero(c) {
        // (b==0 || c==0) ==> a!=0 && d!=0

        let y_cover_d = if y_constr_infinite {
            f32::INFINITY
        } else {
            (y_constr / d).abs()
        };
        let x_cover_a = if x_constr_infinite {
            f32::INFINITY
        } else {
            (x_constr / a).abs()
        };

        if is_approx_zero(b) {
            if is_approx_zero(c) {
                // Case: b=0, c=0, a!=0, d!=0

                // No constraint relation; use maximal width and height

                h = y_cover_d;
                w = x_cover_a;
            } else {
                // Case: b==0, a!=0, c!=0, d!=0

                // Maximizing under line (hIntercept=xConstr/c, wIntercept=xConstr/a)
                // BUT we still have constraint: h <= yConstr/d

                h = (0.5 * (x_constr / c).abs()).min(y_cover_d);
                w = x_cover_a - ((c * h) / a);
            }
        } else {
            // Case: c==0, a!=0, b!=0, d!=0

            // Maximizing under line (hIntercept=yConstr/d, wIntercept=yConstr/b)
            // BUT we still have constraint: w <= xConstr/a

            w = (0.5 * (y_constr / b).abs()).min(x_cover_a);
            h = y_cover_d - ((b * w) / d);
        }
    } else if is_approx_zero(a) || is_approx_zero(d) {
        // (a==0 || d==0) ==> b!=0 && c!=0

        let y_cover_b = (y_constr / b).abs();
        let x_cover_c = (x_constr / c).abs();

        if is_approx_zero(a) {
            if is_approx_zero(d) {
                // Case: a=0, d=0, b!=0, c!=0

                // No constraint relation; use maximal width and height

                h = x_cover_c;
                w = y_cover_b;
            } else {
                // Case: a==0, b!=0, c!=0, d!=0

                // Maximizing under line (hIntercept=yConstr/d, wIntercept=yConstr/b)
                // BUT we still have constraint: h <= xConstr/c

                h = (0.5 * (y_constr / d).abs()).min(x_cover_c);
                w = y_cover_b - ((d * h) / b);
            }
        } else {
            // Case: d==0, a!=0, b!=0, c!=0

            // Maximizing under line (hIntercept=xConstr/c, wIntercept=xConstr/a)
            // BUT we still have constraint: w <= yConstr/b

            w = (0.5 * (x_constr / a).abs()).min(y_cover_b);
            h = x_cover_c - ((a * w) / c);
        }
    } else {
        let x_cover_a = (x_constr / a).abs(); // w-intercept of x-constraint line.
        let x_cover_c = (x_constr / c).abs(); // h-intercept of x-constraint line.

        let y_cover_b = (y_constr / b).abs(); // w-intercept of y-constraint line.
        let y_cover_d = (y_constr / d).abs(); // h-intercept of y-constraint line.

        // The tighest constraint governs, so we pick the lowest constraint line.
        //
        //   The optimal point (w,h) for which Area = w*h is maximized occurs halfway
        //   to each intercept.

        w = y_cover_b.min(x_cover_a) * 0.5;
        h = x_cover_c.min(y_cover_d) * 0.5;

        if (greater_than_or_close(x_cover_a, y_cover_b) && less_than_or_close(x_cover_c, y_cover_d))
            || (less_than_or_close(x_cover_a, y_cover_b)
                && greater_than_or_close(x_cover_c, y_cover_d))
        {
            // Constraint lines cross; since the most restrictive constraint wins,
            // we have to maximize under two line segments, which together are discontinuous.
            // Instead, we maximize w*h under the line segment from the two smallest endpoints.

            // Since we are not (except for in corner cases) on the original constraint lines,
            // we are not using up all the available area in transform space.  So scale our shape up
            // until it does in at least one dimension.

            let child_bounds_tr = Rect::new(0.0, 0.0, w, h).transform(matrix);
            let expand_factor =
                (x_constr / child_bounds_tr.size.x).min(y_constr / child_bounds_tr.size.y);

            if !expand_factor.is_nan() && !expand_factor.is_infinite() {
                w *= expand_factor;
                h *= expand_factor;
            }
        }
    }

    Vector2::new(w, h)
}

uuid_provider!(UserInterface = "0d065c93-ef9c-4dd2-9fe7-e2b33c1a21b6");

impl ResourceData for UserInterface {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn type_uuid(&self) -> Uuid {
        <Self as TypeUuidProvider>::type_uuid()
    }

    fn save(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        self.save(path)?;
        Ok(())
    }

    fn can_be_saved(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod test {
    use crate::message::{ButtonState, KeyCode};
    use crate::{
        border::BorderBuilder,
        core::algebra::{Rotation2, UnitComplex, Vector2},
        message::MessageDirection,
        text_box::TextBoxBuilder,
        transform_size,
        widget::{WidgetBuilder, WidgetMessage},
        OsEvent, UserInterface,
    };
    use fyrox_graph::BaseSceneGraph;

    #[test]
    fn test_transform_size() {
        let input = Vector2::new(100.0, 100.0);
        let transform =
            Rotation2::from(UnitComplex::from_angle(45.0f32.to_radians())).to_homogeneous();
        let transformed = transform_size(input, &transform);
        dbg!(input, transformed);
    }

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
        ui.update(screen_size, 0.0, &Default::default()); // Make sure layout was calculated.
        ui.send_message(WidgetMessage::center(widget, MessageDirection::ToWidget));
        while ui.poll_message().is_some() {}
        ui.update(screen_size, 0.0, &Default::default());
        let expected_position = (screen_size - widget_size).scale(0.5);
        let actual_position = ui.node(widget).actual_local_position();
        assert_eq!(actual_position, expected_position);
    }

    #[test]
    fn test_keyboard_focus() {
        let screen_size = Vector2::new(1000.0, 1000.0);
        let mut ui = UserInterface::new(screen_size);

        let text_box = TextBoxBuilder::new(WidgetBuilder::new()).build(&mut ui.build_ctx());

        // Make sure layout was calculated.
        ui.update(screen_size, 0.0, &Default::default());

        assert!(ui.poll_message().is_none());

        ui.send_message(WidgetMessage::focus(text_box, MessageDirection::ToWidget));

        // Ensure that the message has gotten in the queue.
        assert_eq!(
            ui.poll_message(),
            Some(WidgetMessage::focus(text_box, MessageDirection::ToWidget))
        );
        // Root must be unfocused right before new widget is focused.
        assert_eq!(
            ui.poll_message(),
            Some(WidgetMessage::unfocus(
                ui.root(),
                MessageDirection::FromWidget
            ))
        );
        // Finally there should be a response from newly focused node.
        assert_eq!(
            ui.poll_message(),
            Some(WidgetMessage::focus(text_box, MessageDirection::FromWidget))
        );

        // Do additional check - emulate key press of "A" and check if the focused text box has accepted it.
        ui.process_os_event(&OsEvent::KeyboardInput {
            button: KeyCode::KeyA,
            state: ButtonState::Pressed,
            text: "A".to_string(),
        });

        let msg = WidgetMessage::key_down(text_box, MessageDirection::FromWidget, KeyCode::KeyA);
        msg.set_handled(true);
        assert_eq!(ui.poll_message(), Some(msg));

        assert_eq!(
            ui.poll_message(),
            Some(WidgetMessage::text(
                text_box,
                MessageDirection::FromWidget,
                'A'.to_string()
            ))
        );

        assert!(ui.poll_message().is_none());
    }
}
