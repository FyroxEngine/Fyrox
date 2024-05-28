//! Message and events module contains all possible widget messages and OS events. See [`UiMessage`] docs for more info and
//! examples.

#![warn(missing_docs)]

use crate::{
    core::{algebra::Vector2, pool::Handle, reflect::prelude::*, visitor::prelude::*},
    UiNode,
};
use fyrox_core::uuid_provider;
use serde::{Deserialize, Serialize};
use std::{any::Any, cell::Cell, fmt::Debug};
use strum_macros::{AsRefStr, EnumString, VariantNames};

/// Defines a new message constructor for a enum variant. It is widely used in this crate to create shortcuts to create
/// messages. Why is it needed anyway? Just to reduce boilerplate code as much as possible.
///
/// ## Examples
///
/// The following example shows how to create message constructors for various kinds of enum variants:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle, define_constructor, message::MessageDirection, message::UiMessage, UiNode,
/// #     UserInterface,
/// # };
/// #
/// // Message must be debuggable, comparable, cloneable.
/// #[derive(Debug, PartialEq, Clone)]
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
                data: Box::new($inner::$inner_var),
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
                data: Box::new($inner::$inner_var(value)),
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
                data: Box::new($inner::$inner_var { $($params),+ }),
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
pub trait MessageData: 'static + Debug + Any + Send {
    /// Casts `self` as [`Any`] reference.
    fn as_any(&self) -> &dyn Any;

    /// Compares this message data with some other.
    fn compare(&self, other: &dyn MessageData) -> bool;

    /// Clones self as boxed value.
    fn clone_box(&self) -> Box<dyn MessageData>;
}

impl<T> MessageData for T
where
    T: 'static + Debug + PartialEq + Any + Send + Clone,
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

    fn clone_box(&self) -> Box<dyn MessageData> {
        Box::new(self.clone())
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
///     core::pool::Handle, define_constructor, message::MessageDirection, message::UiMessage, UiNode,
///     UserInterface,
/// };
///
/// // Message must be debuggable and comparable.
/// #[derive(Debug, PartialEq, Clone)]
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
pub struct UiMessage {
    /// Useful flag to check if a message was already handled. It could be used to mark messages as "handled" to prevent
    /// any further responses to them. It is especially useful in bubble message routing, when a message is passed through
    /// the entire chain of parent nodes starting from current. In this, you can mark a message as "handled" and also check
    /// if it is handled or not. For example, this is used in [`crate::tree::Tree`] implementation, to prevent double-click
    /// to close all the parent trees from current.
    pub handled: Cell<bool>,

    /// Actual message data. Use [`UiMessage::data`] method to try to downcast the internal data to a specific type.
    pub data: Box<dyn MessageData>,

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

impl Debug for UiMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UiMessage({}:({})",
            match self.direction {
                MessageDirection::ToWidget => "To",
                MessageDirection::FromWidget => "From",
            },
            self.destination
        )?;
        if self.handled.get() {
            write!(f, ",handled")?;
        }
        if self.perform_layout.get() {
            write!(f, ",layout")?;
        }
        if self.flags != 0 {
            write!(f, ",flags:{}", self.flags)?;
        }
        write!(f, "):{:?}", self.data)
    }
}

impl Clone for UiMessage {
    fn clone(&self) -> Self {
        Self {
            handled: self.handled.clone(),
            data: self.data.clone_box(),
            destination: self.destination,
            direction: self.direction,
            perform_layout: self.perform_layout.clone(),
            flags: self.flags,
        }
    }
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
            data: Box::new(data),
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

    /// Sets the desired flags of the message.
    pub fn with_flags(mut self, flags: u64) -> Self {
        self.flags = flags;
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
            data: self.data.clone_box(),
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
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Visit, Reflect)]
pub enum ButtonState {
    /// Pressed state.
    Pressed,
    /// Released state.
    Released,
}

/// A set of possible mouse buttons.
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Default, Visit, Reflect)]
pub enum MouseButton {
    /// Left mouse button.
    #[default]
    Left,
    /// Right mouse button.
    Right,
    /// Middle mouse button.
    Middle,
    /// Back mouse button.
    Back,
    /// Forward mouse button.
    Forward,
    /// Any other mouse button.
    Other(u16),
}

