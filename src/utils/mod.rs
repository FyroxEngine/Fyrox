#![warn(missing_docs)]

//! Utilities module provides set of commonly used algorithms.

pub mod astar;
pub mod behavior;
pub mod lightmap;
pub mod log;
pub mod navmesh;
pub mod raw_mesh;
pub mod uvgen;

use crate::core::algebra::Vector2;
use crate::{
    event::{ElementState, ModifiersState, MouseScrollDelta, VirtualKeyCode, WindowEvent},
    gui::{
        draw,
        message::{ButtonState, KeyCode, KeyboardModifiers, OsEvent},
    },
    resource::texture::Texture,
};
use std::hash::Hasher;
use std::{any::Any, sync::Arc};

/// Translated key code to fyrox-ui key code.
pub fn translate_key(key: VirtualKeyCode) -> KeyCode {
    match key {
        VirtualKeyCode::Key1 => KeyCode::Key1,
        VirtualKeyCode::Key2 => KeyCode::Key2,
        VirtualKeyCode::Key3 => KeyCode::Key3,
        VirtualKeyCode::Key4 => KeyCode::Key4,
        VirtualKeyCode::Key5 => KeyCode::Key5,
        VirtualKeyCode::Key6 => KeyCode::Key6,
        VirtualKeyCode::Key7 => KeyCode::Key7,
        VirtualKeyCode::Key8 => KeyCode::Key8,
        VirtualKeyCode::Key9 => KeyCode::Key9,
        VirtualKeyCode::Key0 => KeyCode::Key0,
        VirtualKeyCode::A => KeyCode::A,
        VirtualKeyCode::B => KeyCode::B,
        VirtualKeyCode::C => KeyCode::C,
        VirtualKeyCode::D => KeyCode::D,
        VirtualKeyCode::E => KeyCode::E,
        VirtualKeyCode::F => KeyCode::F,
        VirtualKeyCode::G => KeyCode::G,
        VirtualKeyCode::H => KeyCode::H,
        VirtualKeyCode::I => KeyCode::I,
        VirtualKeyCode::J => KeyCode::J,
        VirtualKeyCode::K => KeyCode::K,
        VirtualKeyCode::L => KeyCode::L,
        VirtualKeyCode::M => KeyCode::M,
        VirtualKeyCode::N => KeyCode::N,
        VirtualKeyCode::O => KeyCode::O,
        VirtualKeyCode::P => KeyCode::P,
        VirtualKeyCode::Q => KeyCode::Q,
        VirtualKeyCode::R => KeyCode::R,
        VirtualKeyCode::S => KeyCode::S,
        VirtualKeyCode::T => KeyCode::T,
        VirtualKeyCode::U => KeyCode::U,
        VirtualKeyCode::V => KeyCode::V,
        VirtualKeyCode::W => KeyCode::W,
        VirtualKeyCode::X => KeyCode::X,
        VirtualKeyCode::Y => KeyCode::Y,
        VirtualKeyCode::Z => KeyCode::Z,
        VirtualKeyCode::Escape => KeyCode::Escape,
        VirtualKeyCode::F1 => KeyCode::F1,
        VirtualKeyCode::F2 => KeyCode::F2,
        VirtualKeyCode::F3 => KeyCode::F3,
        VirtualKeyCode::F4 => KeyCode::F4,
        VirtualKeyCode::F5 => KeyCode::F5,
        VirtualKeyCode::F6 => KeyCode::F6,
        VirtualKeyCode::F7 => KeyCode::F7,
        VirtualKeyCode::F8 => KeyCode::F8,
        VirtualKeyCode::F9 => KeyCode::F9,
        VirtualKeyCode::F10 => KeyCode::F10,
        VirtualKeyCode::F11 => KeyCode::F11,
        VirtualKeyCode::F12 => KeyCode::F12,
        VirtualKeyCode::F13 => KeyCode::F13,
        VirtualKeyCode::F14 => KeyCode::F14,
        VirtualKeyCode::F15 => KeyCode::F15,
        VirtualKeyCode::F16 => KeyCode::F16,
        VirtualKeyCode::F17 => KeyCode::F17,
        VirtualKeyCode::F18 => KeyCode::F18,
        VirtualKeyCode::F19 => KeyCode::F19,
        VirtualKeyCode::F20 => KeyCode::F20,
        VirtualKeyCode::F21 => KeyCode::F21,
        VirtualKeyCode::F22 => KeyCode::F22,
        VirtualKeyCode::F23 => KeyCode::F23,
        VirtualKeyCode::F24 => KeyCode::F24,
        VirtualKeyCode::Snapshot => KeyCode::Snapshot,
        VirtualKeyCode::Scroll => KeyCode::Scroll,
        VirtualKeyCode::Pause => KeyCode::Pause,
        VirtualKeyCode::Insert => KeyCode::Insert,
        VirtualKeyCode::Home => KeyCode::Home,
        VirtualKeyCode::Delete => KeyCode::Delete,
        VirtualKeyCode::End => KeyCode::End,
        VirtualKeyCode::PageDown => KeyCode::PageDown,
        VirtualKeyCode::PageUp => KeyCode::PageUp,
        VirtualKeyCode::Left => KeyCode::Left,
        VirtualKeyCode::Up => KeyCode::Up,
        VirtualKeyCode::Right => KeyCode::Right,
        VirtualKeyCode::Down => KeyCode::Down,
        VirtualKeyCode::Back => KeyCode::Backspace,
        VirtualKeyCode::Return => KeyCode::Return,
        VirtualKeyCode::Space => KeyCode::Space,
        VirtualKeyCode::Compose => KeyCode::Compose,
        VirtualKeyCode::Caret => KeyCode::Caret,
        VirtualKeyCode::Numlock => KeyCode::Numlock,
        VirtualKeyCode::Numpad0 => KeyCode::Numpad0,
        VirtualKeyCode::Numpad1 => KeyCode::Numpad1,
        VirtualKeyCode::Numpad2 => KeyCode::Numpad2,
        VirtualKeyCode::Numpad3 => KeyCode::Numpad3,
        VirtualKeyCode::Numpad4 => KeyCode::Numpad4,
        VirtualKeyCode::Numpad5 => KeyCode::Numpad5,
        VirtualKeyCode::Numpad6 => KeyCode::Numpad6,
        VirtualKeyCode::Numpad7 => KeyCode::Numpad7,
        VirtualKeyCode::Numpad8 => KeyCode::Numpad8,
        VirtualKeyCode::Numpad9 => KeyCode::Numpad9,
        VirtualKeyCode::AbntC1 => KeyCode::AbntC1,
        VirtualKeyCode::AbntC2 => KeyCode::AbntC2,
        VirtualKeyCode::NumpadAdd => KeyCode::NumpadAdd,
        VirtualKeyCode::Apostrophe => KeyCode::Apostrophe,
        VirtualKeyCode::Apps => KeyCode::Apps,
        VirtualKeyCode::At => KeyCode::At,
        VirtualKeyCode::Ax => KeyCode::Ax,
        VirtualKeyCode::Backslash => KeyCode::Backslash,
        VirtualKeyCode::Calculator => KeyCode::Calculator,
        VirtualKeyCode::Capital => KeyCode::Capital,
        VirtualKeyCode::Colon => KeyCode::Colon,
        VirtualKeyCode::Comma => KeyCode::Comma,
        VirtualKeyCode::Convert => KeyCode::Convert,
        VirtualKeyCode::NumpadDecimal => KeyCode::NumpadDecimal,
        VirtualKeyCode::NumpadDivide => KeyCode::NumpadDivide,
        VirtualKeyCode::Equals => KeyCode::Equals,
        VirtualKeyCode::Grave => KeyCode::Grave,
        VirtualKeyCode::Kana => KeyCode::Kana,
        VirtualKeyCode::Kanji => KeyCode::Kanji,
        VirtualKeyCode::LAlt => KeyCode::LAlt,
        VirtualKeyCode::LBracket => KeyCode::LBracket,
        VirtualKeyCode::LControl => KeyCode::LControl,
        VirtualKeyCode::LShift => KeyCode::LShift,
        VirtualKeyCode::LWin => KeyCode::LWin,
        VirtualKeyCode::Mail => KeyCode::Mail,
        VirtualKeyCode::MediaSelect => KeyCode::MediaSelect,
        VirtualKeyCode::MediaStop => KeyCode::MediaStop,
        VirtualKeyCode::Minus => KeyCode::Minus,
        VirtualKeyCode::NumpadMultiply => KeyCode::NumpadMultiply,
        VirtualKeyCode::Mute => KeyCode::Mute,
        VirtualKeyCode::MyComputer => KeyCode::MyComputer,
        VirtualKeyCode::NavigateForward => KeyCode::NavigateForward,
        VirtualKeyCode::NavigateBackward => KeyCode::NavigateBackward,
        VirtualKeyCode::NextTrack => KeyCode::NextTrack,
        VirtualKeyCode::NoConvert => KeyCode::NoConvert,
        VirtualKeyCode::NumpadComma => KeyCode::NumpadComma,
        VirtualKeyCode::NumpadEnter => KeyCode::NumpadEnter,
        VirtualKeyCode::NumpadEquals => KeyCode::NumpadEquals,
        VirtualKeyCode::OEM102 => KeyCode::OEM102,
        VirtualKeyCode::Period => KeyCode::Period,
        VirtualKeyCode::PlayPause => KeyCode::PlayPause,
        VirtualKeyCode::Power => KeyCode::Power,
        VirtualKeyCode::PrevTrack => KeyCode::PrevTrack,
        VirtualKeyCode::RAlt => KeyCode::RAlt,
        VirtualKeyCode::RBracket => KeyCode::RBracket,
        VirtualKeyCode::RControl => KeyCode::RControl,
        VirtualKeyCode::RShift => KeyCode::RShift,
        VirtualKeyCode::RWin => KeyCode::RWin,
        VirtualKeyCode::Semicolon => KeyCode::Semicolon,
        VirtualKeyCode::Slash => KeyCode::Slash,
        VirtualKeyCode::Sleep => KeyCode::Sleep,
        VirtualKeyCode::Stop => KeyCode::Stop,
        VirtualKeyCode::NumpadSubtract => KeyCode::NumpadSubtract,
        VirtualKeyCode::Sysrq => KeyCode::Sysrq,
        VirtualKeyCode::Tab => KeyCode::Tab,
        VirtualKeyCode::Underline => KeyCode::Underline,
        VirtualKeyCode::Unlabeled => KeyCode::Unlabeled,
        VirtualKeyCode::VolumeDown => KeyCode::VolumeDown,
        VirtualKeyCode::VolumeUp => KeyCode::VolumeUp,
        VirtualKeyCode::Wake => KeyCode::Wake,
        VirtualKeyCode::WebBack => KeyCode::WebBack,
        VirtualKeyCode::WebFavorites => KeyCode::WebFavorites,
        VirtualKeyCode::WebForward => KeyCode::WebForward,
        VirtualKeyCode::WebHome => KeyCode::WebHome,
        VirtualKeyCode::WebRefresh => KeyCode::WebRefresh,
        VirtualKeyCode::WebSearch => KeyCode::WebSearch,
        VirtualKeyCode::WebStop => KeyCode::WebStop,
        VirtualKeyCode::Yen => KeyCode::Yen,
        VirtualKeyCode::Copy => KeyCode::Copy,
        VirtualKeyCode::Paste => KeyCode::Paste,
        VirtualKeyCode::Cut => KeyCode::Cut,
        VirtualKeyCode::Asterisk => KeyCode::Asterisk,
        VirtualKeyCode::Plus => KeyCode::Plus,
    }
}

