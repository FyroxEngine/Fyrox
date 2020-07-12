//! Message and events module contains all possible widget messages and OS events.
//!
//! This UI library uses message passing mechanism to communicate with widgets.
//! This is very simple and reliable mechanism that effectively decouples widgets
//! from each other. There is no direct way of modify something during runtime,
//! you have to use messages to change state of ui elements.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex}
};
use crate::{
    ttf::Font,
    core::{
        math::{
            vec2::Vec2,
            vec3::Vec3,
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
    messagebox::MessageBoxResult,
    window::WindowTitle,
    dock::TileContent
};
use crate::draw::Texture;

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
    DragStarted(Handle<UINode<M, C>>),
    DragOver(Handle<UINode<M, C>>),
    Drop(Handle<UINode<M, C>>),
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
    /// Set desired position at center in local coordinates.
    Center,
}

impl<M: 'static, C: 'static + Control<M, C>> WidgetMessage<M, C> {
    fn make(destination: Handle<UINode<M, C>>, msg: WidgetMessage<M, C>) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::Widget(msg),
            destination,
        }
    }

    /// Creates a message to remove `destination` node.
    pub fn remove(destination: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, WidgetMessage::Remove)
    }

    /// Creates a message to unlink `destination` node from its current parent.
    pub fn unlink(destination: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, WidgetMessage::Unlink)
    }

    /// Creates message to link `destination` node with specified `parent` node.
    pub fn link(destination: Handle<UINode<M, C>>, parent: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, WidgetMessage::LinkWith(parent))
    }

    /// Creates message to set background of `destination` node.
    pub fn background(destination: Handle<UINode<M, C>>, background: Brush) -> UiMessage<M, C> {
        Self::make(destination, WidgetMessage::Background(background))
    }

    /// Creates message to set visibility of `destination` node.
    pub fn visibility(destination: Handle<UINode<M, C>>, visibility: bool) -> UiMessage<M, C> {
        Self::make(destination, WidgetMessage::Visibility(visibility))
    }

    /// Creates message to set width of `destination` node.
    pub fn width(destination: Handle<UINode<M, C>>, width: f32) -> UiMessage<M, C> {
        Self::make(destination, WidgetMessage::Width(width))
    }

    /// Creates message to set height of `destination` node.
    pub fn height(destination: Handle<UINode<M, C>>, height: f32) -> UiMessage<M, C> {
        Self::make(destination, WidgetMessage::Height(height))
    }

    /// Creates message to set desired position of `destination` node.
    pub fn desired_position(destination: Handle<UINode<M, C>>, position: Vec2) -> UiMessage<M, C> {
        Self::make(destination, WidgetMessage::DesiredPosition(position))
    }

    /// Creates message to set desired position of `destination` node.
    pub fn center(destination: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, WidgetMessage::Center)
    }

    // TODO: Add rest items.
}

#[derive(Debug)]
pub enum ButtonMessage<M: 'static, C: 'static + Control<M, C>> {
    Click,
    Content(Handle<UINode<M, C>>),
}

impl<M: 'static, C: 'static + Control<M, C>> ButtonMessage<M, C> {
    fn make(destination: Handle<UINode<M, C>>, msg: ButtonMessage<M, C>) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::Button(msg),
            destination,
        }
    }

    pub fn click(destination: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, ButtonMessage::Click)
    }

    pub fn content(destination: Handle<UINode<M, C>>, content: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, ButtonMessage::Content(content))
    }
}

#[derive(Debug)]
pub enum ScrollBarMessage {
    Value(f32),
    MinValue(f32),
    MaxValue(f32),
}

impl ScrollBarMessage {
    fn make<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, msg: ScrollBarMessage) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::ScrollBar(msg),
            destination,
        }
    }

    pub fn value<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, value: f32) -> UiMessage<M, C> {
        Self::make(destination, ScrollBarMessage::Value(value))
    }

    pub fn max_value<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, value: f32) -> UiMessage<M, C> {
        Self::make(destination, ScrollBarMessage::MaxValue(value))
    }

    pub fn min_value<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, value: f32) -> UiMessage<M, C> {
        Self::make(destination, ScrollBarMessage::MinValue(value))
    }
}

