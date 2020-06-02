//! Message and events module contains all possible widget messages and OS events.
//!
//! This UI library uses message passing mechanism to communicate with widgets.
//! This is very simple and more or less reliable mechanism that effectively
//! decouples widgets from each other. However message passing is very restrictive
//! by itself and it is mixed together with a bit of imperative style where you
//! modify widgets directly by calling appropriate method.

use crate::{
    core::{
        math::{
            vec2::Vec2,
            vec3::Vec3
        },
        pool::Handle,
    },
    UINode,
    VerticalAlignment,
    HorizontalAlignment,
    Thickness,
    brush::Brush,
    Control,
    popup::Placement,
    MouseState,
    messagebox::MessageBoxResult
};
use std::path::PathBuf;
use crate::window::WindowTitle;

#[derive(Debug)]
pub enum WidgetProperty {
    Background(Brush),
    Foreground(Brush),
    Name(String),
    Width(f32),
    Height(f32),
    VerticalAlignment(VerticalAlignment),
    HorizontalAlignment(HorizontalAlignment),
    MaxSize(Vec2),
    MinSize(Vec2),
    Row(usize),
    Column(usize),
    Margin(Thickness),
    HitTestVisibility(bool),
    Visibility(bool),
    ZIndex(usize),
    DesiredPosition(Vec2),
}

#[derive(Debug)]
pub enum WidgetMessage<M: 'static, C: 'static + Control<M, C>> {
    MouseDown {
        pos: Vec2,
        button: MouseButton,
    },
    MouseUp {
        pos: Vec2,
        button: MouseButton,
    },
    MouseMove {
        pos: Vec2,
        state: MouseState,
    },
    Text(char),
    KeyDown(KeyCode),
    KeyUp(KeyCode),
    MouseWheel {
        pos: Vec2,
        amount: f32,
    },
    GotFocus,
    LostFocus,
    MouseLeave,
    MouseEnter,
    TopMost,
    Unlink,
    Remove,
    LinkWith(Handle<UINode<M, C>>),
    Property(WidgetProperty),
    DragStarted(Handle<UINode<M, C>>),
    DragOver(Handle<UINode<M, C>>),
    Drop(Handle<UINode<M, C>>),

    /// Set desired position at center in local coordinates.
    Center,
}

#[derive(Debug)]
pub enum ButtonMessage<M: 'static, C: 'static + Control<M, C>> {
    Click,
    Content(Handle<UINode<M, C>>),
}

#[derive(Debug)]
pub enum ScrollBarMessage {
    Value(f32),
    MinValue(f32),
    MaxValue(f32),
}

#[derive(Debug)]
pub enum CheckBoxMessage {
    Check(Option<bool>),
}

#[derive(Debug)]
pub enum WindowMessage<M: 'static, C: 'static + Control<M, C>> {
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
    Title(WindowTitle<M, C>)
}

#[derive(Debug)]
pub enum ScrollViewerMessage<M: 'static, C: 'static + Control<M, C>> {
    Content(Handle<UINode<M, C>>)
}

#[derive(Debug)]
pub enum ListViewMessage<M: 'static, C: 'static + Control<M, C>> {
    SelectionChanged(Option<usize>),
    Items(Vec<Handle<UINode<M, C>>>),
    AddItem(Handle<UINode<M, C>>),
}

#[derive(Debug)]
pub enum PopupMessage<M: 'static, C: 'static + Control<M, C>> {
    Open,
    Close,
    Content(Handle<UINode<M, C>>),
    Placement(Placement),
}

#[derive(Debug)]
pub enum TreeMessage<M: 'static, C: 'static + Control<M, C>> {
    Expand(bool),
    AddItem(Handle<UINode<M, C>>),
    RemoveItem(Handle<UINode<M, C>>),
    SetItems(Vec<Handle<UINode<M, C>>>),
}

#[derive(Debug)]
pub enum TreeRootMessage<M: 'static, C: 'static + Control<M, C>> {
    AddItem(Handle<UINode<M, C>>),
    RemoveItem(Handle<UINode<M, C>>),
    SetItems(Vec<Handle<UINode<M, C>>>),
    SetSelected(Handle<UINode<M, C>>),
}

#[derive(Debug)]
pub enum FileBrowserMessage {
    Path(PathBuf),
    SelectionChanged(PathBuf),
}

#[derive(Debug)]
pub enum TextBoxMessage {
    Text(String)
}

#[derive(Debug)]
pub enum NumericUpDownMessage {
    Value(f32),
}

#[derive(Debug)]
pub enum Vec3EditorMessage {
    Value(Vec3)
}

#[derive(Debug)]
pub enum MenuMessage {
    Activate,
    Deactivate
}

#[derive(Debug)]
pub enum MenuItemMessage {
    Open,
    Close,
    Click
}

#[derive(Debug)]
pub enum MessageBoxMessage {
    Open {
        title: Option<String>,
        text: Option<String>,
    },
    Close(MessageBoxResult),
}

#[derive(Debug)]
pub enum UiMessageData<M: 'static, C: 'static + Control<M, C>> {
    Widget(WidgetMessage<M, C>),
    Button(ButtonMessage<M, C>),
    ScrollBar(ScrollBarMessage),
    CheckBox(CheckBoxMessage),
    Window(WindowMessage<M, C>),
    ListView(ListViewMessage<M, C>),
    Popup(PopupMessage<M, C>),
    ScrollViewer(ScrollViewerMessage<M, C>),
    Tree(TreeMessage<M, C>),
    TreeRoot(TreeRootMessage<M, C>),
    FileBrowser(FileBrowserMessage),
    TextBox(TextBoxMessage),
    NumericUpDown(NumericUpDownMessage),
    Vec3Editor(Vec3EditorMessage),
    Menu(MenuMessage),
    MenuItem(MenuItemMessage),
    MessageBox(MessageBoxMessage),
    User(M),
}

/// Event is basic communication element that is used to deliver information to UI nodes
/// or to user code.
#[derive(Debug)]
pub struct UiMessage<M: 'static, C: 'static + Control<M, C>> {
    /// Useful flag to check if a message was already handled.
    pub handled: bool,

    /// Actual message data. Use pattern matching to get node-specific data.
    ///
    /// # Notes
    ///
    /// This field should be read-only.
    pub data: UiMessageData<M, C>,

    /// Handle of node that will receive message. Please note that all nodes in hierarchy will
    /// also receive this message, order is defined by routing strategy.
    ///
    /// # Notes
    ///
    /// This field should be read-only.
    pub destination: Handle<UINode<M, C>>,
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
        position: Vec2
    },
    KeyboardInput {
        button: KeyCode,
        state: ButtonState,
    },
    Character(char),
    MouseWheel(f32, f32),
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