/// Translates cursor icon from fyrox-ui library to glutin format.
pub fn translate_cursor_icon(icon: crate::gui::message::CursorIcon) -> crate::window::CursorIcon {
    match icon {
        crate::gui::message::CursorIcon::Default => crate::window::CursorIcon::Default,
        crate::gui::message::CursorIcon::Crosshair => crate::window::CursorIcon::Crosshair,
        crate::gui::message::CursorIcon::Hand => crate::window::CursorIcon::Hand,
        crate::gui::message::CursorIcon::Arrow => crate::window::CursorIcon::Arrow,
        crate::gui::message::CursorIcon::Move => crate::window::CursorIcon::Move,
        crate::gui::message::CursorIcon::Text => crate::window::CursorIcon::Text,
        crate::gui::message::CursorIcon::Wait => crate::window::CursorIcon::Wait,
        crate::gui::message::CursorIcon::Help => crate::window::CursorIcon::Help,
        crate::gui::message::CursorIcon::Progress => crate::window::CursorIcon::Progress,
        crate::gui::message::CursorIcon::NotAllowed => crate::window::CursorIcon::NotAllowed,
        crate::gui::message::CursorIcon::ContextMenu => crate::window::CursorIcon::ContextMenu,
        crate::gui::message::CursorIcon::Cell => crate::window::CursorIcon::Cell,
        crate::gui::message::CursorIcon::VerticalText => crate::window::CursorIcon::VerticalText,
        crate::gui::message::CursorIcon::Alias => crate::window::CursorIcon::Alias,
        crate::gui::message::CursorIcon::Copy => crate::window::CursorIcon::Copy,
        crate::gui::message::CursorIcon::NoDrop => crate::window::CursorIcon::NoDrop,
        crate::gui::message::CursorIcon::Grab => crate::window::CursorIcon::Grab,
        crate::gui::message::CursorIcon::Grabbing => crate::window::CursorIcon::Grabbing,
        crate::gui::message::CursorIcon::AllScroll => crate::window::CursorIcon::AllScroll,
        crate::gui::message::CursorIcon::ZoomIn => crate::window::CursorIcon::ZoomIn,
        crate::gui::message::CursorIcon::ZoomOut => crate::window::CursorIcon::ZoomOut,
        crate::gui::message::CursorIcon::EResize => crate::window::CursorIcon::EResize,
        crate::gui::message::CursorIcon::NResize => crate::window::CursorIcon::NResize,
        crate::gui::message::CursorIcon::NeResize => crate::window::CursorIcon::NeResize,
        crate::gui::message::CursorIcon::NwResize => crate::window::CursorIcon::NwResize,
        crate::gui::message::CursorIcon::SResize => crate::window::CursorIcon::SResize,
        crate::gui::message::CursorIcon::SeResize => crate::window::CursorIcon::SeResize,
        crate::gui::message::CursorIcon::SwResize => crate::window::CursorIcon::SwResize,
        crate::gui::message::CursorIcon::WResize => crate::window::CursorIcon::WResize,
        crate::gui::message::CursorIcon::EwResize => crate::window::CursorIcon::EwResize,
        crate::gui::message::CursorIcon::NsResize => crate::window::CursorIcon::NsResize,
        crate::gui::message::CursorIcon::NeswResize => crate::window::CursorIcon::NeswResize,
        crate::gui::message::CursorIcon::NwseResize => crate::window::CursorIcon::NwseResize,
        crate::gui::message::CursorIcon::ColResize => crate::window::CursorIcon::ColResize,
        crate::gui::message::CursorIcon::RowResize => crate::window::CursorIcon::RowResize,
    }
}

