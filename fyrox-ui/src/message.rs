//! Message and events module contains all possible widget messages and OS events. See [`UiMessage`] docs for more info and
//! examples.

use crate::{
    core::{algebra::Vector2, pool::Handle, reflect::prelude::*},
    UiNode,
};
use serde::{Deserialize, Serialize};
use std::{any::Any, cell::Cell, fmt::Debug, rc::Rc};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

mod kek {
    use crate::{
        core::pool::Handle, define_constructor, message::MessageDirection, UiMessage, UiNode,
        UserInterface,
    };

    // Message must be debuggable and comparable.
    #[derive(Debug, PartialEq)]
    enum MyWidgetMessage {
        DoSomething,
        Foo(u32),
        Bar { foo: u32, baz: u8 },
    }

    impl MyWidgetMessage {
        define_constructor!(MyWidgetMessage:DoSomething => fn do_something(), layout: false);
        define_constructor!(MyWidgetMessage:Foo => fn foo(u32), layout: false);
        define_constructor!(MyWidgetMessage:Bar => fn bar(foo: u32, baz: u8), layout: false);
    }

    fn using_messages(my_widget: Handle<UiNode>, ui: &UserInterface) {
        // Send MyWidgetMessage::DoSomething
        ui.send_message(MyWidgetMessage::do_something(
            my_widget,
            MessageDirection::ToWidget,
        ));
        // Send MyWidgetMessage::Foo
        ui.send_message(MyWidgetMessage::foo(
            my_widget,
            MessageDirection::ToWidget,
            5,
        ));
        // Send MyWidgetMessage::Bar
        ui.send_message(MyWidgetMessage::bar(
            my_widget,
            MessageDirection::ToWidget,
            1,
            2,
        ));
    }
}

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

    pub fn with_handled(self, handled: bool) -> Self {
        self.handled.set(handled);
        self
    }

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
