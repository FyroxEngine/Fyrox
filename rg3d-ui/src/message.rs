//! Message and events module contains all possible widget messages and OS events.
//!
//! This UI library uses message passing mechanism to communicate with widgets.
//! This is very simple and reliable mechanism that effectively decouples widgets
//! from each other. There is no direct way of modify something during runtime,
//! you have to use messages to change state of ui elements.
//!
//! # Direction
//!
//! Each message marked with "Direction" field, which means supported routes for
//! message. For example [ButtonMessage::Click](enum.ButtonMessage.html) has "Direction: To/From UI" which
//! means that it can be sent either from internals of library or from user code.
//! However [WidgetMessage::GotFocus](enum.WidgetMessage.html) has "Direction: From UI" which means that only
//! internal library code can send such messages without a risk of breaking anything.

use crate::{
    brush::Brush,
    core::{
        algebra::Vector2,
        color::{Color, Hsv},
        curve::{Curve, CurveKeyKind},
        inspect::PropertyValue,
        pool::Handle,
    },
    dock::{SplitDirection, TileContent},
    draw::SharedTexture,
    file_browser::Filter,
    formatted_text::WrapMode,
    inspector::InspectorContext,
    messagebox::MessageBoxResult,
    popup::Placement,
    ttf::SharedFont,
    window::WindowTitle,
    HorizontalAlignment, MouseState, Orientation, Thickness, UiNode, VerticalAlignment,
};
use std::{
    any::{Any, TypeId},
    cell::Cell,
    fmt::Debug,
    ops::{Deref, DerefMut},
    path::PathBuf,
    rc::Rc,
};

#[macro_export]
macro_rules! define_constructor {
    ($inner:ident : $inner_var:tt => fn $name:ident(), layout: $perform_layout:expr) => {
        pub fn $name(destination: Handle<UiNode>, direction: MessageDirection) -> UiMessage {
            UiMessage {
                handled: std::cell::Cell::new(false),
                data: std::rc::Rc::new($inner::$inner_var),
                destination,
                direction,
                perform_layout: std::cell::Cell::new($perform_layout),
                flags: 0
            }
        }
    };

    ($inner:ident : $inner_var:tt => fn $name:ident($typ:ty), layout: $perform_layout:expr) => {
        pub fn $name(destination: Handle<UiNode>, direction: MessageDirection, value:$typ) -> UiMessage {
            UiMessage {
                handled: std::cell::Cell::new(false),
                data: std::rc::Rc::new($inner::$inner_var(value)),
                destination,
                direction,
                perform_layout: std::cell::Cell::new($perform_layout),
                flags: 0
            }
        }
    };

    ($inner:ident : $inner_var:tt => fn $name:ident( $($params:ident : $types:ty),+ ), layout: $perform_layout:expr) => {
        pub fn $name(destination: Handle<UiNode>, direction: MessageDirection, $($params : $types),+) -> UiMessage {
            UiMessage {
                handled: std::cell::Cell::new(false),
                data: std::rc::Rc::new($inner::$inner_var { $($params),+ }),
                destination,
                direction,
                perform_layout: std::cell::Cell::new($perform_layout),
                flags: 0
            }
        }
    }
}

/// A set of messages for any kind of widgets (including user controls). These messages provides basic
/// communication elements of the UI library.
#[derive(Debug, Clone, PartialEq)]
pub enum WidgetMessage {
    /// Initiated when user clicks on a widget's geometry.
    ///
    /// Direction: **From UI**.
    MouseDown {
        /// Position of cursor.
        pos: Vector2<f32>,
        /// A button that was pressed.
        button: MouseButton,
    },

    /// Initiated when user releases mouse button while cursor is over widget's geometry.
    ///
    /// Direction: **From UI**.
    MouseUp {
        /// Position of cursor.
        pos: Vector2<f32>,
        /// A button that was released.
        button: MouseButton,
    },

    /// Initiated when user moves cursor over widget's geometry.
    ///
    /// Direction: **From/To UI**.
    MouseMove {
        /// New position of cursor in screen coordinates.
        pos: Vector2<f32>,
        /// State of mouse buttons.
        state: MouseState,
    },

    /// Initiated when user scrolls mouse wheel while cursor is over widget's geometry.
    ///
    /// Direction: **From/To UI**.
    MouseWheel {
        /// Position of cursor.
        pos: Vector2<f32>,
        /// Amount of lines per mouse wheel turn.
        amount: f32,
    },

    /// Initiated when cursor leaves geometry of a widget.
    ///
    /// Direction: **From UI**.
    MouseLeave,

    /// Initiated when cursor enters geometry of a widget.
    ///
    /// Direction: **From UI**.
    MouseEnter,

    /// Initiated when widget is in focus and user types something on a keyboard.
    ///
    /// Direction: **From/To UI**.
    Text(char),

    /// Initiated when widget is in focus and user presses a button on a keyboard.
    ///
    /// Direction: **From UI**.
    KeyDown(KeyCode),

    /// Initiated when widget is in focus and user releases a button on a keyboard.
    ///
    /// Direction: **From UI**.
    KeyUp(KeyCode),

    /// Initiated when widget received focus. In most cases focus is received by clicking on
    /// widget.
    ///
    /// Direction: **From UI**.
    GotFocus,

    /// Initiated when dragging of a widget has started.
    ///
    /// Direction: **From UI**.
    DragStarted(Handle<UiNode>),

    /// Initiated when user drags a widget over some other widget.
    ///
    /// Direction: **From UI**.
    DragOver(Handle<UiNode>),

