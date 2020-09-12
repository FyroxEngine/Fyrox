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
        math::{vec2::Vec2, vec3::Vec3},
        pool::Handle,
    },
    dock::TileContent,
    draw::Texture,
    messagebox::MessageBoxResult,
    popup::Placement,
    ttf::Font,
    window::WindowTitle,
    Control, HorizontalAlignment, MouseState, Thickness, UINode, VerticalAlignment,
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

macro_rules! define_constructor {
    ($var:tt($inner:ident : $inner_var:tt) => fn $name:ident()) => {
        pub fn $name(destination: Handle<UINode<M, C>>) -> UiMessage<M, C> {
            UiMessage {
                handled: false,
                data: UiMessageData::$var($inner::$inner_var),
                destination,
            }
        }
    };

    ($var:tt($inner:ident : $inner_var:tt) => fn $name:ident($typ:ty)) => {
        pub fn $name(destination: Handle<UINode<M, C>>, value:$typ) -> UiMessage<M, C> {
            UiMessage {
                handled: false,
                data: UiMessageData::$var($inner::$inner_var(value)),
                destination,
            }
        }
    };

    ($var:tt($inner:ident : $inner_var:tt) => fn $name:ident( $($params:ident : $types:ty),+ )) => {
        pub fn $name(destination: Handle<UINode<M, C>>, $($params : $types),+) -> UiMessage<M, C> {
            UiMessage {
                handled: false,
                data: UiMessageData::$var($inner::$inner_var { $($params),+ } ),
                destination,
            }
        }
    }
}

macro_rules! define_constructor_unbound {
    ($var:tt($inner:ident : $inner_var:tt) => fn $name:ident()) => {
        pub fn $name<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>) -> UiMessage<M, C> {
            UiMessage {
                handled: false,
                data: UiMessageData::$var($inner::$inner_var),
                destination,
            }
        }
    };

    ($var:tt($inner:ident : $inner_var:tt) => fn $name:ident($typ:ty)) => {
        pub fn $name<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, value:$typ) -> UiMessage<M, C> {
            UiMessage {
                handled: false,
                data: UiMessageData::$var($inner::$inner_var(value)),
                destination,
            }
        }
    };

    ($var:tt($inner:ident : $inner_var:tt) => fn $name:ident( $($params:ident : $types:ty),+ )) => {
        pub fn $name<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, $($params : $types),+) -> UiMessage<M, C> {
            UiMessage {
                handled: false,
                data: UiMessageData::$var($inner::$inner_var { $($params),+ } ),
                destination,
            }
        }
    }
}

/// A set of messages for any kind of widgets (including user controls). These messages provides basic
/// communication elements of the UI library.
#[derive(Debug, Clone)]
pub enum WidgetMessage<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> {
    /// Initiated when user clicks on a widget's geometry.
    ///
    /// Direction: **From UI**.
    MouseDown {
        /// Position of cursor.
        pos: Vec2,
        /// A button that was pressed.
        button: MouseButton,
    },

    /// Initiated when user releases mouse button while cursor is over widget's geometry.
    ///
    /// Direction: **From UI**.
    MouseUp {
        /// Position of cursor.
        pos: Vec2,
        /// A button that was released.
        button: MouseButton,
    },

    /// Initiated when user moves cursor over widget's geometry.
    ///
    /// Direction: **From/To UI**.
    MouseMove {
        /// New position of cursor in screen coordinates.
        pos: Vec2,
        /// State of mouse buttons.
        state: MouseState,
    },

