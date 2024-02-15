//! A set of editors for hot keys and key bindings. See [`HotKeyEditor`] and [`KeyBindingEditor`] widget's docs
//! for more info and usage examples.

#![warn(missing_docs)]

use crate::{
    brush::Brush,
    core::{
        color::Color, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*,
    },
    define_constructor, define_widget_deref,
    draw::{CommandTexture, Draw, DrawingContext},
    message::{KeyCode, KeyboardModifiers, MessageDirection, MouseButton, UiMessage},
    text::{TextBuilder, TextMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, UiNode, UserInterface,
};
use fyrox_core::uuid_provider;
use fyrox_core::variable::InheritableVariable;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    ops::{Deref, DerefMut},
};

/// Hot key is a combination of a key code with an arbitrary set of keyboard modifiers (such as Ctrl, Shift, Alt keys).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Reflect, Default, Visit)]
pub enum HotKey {
    /// Unset hot key. Does nothing. This is default value.
    #[default]
    NotSet,
    /// Some hot key.
    Some {
        /// Physical key code.
        code: KeyCode,
        /// A set of keyboard modifiers.
        modifiers: KeyboardModifiers,
    },
}

impl HotKey {
    /// Creates a new hot key that consists of a single key, without any modifiers.
    pub fn from_key_code(key: KeyCode) -> Self {
        Self::Some {
            code: key,
            modifiers: Default::default(),
        }
    }

    /// Creates a new hot key, that consists of combination `Ctrl + Key`.
    pub fn ctrl_key(key: KeyCode) -> Self {
        Self::Some {
            code: key,
            modifiers: KeyboardModifiers {
                control: true,
                ..Default::default()
            },
        }
    }

    /// Creates a new hot key, that consists of combination `Shift + Key`.
    pub fn shift_key(key: KeyCode) -> Self {
        Self::Some {
            code: key,
            modifiers: KeyboardModifiers {
                shift: true,
                ..Default::default()
            },
        }
    }

    /// Creates a new hot key, that consists of combination `Alt + Key`.
    pub fn alt_key(key: KeyCode) -> Self {
        Self::Some {
            code: key,
            modifiers: KeyboardModifiers {
                shift: true,
                ..Default::default()
            },
        }
    }
}

impl Display for HotKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HotKey::NotSet => f.write_str("Not Set"),
            HotKey::Some { code, modifiers } => {
                if modifiers.control {
                    f.write_str("Ctrl+")?;
                }
                if modifiers.alt {
                    f.write_str("Alt+")?;
                }
                if modifiers.shift {
                    f.write_str("Shift+")?;
                }
                if modifiers.system {
                    f.write_str("Sys+")?;
                }
                write!(f, "{}", code.as_ref())
            }
        }
    }
}

/// A set of messages, that is used to alternate the state of [`HotKeyEditor`] widget or to listen to its changes.
#[derive(Debug, Clone, PartialEq)]
pub enum HotKeyEditorMessage {
    /// A message, that is either used to modify current value of a [`HotKey`] widget instance (with [`MessageDirection::ToWidget`])
    /// or to listen to its changes (with [`MessageDirection::FromWidget`]).
    Value(HotKey),
}

impl HotKeyEditorMessage {
    define_constructor!(
        /// Creates [`HotKeyEditorMessage::Value`] message.
        HotKeyEditorMessage:Value => fn value(HotKey), layout: false
    );
}

/// Hot key editor is used to provide a unified way of editing an arbitrary combination of modifiers keyboard keys (such
/// as Ctrl, Shift, Alt) with any other key. It could be used, if you need a simple way to add an editor for [`HotKey`].
///
/// ## Examples
///
/// The following example creates a new hot key editor with a `Ctrl+C` hot key as default value:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     key::{HotKey, HotKeyEditorBuilder},
/// #     message::{KeyCode, KeyboardModifiers},
/// #     widget::WidgetBuilder,
/// #     BuildContext, UiNode,
/// # };
/// #
/// fn create_hot_key_editor(ctx: &mut BuildContext) -> Handle<UiNode> {
///     HotKeyEditorBuilder::new(WidgetBuilder::new())
///         .with_value(
///             // Ctrl+C hot key.
///             HotKey::Some {
///                 code: KeyCode::KeyC,
///                 modifiers: KeyboardModifiers {
///                     control: true,
///                     ..Default::default()
///                 },
///             },
///         )
///         .build(ctx)
/// }
/// ```
///
/// ## Messages
///
/// Use [`HotKeyEditorMessage`] message to alternate the state of a hot key widget, or to listen to its changes.
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct HotKeyEditor {
    widget: Widget,
    text: InheritableVariable<Handle<UiNode>>,
    value: InheritableVariable<HotKey>,
    editing: InheritableVariable<bool>,
}