    /// Initiated when user drops a widget onto some other widget.
    ///
    /// Direction: **From UI**.
    Drop(Handle<UiNode>),

    /// Initiated when widget has lost its focus.
    ///
    /// Direction: **From UI**.
    LostFocus,

    /// A request to make widget topmost. Widget can be made topmost only in the same hierarchy
    /// level only!
    ///
    /// Direction: **From/To UI**.
    TopMost,

    /// A request to detach widget from its current parent and attach to root canvas.
    ///
    /// Direction: **From/To UI**.
    Unlink,

    /// A request to delete widget with all its children widgets. All handles to a node and its
    /// children will be invalid after processing such message!
    ///
    /// Direction: **From/To UI**.
    Remove,

    /// A request to link initiator with specified widget.
    ///
    /// Direction: **From/To UI**.
    LinkWith(Handle<UiNode>),

    /// A request to link initiator with specified widget and put it in front of children list.
    ///
    /// Direction: **From/To UI**.
    LinkWithReverse(Handle<UiNode>),

    /// A request to change background brush of a widget. Background brushes are used to fill volume of widgets.
    ///
    /// Direction: **From/To UI**
    Background(Brush),

    /// A request to change foreground brush of a widget. Foreground brushes are used for text, borders and so on.
    ///
    /// Direction: **From/To UI**
    Foreground(Brush),

    /// A request to change name of a widget. Name is given to widget mostly for debugging purposes.
    ///
    /// Direction: **From/To UI**
    Name(String),

    /// A request to set width of a widget. In most cases there is no need to explicitly set width of a widget,
    /// because rg3d-ui uses automatic layout engine which will correctly calculate desired width of a widget.
    ///
    /// Direction: **From/To UI**
    Width(f32),

    /// A request to set height of a widget. In most cases there is no need to explicitly set height of a widget,
    /// because rg3d-ui uses automatic layout engine which will correctly calculate desired height of a widget.
    ///
    /// Direction: **From/To UI**
    Height(f32),

    /// A request to set vertical alignment of a widget. Vertical alignment tells where to put widget in the parent
    /// widget's bounds in vertical direction.
    ///
    /// Direction: **From/To UI**
    VerticalAlignment(VerticalAlignment),

    /// A request to set horizontal alignment of a widget. Horizontal alignment tells where to put widget in the parent
    /// widget's bounds in horizontal direction.
    ///
    /// Direction: **From/To UI**
    HorizontalAlignment(HorizontalAlignment),

    /// A request to set maximum size of widget. Maximum size restricts size of a widget during layout pass. For example
    /// you can set maximum size to a button which was placed into a grid's cell, if maximum size wouldn't be set, button
    /// would be stretched to fill entire cell.
    ///
    /// Direction: **From/To UI**
    MaxSize(Vector2<f32>),

    /// A request to set minimum size of widget. Minimum size restricts size of a widget during layout pass. For example
    /// you can set minimum size to a button which was placed into a grid's cell, if minimum size wouldn't be set, button
    /// would be compressed to fill entire cell.
    ///
    /// Direction: **From/To UI**
    MinSize(Vector2<f32>),

    /// A request to set row number of a grid to which widget should belong to.
    ///
    /// Direction: **From/To UI**
    ///
    /// # Notes
    ///
    /// This is bad API and it should be changed in future. Grid should have explicit list of pairs (row, child) instead
    /// of this indirect attachment.
    Row(usize),

    /// A request to set column number of a grid to which widget should belong to.
    ///
    /// Direction: **From/To UI**
    ///
    /// # Notes
    ///
    /// This is bad API and it should be changed in future. Grid should have explicit list of pairs (column, child) instead
    /// of this indirect attachment.
    Column(usize),

    /// A request to set new margin of widget. Margin could be used to add some free space around widget to make UI look less
    /// dense.
    ///
    /// Direction: **From/To UI**
    Margin(Thickness),

    /// A request to set new state hit test visibility. If set to false, widget will become "non-clickable". It is useful for
    /// decorations which should be transparent for mouse events.
    ///
    /// Direction: **From/To UI**
    HitTestVisibility(bool),

    /// A request to set new visibility of a widget. Widget can be either visible or not. Invisible widgets does not take space
    /// in layout pass and collapsed to a point.
    ///
    /// Direction: **From/To UI**
    Visibility(bool),

    /// A request to set new z index of a widget. Z index is used to change drawing order of widgets. Please note that it works
    /// only in same hierarchy level, which means that it is impossible to set z index to 9999 (or similar huge value) to force
    /// widget to be drawn on top of everything.
    ///
    /// Direction: **From/To UI**
    ZIndex(usize),

    /// A request to set new desired position of a widget. It is called "desired" because layout system may ignore it and set
    /// some other position. Desired position works with a combination of a layout panel that supports direct coordinated
    /// (Canvas for example).
    ///
    /// Direction: **From/To UI**
    DesiredPosition(Vector2<f32>),

    /// A request to enable or disable widget. Disabled widget won't receive mouse events and may look differently (it is defined
    /// by internal styling).
    ///
    /// Direction: **From/To UI**
    Enabled(bool),

    /// A request to set desired position at center in local coordinates.
    ///
    /// Direction: **From/To UI**
    Center,

    /// A request to set new cursor icon for widget.
    ///
    /// Direction: **From/To UI**
    Cursor(Option<CursorIcon>),