/// A set of possible touch phases
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Visit, Reflect)]
pub enum TouchPhase {
    /// Touch started
    Started,
    /// Touch and drag
    Moved,
    /// Touch ended
    Ended,
    /// Touch cancelled
    Cancelled,
}

/// Describes the force of a touch event
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Visit, Reflect)]
pub enum Force {
    /// On iOS, the force is calibrated so that the same number corresponds to
    /// roughly the same amount of pressure on the screen regardless of the
    /// device.
    Calibrated {
        /// The force of the touch, where a value of 1.0 represents the force of
        /// an average touch (predetermined by the system, not user-specific).
        ///
        /// The force reported by Apple Pencil is measured along the axis of the
        /// pencil. If you want a force perpendicular to the device, you need to
        /// calculate this value using the `altitude_angle` value.
        force: [u8; 8],
        /// The maximum possible force for a touch.
        ///
        /// The value of this field is sufficiently high to provide a wide
        /// dynamic range for values of the `force` field.
        max_possible_force: [u8; 8],
        /// The altitude (in radians) of the stylus.
        ///
        /// A value of 0 radians indicates that the stylus is parallel to the
        /// surface. The value of this property is Pi/2 when the stylus is
        /// perpendicular to the surface.
        altitude_angle: Option<[u8; 8]>,
    },
    /// If the platform reports the force as normalized, we have no way of
    /// knowing how much pressure 1.0 corresponds to – we know it's the maximum
    /// amount of force, but as to how much force, you might either have to
    /// press really really hard, or not hard at all, depending on the device.
    Normalized([u8; 8]),
}

impl Force {
    /// Returns the force normalized to the range between 0.0 and 1.0 inclusive.
    ///
    /// Instead of normalizing the force, you should prefer to handle
    /// [`Force::Calibrated`] so that the amount of force the user has to apply is
    /// consistent across devices.
    pub fn normalized(&self) -> f64 {
        match self {
            Force::Calibrated {
                force,
                max_possible_force,
                altitude_angle,
            } => {
                let force = match altitude_angle {
                    Some(altitude_angle) => {
                        f64::from_be_bytes(*force) / f64::from_be_bytes(*altitude_angle).sin()
                    }
                    None => f64::from_be_bytes(*force),
                };
                force / f64::from_be_bytes(*max_possible_force)
            }
            Force::Normalized(force) => f64::from_be_bytes(*force),
        }
    }
}

/// An event that an OS sends to a window, that is then can be used to "feed" the user interface so it can do some actions.
#[derive(Debug)]
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
        /// Text of the key.
        text: String,
    },
    /// Keyboard modifier event (used for key combinations such as Ctrl+A, Ctrl+C, etc).
    KeyboardModifiers(KeyboardModifiers),
    /// Mouse wheel event, with a tuple that stores the (x, y) offsets.
    MouseWheel(f32, f32),
    /// Touch event.
    Touch {
        /// Phase of the touch event
        phase: TouchPhase,
        /// Screen location of touch event
        location: Vector2<f32>,
        /// Pressure exerted during force event
        force: Option<Force>,
        /// Unique touch event identifier to distinguish between fingers, for example
        id: u64,
    },
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
    Visit,
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