define_widget_deref!(HotKeyEditor);

impl HotKeyEditor {
    fn set_editing(&mut self, editing: bool, ui: &UserInterface) {
        self.editing.set_value_and_mark_modified(editing);
        ui.send_message(TextMessage::text(
            *self.text,
            MessageDirection::ToWidget,
            if *self.editing {
                "[WAITING INPUT]".to_string()
            } else {
                format!("{}", *self.value)
            },
        ));
    }
}

uuid_provider!(HotKeyEditor = "7bc49843-1302-4e36-b901-63af5cea6c60");

impl Control for HotKeyEditor {
    fn draw(&self, drawing_context: &mut DrawingContext) {
        // Make background clickable.
        drawing_context.push_rect_filled(&self.bounding_rect(), None);
        drawing_context.commit(
            self.clip_bounds(),
            Brush::Solid(Color::TRANSPARENT),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<WidgetMessage>() {
            match msg {
                WidgetMessage::KeyDown(key) => {
                    if *self.editing
                        && !matches!(
                            *key,
                            KeyCode::ControlLeft
                                | KeyCode::ControlRight
                                | KeyCode::ShiftLeft
                                | KeyCode::ShiftRight
                                | KeyCode::AltLeft
                                | KeyCode::AltRight
                        )
                    {
                        ui.send_message(HotKeyEditorMessage::value(
                            self.handle,
                            MessageDirection::ToWidget,
                            HotKey::Some {
                                code: *key,
                                modifiers: ui.keyboard_modifiers,
                            },
                        ));

                        message.set_handled(true);
                    }
                }
                WidgetMessage::MouseDown { button, .. } => {
                    if *button == MouseButton::Left {
                        if *self.editing {
                            self.set_editing(false, ui);
                        } else {
                            self.set_editing(true, ui);
                        }
                    }
                }
                WidgetMessage::Unfocus => {
                    if *self.editing {
                        self.set_editing(false, ui);
                    }
                }
                _ => (),
            }
        }

        if message.destination() == self.handle && message.direction() == MessageDirection::ToWidget
        {
            if let Some(HotKeyEditorMessage::Value(value)) = message.data() {
                if value != &*self.value {
                    self.value.set_value_and_mark_modified(value.clone());

                    ui.send_message(TextMessage::text(
                        *self.text,
                        MessageDirection::ToWidget,
                        format!("{}", *self.value),
                    ));

                    ui.send_message(message.reverse());
                }
            }
        }
    }
}

/// Hot key editor builder creates [`HotKeyEditor`] widget instances and adds them to the user interface.
pub struct HotKeyEditorBuilder {
    widget_builder: WidgetBuilder,
    value: HotKey,
}

impl HotKeyEditorBuilder {
    /// Creates a new hot key editor builder.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: HotKey::NotSet,
        }
    }

    /// Sets the desired default value of the hot key editor.
    pub fn with_value(mut self, hot_key: HotKey) -> Self {
        self.value = hot_key;
        self
    }

    /// Finishes widget building and adds it to the user interface, returning a handle to the new instance.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let text = TextBuilder::new(WidgetBuilder::new())
            .with_text(format!("{}", self.value))
            .build(ctx);

        let editor = HotKeyEditor {
            widget: self.widget_builder.with_child(text).build(),
            text: text.into(),
            editing: false.into(),
            value: self.value.into(),
        };

        ctx.add_node(UiNode::new(editor))
    }
}

/// Key binding is a simplified version of [`HotKey`] that consists of a single physical key code. It is usually
/// used for "unconditional" (independent of modifier keys state) triggering of some action.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Reflect, Visit, Default)]
pub enum KeyBinding {
    /// Unset key binding. Does nothing.
    #[default]
    NotSet,
    /// Some physical key binding.
    Some(KeyCode),
}

impl PartialEq<KeyCode> for KeyBinding {
    fn eq(&self, other: &KeyCode) -> bool {
        match self {
            KeyBinding::NotSet => false,
            KeyBinding::Some(code) => code == other,
        }
    }
}

impl KeyBinding {
    /// Creates a new key binding from a physical key code.
    pub fn from_key_code(key: KeyCode) -> Self {
        Self::Some(key)
    }
}

impl Display for KeyBinding {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotSet => f.write_str("Not Set"),
            Self::Some(code) => write!(f, "{}", code.as_ref()),
        }
    }
}

