//! Message and events module contains all possible widget messages and OS events. See [`UiMessage`] docs for more info and
//! examples.

#![warn(missing_docs)]

use crate::{
    core::{algebra::Vector2, pool::Handle, reflect::prelude::*},
    UiNode,
};
use serde::{Deserialize, Serialize};
use std::{any::Any, cell::Cell, fmt::Debug, rc::Rc};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

/// Defines a new message constructor for a enum variant. It is widely used in this crate to create shortcuts to create
/// messages. Why is it needed anyway? Just to reduce boilerplate code as much as possible.
///
/// ## Examples
///
/// The following example shows how to create message constructors for various kinds of enum variants:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle, define_constructor, message::MessageDirection, UiMessage, UiNode,
/// #     UserInterface,
/// # };
/// #
/// // Message must be debuggable and comparable.
/// #[derive(Debug, PartialEq)]
/// enum MyWidgetMessage {
///     DoSomething,
///     Foo(u32),
///     Bar { foo: u32, baz: u8 },
/// }
///
/// impl MyWidgetMessage {
///     // The first option is used to create constructors plain enum variants:
///     //
///     //                  enum name       variant            name          perform layout?
///     //                      v              v                 v                  v
///     define_constructor!(MyWidgetMessage:DoSomething => fn do_something(), layout: false);
///
///     // The second option is used to create constructors for single-arg tuple enum variants:
///     //
///     //                  enum name     variant    name arg    perform layout?
///     //                      v            v         v   v           v
///     define_constructor!(MyWidgetMessage:Foo => fn foo(u32), layout: false);
///
///     // The third option is used to create constructors for enum variants with fields:
///     //
///     //                  enum name     variant    name arg  type arg type  perform layout?
///     //                      v            v         v   v     v   v    v          v
///     define_constructor!(MyWidgetMessage:Bar => fn bar(foo: u32, baz: u8), layout: false);
/// }
///
/// fn using_messages(my_widget: Handle<UiNode>, ui: &UserInterface) {
///     // Send MyWidgetMessage::DoSomething
///     ui.send_message(MyWidgetMessage::do_something(
///         my_widget,
///         MessageDirection::ToWidget,
///     ));
///
///     // Send MyWidgetMessage::Foo
///     ui.send_message(MyWidgetMessage::foo(
///         my_widget,
///         MessageDirection::ToWidget,
///         5,
///     ));
///
///     // Send MyWidgetMessage::Bar
///     ui.send_message(MyWidgetMessage::bar(
///         my_widget,
///         MessageDirection::ToWidget,
///         1,
///         2,
///     ));
/// }
/// ```
#[macro_export]
macro_rules! define_constructor {
    ($(#[$meta:meta])* $inner:ident : $inner_var:tt => fn $name:ident(), layout: $perform_layout:expr) => {
        $(#[$meta])*
        #[must_use = "message does nothing until sent to ui"]
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

    ($(#[$meta:meta])* $inner:ident : $inner_var:tt => fn $name:ident($typ:ty), layout: $perform_layout:expr) => {
        $(#[$meta])*
        #[must_use = "message does nothing until sent to ui"]
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

    ($(#[$meta:meta])* $inner:ident : $inner_var:tt => fn $name:ident( $($params:ident : $types:ty),+ ), layout: $perform_layout:expr) => {
        $(#[$meta])*
        #[must_use = "message does nothing until sent to ui"]
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

/// Message direction allows you to distinguish from where message has came from. Often there is a need to find out who
/// created a message to respond properly. Imagine that we have a NumericUpDown input field for a property and we using
/// some data source to feed data into input field. When we change something in the input field by typing, it creates a
/// message with new value. On other hand we often need to put new value in the input field from some code, in this case
/// we again creating a message. But how to understand from which "side" message has came from? Was it filled in by user
/// and we should create a command  to change value in the data source, or it was created from syncing code just to pass
/// new value to UI? This problem solved by setting a direction to a message. Also it solves another problem: often we
/// need to respond to a message only if it did some changes. In this case at first we fire a message with ToWidget direction,
/// widget catches it and checks if changes are needed and if so, it "rethrows" message with direction FromWidget. Listeners
/// are "subscribed" to FromWidget messages only and won't respond to ToWidget messages.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Hash, Eq)]
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

/// A trait, that is used by every messages used in the user interface. It contains utility methods, that are used
/// for downcasting and equality comparison.
pub trait MessageData: 'static + Debug + Any {
    /// Casts `self` as [`Any`] reference.
    fn as_any(&self) -> &dyn Any;

    /// Compares this message data with some other.
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

/// Message is basic communication element that is used to deliver information to widget or to user code.
///
/// ## Motivation
///
/// This UI library uses message passing mechanism to communicate with widgets. This is very simple and reliable mechanism that
/// effectively decouples widgets from each other. There is no direct way of modify something during runtime, you have to use
/// messages to change state of ui elements.
///
/// ## Direction
///
/// Each message marked with "Direction" field, which means supported routes for message. For example [`crate::button::ButtonMessage::Click`]
/// has "Direction: To/From UI" which means that it can be sent either from internals of library or from user code. However
/// [`crate::widget::WidgetMessage::Focus`] has "Direction: From UI" which means that only internal library code can send such messages without
/// a risk of breaking anything.
///
/// ## Threading
///
/// UiMessage is nor Send or Sync. User interface is a single-thread thing, as well as its messages.
///
/// ## Examples
///
/// ```rust
/// use fyrox_ui::{
///     core::pool::Handle, define_constructor, message::MessageDirection, UiMessage, UiNode,
///     UserInterface,
/// };
///
/// // Message must be debuggable and comparable.
/// #[derive(Debug, PartialEq)]
/// enum MyWidgetMessage {
///     DoSomething,
///     Foo(u32),
///     Bar { foo: u32, baz: u8 },
/// }
///
/// impl MyWidgetMessage {
///     define_constructor!(MyWidgetMessage:DoSomething => fn do_something(), layout: false);     
///     define_constructor!(MyWidgetMessage:Foo => fn foo(u32), layout: false);
///     define_constructor!(MyWidgetMessage:Bar => fn bar(foo: u32, baz: u8), layout: false);
/// }
///
/// fn using_messages(my_widget: Handle<UiNode>, ui: &UserInterface) {
///     // Send MyWidgetMessage::DoSomething
///     ui.send_message(MyWidgetMessage::do_something(
///         my_widget,
///         MessageDirection::ToWidget,
///     ));
///     // Send MyWidgetMessage::Foo
///     ui.send_message(MyWidgetMessage::foo(
///         my_widget,
///         MessageDirection::ToWidget,
///         5,
///     ));
///     // Send MyWidgetMessage::Bar
///     ui.send_message(MyWidgetMessage::bar(
///         my_widget,
///         MessageDirection::ToWidget,
///         1,
///         2,
///     ));
/// }
/// ```
///
///
#[derive(Debug, Clone)]
pub struct UiMessage {
    /// Useful flag to check if a message was already handled. It could be used to mark messages as "handled" to prevent
    /// any further responses to them. It is especially useful in bubble message routing, when a message is passed through
    /// the entire chain of parent nodes starting from current. In this, you can mark a message as "handled" and also check
    /// if it is handled or not. For example, this is used in [`crate::tree::Tree`] implementation, to prevent double-click
    /// to close all the parent trees from current.
    pub handled: Cell<bool>,

    /// Actual message data. Use [`UiMessage::data`] method to try to downcast the internal data to a specific type.
    pub data: Rc<dyn MessageData>,

    /// Handle of node that will receive message. Please note that **all** nodes in hierarchy will also receive this message,
    /// order is "up-on-tree" (so called "bubble" message routing). T
    pub destination: Handle<UiNode>,

    /// Indicates the direction of the message. See [`MessageDirection`] docs for more info.
    pub direction: MessageDirection,

    /// Whether or not message requires layout to be calculated first.
    ///
    /// ## Motivation
    ///
    /// Some of message handling routines uses layout info, but message loop performed right after layout pass, but some of messages
    /// may change layout and this flag tells UI to perform layout before passing message further. In ideal case we'd perform layout
    /// after **each** message, but since layout pass is super heavy we should do it **only** when it is actually needed.
    pub perform_layout: Cell<bool>,

    /// A custom user flags. Use it if `handled` flag is not enough.
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
    /// Creates new UI message with desired data.
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

    /// Sets the desired destination of the message.
    pub fn with_destination(mut self, destination: Handle<UiNode>) -> Self {
        self.destination = destination;
        self
    }

    /// Sets the desired direction of the message.
    pub fn with_direction(mut self, direction: MessageDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Sets the desired handled flag of the message.
    pub fn with_handled(self, handled: bool) -> Self {
        self.handled.set(handled);
        self
    }

    /// Sets the desired perform layout flag of the message.
    pub fn with_perform_layout(self, perform_layout: bool) -> Self {
        self.perform_layout.set(perform_layout);
        self
    }

    /// Creates a new copy of the message with reversed direction. Typical use case is to re-send messages to create "response"
    /// in a widget. For example you have a float input field and it has Value message. When the input field receives Value message
    /// with [`MessageDirection::ToWidget`] it checks if value needs to be changed and if it does, it re-sends same message, but with
    /// reversed direction back to message queue so every "listener" can reach properly. The input field won't react at
    /// [`MessageDirection::FromWidget`] message so there will be no infinite message loop.
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

    /// Returns destination widget handle of the message.
    pub fn destination(&self) -> Handle<UiNode> {
        self.destination
    }

    /// Tries to downcast current data of the message to a particular type.
    pub fn data<T: MessageData>(&self) -> Option<&T> {
        (*self.data).as_any().downcast_ref::<T>()
    }

    /// Sets handled flag.
    pub fn set_handled(&self, handled: bool) {
        self.handled.set(handled);
    }

    /// Returns handled flag.
    pub fn handled(&self) -> bool {
        self.handled.get()
    }

    /// Returns direction of the message.
    pub fn direction(&self) -> MessageDirection {
        self.direction
    }

    /// Sets perform layout flag.
    pub fn set_perform_layout(&self, value: bool) {
        self.perform_layout.set(value);
    }

    /// Returns perform layout flag.
    pub fn need_perform_layout(&self) -> bool {
        self.perform_layout.get()
    }

    /// Checks if the message has particular flags.
    pub fn has_flags(&self, flags: u64) -> bool {
        self.flags & flags != 0
    }
}

/// Mouse button state.
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
pub enum ButtonState {
    /// Pressed state.
    Pressed,
    /// Released state.
    Released,
}

/// A set of possible mouse buttons.
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
pub enum MouseButton {
    /// Left mouse button.
    Left,
    /// Right mouse button.
    Right,
    /// Middle mouse button.
    Middle,
    /// Any other mouse button.
    Other(u16),
}

/// An event that an OS sends to a window, that is then can be used to "feed" the user interface so it can do some actions.
pub enum OsEvent {
    /// Mouse input event.
    MouseInput {
        /// Mouse button.
        button: MouseButton,
        /// Mouse button state.
        state: ButtonState,
    },
    /// Cursor event.
    CursorMoved {
        /// New position of the cursor.
        position: Vector2<f32>,
    },
    /// Keyboard input event.
    KeyboardInput {
        /// Code of a key.
        button: KeyCode,
        /// Key state.
        state: ButtonState,
    },
    /// Text character event.
    Character(char),
    /// Keyboard modifier event (used for key combinations such as Ctrl+A, Ctrl+C, etc).
    KeyboardModifiers(KeyboardModifiers),
    /// Mouse wheel event, with a tuple that stores the (x, y) offsets.
    MouseWheel(f32, f32),
}

/// A set of possible keyboard modifiers.
#[derive(
    Debug,
    Hash,
    Ord,
    PartialOrd,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Default,
    Serialize,
    Deserialize,
    Reflect,
)]
pub struct KeyboardModifiers {
    /// `Alt` key is pressed.
    pub alt: bool,
    /// `Shift` key is pressed.
    pub shift: bool,
    /// `Ctrl` key is pressed.
    pub control: bool,
    /// `System` key is pressed.
    pub system: bool,
}

impl KeyboardModifiers {
    /// Checks if the modifiers is empty (nothing is pressed).
    pub fn is_none(self) -> bool {
        !self.shift && !self.control && !self.alt && !self.system
    }
}

/// Code of a key on keyboard.
#[derive(
    Debug,
    Hash,
    Ord,
    PartialOrd,
    PartialEq,
    Eq,
    Clone,
    Copy,
    AsRefStr,
    EnumString,
    EnumVariantNames,
    Serialize,
    Deserialize,
    Reflect,
)]
#[repr(u32)]
pub enum KeyCode {
    /// 1 key.
    Key1,
    /// 2 key.
    Key2,
    /// 3 key.
    Key3,
    /// 4 key.
    Key4,
    /// 5 key.
    Key5,
    /// 6 key.
    Key6,
    /// 7 key.
    Key7,
    /// 8 key.
    Key8,
    /// 9 key.
    Key9,
    /// 0 key.
    Key0,
    /// A key.
    A,
    /// B key.
    B,
    /// C key.
    C,
    /// D key.
    D,
    /// E key.
    E,
    /// F key.
    F,
    /// G key.
    G,
    /// H key.
    H,
    /// I key.
    I,
    /// J key.
    J,
    /// K key.
    K,
    /// L key.
    L,
    /// M key.
    M,
    /// N key.
    N,
    /// O key.
    O,
    /// P key.
    P,
    /// Q key.
    Q,
    /// R key.
    R,
    /// S key.
    S,
    /// T key.
    T,
    /// U key.
    U,
    /// V key.
    V,
    /// W key.
    W,
    /// X key.
    X,
    /// Y key.
    Y,
    /// Z key.
    Z,
    /// Escape key.
    Escape,
    /// F1 key.
    F1,
    /// F2 key.
    F2,
    /// F3 key.
    F3,
    /// F4 key.
    F4,
    /// F5 key.
    F5,
    /// F6 key.
    F6,
    /// F7 key.
    F7,
    /// F8 key.
    F8,
    /// F9 key.
    F9,
    /// F10 key.
    F10,
    /// F11 key.
    F11,
    /// F12 key.
    F12,
    /// F13 key.
    F13,
    /// F14 key.
    F14,
    /// F15 key.
    F15,
    /// F16 key.
    F16,
    /// F17 key.
    F17,
    /// F18 key.
    F18,
    /// F19 key.
    F19,
    /// F20 key.
    F20,
    /// F21 key.
    F21,
    /// F22 key.
    F22,
    /// F23 key.
    F23,
    /// F24 key.
    F24,
    /// Snapshot key.
    Snapshot,
    /// Scroll key.
    Scroll,
    /// Pause key.
    Pause,
    /// Insert key.
    Insert,
    /// Home key.
    Home,
    /// Delete key.
    Delete,
    /// End key.
    End,
    /// PageDown key.
    PageDown,
    /// PageUp key.
    PageUp,
    /// Left key.
    Left,
    /// Up key.
    Up,
    /// Right key.
    Right,
    /// Down key.
    Down,
    /// Backspace key.
    Backspace,
    /// Return key.
    Return,
    /// Space key.
    Space,
    /// Compose key.
    Compose,
    /// Caret key.
    Caret,
    /// Numlock key.
    Numlock,
    /// Numpad0 key.
    Numpad0,
    /// Numpad1 key.
    Numpad1,
    /// Numpad2 key.
    Numpad2,
    /// Numpad3 key.
    Numpad3,
    /// Numpad4 key.
    Numpad4,
    /// Numpad5 key.
    Numpad5,
    /// Numpad6 key.
    Numpad6,
    /// Numpad7 key.
    Numpad7,
    /// Numpad8 key.
    Numpad8,
    /// Numpad9 key.
    Numpad9,
    /// AbntC1 key.
    AbntC1,
    /// AbntC2 key.
    AbntC2,
    /// NumpadAdd key.
    NumpadAdd,
    /// Apostrophe key.
    Apostrophe,
    /// Apps key.
    Apps,
    /// At key.
    At,
    /// Ax key.
    Ax,
    /// Backslash key.
    Backslash,
    /// Calculator key.
    Calculator,
    /// Capital key.
    Capital,
    /// Colon key.
    Colon,
    /// Comma key.
    Comma,
    /// Convert key.
    Convert,
    /// NumpadDecimal key.
    NumpadDecimal,
    /// NumpadDivide key.
    NumpadDivide,
    /// Equals key.
    Equals,
    /// Grave key.
    Grave,
    /// Kana key.
    Kana,
    /// Kanji key.
    Kanji,
    /// LAlt key.
    LAlt,
    /// LBracket key.
    LBracket,
    /// LControl key.
    LControl,
    /// LShift key.
    LShift,
    /// LWin key.
    LWin,
    /// Mail key.
    Mail,
    /// MediaSelect key.
    MediaSelect,
    /// MediaStop key.
    MediaStop,
    /// Minus key.
    Minus,
    /// NumpadMultiply key.
    NumpadMultiply,
    /// Mute key.
    Mute,
    /// MyComputer key.
    MyComputer,
    /// NavigateForward key.
    NavigateForward,
    /// NavigateBackward key.
    NavigateBackward,
    /// NextTrack key.
    NextTrack,
    /// NoConvert key.
    NoConvert,
    /// NumpadComma key.
    NumpadComma,
    /// NumpadEnter key.
    NumpadEnter,
    /// NumpadEquals key.
    NumpadEquals,
    /// OEM102 key.
    OEM102,
    /// Period key.
    Period,
    /// PlayPause key.
    PlayPause,
    /// Power key.
    Power,
    /// PrevTrack key.
    PrevTrack,
    /// RAlt key.
    RAlt,
    /// RBracket key.
    RBracket,
    /// RControl key.
    RControl,
    /// RShift key.
    RShift,
    /// RWin key.
    RWin,
    /// Semicolon key.
    Semicolon,
    /// Slash key.
    Slash,
    /// Sleep key.
    Sleep,
    /// Stop key.
    Stop,
    /// NumpadSubtract key.
    NumpadSubtract,
    /// Sysrq key.
    Sysrq,
    /// Tab key.
    Tab,
    /// Underline key.
    Underline,
    /// Unlabeled key.
    Unlabeled,
    /// VolumeDown key.
    VolumeDown,
    /// VolumeUp key.
    VolumeUp,
    /// Wake key.
    Wake,
    /// WebBack key.
    WebBack,
    /// WebFavorites key.
    WebFavorites,
    /// WebForward key.
    WebForward,
    /// WebHome key.
    WebHome,
    /// WebRefresh key.
    WebRefresh,
    /// WebSearch key.
    WebSearch,
    /// WebStop key.
    WebStop,
    /// Yen key.
    Yen,
    /// Copy key.
    Copy,
    /// Paste key.
    Paste,
    /// Plus key.
    Cut,
    /// Plus key.
    Asterisk,
    /// Plus key.
    Plus,
}

/// A fixed set of cursor icons that available on most OSes.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CursorIcon {
    /// Default OS-dependent cursor icon.
    Default,
    /// Crosshair cursor icon.
    Crosshair,
    /// Hand cursor icon.
    Hand,
    /// Arrow cursor icon.
    Arrow,
    /// Move cursor icon.
    Move,
    /// Text cursor icon.
    Text,
    /// Wait cursor icon.
    Wait,
    /// Help cursor icon.
    Help,
    /// Progress cursor icon.
    Progress,
    /// NotAllowed cursor icon.
    NotAllowed,
    /// ContextMenu cursor icon.
    ContextMenu,
    /// Cell cursor icon.
    Cell,
    /// VerticalText cursor icon.
    VerticalText,
    /// Alias cursor icon.
    Alias,
    /// Copy cursor icon.
    Copy,
    /// NoDrop cursor icon.
    NoDrop,
    /// Grab cursor icon.
    Grab,
    /// Grabbing cursor icon.
    Grabbing,
    /// AllScroll cursor icon.
    AllScroll,
    /// ZoomIn cursor icon.
    ZoomIn,
    /// ZoomOut cursor icon.
    ZoomOut,
    /// EResize cursor icon.
    EResize,
    /// NResize cursor icon.
    NResize,
    /// NeResize cursor icon.
    NeResize,
    /// NwResize cursor icon.
    NwResize,
    /// SResize cursor icon.
    SResize,
    /// SeResize cursor icon.
    SeResize,
    /// SwResize cursor icon.
    SwResize,
    /// WResize cursor icon.
    WResize,
    /// EwResize cursor icon.
    EwResize,
    /// NsResize cursor icon.
    NsResize,
    /// NeswResize cursor icon.
    NeswResize,
    /// NwseResize cursor icon.
    NwseResize,
    /// ColResize cursor icon.
    ColResize,
    /// RowResize cursor icon.
    RowResize,
}

impl Default for CursorIcon {
    fn default() -> Self {
        CursorIcon::Default
    }
}