/// Code of a key on keyboard. Shamelessly taken from `winit` source code to match their key codes with
/// `fyrox-ui`'s.
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
    VariantNames,
    Serialize,
    Deserialize,
    Reflect,
    Visit,
    Default,
)]
#[repr(u32)]
#[allow(missing_docs)]
pub enum KeyCode {
    /// This variant is used when the key cannot be translated to any other variant.
    #[default]
    Unknown,
    /// <kbd>`</kbd> on a US keyboard. This is also called a backtick or grave.
    /// This is the <kbd>半角</kbd>/<kbd>全角</kbd>/<kbd>漢字</kbd>
    /// (hankaku/zenkaku/kanji) key on Japanese keyboards
    Backquote,
    /// Used for both the US <kbd>\\</kbd> (on the 101-key layout) and also for the key
    /// located between the <kbd>"</kbd> and <kbd>Enter</kbd> keys on row C of the 102-,
    /// 104- and 106-key layouts.
    /// Labeled <kbd>#</kbd> on a UK (102) keyboard.
    Backslash,
    /// <kbd>[</kbd> on a US keyboard.
    BracketLeft,
    /// <kbd>]</kbd> on a US keyboard.
    BracketRight,
    /// <kbd>,</kbd> on a US keyboard.
    Comma,
    /// <kbd>0</kbd> on a US keyboard.
    Digit0,
    /// <kbd>1</kbd> on a US keyboard.
    Digit1,
    /// <kbd>2</kbd> on a US keyboard.
    Digit2,
    /// <kbd>3</kbd> on a US keyboard.
    Digit3,
    /// <kbd>4</kbd> on a US keyboard.
    Digit4,
    /// <kbd>5</kbd> on a US keyboard.
    Digit5,
    /// <kbd>6</kbd> on a US keyboard.
    Digit6,
    /// <kbd>7</kbd> on a US keyboard.
    Digit7,
    /// <kbd>8</kbd> on a US keyboard.
    Digit8,
    /// <kbd>9</kbd> on a US keyboard.
    Digit9,
    /// <kbd>=</kbd> on a US keyboard.
    Equal,
    /// Located between the left <kbd>Shift</kbd> and <kbd>Z</kbd> keys.
    /// Labeled <kbd>\\</kbd> on a UK keyboard.
    IntlBackslash,
    /// Located between the <kbd>/</kbd> and right <kbd>Shift</kbd> keys.
    /// Labeled <kbd>\\</kbd> (ro) on a Japanese keyboard.
    IntlRo,
    /// Located between the <kbd>=</kbd> and <kbd>Backspace</kbd> keys.
    /// Labeled <kbd>¥</kbd> (yen) on a Japanese keyboard. <kbd>\\</kbd> on a
    /// Russian keyboard.
    IntlYen,
    /// <kbd>a</kbd> on a US keyboard.
    /// Labeled <kbd>q</kbd> on an AZERTY (e.g., French) keyboard.
    KeyA,
    /// <kbd>b</kbd> on a US keyboard.
    KeyB,
    /// <kbd>c</kbd> on a US keyboard.
    KeyC,
    /// <kbd>d</kbd> on a US keyboard.
    KeyD,
    /// <kbd>e</kbd> on a US keyboard.
    KeyE,
    /// <kbd>f</kbd> on a US keyboard.
    KeyF,
    /// <kbd>g</kbd> on a US keyboard.
    KeyG,
    /// <kbd>h</kbd> on a US keyboard.
    KeyH,
    /// <kbd>i</kbd> on a US keyboard.
    KeyI,
    /// <kbd>j</kbd> on a US keyboard.
    KeyJ,
    /// <kbd>k</kbd> on a US keyboard.
    KeyK,
    /// <kbd>l</kbd> on a US keyboard.
    KeyL,
    /// <kbd>m</kbd> on a US keyboard.
    KeyM,
    /// <kbd>n</kbd> on a US keyboard.
    KeyN,
    /// <kbd>o</kbd> on a US keyboard.
    KeyO,
    /// <kbd>p</kbd> on a US keyboard.
    KeyP,
    /// <kbd>q</kbd> on a US keyboard.
    /// Labeled <kbd>a</kbd> on an AZERTY (e.g., French) keyboard.
    KeyQ,
    /// <kbd>r</kbd> on a US keyboard.
    KeyR,
    /// <kbd>s</kbd> on a US keyboard.
    KeyS,
    /// <kbd>t</kbd> on a US keyboard.
    KeyT,
    /// <kbd>u</kbd> on a US keyboard.
    KeyU,
    /// <kbd>v</kbd> on a US keyboard.
    KeyV,
    /// <kbd>w</kbd> on a US keyboard.
    /// Labeled <kbd>z</kbd> on an AZERTY (e.g., French) keyboard.
    KeyW,
    /// <kbd>x</kbd> on a US keyboard.
    KeyX,
    /// <kbd>y</kbd> on a US keyboard.
    /// Labeled <kbd>z</kbd> on a QWERTZ (e.g., German) keyboard.
    KeyY,
    /// <kbd>z</kbd> on a US keyboard.
    /// Labeled <kbd>w</kbd> on an AZERTY (e.g., French) keyboard, and <kbd>y</kbd> on a
    /// QWERTZ (e.g., German) keyboard.
    KeyZ,
    /// <kbd>-</kbd> on a US keyboard.
    Minus,
    /// <kbd>.</kbd> on a US keyboard.
    Period,
    /// <kbd>'</kbd> on a US keyboard.
    Quote,
    /// <kbd>;</kbd> on a US keyboard.
    Semicolon,
    /// <kbd>/</kbd> on a US keyboard.
    Slash,
    /// <kbd>Alt</kbd>, <kbd>Option</kbd>, or <kbd>⌥</kbd>.
    AltLeft,
    /// <kbd>Alt</kbd>, <kbd>Option</kbd>, or <kbd>⌥</kbd>.
    /// This is labeled <kbd>AltGr</kbd> on many keyboard layouts.
    AltRight,
    /// <kbd>Backspace</kbd> or <kbd>⌫</kbd>.
    /// Labeled <kbd>Delete</kbd> on Apple keyboards.
    Backspace,
    /// <kbd>CapsLock</kbd> or <kbd>⇪</kbd>
    CapsLock,
    /// The application context menu key, which is typically found between the right
    /// <kbd>Super</kbd> key and the right <kbd>Control</kbd> key.
    ContextMenu,
    /// <kbd>Control</kbd> or <kbd>⌃</kbd>
    ControlLeft,
    /// <kbd>Control</kbd> or <kbd>⌃</kbd>
    ControlRight,
    /// <kbd>Enter</kbd> or <kbd>↵</kbd>. Labeled <kbd>Return</kbd> on Apple keyboards.
    Enter,
    /// The Windows, <kbd>⌘</kbd>, <kbd>Command</kbd>, or other OS symbol key.
    SuperLeft,
    /// The Windows, <kbd>⌘</kbd>, <kbd>Command</kbd>, or other OS symbol key.
    SuperRight,
    /// <kbd>Shift</kbd> or <kbd>⇧</kbd>
    ShiftLeft,
    /// <kbd>Shift</kbd> or <kbd>⇧</kbd>
    ShiftRight,
    /// <kbd> </kbd> (space)
    Space,
    /// <kbd>Tab</kbd> or <kbd>⇥</kbd>
    Tab,
    /// Japanese: <kbd>変</kbd> (henkan)
    Convert,
    /// Japanese: <kbd>カタカナ</kbd>/<kbd>ひらがな</kbd>/<kbd>ローマ字</kbd> (katakana/hiragana/romaji)
    KanaMode,
    /// Korean: HangulMode <kbd>한/영</kbd> (han/yeong)
    ///
    /// Japanese (Mac keyboard): <kbd>か</kbd> (kana)
    Lang1,
    /// Korean: Hanja <kbd>한</kbd> (hanja)
    ///
    /// Japanese (Mac keyboard): <kbd>英</kbd> (eisu)
    Lang2,
    /// Japanese (word-processing keyboard): Katakana
    Lang3,
    /// Japanese (word-processing keyboard): Hiragana
    Lang4,
    /// Japanese (word-processing keyboard): Zenkaku/Hankaku
    Lang5,
    /// Japanese: <kbd>無変換</kbd> (muhenkan)
    NonConvert,
    /// <kbd>⌦</kbd>. The forward delete key.
    /// Note that on Apple keyboards, the key labelled <kbd>Delete</kbd> on the main part of
    /// the keyboard is encoded as [`Backspace`].
    ///
    /// [`Backspace`]: Self::Backspace
    Delete,
    /// <kbd>Page Down</kbd>, <kbd>End</kbd>, or <kbd>↘</kbd>
    End,
    /// <kbd>Help</kbd>. Not present on standard PC keyboards.
    Help,
    /// <kbd>Home</kbd> or <kbd>↖</kbd>
    Home,
    /// <kbd>Insert</kbd> or <kbd>Ins</kbd>. Not present on Apple keyboards.
    Insert,
    /// <kbd>Page Down</kbd>, <kbd>PgDn</kbd>, or <kbd>⇟</kbd>
    PageDown,
    /// <kbd>Page Up</kbd>, <kbd>PgUp</kbd>, or <kbd>⇞</kbd>
    PageUp,
    /// <kbd>↓</kbd>
    ArrowDown,
    /// <kbd>←</kbd>
    ArrowLeft,
    /// <kbd>→</kbd>
    ArrowRight,
    /// <kbd>↑</kbd>
    ArrowUp,
    /// On the Mac, this is used for the numpad <kbd>Clear</kbd> key.
    NumLock,
    /// <kbd>0 Ins</kbd> on a keyboard. <kbd>0</kbd> on a phone or remote control
    Numpad0,
    /// <kbd>1 End</kbd> on a keyboard. <kbd>1</kbd> or <kbd>1 QZ</kbd> on a phone or remote control
    Numpad1,
    /// <kbd>2 ↓</kbd> on a keyboard. <kbd>2 ABC</kbd> on a phone or remote control
    Numpad2,
    /// <kbd>3 PgDn</kbd> on a keyboard. <kbd>3 DEF</kbd> on a phone or remote control
    Numpad3,
    /// <kbd>4 ←</kbd> on a keyboard. <kbd>4 GHI</kbd> on a phone or remote control
    Numpad4,
    /// <kbd>5</kbd> on a keyboard. <kbd>5 JKL</kbd> on a phone or remote control
    Numpad5,
    /// <kbd>6 →</kbd> on a keyboard. <kbd>6 MNO</kbd> on a phone or remote control
    Numpad6,
    /// <kbd>7 Home</kbd> on a keyboard. <kbd>7 PQRS</kbd> or <kbd>7 PRS</kbd> on a phone
    /// or remote control
    Numpad7,
    /// <kbd>8 ↑</kbd> on a keyboard. <kbd>8 TUV</kbd> on a phone or remote control
    Numpad8,
    /// <kbd>9 PgUp</kbd> on a keyboard. <kbd>9 WXYZ</kbd> or <kbd>9 WXY</kbd> on a phone
    /// or remote control
    Numpad9,
    /// <kbd>+</kbd>
    NumpadAdd,
    /// Found on the Microsoft Natural Keyboard.
    NumpadBackspace,
    /// <kbd>C</kbd> or <kbd>A</kbd> (All Clear). Also for use with numpads that have a
    /// <kbd>Clear</kbd> key that is separate from the <kbd>NumLock</kbd> key. On the Mac, the
    /// numpad <kbd>Clear</kbd> key is encoded as [`NumLock`].
    ///
    /// [`NumLock`]: Self::NumLock
    NumpadClear,
    /// <kbd>C</kbd> (Clear Entry)
    NumpadClearEntry,
    /// <kbd>,</kbd> (thousands separator). For locales where the thousands separator
    /// is a "." (e.g., Brazil), this key may generate a <kbd>.</kbd>.
    NumpadComma,
    /// <kbd>. Del</kbd>. For locales where the decimal separator is "," (e.g.,
    /// Brazil), this key may generate a <kbd>,</kbd>.
    NumpadDecimal,
    /// <kbd>/</kbd>
    NumpadDivide,
    NumpadEnter,
    /// <kbd>=</kbd>
    NumpadEqual,
    /// <kbd>#</kbd> on a phone or remote control device. This key is typically found
    /// below the <kbd>9</kbd> key and to the right of the <kbd>0</kbd> key.
    NumpadHash,
    /// <kbd>M</kbd> Add current entry to the value stored in memory.
    NumpadMemoryAdd,
    /// <kbd>M</kbd> Clear the value stored in memory.
    NumpadMemoryClear,
    /// <kbd>M</kbd> Replace the current entry with the value stored in memory.
    NumpadMemoryRecall,
    /// <kbd>M</kbd> Replace the value stored in memory with the current entry.
    NumpadMemoryStore,
    /// <kbd>M</kbd> Subtract current entry from the value stored in memory.
    NumpadMemorySubtract,
    /// <kbd>*</kbd> on a keyboard. For use with numpads that provide mathematical
    /// operations (<kbd>+</kbd>, <kbd>-</kbd> <kbd>*</kbd> and <kbd>/</kbd>).
    ///
    /// Use `NumpadStar` for the <kbd>*</kbd> key on phones and remote controls.
    NumpadMultiply,
    /// <kbd>(</kbd> Found on the Microsoft Natural Keyboard.
    NumpadParenLeft,
    /// <kbd>)</kbd> Found on the Microsoft Natural Keyboard.
    NumpadParenRight,
    /// <kbd>*</kbd> on a phone or remote control device.
    ///
    /// This key is typically found below the <kbd>7</kbd> key and to the left of
    /// the <kbd>0</kbd> key.
    ///
    /// Use <kbd>"NumpadMultiply"</kbd> for the <kbd>*</kbd> key on
    /// numeric keypads.
    NumpadStar,
    /// <kbd>-</kbd>
    NumpadSubtract,
    /// <kbd>Esc</kbd> or <kbd>⎋</kbd>
    Escape,
    /// <kbd>Fn</kbd> This is typically a hardware key that does not generate a separate code.
    Fn,
    /// <kbd>FLock</kbd> or <kbd>FnLock</kbd>. Function Lock key. Found on the Microsoft
    /// Natural Keyboard.
    FnLock,
    /// <kbd>PrtScr SysRq</kbd> or <kbd>Print Screen</kbd>
    PrintScreen,
    /// <kbd>Scroll Lock</kbd>
    ScrollLock,
    /// <kbd>Pause Break</kbd>
    Pause,
    /// Some laptops place this key to the left of the <kbd>↑</kbd> key.
    ///
    /// This also the "back" button (triangle) on Android.
    BrowserBack,
    BrowserFavorites,
    /// Some laptops place this key to the right of the <kbd>↑</kbd> key.
    BrowserForward,
    /// The "home" button on Android.
    BrowserHome,
    BrowserRefresh,
    BrowserSearch,
    BrowserStop,
    /// <kbd>Eject</kbd> or <kbd>⏏</kbd>. This key is placed in the function section on some Apple
    /// keyboards.
    Eject,
    /// Sometimes labelled <kbd>My Computer</kbd> on the keyboard
    LaunchApp1,
    /// Sometimes labelled <kbd>Calculator</kbd> on the keyboard
    LaunchApp2,
    LaunchMail,
    MediaPlayPause,
    MediaSelect,
    MediaStop,
    MediaTrackNext,
    MediaTrackPrevious,
    /// This key is placed in the function section on some Apple keyboards, replacing the
    /// <kbd>Eject</kbd> key.
    Power,
    Sleep,
    AudioVolumeDown,
    AudioVolumeMute,
    AudioVolumeUp,
    WakeUp,
    // Legacy modifier key. Also called "Super" in certain places.
    Meta,
    // Legacy modifier key.
    Hyper,
    Turbo,
    Abort,
    Resume,
    Suspend,
    /// Found on Sun’s USB keyboard.
    Again,
    /// Found on Sun’s USB keyboard.
    Copy,
    /// Found on Sun’s USB keyboard.
    Cut,
    /// Found on Sun’s USB keyboard.
    Find,
    /// Found on Sun’s USB keyboard.
    Open,
    /// Found on Sun’s USB keyboard.
    Paste,
    /// Found on Sun’s USB keyboard.
    Props,
    /// Found on Sun’s USB keyboard.
    Select,
    /// Found on Sun’s USB keyboard.
    Undo,
    /// Use for dedicated <kbd>ひらがな</kbd> key found on some Japanese word processing keyboards.
    Hiragana,
    /// Use for dedicated <kbd>カタカナ</kbd> key found on some Japanese word processing keyboards.
    Katakana,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F1,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F2,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F3,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F4,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F5,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F6,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F7,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F8,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F9,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F10,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F11,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F12,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F13,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F14,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F15,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F16,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F17,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F18,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F19,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F20,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F21,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F22,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F23,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F24,
    /// General-purpose function key.
    F25,
    /// General-purpose function key.
    F26,
    /// General-purpose function key.
    F27,
    /// General-purpose function key.
    F28,
    /// General-purpose function key.
    F29,
    /// General-purpose function key.
    F30,
    /// General-purpose function key.
    F31,
    /// General-purpose function key.
    F32,
    /// General-purpose function key.
    F33,
    /// General-purpose function key.
    F34,
    /// General-purpose function key.
    F35,
}