/// Translates window mouse button into fyrox-ui mouse button.
pub fn translate_button(button: crate::event::MouseButton) -> crate::gui::message::MouseButton {
    match button {
        crate::event::MouseButton::Left => crate::gui::message::MouseButton::Left,
        crate::event::MouseButton::Right => crate::gui::message::MouseButton::Right,
        crate::event::MouseButton::Middle => crate::gui::message::MouseButton::Middle,
        crate::event::MouseButton::Other(i) => crate::gui::message::MouseButton::Other(i),
    }
}

/// Translates library button state into fyrox-ui button state.
pub fn translate_state(state: ElementState) -> ButtonState {
    match state {
        ElementState::Pressed => ButtonState::Pressed,
        ElementState::Released => ButtonState::Released,
    }
}

/// Translates window event to fyrox-ui event.
pub fn translate_event(event: &WindowEvent) -> Option<OsEvent> {
    match event {
        WindowEvent::ReceivedCharacter(c) => Some(OsEvent::Character(*c)),
        WindowEvent::KeyboardInput { input, .. } => {
            input.virtual_keycode.map(|key| OsEvent::KeyboardInput {
                button: translate_key(key),
                state: translate_state(input.state),
            })
        }
        WindowEvent::CursorMoved { position, .. } => Some(OsEvent::CursorMoved {
            position: Vector2::new(position.x as f32, position.y as f32),
        }),
        WindowEvent::MouseWheel { delta, .. } => match delta {
            MouseScrollDelta::LineDelta(x, y) => Some(OsEvent::MouseWheel(*x, *y)),
            MouseScrollDelta::PixelDelta(pos) => {
                Some(OsEvent::MouseWheel(pos.x as f32, pos.y as f32))
            }
        },
        WindowEvent::MouseInput { state, button, .. } => Some(OsEvent::MouseInput {
            button: translate_button(*button),
            state: translate_state(*state),
        }),
        &WindowEvent::ModifiersChanged(modifiers) => Some(OsEvent::KeyboardModifiers(
            translate_keyboard_modifiers(modifiers),
        )),
        _ => None,
    }
}

