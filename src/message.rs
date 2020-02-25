//! Message and events module contains all possible widget messages and OS events.
//!
//! This UI library uses message passing mechanism to communicate with widgets.
//! This is very simple and more or less reliable mechanism that effectively
//! decouples widgets from each other. However message passing is very restrictive
//! by itself and it is mixed together with a bit of imperative style where you
//! modify widgets directly by calling appropriate method.

use crate::{
    core::{
        math::vec2::Vec2,
        pool::Handle,
    },
    UINode,
    VerticalAlignment,
    HorizontalAlignment,
    Thickness,
    brush::Brush,
    Control,
    popup::Placement
};

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
}

#[derive(Debug)]
pub enum WidgetMessage {
    MouseDown {
        pos: Vec2,
        button: MouseButton,
    },
    MouseUp {
        pos: Vec2,
        button: MouseButton,
    },
    MouseMove(Vec2),
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
    Property(WidgetProperty)
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
    Checked(Option<bool>),
}

#[derive(Debug)]
pub enum WindowMessage {
    Opened,
    Closed,
    Minimized(bool),
    CanMinimize(bool),
    CanClose(bool),
}

#[derive(Debug)]
pub enum ScrollViewerMessage<M: 'static, C: 'static + Control<M, C>> {
    Content(Handle<UINode<M, C>>)
}

#[derive(Debug)]
pub enum ItemsControlMessage<M: 'static, C: 'static + Control<M, C>> {
    SelectionChanged(Option<usize>),
    Items(Vec<Handle<UINode<M, C>>>)
}

#[derive(Debug)]
pub enum PopupMessage<M: 'static, C: 'static + Control<M, C>> {
    Open,
    Close,
    Content(Handle<UINode<M, C>>),
    Placement(Placement)
}

#[derive(Debug)]
pub enum UiMessageData<M: 'static, C: 'static + Control<M, C>> {
    Widget(WidgetMessage),
    Button(ButtonMessage<M, C>),
    ScrollBar(ScrollBarMessage),
    CheckBox(CheckBoxMessage),
    Window(WindowMessage),
    ItemsControl(ItemsControlMessage<M, C>),
    Popup(PopupMessage<M, C>),
    ScrollViewer(ScrollViewerMessage<M, C>),
    User(M),
}

/// Event is basic communication element that is used to deliver information to UI nodes
/// or to user code.
#[derive(Debug)]
pub struct UiMessage<M: 'static, C: 'static + Control<M, C>> {
    /// Useful flag to check if a message was already handled, this flag does *not* affects
    /// dispatcher.
    pub handled: bool,

    /// Actual message data. Use pattern matching to get node-specific data.
    pub data: UiMessageData<M, C>,

    /// Handle of node for which this event was produced. Can be NONE if target is undefined,
    /// this is the case when user click a button, button produces Click event but it does
    /// not know who will handle it. Targeted events are useful to send some data to specific
    /// nodes. Even if message has `target` it still will be available to all other message
    /// handlers.
    pub(in crate) target: Handle<UINode<M, C>>,

    /// Source of event. Can be NONE if event is targeted, however if there is source and target
    /// both present, then it means node-to-node communication.
    pub(in crate) source: Handle<UINode<M, C>>,
}

impl<M, C: 'static + Control<M, C>> UiMessage<M, C> {
    #[inline]
    pub fn targeted(target: Handle<UINode<M, C>>, kind: UiMessageData<M, C>) -> Self {
        Self {
            data: kind,
            handled: false,
            source: Handle::NONE,
            target,
        }
    }

    #[inline]
    pub fn new(kind: UiMessageData<M, C>) -> Self {
        Self {
            data: kind,
            handled: false,
            source: Handle::NONE,
            target: Handle::NONE,
        }
    }

    #[inline]
    pub fn target(&self) -> Handle<UINode<M, C>> {
        self.target
    }

    #[inline]
    pub fn source(&self) -> Handle<UINode<M, C>> {
        self.source
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