/// A fixed set of cursor icons that available on most OSes.
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Default,
    Visit,
    Reflect,
    AsRefStr,
    EnumString,
    VariantNames,
)]
pub enum CursorIcon {
    /// The platform-dependent default cursor. Often rendered as arrow.
    #[default]
    Default,

    /// A context menu is available for the object under the cursor. Often
    /// rendered as an arrow with a small menu-like graphic next to it.
    ContextMenu,

    /// Help is available for the object under the cursor. Often rendered as a
    /// question mark or a balloon.
    Help,

    /// The cursor is a pointer that indicates a link. Often rendered as the
    /// backside of a hand with the index finger extended.
    Pointer,

    /// A progress indicator. The program is performing some processing, but is
    /// different from [`CursorIcon::Wait`] in that the user may still interact
    /// with the program.
    Progress,

    /// Indicates that the program is busy and the user should wait. Often
    /// rendered as a watch or hourglass.
    Wait,

    /// Indicates that a cell or set of cells may be selected. Often rendered as
    /// a thick plus-sign with a dot in the middle.
    Cell,

    /// A simple crosshair (e.g., short line segments resembling a "+" sign).
    /// Often used to indicate a two dimensional bitmap selection mode.
    Crosshair,

    /// Indicates text that may be selected. Often rendered as an I-beam.
    Text,