/// Translates keyboard modifiers to fyrox-ui keyboard modifiers.
pub fn translate_keyboard_modifiers(modifiers: ModifiersState) -> KeyboardModifiers {
    KeyboardModifiers {
        alt: modifiers.alt(),
        shift: modifiers.shift(),
        control: modifiers.ctrl(),
        system: modifiers.logo(),
    }
}

/// Maps key code to its name. Can be useful if you making adjustable key bindings in your
/// game and you need quickly map key code to its name.
pub fn virtual_key_code_name(code: VirtualKeyCode) -> &'static str {
    match code {
        VirtualKeyCode::Key1 => "1",
        VirtualKeyCode::Key2 => "2",
        VirtualKeyCode::Key3 => "3",
        VirtualKeyCode::Key4 => "4",
        VirtualKeyCode::Key5 => "5",
        VirtualKeyCode::Key6 => "6",
        VirtualKeyCode::Key7 => "7",
        VirtualKeyCode::Key8 => "8",
        VirtualKeyCode::Key9 => "9",
        VirtualKeyCode::Key0 => "0",
        VirtualKeyCode::A => "A",
        VirtualKeyCode::B => "B",
        VirtualKeyCode::C => "C",
        VirtualKeyCode::D => "D",
        VirtualKeyCode::E => "E",
        VirtualKeyCode::F => "F",
        VirtualKeyCode::G => "G",
        VirtualKeyCode::H => "H",
        VirtualKeyCode::I => "I",
        VirtualKeyCode::J => "J",
        VirtualKeyCode::K => "K",
        VirtualKeyCode::L => "L",
        VirtualKeyCode::M => "M",
        VirtualKeyCode::N => "N",
        VirtualKeyCode::O => "O",
        VirtualKeyCode::P => "P",
        VirtualKeyCode::Q => "Q",
        VirtualKeyCode::R => "R",
        VirtualKeyCode::S => "S",
        VirtualKeyCode::T => "T",
        VirtualKeyCode::U => "U",
        VirtualKeyCode::V => "V",
        VirtualKeyCode::W => "W",
        VirtualKeyCode::X => "X",
        VirtualKeyCode::Y => "Y",
        VirtualKeyCode::Z => "Z",
        VirtualKeyCode::Escape => "Escape",
        VirtualKeyCode::F1 => "F1",
        VirtualKeyCode::F2 => "F2",
        VirtualKeyCode::F3 => "F3",
        VirtualKeyCode::F4 => "F4",
        VirtualKeyCode::F5 => "F5",
        VirtualKeyCode::F6 => "F6",
        VirtualKeyCode::F7 => "F7",
        VirtualKeyCode::F8 => "F8",
        VirtualKeyCode::F9 => "F9",
        VirtualKeyCode::F10 => "F10",
        VirtualKeyCode::F11 => "F11",
        VirtualKeyCode::F12 => "F12",
        VirtualKeyCode::F13 => "F13",
        VirtualKeyCode::F14 => "F14",
        VirtualKeyCode::F15 => "F15",
        VirtualKeyCode::F16 => "F16",
        VirtualKeyCode::F17 => "F17",
        VirtualKeyCode::F18 => "F18",
        VirtualKeyCode::F19 => "F19",
        VirtualKeyCode::F20 => "F20",
        VirtualKeyCode::F21 => "F21",
        VirtualKeyCode::F22 => "F22",
        VirtualKeyCode::F23 => "F23",
        VirtualKeyCode::F24 => "F24",
        VirtualKeyCode::Snapshot => "Snapshot",
        VirtualKeyCode::Scroll => "Scroll",
        VirtualKeyCode::Pause => "Pause",
        VirtualKeyCode::Insert => "Insert",
        VirtualKeyCode::Home => "Home",
        VirtualKeyCode::Delete => "Delete",
        VirtualKeyCode::End => "End",
        VirtualKeyCode::PageDown => "PageDown",
        VirtualKeyCode::PageUp => "PageUp",
        VirtualKeyCode::Left => "Left",
        VirtualKeyCode::Up => "Up",
        VirtualKeyCode::Right => "Right",
        VirtualKeyCode::Down => "Down",
        VirtualKeyCode::Back => "Back",
        VirtualKeyCode::Return => "Return",
        VirtualKeyCode::Space => "Space",
        VirtualKeyCode::Compose => "Compose",
        VirtualKeyCode::Caret => "Caret",
        VirtualKeyCode::Numlock => "Numlock",
        VirtualKeyCode::Numpad0 => "Numpad0",
        VirtualKeyCode::Numpad1 => "Numpad1",
        VirtualKeyCode::Numpad2 => "Numpad2",
        VirtualKeyCode::Numpad3 => "Numpad3",
        VirtualKeyCode::Numpad4 => "Numpad4",
        VirtualKeyCode::Numpad5 => "Numpad5",
        VirtualKeyCode::Numpad6 => "Numpad6",
        VirtualKeyCode::Numpad7 => "Numpad7",
        VirtualKeyCode::Numpad8 => "Numpad8",
        VirtualKeyCode::Numpad9 => "Numpad9",
        VirtualKeyCode::AbntC1 => "AbntC1",
        VirtualKeyCode::AbntC2 => "AbntC2",
        VirtualKeyCode::NumpadAdd => "NumpadAdd",
        VirtualKeyCode::Apostrophe => "Apostrophe",
        VirtualKeyCode::Apps => "Apps",
        VirtualKeyCode::At => "At",
        VirtualKeyCode::Ax => "Ax",
        VirtualKeyCode::Backslash => "Backslash",
        VirtualKeyCode::Calculator => "Calculator",
        VirtualKeyCode::Capital => "Capital",
        VirtualKeyCode::Colon => "Colon",
        VirtualKeyCode::Comma => "Comma",
        VirtualKeyCode::Convert => "Convert",
        VirtualKeyCode::NumpadDecimal => "NumpadDecimal",
        VirtualKeyCode::NumpadDivide => "NumpadDivide",
        VirtualKeyCode::Equals => "Equals",
        VirtualKeyCode::Grave => "Grave",
        VirtualKeyCode::Kana => "Kana",
        VirtualKeyCode::Kanji => "Kanji",
        VirtualKeyCode::LAlt => "LAlt",
        VirtualKeyCode::LBracket => "LBracket",
        VirtualKeyCode::LControl => "LControl",
        VirtualKeyCode::LShift => "LShift",
        VirtualKeyCode::LWin => "LWin",
        VirtualKeyCode::Mail => "Mail",
        VirtualKeyCode::MediaSelect => "MediaSelect",
        VirtualKeyCode::MediaStop => "MediaStop",
        VirtualKeyCode::Minus => "Minus",
        VirtualKeyCode::NumpadMultiply => "NumpadMultiply",
        VirtualKeyCode::Mute => "Mute",
        VirtualKeyCode::MyComputer => "MyComputer",
        VirtualKeyCode::NavigateForward => "NavigateForward",
        VirtualKeyCode::NavigateBackward => "NavigateBackward",
        VirtualKeyCode::NextTrack => "NextTrack",
        VirtualKeyCode::NoConvert => "NoConvert",
        VirtualKeyCode::NumpadComma => "NumpadComma",
        VirtualKeyCode::NumpadEnter => "NumpadEnter",
        VirtualKeyCode::NumpadEquals => "NumpadEquals",
        VirtualKeyCode::OEM102 => "OEM102",
        VirtualKeyCode::Period => "Period",
        VirtualKeyCode::PlayPause => "PlayPause",
        VirtualKeyCode::Power => "Power",
        VirtualKeyCode::PrevTrack => "PrevTrack",
        VirtualKeyCode::RAlt => "RAlt",
        VirtualKeyCode::RBracket => "RBracket",
        VirtualKeyCode::RControl => "RControl",
        VirtualKeyCode::RShift => "RShift",
        VirtualKeyCode::RWin => "RWin",
        VirtualKeyCode::Semicolon => "Semicolon",
        VirtualKeyCode::Slash => "Slash",
        VirtualKeyCode::Sleep => "Sleep",
        VirtualKeyCode::Stop => "Stop",
        VirtualKeyCode::NumpadSubtract => "NumpadSubtract",
        VirtualKeyCode::Sysrq => "Sysrq",
        VirtualKeyCode::Tab => "Tab",
        VirtualKeyCode::Underline => "Underline",
        VirtualKeyCode::Unlabeled => "Unlabeled",
        VirtualKeyCode::VolumeDown => "VolumeDown",
        VirtualKeyCode::VolumeUp => "VolumeUp",
        VirtualKeyCode::Wake => "Wake",
        VirtualKeyCode::WebBack => "WebBack",
        VirtualKeyCode::WebFavorites => "WebFavorites",
        VirtualKeyCode::WebForward => "WebForward",
        VirtualKeyCode::WebHome => "WebHome",
        VirtualKeyCode::WebRefresh => "WebRefresh",
        VirtualKeyCode::WebSearch => "WebSearch",
        VirtualKeyCode::WebStop => "WebStop",
        VirtualKeyCode::Yen => "Yen",
        VirtualKeyCode::Copy => "Copy",
        VirtualKeyCode::Paste => "Paste",
        VirtualKeyCode::Cut => "Cut",
        VirtualKeyCode::Asterisk => "Asterisk",
        VirtualKeyCode::Plus => "Plus",
    }
}

