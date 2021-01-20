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

use crate::core::algebra::{Vector2, Vector3};
use crate::dock::SplitDirection;
use crate::{
    brush::Brush,
    core::{
        color::{Color, Hsv},
        pool::Handle,
    },
    dock::TileContent,
    draw::SharedTexture,
    messagebox::MessageBoxResult,
    popup::Placement,
    ttf::SharedFont,
    window::WindowTitle,
    Control, HorizontalAlignment, MouseState, Orientation, Thickness, UINode, VerticalAlignment,
};
use std::{cell::Cell, fmt::Debug, path::PathBuf};

macro_rules! define_constructor {
    ($var:tt($inner:ident : $inner_var:tt) => fn $name:ident(), layout: $perform_layout:expr) => {
        pub fn $name(destination: Handle<UINode<M, C>>, direction: MessageDirection) -> UiMessage<M, C> {
            UiMessage {
                handled: Cell::new(false),
                data: UiMessageData::$var($inner::$inner_var),
                destination,
                direction,
                perform_layout: Cell::new($perform_layout),
                flags: 0
            }
        }
    };

    ($var:tt($inner:ident : $inner_var:tt) => fn $name:ident($typ:ty), layout: $perform_layout:expr) => {
        pub fn $name(destination: Handle<UINode<M, C>>, direction: MessageDirection, value:$typ) -> UiMessage<M, C> {
            UiMessage {
                handled: Cell::new(false),
                data: UiMessageData::$var($inner::$inner_var(value)),
                destination,
                direction,
                perform_layout: Cell::new($perform_layout),
                flags: 0
            }
        }
    };

    ($var:tt($inner:ident : $inner_var:tt) => fn $name:ident( $($params:ident : $types:ty),+ ), layout: $perform_layout:expr) => {
        pub fn $name(destination: Handle<UINode<M, C>>, direction: MessageDirection, $($params : $types),+) -> UiMessage<M, C> {
            UiMessage {
                handled: Cell::new(false),
                data: UiMessageData::$var($inner::$inner_var { $($params),+ } ),
                destination,
                direction,
                perform_layout: Cell::new($perform_layout),
                flags: 0
            }
        }
    }
}

macro_rules! define_constructor_unbound {
    ($var:tt($inner:ident : $inner_var:tt) => fn $name:ident(), layout: $perform_layout:expr) => {
        pub fn $name<M: MessageData, C: Control<M, C>>(destination: Handle<UINode<M, C>>, direction: MessageDirection) -> UiMessage<M, C> {
            UiMessage {
                handled: Cell::new(false),
                data: UiMessageData::$var($inner::$inner_var),
                destination,
                direction,
                perform_layout: Cell::new($perform_layout),
                flags: 0
            }
        }
    };

    ($var:tt($inner:ident : $inner_var:tt) => fn $name:ident($typ:ty), layout: $perform_layout:expr) => {
        pub fn $name<M: MessageData, C: Control<M, C>>(destination: Handle<UINode<M, C>>, direction: MessageDirection, value:$typ) -> UiMessage<M, C> {
            UiMessage {
                handled: Cell::new(false),
                data: UiMessageData::$var($inner::$inner_var(value)),
                destination,
                direction,
                perform_layout: Cell::new($perform_layout),
                flags: 0
            }
        }
    };

    ($var:tt($inner:ident : $inner_var:tt) => fn $name:ident( $($params:ident : $types:ty),+ ), layout: $perform_layout:expr) => {
        pub fn $name<M: MessageData, C: Control<M, C>>(destination: Handle<UINode<M, C>>, direction: MessageDirection, $($params : $types),+) -> UiMessage<M, C> {
            UiMessage {
                handled: Cell::new(false),
                data: UiMessageData::$var($inner::$inner_var { $($params),+ } ),
                destination,
                direction,
                perform_layout: Cell::new($perform_layout),
                flags: 0
            }
        }
    }
}

/// A set of messages for any kind of widgets (including user controls). These messages provides basic
/// communication elements of the UI library.
#[derive(Debug, Clone, PartialEq)]
pub enum WidgetMessage<M: MessageData, C: Control<M, C>> {
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
    DragStarted(Handle<UINode<M, C>>),