    /// A request to set new opacity for widget.
    ///
    /// Direction: **From/To UI**
    Opacity(Option<f32>),
}

impl WidgetMessage {
    define_constructor!(WidgetMessage:Remove => fn remove(), layout: false);
    define_constructor!(WidgetMessage:Unlink => fn unlink(), layout: false);
    define_constructor!(WidgetMessage:LinkWith => fn link(Handle<UiNode>), layout: false);
    define_constructor!(WidgetMessage:LinkWithReverse => fn link_reverse(Handle<UiNode>), layout: false);
    define_constructor!(WidgetMessage:Background => fn background(Brush), layout: false);
    define_constructor!(WidgetMessage:Foreground => fn foreground(Brush), layout: false);
    define_constructor!(WidgetMessage:Visibility => fn visibility(bool), layout: false);
    define_constructor!(WidgetMessage:Width => fn width(f32), layout: false);
    define_constructor!(WidgetMessage:Height => fn height(f32), layout: false);
    define_constructor!(WidgetMessage:DesiredPosition => fn desired_position(Vector2<f32>), layout: false);
    define_constructor!(WidgetMessage:Center => fn center(), layout: true);
    define_constructor!(WidgetMessage:TopMost => fn topmost(), layout: false);
    define_constructor!(WidgetMessage:Enabled => fn enabled(bool), layout: false);
    define_constructor!(WidgetMessage:Name => fn name(String), layout: false);
    define_constructor!(WidgetMessage:Row => fn row(usize), layout: false);
    define_constructor!(WidgetMessage:Column => fn column(usize), layout: false);
    define_constructor!(WidgetMessage:Cursor => fn cursor(Option<CursorIcon>), layout: false);
    define_constructor!(WidgetMessage:ZIndex => fn z_index(usize), layout: false);
    define_constructor!(WidgetMessage:HitTestVisibility => fn hit_test_visibility(bool), layout: false);
    define_constructor!(WidgetMessage:Margin => fn margin(Thickness), layout: false);
    define_constructor!(WidgetMessage:MinSize => fn min_size(Vector2<f32>), layout: false);
    define_constructor!(WidgetMessage:MaxSize => fn max_size(Vector2<f32>), layout: false);
    define_constructor!(WidgetMessage:HorizontalAlignment => fn horizontal_alignment(HorizontalAlignment), layout: false);
    define_constructor!(WidgetMessage:VerticalAlignment => fn vertical_alignment(VerticalAlignment), layout: false);
    define_constructor!(WidgetMessage:Opacity => fn opacity(Option<f32>), layout: false);