/// Helper function to convert Option<Arc<T>> to Option<Arc<dyn Any>>.
#[allow(clippy::manual_map)]
pub fn into_any_arc<T: Any + Send + Sync>(
    opt: Option<Arc<T>>,
) -> Option<Arc<dyn Any + Send + Sync>> {
    match opt {
        Some(r) => Some(r),
        None => None,
    }
}

/// Converts engine's optional texture "pointer" to fyrox-ui's.
pub fn into_gui_texture(this: Texture) -> draw::SharedTexture {
    draw::SharedTexture(this.0.into_inner())
}

/// "Transmutes" array of any sized type to a slice of bytes.
pub fn array_as_u8_slice<T: Sized>(v: &[T]) -> &'_ [u8] {
    // SAFETY: It is safe to reinterpret data to read it.
    unsafe {
        std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of::<T>() * v.len())
    }
}

/// "Transmutes" value of any sized type to a slice of bytes.
pub fn value_as_u8_slice<T: Sized>(v: &T) -> &'_ [u8] {
    // SAFETY: It is safe to reinterpret data to read it.
    unsafe { std::slice::from_raw_parts(v as *const T as *const u8, std::mem::size_of::<T>()) }
}

/// Performs hashing of a sized value by interpreting it as raw memory.
pub fn hash_as_bytes<T: Sized, H: Hasher>(value: &T, hasher: &mut H) {
    hasher.write(value_as_u8_slice(value))
}