    /// Initiated when user drags a widget over some other widget.
    ///
    /// Direction: **From UI**.
    DragOver(Handle<UINode<M, C>>),

    /// Initiated when user drops a widget onto some other widget.
    ///
    /// Direction: **From UI**.
    Drop(Handle<UINode<M, C>>),

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
    LinkWith(Handle<UINode<M, C>>),

    /// A request to link initiator with specified widget and put it in front of children list.
    ///
    /// Direction: **From/To UI**.
    LinkWithReverse(Handle<UINode<M, C>>),

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
    Opacity(f32),
}

impl<M: MessageData, C: Control<M, C>> WidgetMessage<M, C> {
    define_constructor!(Widget(WidgetMessage:Remove) => fn remove(), layout: false);
    define_constructor!(Widget(WidgetMessage:Unlink) => fn unlink(), layout: false);
    define_constructor!(Widget(WidgetMessage:LinkWith) => fn link(Handle<UINode<M, C>>), layout: false);
    define_constructor!(Widget(WidgetMessage:LinkWithReverse) => fn link_reverse(Handle<UINode<M, C>>), layout: false);
    define_constructor!(Widget(WidgetMessage:Background) => fn background(Brush), layout: false);
    define_constructor!(Widget(WidgetMessage:Foreground) => fn foreground(Brush), layout: false);
    define_constructor!(Widget(WidgetMessage:Visibility) => fn visibility(bool), layout: false);
    define_constructor!(Widget(WidgetMessage:Width) => fn width(f32), layout: false);
    define_constructor!(Widget(WidgetMessage:Height) => fn height(f32), layout: false);
    define_constructor!(Widget(WidgetMessage:DesiredPosition) => fn desired_position(Vector2<f32>), layout: false);
    define_constructor!(Widget(WidgetMessage:Center) => fn center(), layout: true);
    define_constructor!(Widget(WidgetMessage:TopMost) => fn topmost(), layout: false);
    define_constructor!(Widget(WidgetMessage:Enabled) => fn enabled(bool), layout: false);
    define_constructor!(Widget(WidgetMessage:Name) => fn name(String), layout: false);
    define_constructor!(Widget(WidgetMessage:Row) => fn row(usize), layout: false);
    define_constructor!(Widget(WidgetMessage:Column) => fn column(usize), layout: false);
    define_constructor!(Widget(WidgetMessage:Cursor) => fn cursor(Option<CursorIcon>), layout: false);
    define_constructor!(Widget(WidgetMessage:ZIndex) => fn z_index(usize), layout: false);
    define_constructor!(Widget(WidgetMessage:HitTestVisibility) => fn hit_test_visibility(bool), layout: false);
    define_constructor!(Widget(WidgetMessage:Margin) => fn margin(Thickness), layout: false);
    define_constructor!(Widget(WidgetMessage:MinSize) => fn min_size(Vector2<f32>), layout: false);
    define_constructor!(Widget(WidgetMessage:MaxSize) => fn max_size(Vector2<f32>), layout: false);
    define_constructor!(Widget(WidgetMessage:HorizontalAlignment) => fn horizontal_alignment(HorizontalAlignment), layout: false);
    define_constructor!(Widget(WidgetMessage:VerticalAlignment) => fn vertical_alignment(VerticalAlignment), layout: false);
    define_constructor!(Widget(WidgetMessage:Opacity) => fn opacity(f32), layout: false);