#[derive(Debug)]
pub enum CheckBoxMessage {
    Check(Option<bool>),
}

impl CheckBoxMessage {
    fn make<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, msg: CheckBoxMessage) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::CheckBox(msg),
            destination,
        }
    }

    pub fn check<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, value: Option<bool>) -> UiMessage<M, C> {
        Self::make(destination, CheckBoxMessage::Check(value))
    }
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
    Title(WindowTitle<M, C>),
}

impl<M: 'static, C: 'static + Control<M, C>> WindowMessage<M, C> {
    fn make(destination: Handle<UINode<M, C>>, msg: WindowMessage<M, C>) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::Window(msg),
            destination,
        }
    }

    pub fn open(destination: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, WindowMessage::Open)
    }

    pub fn open_modal(destination: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, WindowMessage::OpenModal)
    }

    pub fn close(destination: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, WindowMessage::Close)
    }
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

#[derive(Debug, Copy, Clone)]
pub struct SelectionState(pub(in crate) bool);

#[derive(Debug)]
pub enum TreeMessage<M: 'static, C: 'static + Control<M, C>> {
    Expand(bool),
    AddItem(Handle<UINode<M, C>>),
    RemoveItem(Handle<UINode<M, C>>),
    SetItems(Vec<Handle<UINode<M, C>>>),
    // Private, do not use. For internal needs only. Use TreeRootMessage::Selected.
    Select(SelectionState)
}

impl<M: 'static, C: 'static + Control<M, C>> TreeMessage<M, C> {
    fn make(destination: Handle<UINode<M, C>>, msg: TreeMessage<M, C>) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::Tree(msg),
            destination,
        }
    }

    pub fn add_item(destination: Handle<UINode<M, C>>, item: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, TreeMessage::AddItem(item))
    }

    pub fn remove_item(destination: Handle<UINode<M, C>>, item: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, TreeMessage::RemoveItem(item))
    }

    pub fn set_items(destination: Handle<UINode<M, C>>, items: Vec<Handle<UINode<M, C>>>) -> UiMessage<M, C> {
        Self::make(destination, TreeMessage::SetItems(items))
    }

    pub fn expand(destination: Handle<UINode<M, C>>, expand: bool) -> UiMessage<M, C> {
        Self::make(destination, TreeMessage::Expand(expand))
    }

    pub(in crate) fn select(destination: Handle<UINode<M, C>>, select: bool) -> UiMessage<M, C> {
        Self::make(destination, TreeMessage::Select(SelectionState(select)))
    }
}

#[derive(Debug)]
pub enum TreeRootMessage<M: 'static, C: 'static + Control<M, C>> {
    AddItem(Handle<UINode<M, C>>),
    RemoveItem(Handle<UINode<M, C>>),
    Items(Vec<Handle<UINode<M, C>>>),
    Selected(Handle<UINode<M, C>>),
}

impl<M: 'static, C: 'static + Control<M, C>> TreeRootMessage<M, C> {
    fn make(destination: Handle<UINode<M, C>>, msg: TreeRootMessage<M, C>) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::TreeRoot(msg),
            destination,
        }
    }

    pub fn add_item(destination: Handle<UINode<M, C>>, item: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, TreeRootMessage::AddItem(item))
    }

    pub fn remove_item(destination: Handle<UINode<M, C>>, item: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, TreeRootMessage::RemoveItem(item))
    }

    pub fn items(destination: Handle<UINode<M, C>>, items: Vec<Handle<UINode<M, C>>>) -> UiMessage<M, C> {
        Self::make(destination, TreeRootMessage::Items(items))
    }

    pub fn select(destination: Handle<UINode<M, C>>, item: Handle<UINode<M, C>>) -> UiMessage<M, C> {
        Self::make(destination, TreeRootMessage::Selected(item))
    }
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
pub enum TextMessage {
    Text(String),
    Wrap(bool),
    Font(Arc<Mutex<Font>>),
    VerticalAlignment(VerticalAlignment),
    HorizontalAlignment(HorizontalAlignment),
}

#[derive(Debug)]
pub enum ImageMessage {
    Texture(Arc<Texture>),
    Flip(bool)
}