    // Internal messages. Do not use.
    define_constructor!(WidgetMessage:GotFocus => fn got_focus(), layout: false);
    define_constructor!(WidgetMessage:LostFocus => fn lost_focus(), layout: false);
    define_constructor!(WidgetMessage:MouseDown => fn mouse_down(pos: Vector2<f32>, button: MouseButton), layout: false);
    define_constructor!(WidgetMessage:MouseUp => fn mouse_up(pos: Vector2<f32>, button: MouseButton), layout: false);
    define_constructor!(WidgetMessage:MouseMove => fn mouse_move(pos: Vector2<f32>, state: MouseState), layout: false);
    define_constructor!(WidgetMessage:MouseWheel => fn mouse_wheel(pos: Vector2<f32>, amount: f32), layout: false);
    define_constructor!(WidgetMessage:MouseLeave => fn mouse_leave(), layout: false);
    define_constructor!(WidgetMessage:MouseEnter => fn mouse_enter(), layout: false);
    define_constructor!(WidgetMessage:Text => fn text(char), layout: false);
    define_constructor!(WidgetMessage:KeyDown => fn key_down(KeyCode), layout: false);
    define_constructor!(WidgetMessage:KeyUp => fn key_up(KeyCode), layout: false);
    define_constructor!(WidgetMessage:DragStarted => fn drag_started(Handle<UiNode>), layout: false);
    define_constructor!(WidgetMessage:DragOver => fn drag_over(Handle<UiNode>), layout: false);
    define_constructor!(WidgetMessage:Drop => fn drop(Handle<UiNode>), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum ButtonMessage {
    Click,
    Content(Handle<UiNode>),
}

impl ButtonMessage {
    define_constructor!(ButtonMessage:Click => fn click(), layout: false);
    define_constructor!(ButtonMessage:Content => fn content(Handle<UiNode>), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScrollBarMessage {
    Value(f32),
    MinValue(f32),
    MaxValue(f32),
}

impl ScrollBarMessage {
    define_constructor!(ScrollBarMessage:Value => fn value(f32), layout: false);
    define_constructor!(ScrollBarMessage:MaxValue => fn max_value(f32), layout: false);
    define_constructor!(ScrollBarMessage:MinValue => fn min_value(f32), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckBoxMessage {
    Check(Option<bool>),
}

impl CheckBoxMessage {
    define_constructor!(CheckBoxMessage:Check => fn checked(Option<bool>), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExpanderMessage {
    Expand(bool),
}

impl ExpanderMessage {
    define_constructor!(ExpanderMessage:Expand => fn expand(bool), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum WindowMessage {
    /// Opens a window.
    Open { center: bool },

    /// Opens window in modal mode. Modal mode does **not** blocks current thread, instead
    /// it just restricts mouse and keyboard events only to window so other content is not
    /// clickable/type-able. Closing a window removes that restriction.
    OpenModal { center: bool },

    /// Closes a window.
    Close,

    /// Minimizes a window - it differs from classic minimization in window managers,
    /// instead of putting window in system tray, it just collapses internal content panel.
    Minimize(bool),

    /// Whether or not window can be minimized by _ mark. false hides _ mark.
    CanMinimize(bool),

    /// Whether or not window can be closed by X mark. false hides X mark.
    CanClose(bool),

    /// Whether or not window can be resized by resize grips.
    CanResize(bool),

    /// Indicates that move has been started. You should never send this message by hand.
    MoveStart,

    /// Moves window to a new position in local coordinates.
    Move(Vector2<f32>),

    /// Indicated that move has ended. You should never send this message by hand.
    MoveEnd,

    /// Sets new window title.
    Title(WindowTitle),
}

impl WindowMessage {
    define_constructor!(WindowMessage:Open => fn open(center: bool), layout: false);
    define_constructor!(WindowMessage:OpenModal => fn open_modal(center: bool), layout: false);
    define_constructor!(WindowMessage:Close => fn close(), layout: false);
    define_constructor!(WindowMessage:Minimize => fn minimize(bool), layout: false);
    define_constructor!(WindowMessage:CanMinimize => fn can_minimize(bool), layout: false);
    define_constructor!(WindowMessage:CanClose => fn can_close(bool), layout: false);
    define_constructor!(WindowMessage:CanResize => fn can_resize(bool), layout: false);
    define_constructor!(WindowMessage:MoveStart => fn move_start(), layout: false);
    define_constructor!(WindowMessage:Move => fn move_to(Vector2<f32>), layout: false);
    define_constructor!(WindowMessage:MoveEnd => fn move_end(), layout: false);
    define_constructor!(WindowMessage:Title => fn title(WindowTitle), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScrollViewerMessage {
    Content(Handle<UiNode>),
    /// Adjusts vertical and horizontal scroll values so given node will be in "view box"
    /// of scroll viewer.
    BringIntoView(Handle<UiNode>),
}

impl ScrollViewerMessage {
    define_constructor!(ScrollViewerMessage:Content => fn content(Handle<UiNode>), layout: false);
    define_constructor!(ScrollViewerMessage:BringIntoView=> fn bring_into_view(Handle<UiNode>), layout: true);
}

#[derive(Debug, Clone, PartialEq)]
pub enum ListViewMessage {
    SelectionChanged(Option<usize>),
    Items(Vec<Handle<UiNode>>),
    AddItem(Handle<UiNode>),
    RemoveItem(Handle<UiNode>),
}

impl ListViewMessage {
    define_constructor!(ListViewMessage:SelectionChanged => fn selection(Option<usize>), layout: false);
    define_constructor!(ListViewMessage:Items => fn items(Vec<Handle<UiNode >>), layout: false);
    define_constructor!(ListViewMessage:AddItem => fn add_item(Handle<UiNode>), layout: false);
    define_constructor!(ListViewMessage:RemoveItem => fn remove_item(Handle<UiNode>), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum DropdownListMessage {
    SelectionChanged(Option<usize>),
    Items(Vec<Handle<UiNode>>),
    AddItem(Handle<UiNode>),
    Open,
    Close,
}

impl DropdownListMessage {
    define_constructor!(DropdownListMessage:SelectionChanged => fn selection(Option<usize>), layout: false);
    define_constructor!(DropdownListMessage:Items => fn items(Vec<Handle<UiNode >>), layout: false);
    define_constructor!(DropdownListMessage:AddItem => fn add_item(Handle<UiNode>), layout: false);
    define_constructor!(DropdownListMessage:Open => fn open(), layout: false);
    define_constructor!(DropdownListMessage:Close => fn close(), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum PopupMessage {
    Open,
    Close,
    Content(Handle<UiNode>),
    Placement(Placement),
}

impl PopupMessage {
    define_constructor!(PopupMessage:Open => fn open(), layout: false);
    define_constructor!(PopupMessage:Close => fn close(), layout: false);
    define_constructor!(PopupMessage:Content => fn content(Handle<UiNode>), layout: false);
    define_constructor!(PopupMessage:Placement => fn placement(Placement), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileSelectorMessage {
    Root(Option<PathBuf>),
    Path(PathBuf),
    Commit(PathBuf),
    Cancel,
    Filter(Option<Filter>),
}

impl FileSelectorMessage {
    define_constructor!(FileSelectorMessage:Commit => fn commit(PathBuf), layout: false);
    define_constructor!(FileSelectorMessage:Root => fn root(Option<PathBuf>), layout: false);
    define_constructor!(FileSelectorMessage:Path => fn path(PathBuf), layout: false);
    define_constructor!(FileSelectorMessage:Cancel => fn cancel(), layout: false);
    define_constructor!(FileSelectorMessage:Filter => fn filter(Option<Filter>), layout: false);
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash, Debug)]
pub enum TreeExpansionStrategy {
    /// Expand a single item.
    Direct,
    /// Expand an item and its descendants.
    RecursiveDescendants,
    /// Expand an item and its ancestors (chain of parent trees).
    RecursiveAncestors,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SelectionState(pub(in crate) bool);

#[derive(Debug, Clone, PartialEq)]
pub enum TreeMessage {
    Expand {
        expand: bool,
        expansion_strategy: TreeExpansionStrategy,
    },
    AddItem(Handle<UiNode>),
    RemoveItem(Handle<UiNode>),
    SetExpanderShown(bool),
    SetItems(Vec<Handle<UiNode>>),
    // Private, do not use. For internal needs only. Use TreeRootMessage::Selected.
    Select(SelectionState),
}

impl TreeMessage {
    define_constructor!(TreeMessage:Expand => fn expand(expand: bool, expansion_strategy: TreeExpansionStrategy), layout: false);
    define_constructor!(TreeMessage:AddItem => fn add_item(Handle<UiNode>), layout: false);
    define_constructor!(TreeMessage:RemoveItem => fn remove_item(Handle<UiNode>), layout: false);
    define_constructor!(TreeMessage:SetExpanderShown => fn set_expander_shown(bool), layout: false);
    define_constructor!(TreeMessage:SetItems => fn set_items(Vec<Handle<UiNode >>), layout: false);
    define_constructor!(TreeMessage:Select => fn select(SelectionState), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum TreeRootMessage {
    AddItem(Handle<UiNode>),
    RemoveItem(Handle<UiNode>),
    Items(Vec<Handle<UiNode>>),
    Selected(Vec<Handle<UiNode>>),
    ExpandAll,
    CollapseAll,
}

impl TreeRootMessage {
    define_constructor!(TreeRootMessage:AddItem => fn add_item(Handle<UiNode>), layout: false);
    define_constructor!(TreeRootMessage:RemoveItem=> fn remove_item(Handle<UiNode>), layout: false);
    define_constructor!(TreeRootMessage:Items => fn items(Vec<Handle<UiNode >>), layout: false);
    define_constructor!(TreeRootMessage:Selected => fn select(Vec<Handle<UiNode >>), layout: false);
    define_constructor!(TreeRootMessage:ExpandAll => fn expand_all(), layout: false);
    define_constructor!(TreeRootMessage:CollapseAll => fn collapse_all(), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileBrowserMessage {
    Root(Option<PathBuf>),
    Path(PathBuf),
    Filter(Option<Filter>),
    Add(PathBuf),
    Remove(PathBuf),
    Rescan,
}

impl FileBrowserMessage {
    define_constructor!(FileBrowserMessage:Root => fn root(Option<PathBuf>), layout: false);
    define_constructor!(FileBrowserMessage:Path => fn path(PathBuf), layout: false);
    define_constructor!(FileBrowserMessage:Filter => fn filter(Option<Filter>), layout: false);
    define_constructor!(FileBrowserMessage:Add => fn add(PathBuf), layout: false);
    define_constructor!(FileBrowserMessage:Remove => fn remove(PathBuf), layout: false);
    define_constructor!(FileBrowserMessage:Rescan => fn rescan(), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextBoxMessage {
    Text(String),
}

impl TextBoxMessage {
    define_constructor!(TextBoxMessage:Text => fn text(String), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextMessage {
    Text(String),
    Wrap(WrapMode),
    Font(SharedFont),
    VerticalAlignment(VerticalAlignment),
    HorizontalAlignment(HorizontalAlignment),
}

impl TextMessage {
    define_constructor!(TextMessage:Text => fn text(String), layout: false);
    define_constructor!(TextMessage:Wrap=> fn wrap(WrapMode), layout: false);
    define_constructor!(TextMessage:Font => fn font(SharedFont), layout: false);
    define_constructor!(TextMessage:VerticalAlignment => fn vertical_alignment(VerticalAlignment), layout: false);
    define_constructor!(TextMessage:HorizontalAlignment => fn horizontal_alignment(HorizontalAlignment), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImageMessage {
    Texture(Option<SharedTexture>),
    Flip(bool),
}

impl ImageMessage {
    define_constructor!(ImageMessage:Texture => fn texture(Option<SharedTexture>), layout: false);
    define_constructor!(ImageMessage:Flip => fn flip(bool), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum TileMessage {
    Content(TileContent),
    /// Internal. Do not use.
    Split {
        window: Handle<UiNode>,
        direction: SplitDirection,
        first: bool,
    },
}

impl TileMessage {
    define_constructor!(TileMessage:Content => fn content(TileContent), layout: false);
    define_constructor!(TileMessage:Split => fn split(window: Handle<UiNode>,
        direction: SplitDirection,
        first: bool), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScrollPanelMessage {
    VerticalScroll(f32),
    HorizontalScroll(f32),
    /// Adjusts vertical and horizontal scroll values so given node will be in "view box"
    /// of scroll panel.
    BringIntoView(Handle<UiNode>),
}

impl ScrollPanelMessage {
    define_constructor!(ScrollPanelMessage:VerticalScroll => fn vertical_scroll(f32), layout: false);
    define_constructor!(ScrollPanelMessage:HorizontalScroll => fn horizontal_scroll(f32), layout: false);
    define_constructor!(ScrollPanelMessage:BringIntoView => fn bring_into_view(Handle<UiNode>), layout: true);
}

#[derive(Debug, Clone, PartialEq)]
pub enum MenuMessage {
    Activate,
    Deactivate,
}

impl MenuMessage {
    define_constructor!(MenuMessage:Activate => fn activate(), layout: false);
    define_constructor!(MenuMessage:Deactivate => fn deactivate(), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum MenuItemMessage {
    Open,
    Close,
    Click,
}

impl MenuItemMessage {
    define_constructor!(MenuItemMessage:Open => fn open(), layout: false);
    define_constructor!(MenuItemMessage:Close => fn close(), layout: false);
    define_constructor!(MenuItemMessage:Click => fn click(), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageBoxMessage {
    Open {
        title: Option<String>,
        text: Option<String>,
    },
    Close(MessageBoxResult),
}

impl MessageBoxMessage {
    define_constructor!(MessageBoxMessage:Open => fn open(title: Option<String>, text: Option<String>), layout: false);
    define_constructor!(MessageBoxMessage:Close => fn close(MessageBoxResult), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum DecoratorMessage {
    Select(bool),
    HoverBrush(Brush),
    NormalBrush(Brush),
    PressedBrush(Brush),
    SelectedBrush(Brush),
}

impl DecoratorMessage {
    define_constructor!(DecoratorMessage:Select => fn select(bool), layout: false);
    define_constructor!(DecoratorMessage:HoverBrush => fn hover_brush(Brush), layout: false);
    define_constructor!(DecoratorMessage:NormalBrush => fn normal_brush(Brush), layout: false);
    define_constructor!(DecoratorMessage:PressedBrush => fn pressed_brush(Brush), layout: false);
    define_constructor!(DecoratorMessage:SelectedBrush => fn selected_brush(Brush), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProgressBarMessage {
    Progress(f32),
}

impl ProgressBarMessage {
    define_constructor!(ProgressBarMessage:Progress => fn progress(f32), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum HueBarMessage {
    /// Sets new hue value.
    Hue(f32),

    /// Sets new orientation
    Orientation(Orientation),
}

impl HueBarMessage {
    define_constructor!(HueBarMessage:Hue => fn hue(f32), layout: false);
    define_constructor!(HueBarMessage:Orientation => fn orientation(Orientation), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum AlphaBarMessage {
    /// Sets new hue value.
    Alpha(f32),

    /// Sets new orientation
    Orientation(Orientation),
}

impl AlphaBarMessage {
    define_constructor!(AlphaBarMessage:Alpha => fn alpha(f32), layout: false);
    define_constructor!(AlphaBarMessage:Orientation => fn orientation(Orientation), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum SaturationBrightnessFieldMessage {
    /// Sets new hue value on the field.
    Hue(f32),

    /// Sets new saturation value on the field.
    Saturation(f32),

    /// Sets new brightness value on the field.
    Brightness(f32),
}

impl SaturationBrightnessFieldMessage {
    define_constructor!(SaturationBrightnessFieldMessage:Hue => fn hue(f32), layout: false);
    define_constructor!(SaturationBrightnessFieldMessage:Saturation => fn saturation(f32), layout: false);
    define_constructor!(SaturationBrightnessFieldMessage:Brightness => fn brightness(f32), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColorPickerMessage {
    /// Sets color in RGB.
    ///
    /// Direction: **To/From Widget**.
    Color(Color),

    /// Sets color in HSV.
    ///
    /// Direction: **To Widget**.
    Hsv(Hsv),
}

impl ColorPickerMessage {
    define_constructor!(ColorPickerMessage:Color => fn color(Color), layout: false);
    define_constructor!(ColorPickerMessage:Hsv => fn hsv(Hsv), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColorFieldMessage {
    Color(Color),
}

impl ColorFieldMessage {
    define_constructor!(ColorFieldMessage:Color => fn color(Color), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum CurveEditorMessage {
    Sync(Curve),
    ViewPosition(Vector2<f32>),
    Zoom(f32),
    ZoomToFit,

    // Internal messages. Use only when you know what you're doing.
    // These are internal because you must use Sync message to request changes
    // in the curve editor.
    ChangeSelectedKeysKind(CurveKeyKind),
    RemoveSelection,
    // Position in screen coordinates.
    AddKey(Vector2<f32>),
}

impl CurveEditorMessage {
    define_constructor!(CurveEditorMessage:Sync => fn sync(Curve), layout: false);
    define_constructor!(CurveEditorMessage:ViewPosition => fn view_position(Vector2<f32>), layout: false);
    define_constructor!(CurveEditorMessage:Zoom => fn zoom(f32), layout: false);
    define_constructor!(CurveEditorMessage:ZoomToFit => fn zoom_to_fit(), layout: false);
    // Internal. Use only when you know what you're doing.
    define_constructor!(CurveEditorMessage:RemoveSelection => fn remove_selection(), layout: false);
    define_constructor!(CurveEditorMessage:ChangeSelectedKeysKind => fn change_selected_keys_kind(CurveKeyKind), layout: false);
    define_constructor!(CurveEditorMessage:AddKey => fn add_key(Vector2<f32>), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum CollectionChanged {
    /// An item should be added in the collection.
    Add,
    /// An item in the collection should be removed.
    Remove(usize),
    /// An item in the collection has changed one of its properties.
    ItemChanged {
        /// Index of an item in the collection.
        index: usize,
        property: PropertyChanged,
    },
}

impl CollectionChanged {
    define_constructor!(CollectionChanged:Add => fn add(), layout: false);
    define_constructor!(CollectionChanged:Remove => fn remove(usize), layout: false);
    define_constructor!(CollectionChanged:ItemChanged => fn item_changed(index: usize, property: PropertyChanged), layout: false);
}

#[derive(Debug, Clone)]
pub enum FieldKind {
    Collection(Box<CollectionChanged>),
    Inspectable(Box<PropertyChanged>),
    Object(ObjectValue),
}

#[derive(Debug, Clone)]
pub struct ObjectValue {
    value: Rc<dyn PropertyValue>,
}

#[allow(clippy::vtable_address_comparisons)]
impl PartialEq for ObjectValue {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&*self.value, &*other.value)
    }
}

impl ObjectValue {
    pub fn cast_value<T: 'static>(&self) -> Option<&T> {
        (*self.value).as_any().downcast_ref::<T>()
    }

    pub fn cast_value_cloned<T: Clone + 'static>(&self) -> Option<T> {
        (*self.value).as_any().downcast_ref::<T>().cloned()
    }
}

impl PartialEq for FieldKind {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (FieldKind::Collection(l), FieldKind::Collection(r)) => std::ptr::eq(&**l, &**r),
            (FieldKind::Inspectable(l), FieldKind::Inspectable(r)) => std::ptr::eq(&**l, &**r),
            (FieldKind::Object(l), FieldKind::Object(r)) => l == r,
            _ => false,
        }
    }
}

impl FieldKind {
    pub fn object<T: PropertyValue>(value: T) -> Self {
        Self::Object(ObjectValue {
            value: Rc::new(value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyChanged {
    pub name: String,
    pub owner_type_id: TypeId,
    pub value: FieldKind,
}

impl PropertyChanged {
    pub fn path(&self) -> String {
        let mut path = self.name.clone();
        match self.value {
            FieldKind::Collection(ref collection_changed) => {
                if let CollectionChanged::ItemChanged {
                    ref property,
                    index,
                } = **collection_changed
                {
                    path += format!("[{}].{}", index, property.path()).as_ref();
                }
            }
            FieldKind::Inspectable(ref inspectable) => {
                path += format!(".{}", inspectable.path()).as_ref();
            }
            FieldKind::Object(_) => {}
        }
        path
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InspectorMessage {
    Context(InspectorContext),
    PropertyChanged(PropertyChanged),
}

impl InspectorMessage {
    define_constructor!(InspectorMessage:Context => fn context(InspectorContext), layout: false);
    define_constructor!(InspectorMessage:PropertyChanged => fn property_changed(PropertyChanged), layout: false);
}

#[derive(Debug)]
pub struct UserMessageData(pub Box<dyn MessageData>);

impl Deref for UserMessageData {
    type Target = dyn MessageData;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for UserMessageData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

impl PartialEq for UserMessageData {
    fn eq(&self, other: &Self) -> bool {
        self.0.compare(&*other.0)
    }
}

/// Message direction allows you to distinguish from where message has came from.
/// Often there is a need to find out who created a message to respond properly.
/// Imagine that we have a NumericUpDown input field for a property and we using
/// some data source to feed data into input field. When we change something in
/// the input field by typing, it creates a message with new value. On other
/// hand we often need to put new value in the input field from some code, in this
/// case we again creating a message. But how to understand from which "side"
/// message has came from? Was it filled in by user and we should create a command
/// to change value in the data source, or it was created from syncing code just to
/// pass new value to UI? This problem solved by setting a direction to a message.
/// Also it solves another problem: often we need to respond to a message only if
/// it did some changes. In this case at first we fire a message with ToWidget
/// direction, widget catches it and checks if changes are needed and if so, it
/// "rethrows" message with direction FromWidget. Listeners are "subscribed" to
/// FromWidget messages only and won't respond to ToWidget messages.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Hash)]
pub enum MessageDirection {
    /// Used to indicate a request for changes in a widget.
    ToWidget,

    /// Used to indicate response from widget if anything has actually changed.
    FromWidget,
}

impl MessageDirection {
    /// Reverses direction.
    pub fn reverse(self) -> Self {
        match self {
            Self::ToWidget => Self::FromWidget,
            Self::FromWidget => Self::ToWidget,
        }
    }
}

pub trait MessageData: 'static + Debug + Any {
    fn as_any(&self) -> &dyn Any;

    fn compare(&self, other: &dyn MessageData) -> bool;
}

impl<T> MessageData for T
where
    T: 'static + Debug + PartialEq + Any,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn compare(&self, other: &dyn MessageData) -> bool {
        other
            .as_any()
            .downcast_ref::<T>()
            .map(|other| other == self)
            .unwrap_or_default()
    }
}

impl dyn MessageData {
    pub fn cast<T: MessageData>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }
}

/// Message is basic communication element that is used to deliver information to UI nodes
/// or to user code.
///
/// # Threading
///
/// UiMessage is nor Send or Sync. User interface is a single-thread thing, as well as its messages.
#[derive(Debug, Clone)]
pub struct UiMessage {
    /// Useful flag to check if a message was already handled.
    pub handled: Cell<bool>,

    /// Actual message data.
    pub data: Rc<dyn MessageData>,

    /// Handle of node that will receive message. Please note that all nodes in hierarchy will
    /// also receive this message, order is "up-on-tree".
    pub destination: Handle<UiNode>,

    /// Indicates the direction of the message.
    ///
    /// See [MessageDirection](enum.MessageDirection.html) for details.
    pub direction: MessageDirection,

    /// Whether or not message requires layout to be calculated first.
    ///
    /// Some of message handling routines uses layout info, but message loop
    /// performed right after layout pass, but some of messages may change
    /// layout and this flag tells UI to perform layout before passing message
    /// further. In ideal case we'd perform layout after **each** message, but
    /// since layout pass is super heavy we should do it **only** when it is
    /// actually needed.
    pub perform_layout: Cell<bool>,

    /// A custom user flags.
    pub flags: u64,
}

impl PartialEq for UiMessage {
    fn eq(&self, other: &Self) -> bool {
        self.handled == other.handled
            && self.data.compare(&*other.data)
            && self.destination == other.destination
            && self.direction == other.direction
            && self.perform_layout == other.perform_layout
            && self.flags == other.flags
    }
}

impl UiMessage {
    pub fn with_data<T: MessageData>(data: T) -> Self {
        Self {
            handled: Cell::new(false),
            data: Rc::new(data),
            destination: Default::default(),
            direction: MessageDirection::ToWidget,
            perform_layout: Cell::new(false),
            flags: 0,
        }
    }

    pub fn with_destination(mut self, destination: Handle<UiNode>) -> Self {
        self.destination = destination;
        self
    }

    pub fn with_direction(mut self, direction: MessageDirection) -> Self {
        self.direction = direction;
        self
    }

    pub fn with_perform_layout(self, perform_layout: bool) -> Self {
        self.perform_layout.set(perform_layout);
        self
    }

    /// Creates a new copy of the message with reversed direction. Typical use case is
    /// to re-send messages to create "response" in widget. For example you have a float
    /// input field and it has Value message. When the input field receives Value message
    /// with [MessageDirection::ToWidget](enum.MessageDirection.html#variant.ToWidget)
    /// it checks if value needs to be changed and if it does, it re-sends same message
    /// but with reversed direction back to message queue so every "listener" can reach
    /// properly. The input field won't react at
    /// [MessageDirection::FromWidget](enum.MessageDirection.html#variant.FromWidget)
    /// message so there will be no infinite message loop.
    #[must_use = "method creates new value which must be used"]
    pub fn reverse(&self) -> Self {
        Self {
            handled: self.handled.clone(),
            data: self.data.clone(),
            destination: self.destination,
            direction: self.direction.reverse(),
            perform_layout: self.perform_layout.clone(),
            flags: self.flags,
        }
    }

    pub fn destination(&self) -> Handle<UiNode> {
        self.destination
    }

    pub fn data<T: MessageData>(&self) -> Option<&T> {
        (*self.data).as_any().downcast_ref::<T>()
    }

    pub fn set_handled(&self, handled: bool) {
        self.handled.set(handled);
    }

    pub fn handled(&self) -> bool {
        self.handled.get()
    }

    pub fn direction(&self) -> MessageDirection {
        self.direction
    }

    pub fn set_perform_layout(&self, value: bool) {
        self.perform_layout.set(value);
    }

    pub fn need_perform_layout(&self) -> bool {
        self.perform_layout.get()
    }

    pub fn has_flags(&self, flags: u64) -> bool {
        self.flags & flags != 0
    }
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
pub enum ButtonState {
    Pressed,
    Released,
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u16),
}

pub enum OsEvent {
    MouseInput {
        button: MouseButton,
        state: ButtonState,
    },
    CursorMoved {
        position: Vector2<f32>,
    },
    KeyboardInput {
        button: KeyCode,
        state: ButtonState,
    },
    Character(char),
    KeyboardModifiers(KeyboardModifiers),
    MouseWheel(f32, f32),
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Default)]
pub struct KeyboardModifiers {
    pub alt: bool,
    pub shift: bool,
    pub control: bool,
    pub system: bool,
}

impl KeyboardModifiers {
    pub fn is_none(self) -> bool {
        !self.shift && !self.control && !self.alt && !self.system
    }
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
#[repr(u32)]
pub enum KeyCode {
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key0,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    Escape,

    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,

    Snapshot,
    Scroll,
    Pause,

    Insert,
    Home,
    Delete,
    End,
    PageDown,
    PageUp,

    Left,
    Up,
    Right,
    Down,

    Backspace,
    Return,
    Space,

    Compose,

    Caret,

    Numlock,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,

    AbntC1,
    AbntC2,
    NumpadAdd,
    Apostrophe,
    Apps,
    At,
    Ax,
    Backslash,
    Calculator,
    Capital,
    Colon,
    Comma,
    Convert,
    NumpadDecimal,
    NumpadDivide,
    Equals,
    Grave,
    Kana,
    Kanji,
    LAlt,
    LBracket,
    LControl,
    LShift,
    LWin,
    Mail,
    MediaSelect,
    MediaStop,
    Minus,
    NumpadMultiply,
    Mute,
    MyComputer,
    NavigateForward,
    NavigateBackward,
    NextTrack,
    NoConvert,
    NumpadComma,
    NumpadEnter,
    NumpadEquals,
    OEM102,
    Period,
    PlayPause,
    Power,
    PrevTrack,
    RAlt,
    RBracket,
    RControl,
    RShift,
    RWin,
    Semicolon,
    Slash,
    Sleep,
    Stop,
    NumpadSubtract,
    Sysrq,
    Tab,
    Underline,
    Unlabeled,
    VolumeDown,
    VolumeUp,
    Wake,
    WebBack,
    WebFavorites,
    WebForward,
    WebHome,
    WebRefresh,
    WebSearch,
    WebStop,
    Yen,
    Copy,
    Paste,
    Cut,
    Asterisk,
    Plus,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CursorIcon {
    Default,
    Crosshair,
    Hand,
    Arrow,
    Move,
    Text,
    Wait,
    Help,
    Progress,
    NotAllowed,
    ContextMenu,
    Cell,
    VerticalText,
    Alias,
    Copy,
    NoDrop,
    Grab,
    Grabbing,
    AllScroll,
    ZoomIn,
    ZoomOut,
    EResize,
    NResize,
    NeResize,
    NwResize,
    SResize,
    SeResize,
    SwResize,
    WResize,
    EwResize,
    NsResize,
    NeswResize,
    NwseResize,
    ColResize,
    RowResize,
}

impl Default for CursorIcon {
    fn default() -> Self {
        CursorIcon::Default
    }
}