    // Internal messages. Do not use.
    define_constructor!(Widget(WidgetMessage:GotFocus) => fn got_focus(), layout: false);
    define_constructor!(Widget(WidgetMessage:LostFocus) => fn lost_focus(), layout: false);
    define_constructor!(Widget(WidgetMessage:MouseDown) => fn mouse_down(pos: Vector2<f32>, button: MouseButton), layout: false);
    define_constructor!(Widget(WidgetMessage:MouseUp) => fn mouse_up(pos: Vector2<f32>, button: MouseButton), layout: false);
    define_constructor!(Widget(WidgetMessage:MouseMove) => fn mouse_move(pos: Vector2<f32>, state: MouseState), layout: false);
    define_constructor!(Widget(WidgetMessage:MouseWheel) => fn mouse_wheel(pos: Vector2<f32>, amount: f32), layout: false);
    define_constructor!(Widget(WidgetMessage:MouseLeave) => fn mouse_leave(), layout: false);
    define_constructor!(Widget(WidgetMessage:MouseEnter) => fn mouse_enter(), layout: false);
    define_constructor!(Widget(WidgetMessage:Text) => fn text(char), layout: false);
    define_constructor!(Widget(WidgetMessage:KeyDown) => fn key_down(KeyCode), layout: false);
    define_constructor!(Widget(WidgetMessage:KeyUp) => fn key_up(KeyCode), layout: false);
    define_constructor!(Widget(WidgetMessage:DragStarted) => fn drag_started(Handle<UINode<M, C>>), layout: false);
    define_constructor!(Widget(WidgetMessage:DragOver) => fn drag_over(Handle<UINode<M, C>>), layout: false);
    define_constructor!(Widget(WidgetMessage:Drop) => fn drop(Handle<UINode<M, C>>), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum ButtonMessage<M: MessageData, C: Control<M, C>> {
    Click,
    Content(Handle<UINode<M, C>>),
}

impl<M: MessageData, C: Control<M, C>> ButtonMessage<M, C> {
    define_constructor!(Button(ButtonMessage:Click) => fn click(), layout: false);
    define_constructor!(Button(ButtonMessage:Content) => fn content(Handle<UINode<M, C>>), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScrollBarMessage {
    Value(f32),
    MinValue(f32),
    MaxValue(f32),
}

impl ScrollBarMessage {
    define_constructor_unbound!(ScrollBar(ScrollBarMessage:Value) => fn value(f32), layout: false);
    define_constructor_unbound!(ScrollBar(ScrollBarMessage:MaxValue) => fn max_value(f32), layout: false);
    define_constructor_unbound!(ScrollBar(ScrollBarMessage:MinValue) => fn min_value(f32), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckBoxMessage {
    Check(Option<bool>),
}

impl CheckBoxMessage {
    define_constructor_unbound!(CheckBox(CheckBoxMessage:Check) => fn checked(Option<bool>), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExpanderMessage {
    Expand(bool),
}

impl ExpanderMessage {
    define_constructor_unbound!(Expander(ExpanderMessage:Expand) => fn expand(bool), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum WindowMessage<M: MessageData, C: Control<M, C>> {
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
    Title(WindowTitle<M, C>),
}

impl<M: MessageData, C: Control<M, C>> WindowMessage<M, C> {
    define_constructor!(Window(WindowMessage:Open) => fn open(center: bool), layout: false);
    define_constructor!(Window(WindowMessage:OpenModal) => fn open_modal(center: bool), layout: false);
    define_constructor!(Window(WindowMessage:Close) => fn close(), layout: false);
    define_constructor!(Window(WindowMessage:Minimize) => fn minimize(bool), layout: false);
    define_constructor!(Window(WindowMessage:CanMinimize) => fn can_minimize(bool), layout: false);
    define_constructor!(Window(WindowMessage:CanClose) => fn can_close(bool), layout: false);
    define_constructor!(Window(WindowMessage:CanResize) => fn can_resize(bool), layout: false);
    define_constructor!(Window(WindowMessage:MoveStart) => fn move_start(), layout: false);
    define_constructor!(Window(WindowMessage:Move) => fn move_to(Vector2<f32>), layout: false);
    define_constructor!(Window(WindowMessage:MoveEnd) => fn move_end(), layout: false);
    define_constructor!(Window(WindowMessage:Title) => fn title(WindowTitle<M, C>), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScrollViewerMessage<M: MessageData, C: Control<M, C>> {
    Content(Handle<UINode<M, C>>),
    /// Adjusts vertical and horizontal scroll values so given node will be in "view box"
    /// of scroll viewer.
    BringIntoView(Handle<UINode<M, C>>),
}

impl<M: MessageData, C: Control<M, C>> ScrollViewerMessage<M, C> {
    define_constructor!(ScrollViewer(ScrollViewerMessage:Content) => fn content(Handle<UINode<M, C>>), layout: false);
    define_constructor!(ScrollViewer(ScrollViewerMessage:BringIntoView) => fn bring_into_view(Handle<UINode<M, C>>), layout: true);
}

#[derive(Debug, Clone, PartialEq)]
pub enum ListViewMessage<M: MessageData, C: Control<M, C>> {
    SelectionChanged(Option<usize>),
    Items(Vec<Handle<UINode<M, C>>>),
    AddItem(Handle<UINode<M, C>>),
}

impl<M: MessageData, C: Control<M, C>> ListViewMessage<M, C> {
    define_constructor!(ListView(ListViewMessage:SelectionChanged) => fn selection(Option<usize>), layout: false);
    define_constructor!(ListView(ListViewMessage:Items) => fn items(Vec<Handle<UINode<M, C>>>), layout: false);
    define_constructor!(ListView(ListViewMessage:AddItem) => fn add_item(Handle<UINode<M, C>>), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum DropdownListMessage<M: MessageData, C: Control<M, C>> {
    SelectionChanged(Option<usize>),
    Items(Vec<Handle<UINode<M, C>>>),
    AddItem(Handle<UINode<M, C>>),
}

impl<M: MessageData, C: Control<M, C>> DropdownListMessage<M, C> {
    define_constructor!(DropdownList(DropdownListMessage:SelectionChanged) => fn selection(Option<usize>), layout: false);
    define_constructor!(DropdownList(DropdownListMessage:Items) => fn items(Vec<Handle<UINode<M, C>>>), layout: false);
    define_constructor!(DropdownList(DropdownListMessage:AddItem) => fn add_item(Handle<UINode<M, C>>), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum PopupMessage<M: MessageData, C: Control<M, C>> {
    Open,
    Close,
    Content(Handle<UINode<M, C>>),
    Placement(Placement),
}

impl<M: MessageData, C: Control<M, C>> PopupMessage<M, C> {
    define_constructor!(Popup(PopupMessage:Open) => fn open(), layout: false);
    define_constructor!(Popup(PopupMessage:Close) => fn close(), layout: false);
    define_constructor!(Popup(PopupMessage:Content) => fn content(Handle<UINode<M, C>>), layout: false);
    define_constructor!(Popup(PopupMessage:Placement) => fn placement(Placement), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileSelectorMessage {
    Root(Option<PathBuf>),
    Path(PathBuf),
    Commit(PathBuf),
    Cancel,
}

impl FileSelectorMessage {
    define_constructor_unbound!(FileSelector(FileSelectorMessage:Commit) => fn commit(PathBuf), layout: false);
    define_constructor_unbound!(FileSelector(FileSelectorMessage:Root) => fn root(Option<PathBuf>), layout: false);
    define_constructor_unbound!(FileSelector(FileSelectorMessage:Path) => fn path(PathBuf), layout: false);
    define_constructor_unbound!(FileSelector(FileSelectorMessage:Cancel) => fn cancel(), layout: false);
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SelectionState(pub(in crate) bool);

#[derive(Debug, Clone, PartialEq)]
pub enum TreeMessage<M: MessageData, C: Control<M, C>> {
    Expand(bool),
    AddItem(Handle<UINode<M, C>>),
    RemoveItem(Handle<UINode<M, C>>),
    SetItems(Vec<Handle<UINode<M, C>>>),
    // Private, do not use. For internal needs only. Use TreeRootMessage::Selected.
    Select(SelectionState),
}

impl<M: MessageData, C: Control<M, C>> TreeMessage<M, C> {
    define_constructor!(Tree(TreeMessage:AddItem) => fn add_item(Handle<UINode<M, C>>), layout: false);
    define_constructor!(Tree(TreeMessage:RemoveItem) => fn remove_item(Handle<UINode<M, C>>), layout: false);
    define_constructor!(Tree(TreeMessage:SetItems) => fn set_items(Vec<Handle<UINode<M, C>>>), layout: false);
    define_constructor!(Tree(TreeMessage:Expand) => fn expand(bool), layout: false);

    pub(in crate) fn select(
        destination: Handle<UINode<M, C>>,
        direction: MessageDirection,
        select: bool,
    ) -> UiMessage<M, C> {
        UiMessage {
            handled: Cell::new(false),
            data: UiMessageData::Tree(TreeMessage::Select(SelectionState(select))),
            destination,
            direction,
            perform_layout: Cell::new(false),
            flags: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TreeRootMessage<M: MessageData, C: Control<M, C>> {
    AddItem(Handle<UINode<M, C>>),
    RemoveItem(Handle<UINode<M, C>>),
    Items(Vec<Handle<UINode<M, C>>>),
    Selected(Vec<Handle<UINode<M, C>>>),
}

impl<M: MessageData, C: Control<M, C>> TreeRootMessage<M, C> {
    define_constructor!(TreeRoot(TreeRootMessage:AddItem) => fn add_item(Handle<UINode<M, C>>), layout: false);
    define_constructor!(TreeRoot(TreeRootMessage:RemoveItem) => fn remove_item(Handle<UINode<M, C>>), layout: false);
    define_constructor!(TreeRoot(TreeRootMessage:Items) => fn items(Vec<Handle<UINode<M, C>>>), layout: false);
    define_constructor!(TreeRoot(TreeRootMessage:Selected) => fn select(Vec<Handle<UINode<M, C>>>), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileBrowserMessage {
    Root(Option<PathBuf>),
    Path(PathBuf),
}

impl FileBrowserMessage {
    define_constructor_unbound!(FileBrowser(FileBrowserMessage:Root) => fn root(Option<PathBuf>), layout: false);
    define_constructor_unbound!(FileBrowser(FileBrowserMessage:Path) => fn path(PathBuf), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextBoxMessage {
    Text(String),
}

impl TextBoxMessage {
    define_constructor_unbound!(TextBox(TextBoxMessage:Text) => fn text(String), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextMessage {
    Text(String),
    Wrap(bool),
    Font(SharedFont),
    VerticalAlignment(VerticalAlignment),
    HorizontalAlignment(HorizontalAlignment),
}

impl TextMessage {
    define_constructor_unbound!(Text(TextMessage:Text) => fn text(String), layout: false);
    define_constructor_unbound!(Text(TextMessage:Wrap) => fn wrap(bool), layout: false);
    define_constructor_unbound!(Text(TextMessage:Font) => fn font(SharedFont), layout: false);
    define_constructor_unbound!(Text(TextMessage:VerticalAlignment) => fn vertical_alignment(VerticalAlignment), layout: false);
    define_constructor_unbound!(Text(TextMessage:HorizontalAlignment) => fn horizontal_alignment(HorizontalAlignment), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImageMessage {
    Texture(Option<SharedTexture>),
    Flip(bool),
}

impl ImageMessage {
    define_constructor_unbound!(Image(ImageMessage:Texture) => fn texture(Option<SharedTexture>), layout: false);
    define_constructor_unbound!(Image(ImageMessage:Flip) => fn flip(bool), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum TileMessage<M: MessageData, C: Control<M, C>> {
    Content(TileContent<M, C>),
    /// Internal. Do not use.
    Split {
        window: Handle<UINode<M, C>>,
        direction: SplitDirection,
        first: bool,
    },
}

impl<M: MessageData, C: Control<M, C>> TileMessage<M, C> {
    define_constructor!(Tile(TileMessage:Content) => fn content(TileContent<M, C>), layout: false);

    pub(in crate) fn split(
        destination: Handle<UINode<M, C>>,
        direction: MessageDirection,
        window: Handle<UINode<M, C>>,
        split_direction: SplitDirection,
        first: bool,
    ) -> UiMessage<M, C> {
        UiMessage {
            handled: Cell::new(false),
            data: UiMessageData::Tile(TileMessage::Split {
                window,
                direction: split_direction,
                first,
            }),
            destination,
            direction,
            perform_layout: Cell::new(false),
            flags: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NumericUpDownMessage {
    Value(f32),
}

impl NumericUpDownMessage {
    define_constructor_unbound!(NumericUpDown(NumericUpDownMessage:Value) => fn value(f32), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum Vec3EditorMessage {
    Value(Vector3<f32>),
}

impl Vec3EditorMessage {
    define_constructor_unbound!(Vec3Editor(Vec3EditorMessage:Value) => fn value(Vector3<f32>), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScrollPanelMessage<M: MessageData, C: Control<M, C>> {
    VerticalScroll(f32),
    HorizontalScroll(f32),
    /// Adjusts vertical and horizontal scroll values so given node will be in "view box"
    /// of scroll panel.
    BringIntoView(Handle<UINode<M, C>>),
}

impl<M: MessageData, C: Control<M, C>> ScrollPanelMessage<M, C> {
    define_constructor!(ScrollPanel(ScrollPanelMessage:VerticalScroll) => fn vertical_scroll(f32), layout: false);
    define_constructor!(ScrollPanel(ScrollPanelMessage:HorizontalScroll) => fn horizontal_scroll(f32), layout: false);
    define_constructor!(ScrollPanel(ScrollPanelMessage:BringIntoView) => fn bring_into_view(Handle<UINode<M, C>>), layout: true);
}

#[derive(Debug, Clone, PartialEq)]
pub enum MenuMessage {
    Activate,
    Deactivate,
}

impl MenuMessage {
    define_constructor_unbound!(Menu(MenuMessage:Activate) => fn activate(), layout: false);
    define_constructor_unbound!(Menu(MenuMessage:Deactivate) => fn deactivate(), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum MenuItemMessage {
    Open,
    Close,
    Click,
}

impl MenuItemMessage {
    define_constructor_unbound!(MenuItem(MenuItemMessage:Open) => fn open(), layout: false);
    define_constructor_unbound!(MenuItem(MenuItemMessage:Close) => fn close(), layout: false);
    define_constructor_unbound!(MenuItem(MenuItemMessage:Click) => fn click(), layout: false);
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
    define_constructor_unbound!(MessageBox(MessageBoxMessage:Open) => fn open(title: Option<String>, text: Option<String>), layout: false);
    define_constructor_unbound!(MessageBox(MessageBoxMessage:Close) => fn close(MessageBoxResult), layout: false);
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
    define_constructor_unbound!(Decorator(DecoratorMessage:Select) => fn select(bool), layout: false);
    define_constructor_unbound!(Decorator(DecoratorMessage:HoverBrush) => fn hover_brush(Brush), layout: false);
    define_constructor_unbound!(Decorator(DecoratorMessage:NormalBrush) => fn normal_brush(Brush), layout: false);
    define_constructor_unbound!(Decorator(DecoratorMessage:PressedBrush) => fn pressed_brush(Brush), layout: false);
    define_constructor_unbound!(Decorator(DecoratorMessage:SelectedBrush) => fn selected_brush(Brush), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProgressBarMessage {
    Progress(f32),
}

impl ProgressBarMessage {
    define_constructor_unbound!(ProgressBar(ProgressBarMessage:Progress) => fn progress(f32), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum HueBarMessage {
    /// Sets new hue value.
    Hue(f32),

    /// Sets new orientation
    Orientation(Orientation),
}

impl HueBarMessage {
    define_constructor_unbound!(HueBar(HueBarMessage:Hue) => fn hue(f32), layout: false);
    define_constructor_unbound!(HueBar(HueBarMessage:Orientation) => fn orientation(Orientation), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum AlphaBarMessage {
    /// Sets new hue value.
    Alpha(f32),

    /// Sets new orientation
    Orientation(Orientation),
}

impl AlphaBarMessage {
    define_constructor_unbound!(AlphaBar(AlphaBarMessage:Alpha) => fn alpha(f32), layout: false);
    define_constructor_unbound!(AlphaBar(AlphaBarMessage:Orientation) => fn orientation(Orientation), layout: false);
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
    define_constructor_unbound!(SaturationBrightnessField(SaturationBrightnessFieldMessage:Hue) => fn hue(f32), layout: false);
    define_constructor_unbound!(SaturationBrightnessField(SaturationBrightnessFieldMessage:Saturation) => fn saturation(f32), layout: false);
    define_constructor_unbound!(SaturationBrightnessField(SaturationBrightnessFieldMessage:Brightness) => fn brightness(f32), layout: false);
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
    define_constructor_unbound!(ColorPicker(ColorPickerMessage:Color) => fn color(Color), layout: false);
    define_constructor_unbound!(ColorPicker(ColorPickerMessage:Hsv) => fn hsv(Hsv), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColorFieldMessage {
    Color(Color),
}

impl ColorFieldMessage {
    define_constructor_unbound!(ColorField(ColorFieldMessage:Color) => fn color(Color), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum UiMessageData<M: MessageData, C: Control<M, C>> {
    Widget(WidgetMessage<M, C>),
    Button(ButtonMessage<M, C>),
    ScrollBar(ScrollBarMessage),
    CheckBox(CheckBoxMessage),
    Window(WindowMessage<M, C>),
    ListView(ListViewMessage<M, C>),
    DropdownList(DropdownListMessage<M, C>),
    Popup(PopupMessage<M, C>),
    ScrollViewer(ScrollViewerMessage<M, C>),
    Tree(TreeMessage<M, C>),
    TreeRoot(TreeRootMessage<M, C>),
    FileBrowser(FileBrowserMessage),
    FileSelector(FileSelectorMessage),
    TextBox(TextBoxMessage),
    NumericUpDown(NumericUpDownMessage),
    Vec3Editor(Vec3EditorMessage),
    Menu(MenuMessage),
    MenuItem(MenuItemMessage),
    MessageBox(MessageBoxMessage),
    Decorator(DecoratorMessage),
    Text(TextMessage),
    ScrollPanel(ScrollPanelMessage<M, C>),
    Tile(TileMessage<M, C>),
    ProgressBar(ProgressBarMessage),
    Image(ImageMessage),
    HueBar(HueBarMessage),
    AlphaBar(AlphaBarMessage),
    ColorPicker(ColorPickerMessage),
    ColorField(ColorFieldMessage),
    Expander(ExpanderMessage),
    SaturationBrightnessField(SaturationBrightnessFieldMessage),
    User(M),
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

pub trait MessageData: 'static + Debug + Clone + PartialEq {}

/// Message is basic communication element that is used to deliver information to UI nodes
/// or to user code.
#[derive(Debug, Clone, PartialEq)]
pub struct UiMessage<M: MessageData, C: Control<M, C>> {
    /// Useful flag to check if a message was already handled.
    handled: Cell<bool>,

    /// Actual message data. Use pattern matching to get type specific data.
    data: UiMessageData<M, C>,

    /// Handle of node that will receive message. Please note that all nodes in hierarchy will
    /// also receive this message, order is "up-on-tree".
    destination: Handle<UINode<M, C>>,

    /// Indicates the direction of the message.
    ///
    /// See [MessageDirection](enum.MessageDirection.html) for details.
    direction: MessageDirection,

    /// Whether or not message requires layout to be calculated first.
    ///
    /// Some of message handling routines uses layout info, but message loop
    /// performed right after layout pass, but some of messages may change
    /// layout and this flag tells UI to perform layout before passing message
    /// further. In ideal case we'd perform layout after **each** message, but
    /// since layout pass is super heavy we should do it **only** when it is
    /// actually needed.
    perform_layout: Cell<bool>,

    /// A custom user flags.
    pub flags: u64,
}

impl<M: MessageData, C: Control<M, C>> UiMessage<M, C> {
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

    pub fn destination(&self) -> Handle<UINode<M, C>> {
        self.destination
    }

    pub fn data(&self) -> &UiMessageData<M, C> {
        &self.data
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

    /// Allows you to construct a new user-defined message.
    pub fn user(destination: Handle<UINode<M, C>>, direction: MessageDirection, msg: M) -> Self {
        Self {
            handled: Cell::new(false),
            data: UiMessageData::User(msg),
            destination,
            direction,
            perform_layout: Cell::new(false),
            flags: 0,
        }
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

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
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

impl Default for KeyboardModifiers {
    fn default() -> Self {
        Self {
            alt: false,
            shift: false,
            control: false,
            system: false,
        }
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