/// A set of messages, that is used to modify [`KeyBindingEditor`] state or to listen to its changes.
#[derive(Debug, Clone, PartialEq)]
pub enum KeyBindingEditorMessage {
    /// A message, that is used to fetch a new value of a key binding, or to set new one.
    Value(KeyBinding),
}

impl KeyBindingEditorMessage {
    define_constructor!(
        /// Creates [`KeyBindingEditorMessage::Value`] message.
        KeyBindingEditorMessage:Value => fn value(KeyBinding), layout: false);
}

/// Key binding editor is used to provide a unified way of setting a key binding.
///
/// ## Examples
///
/// The following example creates a new key binding editor with a `W` key binding as a value.
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     key::{KeyBinding, KeyBindingEditorBuilder},
/// #     message::KeyCode,
/// #     widget::WidgetBuilder,
/// #     BuildContext, UiNode,
/// # };
/// #
/// fn create_key_binding_editor(ctx: &mut BuildContext) -> Handle<UiNode> {
///     KeyBindingEditorBuilder::new(WidgetBuilder::new())
///         .with_value(KeyBinding::Some(KeyCode::KeyW))
///         .build(ctx)
/// }
/// ```
///
/// ## Messages
///
/// Use [`KeyBindingEditorMessage`] message to alternate the state of a key binding widget, or to listen to its changes.
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct KeyBindingEditor {
    widget: Widget,
    text: InheritableVariable<Handle<UiNode>>,
    value: InheritableVariable<KeyBinding>,
    editing: InheritableVariable<bool>,
}

define_widget_deref!(KeyBindingEditor);

impl KeyBindingEditor {
    fn set_editing(&mut self, editing: bool, ui: &UserInterface) {
        self.editing.set_value_and_mark_modified(editing);
        ui.send_message(TextMessage::text(
            *self.text,
            MessageDirection::ToWidget,
            if *self.editing {
                "[WAITING INPUT]".to_string()
            } else {
                format!("{}", *self.value)
            },
        ));
    }
}

uuid_provider!(KeyBindingEditor = "150113ce-f95e-4c76-9ac9-4503e78b960f");

impl Control for KeyBindingEditor {
    fn draw(&self, drawing_context: &mut DrawingContext) {
        // Make background clickable.
        drawing_context.push_rect_filled(&self.bounding_rect(), None);
        drawing_context.commit(
            self.clip_bounds(),
            Brush::Solid(Color::TRANSPARENT),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<WidgetMessage>() {
            match msg {
                WidgetMessage::KeyDown(key) => {
                    ui.send_message(KeyBindingEditorMessage::value(
                        self.handle,
                        MessageDirection::ToWidget,
                        KeyBinding::Some(*key),
                    ));

                    message.set_handled(true);
                }
                WidgetMessage::MouseDown { button, .. } => {
                    if *button == MouseButton::Left {
                        if *self.editing {
                            self.set_editing(false, ui);
                        } else {
                            self.set_editing(true, ui);
                        }
                    }
                }
                WidgetMessage::Unfocus => {
                    if *self.editing {
                        self.set_editing(false, ui);
                    }
                }
                _ => (),
            }
        }

        if message.destination() == self.handle && message.direction() == MessageDirection::ToWidget
        {
            if let Some(KeyBindingEditorMessage::Value(value)) = message.data() {
                if value != &*self.value {
                    self.value.set_value_and_mark_modified(value.clone());

                    ui.send_message(TextMessage::text(
                        *self.text,
                        MessageDirection::ToWidget,
                        format!("{}", *self.value),
                    ));

                    ui.send_message(message.reverse());
                }
            }
        }
    }
}

/// Key binding editor builder is used to create [`KeyBindingEditor`] widgets and add them to the user interface.
pub struct KeyBindingEditorBuilder {
    widget_builder: WidgetBuilder,
    value: KeyBinding,
}

impl KeyBindingEditorBuilder {
    /// Creates a new key binding editor builder.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: KeyBinding::NotSet,
        }
    }

    /// Sets the desired key binding value.
    pub fn with_value(mut self, key_binding: KeyBinding) -> Self {
        self.value = key_binding;
        self
    }

    /// Finishes widget building and adds the new widget instance to the user interface, returning a handle of it.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let text = TextBuilder::new(WidgetBuilder::new())
            .with_text(format!("{}", self.value))
            .build(ctx);

        let editor = KeyBindingEditor {
            widget: self.widget_builder.with_child(text).build(),
            text: text.into(),
            editing: false.into(),
            value: self.value.into(),
        };

        ctx.add_node(UiNode::new(editor))
    }
}