    /// Indicates vertical-text that may be selected. Often rendered as a
    /// horizontal I-beam.
    VerticalText,

    /// Indicates an alias of/shortcut to something is to be created. Often
    /// rendered as an arrow with a small curved arrow next to it.
    Alias,

    /// Indicates something is to be copied. Often rendered as an arrow with a
    /// small plus sign next to it.
    Copy,

    /// Indicates something is to be moved.
    Move,

    /// Indicates that the dragged item cannot be dropped at the current cursor
    /// location. Often rendered as a hand or pointer with a small circle with a
    /// line through it.
    NoDrop,

    /// Indicates that the requested action will not be carried out. Often
    /// rendered as a circle with a line through it.
    NotAllowed,

    /// Indicates that something can be grabbed (dragged to be moved). Often
    /// rendered as the backside of an open hand.
    Grab,

    /// Indicates that something is being grabbed (dragged to be moved). Often
    /// rendered as the backside of a hand with fingers closed mostly out of
    /// view.
    Grabbing,

    /// The east border to be moved.
    EResize,

    /// The north border to be moved.
    NResize,

    /// The north-east corner to be moved.
    NeResize,

    /// The north-west corner to be moved.
    NwResize,

    /// The south border to be moved.
    SResize,

    /// The south-east corner to be moved.
    SeResize,

    /// The south-west corner to be moved.
    SwResize,

    /// The west border to be moved.
    WResize,

    /// The east and west borders to be moved.
    EwResize,

    /// The south and north borders to be moved.
    NsResize,

    /// The north-east and south-west corners to be moved.
    NeswResize,

    /// The north-west and south-east corners to be moved.
    NwseResize,

    /// Indicates that the item/column can be resized horizontally. Often
    /// rendered as arrows pointing left and right with a vertical bar
    /// separating them.
    ColResize,

    /// Indicates that the item/row can be resized vertically. Often rendered as
    /// arrows pointing up and down with a horizontal bar separating them.
    RowResize,

    /// Indicates that the something can be scrolled in any direction. Often
    /// rendered as arrows pointing up, down, left, and right with a dot in the
    /// middle.
    AllScroll,

    /// Indicates that something can be zoomed in. Often rendered as a
    /// magnifying glass with a "+" in the center of the glass.
    ZoomIn,

    /// Indicates that something can be zoomed in. Often rendered as a
    /// magnifying glass with a "-" in the center of the glass.
    ZoomOut,
}

uuid_provider!(CursorIcon = "da7f3a5f-9d26-460a-8e46-38da25f8a8db");