impl ImageMessage {
    fn make<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, msg: ImageMessage) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::Image(msg),
            destination,
        }
    }

    pub fn texture<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, value: Arc<Texture>) -> UiMessage<M, C> {
        Self::make(destination, ImageMessage::Texture(value))
    }

    pub fn flip<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, value: bool) -> UiMessage<M, C> {
        Self::make(destination, ImageMessage::Flip(value))
    }
}

impl TextMessage {
    fn make<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, msg: TextMessage) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::Text(msg),
            destination,
        }
    }

    pub fn text<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, value: String) -> UiMessage<M, C> {
        Self::make(destination, TextMessage::Text(value))
    }

    pub fn wrap<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, value: bool) -> UiMessage<M, C> {
        Self::make(destination, TextMessage::Wrap(value))
    }

    pub fn horizontal_alignment<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, value: HorizontalAlignment) -> UiMessage<M, C> {
        Self::make(destination, TextMessage::HorizontalAlignment(value))
    }

    pub fn vertical_alignment<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, value: VerticalAlignment) -> UiMessage<M, C> {
        Self::make(destination, TextMessage::VerticalAlignment(value))
    }

    pub fn font<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, value: Arc<Mutex<Font>>) -> UiMessage<M, C> {
        Self::make(destination, TextMessage::Font(value))
    }
}

#[derive(Debug)]
pub enum TileMessage<M: 'static, C: 'static + Control<M, C>> {
    Content(TileContent<M, C>)
}

impl<M: 'static, C: 'static + Control<M, C>> TileMessage<M, C> {
    fn make(destination: Handle<UINode<M, C>>, msg: TileMessage<M, C>) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::Tile(msg),
            destination,
        }
    }

    pub fn content(destination: Handle<UINode<M, C>>, content: TileContent<M, C>) -> UiMessage<M, C> {
        Self::make(destination, TileMessage::Content(content))
    }
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
pub enum ScrollPanelMessage {
    VerticalScroll(f32),
    HorizontalScroll(f32)
}

impl ScrollPanelMessage {
    fn make<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, msg: ScrollPanelMessage) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::ScrollPanel(msg),
            destination,
        }
    }

    pub fn vertical_scroll<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, value: f32) -> UiMessage<M, C> {
        Self::make(destination, ScrollPanelMessage::VerticalScroll(value))
    }

    pub fn horizontal_scroll<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, value: f32) -> UiMessage<M, C> {
        Self::make(destination, ScrollPanelMessage::HorizontalScroll(value))
    }
}

#[derive(Debug)]
pub enum MenuMessage {
    Activate,
    Deactivate,
}

#[derive(Debug)]
pub enum MenuItemMessage {
    Open,
    Close,
    Click,
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
pub enum DecoratorMessage {
    Select(bool)
}

impl DecoratorMessage {
    fn make<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, msg: DecoratorMessage) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::Decorator(msg),
            destination,
        }
    }

    pub fn select<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, value: bool) -> UiMessage<M, C> {
        Self::make(destination, DecoratorMessage::Select(value))
    }
}

#[derive(Debug)]
pub enum ProgressBarMessage {
    Progress(f32)
}

impl ProgressBarMessage {
    fn make<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, msg: ProgressBarMessage) -> UiMessage<M, C> {
        UiMessage {
            handled: false,
            data: UiMessageData::ProgressBar(msg),
            destination,
        }
    }

    pub fn progress<M: 'static, C: 'static + Control<M, C>>(destination: Handle<UINode<M, C>>, value: f32) -> UiMessage<M, C> {
        Self::make(destination, ProgressBarMessage::Progress(value))
    }
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
    Decorator(DecoratorMessage),
    Text(TextMessage),
    ScrollPanel(ScrollPanelMessage),
    Tile(TileMessage<M, C>),
    ProgressBar(ProgressBarMessage),
    Image(ImageMessage),
    User(M),
}

/// Message is basic communication element that is used to deliver information to UI nodes
/// or to user code.
#[derive(Debug)]
pub struct UiMessage<M: 'static, C: 'static + Control<M, C>> {
    /// Useful flag to check if a message was already handled.
    pub handled: bool,

    /// Actual message data. Use pattern matching to get type specific data.
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