    /// Initiated when user scrolls mouse wheel while cursor is over widget's geometry.
    ///
    /// Direction: **From/To UI**.
    MouseWheel {
        /// Position of cursor.
        pos: Vec2,
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
    MaxSize(Vec2),

    /// A request to set minimum size of widget. Minimum size restricts size of a widget during layout pass. For example
    /// you can set minimum size to a button which was placed into a grid's cell, if minimum size wouldn't be set, button
    /// would be compressed to fill entire cell.
    ///
    /// Direction: **From/To UI**
    MinSize(Vec2),

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
    DesiredPosition(Vec2),

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
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> WidgetMessage<M, C> {
    define_constructor!(Widget(WidgetMessage:Remove) => fn remove());
    define_constructor!(Widget(WidgetMessage:Unlink) => fn unlink());
    define_constructor!(Widget(WidgetMessage:LinkWith) => fn link(Handle<UINode<M, C>>));
    define_constructor!(Widget(WidgetMessage:Background) => fn background(Brush));
    define_constructor!(Widget(WidgetMessage:Visibility) => fn visibility(bool));
    define_constructor!(Widget(WidgetMessage:Width) => fn width(f32));
    define_constructor!(Widget(WidgetMessage:Height) => fn height(f32));
    define_constructor!(Widget(WidgetMessage:DesiredPosition) => fn desired_position(Vec2));
    define_constructor!(Widget(WidgetMessage:Center) => fn center());
    define_constructor!(Widget(WidgetMessage:TopMost) => fn topmost());
    define_constructor!(Widget(WidgetMessage:Enabled) => fn enabled(bool));
    // TODO: Add rest items.
}

#[derive(Debug, Clone)]
pub enum ButtonMessage<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> {
    Click,
    Content(Handle<UINode<M, C>>),
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> ButtonMessage<M, C> {
    define_constructor!(Button(ButtonMessage:Click) => fn click());
    define_constructor!(Button(ButtonMessage:Content) => fn content(Handle<UINode<M, C>>));
}

#[derive(Debug, Clone)]
pub enum ScrollBarMessage {
    Value(f32),
    MinValue(f32),
    MaxValue(f32),
}

impl ScrollBarMessage {
    define_constructor_unbound!(ScrollBar(ScrollBarMessage:Value) => fn value(f32));
    define_constructor_unbound!(ScrollBar(ScrollBarMessage:MaxValue) => fn max_value(f32));
    define_constructor_unbound!(ScrollBar(ScrollBarMessage:MinValue) => fn min_value(f32));
}

#[derive(Debug, Clone)]
pub enum CheckBoxMessage {
    Check(Option<bool>),
}

impl CheckBoxMessage {
    define_constructor_unbound!(CheckBox(CheckBoxMessage:Check) => fn checked(Option<bool>));
}

#[derive(Debug, Clone)]
pub enum WindowMessage<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> {
    Open,
    OpenModal,
    Close,
    Minimize(bool),
    CanMinimize(bool),
    CanClose(bool),
    MoveStart,
    /// New position is in local coordinates.
    Move(Vec2),
    MoveEnd,
    Title(WindowTitle<M, C>),
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> WindowMessage<M, C> {
    define_constructor!(Window(WindowMessage:Open) => fn open());
    define_constructor!(Window(WindowMessage:OpenModal) => fn open_modal());
    define_constructor!(Window(WindowMessage:Close) => fn close());
    define_constructor!(Window(WindowMessage:Minimize) => fn minimize(bool));
    define_constructor!(Window(WindowMessage:CanMinimize) => fn can_minimize(bool));
    define_constructor!(Window(WindowMessage:CanClose) => fn can_close(bool));
    define_constructor!(Window(WindowMessage:MoveStart) => fn move_start());
    define_constructor!(Window(WindowMessage:Move) => fn move_to(Vec2));
    define_constructor!(Window(WindowMessage:MoveEnd) => fn move_end());
    define_constructor!(Window(WindowMessage:Title) => fn title(WindowTitle<M, C>));
}

#[derive(Debug, Clone)]
pub enum ScrollViewerMessage<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> {
    Content(Handle<UINode<M, C>>),
    /// Adjusts vertical and horizontal scroll values so given node will be in "view box"
    /// of scroll viewer.
    BringIntoView(Handle<UINode<M, C>>),
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> ScrollViewerMessage<M, C> {
    define_constructor!(ScrollViewer(ScrollViewerMessage:Content) => fn content(Handle<UINode<M, C>>));
    define_constructor!(ScrollViewer(ScrollViewerMessage:BringIntoView) => fn bring_into_view(Handle<UINode<M, C>>));
}

#[derive(Debug, Clone)]
pub enum ListViewMessage<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> {
    SelectionChanged(Option<usize>),
    Items(Vec<Handle<UINode<M, C>>>),
    AddItem(Handle<UINode<M, C>>),
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> ListViewMessage<M, C> {
    define_constructor!(ListView(ListViewMessage:SelectionChanged) => fn selection(Option<usize>));
    define_constructor!(ListView(ListViewMessage:Items) => fn items(Vec<Handle<UINode<M, C>>>));
    define_constructor!(ListView(ListViewMessage:AddItem) => fn add_item(Handle<UINode<M, C>>));
}

#[derive(Debug, Clone)]
pub enum DropdownListMessage<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> {
    SelectionChanged(Option<usize>),
    Items(Vec<Handle<UINode<M, C>>>),
    AddItem(Handle<UINode<M, C>>),
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> DropdownListMessage<M, C> {
    define_constructor!(DropdownList(DropdownListMessage:SelectionChanged) => fn selection(Option<usize>));
    define_constructor!(DropdownList(DropdownListMessage:Items) => fn items(Vec<Handle<UINode<M, C>>>));
    define_constructor!(DropdownList(DropdownListMessage:AddItem) => fn add_item(Handle<UINode<M, C>>));
}

#[derive(Debug, Clone)]
pub enum PopupMessage<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> {
    Open,
    Close,
    Content(Handle<UINode<M, C>>),
    Placement(Placement),
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> PopupMessage<M, C> {
    define_constructor!(Popup(PopupMessage:Open) => fn open());
    define_constructor!(Popup(PopupMessage:Close) => fn close());
    define_constructor!(Popup(PopupMessage:Content) => fn content(Handle<UINode<M, C>>));
    define_constructor!(Popup(PopupMessage:Placement) => fn placement(Placement));
}

#[derive(Debug, Clone)]
pub enum FileSelectorMessage {
    Path(PathBuf),
    Commit(PathBuf),
    Cancel,
}

impl FileSelectorMessage {
    define_constructor_unbound!(FileSelector(FileSelectorMessage:Commit) => fn commit(PathBuf));
    define_constructor_unbound!(FileSelector(FileSelectorMessage:Path) => fn path(PathBuf));
    define_constructor_unbound!(FileSelector(FileSelectorMessage:Cancel) => fn cancel());
}

#[derive(Debug, Copy, Clone)]
pub struct SelectionState(pub(in crate) bool);

#[derive(Debug, Clone)]
pub enum TreeMessage<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> {
    Expand(bool),
    AddItem(Handle<UINode<M, C>>),
    RemoveItem(Handle<UINode<M, C>>),
    SetItems(Vec<Handle<UINode<M, C>>>),
    // Private, do not use. For internal needs only. Use TreeRootMessage::Selected.
    Select(SelectionState),
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> TreeMessage<M, C> {
    define_constructor!(Tree(TreeMessage:AddItem) => fn add_item(Handle<UINode<M, C>>));
    define_constructor!(Tree(TreeMessage:RemoveItem) => fn remove_item(Handle<UINode<M, C>>));
    define_constructor!(Tree(TreeMessage:SetItems) => fn set_items(Vec<Handle<UINode<M, C>>>));
    define_constructor!(Tree(TreeMessage:Expand) => fn expand(bool));

    pub(in crate) fn select(destination: Handle<UINode<M, C>>, select: bool) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::Tree(TreeMessage::Select(SelectionState(select))),
            destination,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TreeRootMessage<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> {
    AddItem(Handle<UINode<M, C>>),
    RemoveItem(Handle<UINode<M, C>>),
    Items(Vec<Handle<UINode<M, C>>>),
    Selected(Vec<Handle<UINode<M, C>>>),
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> TreeRootMessage<M, C> {
    define_constructor!(TreeRoot(TreeRootMessage:AddItem) => fn add_item(Handle<UINode<M, C>>));
    define_constructor!(TreeRoot(TreeRootMessage:RemoveItem) => fn remove_item(Handle<UINode<M, C>>));
    define_constructor!(TreeRoot(TreeRootMessage:Items) => fn items(Vec<Handle<UINode<M, C>>>));
    define_constructor!(TreeRoot(TreeRootMessage:Selected) => fn select(Vec<Handle<UINode<M, C>>>));
}

#[derive(Debug, Clone)]
pub enum FileBrowserMessage {
    Path(PathBuf),
}

impl FileBrowserMessage {
    define_constructor_unbound!(FileBrowser(FileBrowserMessage:Path) => fn path(PathBuf));
}

#[derive(Debug, Clone)]
pub enum TextBoxMessage {
    Text(String),
}

impl TextBoxMessage {
    define_constructor_unbound!(TextBox(TextBoxMessage:Text) => fn text(String));
}

#[derive(Debug, Clone)]
pub enum TextMessage {
    Text(String),
    Wrap(bool),
    Font(Arc<Mutex<Font>>),
    VerticalAlignment(VerticalAlignment),
    HorizontalAlignment(HorizontalAlignment),
}

impl TextMessage {
    define_constructor_unbound!(Text(TextMessage:Text) => fn text(String));
    define_constructor_unbound!(Text(TextMessage:Wrap) => fn wrap(bool));
    define_constructor_unbound!(Text(TextMessage:Font) => fn font(Arc<Mutex<Font>>));
    define_constructor_unbound!(Text(TextMessage:VerticalAlignment) => fn vertical_alignment(VerticalAlignment));
    define_constructor_unbound!(Text(TextMessage:HorizontalAlignment) => fn horizontal_alignment(HorizontalAlignment));
}

#[derive(Debug, Clone)]
pub enum ImageMessage {
    Texture(Option<Arc<Texture>>),
    Flip(bool),
}

impl ImageMessage {
    define_constructor_unbound!(Image(ImageMessage:Texture) => fn texture(Option<Arc<Texture>>));
    define_constructor_unbound!(Image(ImageMessage:Flip) => fn flip(bool));
}

#[derive(Debug, Clone)]
pub enum TileMessage<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> {
    Content(TileContent<M, C>),
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> TileMessage<M, C> {
    define_constructor!(Tile(TileMessage:Content) => fn content(TileContent<M, C>));
}

#[derive(Debug, Clone)]
pub enum NumericUpDownMessage {
    Value(f32),
}

impl NumericUpDownMessage {
    define_constructor_unbound!(NumericUpDown(NumericUpDownMessage:Value) => fn value(f32));
}

#[derive(Debug, Clone)]
pub enum Vec3EditorMessage {
    Value(Vec3),
}

impl Vec3EditorMessage {
    define_constructor_unbound!(Vec3Editor(Vec3EditorMessage:Value) => fn value(Vec3));
}

#[derive(Debug, Clone)]
pub enum ScrollPanelMessage<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> {
    VerticalScroll(f32),
    HorizontalScroll(f32),
    /// Adjusts vertical and horizontal scroll values so given node will be in "view box"
    /// of scroll panel.
    BringIntoView(Handle<UINode<M, C>>),
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> ScrollPanelMessage<M, C> {
    define_constructor!(ScrollPanel(ScrollPanelMessage:VerticalScroll) => fn vertical_scroll(f32));
    define_constructor!(ScrollPanel(ScrollPanelMessage:HorizontalScroll) => fn horizontal_scroll(f32));
    define_constructor!(ScrollPanel(ScrollPanelMessage:BringIntoView) => fn bring_into_view(Handle<UINode<M, C>>));
}

#[derive(Debug, Clone)]
pub enum MenuMessage {
    Activate,
    Deactivate,
}

impl MenuMessage {
    define_constructor_unbound!(Menu(MenuMessage:Activate) => fn activate());
    define_constructor_unbound!(Menu(MenuMessage:Deactivate) => fn deactivate());
}

#[derive(Debug, Clone)]
pub enum MenuItemMessage {
    Open,
    Close,
    Click,
}

impl MenuItemMessage {
    define_constructor_unbound!(MenuItem(MenuItemMessage:Open) => fn open());
    define_constructor_unbound!(MenuItem(MenuItemMessage:Close) => fn close());
    define_constructor_unbound!(MenuItem(MenuItemMessage:Click) => fn click());
}

#[derive(Debug, Clone)]
pub enum MessageBoxMessage {
    Open {
        title: Option<String>,
        text: Option<String>,
    },
    Close(MessageBoxResult),
}

impl MessageBoxMessage {
    define_constructor_unbound!(MessageBox(MessageBoxMessage:Open) => fn open(title: Option<String>, text: Option<String>));
    define_constructor_unbound!(MessageBox(MessageBoxMessage:Close) => fn close(MessageBoxResult));
}

#[derive(Debug, Clone)]
pub enum DecoratorMessage {
    Select(bool),
}

impl DecoratorMessage {
    define_constructor_unbound!(Decorator(DecoratorMessage:Select) => fn select(bool));
}

#[derive(Debug, Clone)]
pub enum ProgressBarMessage {
    Progress(f32),
}

impl ProgressBarMessage {
    define_constructor_unbound!(ProgressBar(ProgressBarMessage:Progress) => fn select(f32));
}

#[derive(Debug, Clone)]
pub enum UiMessageData<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> {
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
    User(M),
}

/// Message is basic communication element that is used to deliver information to UI nodes
/// or to user code.
#[derive(Debug)]
pub struct UiMessage<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> {
    /// Useful flag to check if a message was already handled.
    pub handled: bool,

    /// Actual message data. Use pattern matching to get type specific data.
    ///
    /// # Notes
    ///
    /// This field should be read-only.
    pub data: UiMessageData<M, C>,

    /// Handle of node that will receive message. Please note that all nodes in hierarchy will
    /// also receive this message, order is "up-on-tree".
    ///
    /// # Notes
    ///
    /// This field should be read-only.
    pub destination: Handle<UINode<M, C>>,
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> UiMessage<M, C> {
    #[must_use = "method creates new value which must be used"]
    pub fn reverse(&self) -> Self {
        Self {
            handled: self.handled,
            data: self.data.clone(),
            destination: self.destination,
            // direction: self.destination.reverse(),
        }
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
    Other(u8),
}

pub enum OsEvent {
    MouseInput {
        button: MouseButton,
        state: ButtonState,
    },
    CursorMoved {
        position: Vec2,
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
    Add,
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
    Decimal,
    Divide,
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
    Multiply,
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
    Subtract,
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
