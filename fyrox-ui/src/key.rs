use crate::{
    brush::Brush,
    core::{color::Color, pool::Handle, reflect::prelude::*},
    define_constructor, define_widget_deref,
    draw::{CommandTexture, Draw, DrawingContext},
    message::{KeyCode, KeyboardModifiers, MessageDirection, MouseButton, UiMessage},
    text::{TextBuilder, TextMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, UiNode, UserInterface,
};
use serde::{Deserialize, Serialize};
use std::{
    any::{Any, TypeId},
    fmt::{Display, Formatter},
    ops::{Deref, DerefMut},
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub enum HotKey {
    NotSet,
    Some {
        code: KeyCode,
        modifiers: KeyboardModifiers,
    },
}

impl HotKey {
    pub fn from_key_code(key: KeyCode) -> Self {
        Self::Some {
            code: key,
            modifiers: Default::default(),
        }
    }

    pub fn ctrl_key(key: KeyCode) -> Self {
        Self::Some {
            code: key,
            modifiers: KeyboardModifiers {
                control: true,
                ..Default::default()
            },
        }
    }

    pub fn shift_key(key: KeyCode) -> Self {
        Self::Some {
            code: key,
            modifiers: KeyboardModifiers {
                shift: true,
                ..Default::default()
            },
        }
    }

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

impl Default for HotKey {
    fn default() -> Self {
        Self::NotSet
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

#[derive(Debug, Clone, PartialEq)]
pub enum HotKeyEditorMessage {
    Value(HotKey),
}

impl HotKeyEditorMessage {
    define_constructor!(Self:Value => fn value(HotKey), layout: false);
}

#[derive(Clone)]
pub struct HotKeyEditor {
    widget: Widget,
    text: Handle<UiNode>,
    value: HotKey,
    editing: bool,
}

define_widget_deref!(HotKeyEditor);

impl HotKeyEditor {
    fn set_editing(&mut self, editing: bool, ui: &UserInterface) {
        self.editing = editing;
        ui.send_message(TextMessage::text(
            self.text,
            MessageDirection::ToWidget,
            if self.editing {
                "[WAITING INPUT]".to_string()
            } else {
                format!("{}", self.value)
            },
        ));
    }
}

impl Control for HotKeyEditor {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

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
                    if self.editing
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
                        if self.editing {
                            self.set_editing(false, ui);
                        } else {
                            self.set_editing(true, ui);
                        }
                    }
                }
                WidgetMessage::Unfocus => {
                    if self.editing {
                        self.set_editing(false, ui);
                    }
                }
                _ => (),
            }
        }

        if message.destination() == self.handle && message.direction() == MessageDirection::ToWidget
        {
            if let Some(HotKeyEditorMessage::Value(value)) = message.data() {
                if value != &self.value {
                    self.value = value.clone();

                    ui.send_message(TextMessage::text(
                        self.text,
                        MessageDirection::ToWidget,
                        format!("{}", self.value),
                    ));

                    ui.send_message(message.reverse());
                }
            }
        }
    }
}

pub struct HotKeyEditorBuilder {
    widget_builder: WidgetBuilder,
    value: HotKey,
}

impl HotKeyEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: HotKey::NotSet,
        }
    }

    pub fn with_value(mut self, hot_key: HotKey) -> Self {
        self.value = hot_key;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let text = TextBuilder::new(WidgetBuilder::new())
            .with_text(format!("{}", self.value))
            .build(ctx);

        let editor = HotKeyEditor {
            widget: self.widget_builder.with_child(text).build(),
            text,
            editing: false,
            value: self.value,
        };

        ctx.add_node(UiNode::new(editor))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub enum KeyBinding {
    NotSet,
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
    pub fn from_key_code(key: KeyCode) -> Self {
        Self::Some(key)
    }
}

impl Default for KeyBinding {
    fn default() -> Self {
        Self::NotSet
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

#[derive(Debug, Clone, PartialEq)]
pub enum KeyBindingEditorMessage {
    Value(KeyBinding),
}

impl KeyBindingEditorMessage {
    define_constructor!(Self:Value => fn value(KeyBinding), layout: false);
}

#[derive(Clone)]
pub struct KeyBindingEditor {
    widget: Widget,
    text: Handle<UiNode>,
    value: KeyBinding,
    editing: bool,
}

define_widget_deref!(KeyBindingEditor);

impl KeyBindingEditor {
    fn set_editing(&mut self, editing: bool, ui: &UserInterface) {
        self.editing = editing;
        ui.send_message(TextMessage::text(
            self.text,
            MessageDirection::ToWidget,
            if self.editing {
                "[WAITING INPUT]".to_string()
            } else {
                format!("{}", self.value)
            },
        ));
    }
}

impl Control for KeyBindingEditor {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

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
                        if self.editing {
                            self.set_editing(false, ui);
                        } else {
                            self.set_editing(true, ui);
                        }
                    }
                }
                WidgetMessage::Unfocus => {
                    if self.editing {
                        self.set_editing(false, ui);
                    }
                }
                _ => (),
            }
        }

        if message.destination() == self.handle && message.direction() == MessageDirection::ToWidget
        {
            if let Some(KeyBindingEditorMessage::Value(value)) = message.data() {
                if value != &self.value {
                    self.value = value.clone();

                    ui.send_message(TextMessage::text(
                        self.text,
                        MessageDirection::ToWidget,
                        format!("{}", self.value),
                    ));

                    ui.send_message(message.reverse());
                }
            }
        }
    }
}

pub struct KeyBindingEditorBuilder {
    widget_builder: WidgetBuilder,
    value: KeyBinding,
}

impl KeyBindingEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: KeyBinding::NotSet,
        }
    }

    pub fn with_value(mut self, key_binding: KeyBinding) -> Self {
        self.value = key_binding;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let text = TextBuilder::new(WidgetBuilder::new())
            .with_text(format!("{}", self.value))
            .build(ctx);

        let editor = KeyBindingEditor {
            widget: self.widget_builder.with_child(text).build(),
            text,
            editing: false,
            value: self.value,
        };

        ctx.add_node(UiNode::new(editor))
    }
}
