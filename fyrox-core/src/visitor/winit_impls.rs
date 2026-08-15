// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Visit, Reflect impls for `winit` types. It has to be located in core because of "awesome"
//! orphan rule of Rust. This module is excluded from build by default, use `winit_impl` feature
//! to turn it on.

#![cfg(feature = "winit_impls")]

use crate::reflect::prelude::*;
use crate::visitor::prelude::*;
use fyrox_core_derive::{impl_reflect, impl_visit};
use winit::keyboard::KeyCode;

#[rustfmt::skip]
impl_visit!(
    pub enum KeyCode {
        Backquote, Backslash, BracketLeft, BracketRight, Comma, Digit0, Digit1, Digit2, Digit3,
        Digit4, Digit5, Digit6, Digit7, Digit8, Digit9, Equal, IntlBackslash, IntlRo, IntlYen,
        KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO,
        KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ, Minus, Period, Quote,
        Semicolon, Slash, AltLeft, AltRight, Backspace, CapsLock, ContextMenu, ControlLeft,
        ControlRight, Enter, SuperLeft, SuperRight, ShiftLeft, ShiftRight, Space, Tab, Convert,
        KanaMode, Lang1, Lang2, Lang3, Lang4, Lang5, NonConvert, Delete, End, Help, Home, Insert,
        PageDown, PageUp, ArrowDown, ArrowLeft, ArrowRight, ArrowUp, NumLock, Numpad0, Numpad1,
        Numpad2, Numpad3, Numpad4, Numpad5, Numpad6, Numpad7, Numpad8, Numpad9, NumpadAdd,
        NumpadBackspace, NumpadClear, NumpadClearEntry, NumpadComma, NumpadDecimal, NumpadDivide,
        NumpadEnter, NumpadEqual, NumpadHash, NumpadMemoryAdd, NumpadMemoryClear, NumpadMemoryRecall,
        NumpadMemoryStore, NumpadMemorySubtract, NumpadMultiply, NumpadParenLeft, NumpadParenRight,
        NumpadStar, NumpadSubtract, Escape, Fn, FnLock, PrintScreen, ScrollLock, Pause, BrowserBack,
        BrowserFavorites, BrowserForward, BrowserHome, BrowserRefresh, BrowserSearch, BrowserStop,
        Eject, LaunchApp1, LaunchApp2, LaunchMail, MediaPlayPause, MediaSelect, MediaStop,
        MediaTrackNext, MediaTrackPrevious, Power, Sleep, AudioVolumeDown, AudioVolumeMute,
        AudioVolumeUp, WakeUp, Meta, Hyper, Turbo, Abort, Resume, Suspend, Again, Copy, Cut, Find,
        Open, Paste, Props, Select, Undo, Hiragana, Katakana, F1, F2, F3, F4, F5, F6, F7, F8, F9,
        F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24, F25, F26, F27,
        F28, F29, F30, F31, F32, F33, F34, F35,
    }
);

#[rustfmt::skip]
impl_reflect!(
    #[reflect(type_uuid = "efb534c3-e122-4023-a737-a9ed60d80ff2")]
    pub enum KeyCode {
        Backquote, Backslash, BracketLeft, BracketRight, Comma, Digit0, Digit1, Digit2, Digit3,
        Digit4, Digit5, Digit6, Digit7, Digit8, Digit9, Equal, IntlBackslash, IntlRo, IntlYen,
        KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO,
        KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ, Minus, Period, Quote,
        Semicolon, Slash, AltLeft, AltRight, Backspace, CapsLock, ContextMenu, ControlLeft,
        ControlRight, Enter, SuperLeft, SuperRight, ShiftLeft, ShiftRight, Space, Tab, Convert,
        KanaMode, Lang1, Lang2, Lang3, Lang4, Lang5, NonConvert, Delete, End, Help, Home, Insert,
        PageDown, PageUp, ArrowDown, ArrowLeft, ArrowRight, ArrowUp, NumLock, Numpad0, Numpad1,
        Numpad2, Numpad3, Numpad4, Numpad5, Numpad6, Numpad7, Numpad8, Numpad9, NumpadAdd,
        NumpadBackspace, NumpadClear, NumpadClearEntry, NumpadComma, NumpadDecimal, NumpadDivide,
        NumpadEnter, NumpadEqual, NumpadHash, NumpadMemoryAdd, NumpadMemoryClear, NumpadMemoryRecall,
        NumpadMemoryStore, NumpadMemorySubtract, NumpadMultiply, NumpadParenLeft, NumpadParenRight,
        NumpadStar, NumpadSubtract, Escape, Fn, FnLock, PrintScreen, ScrollLock, Pause, BrowserBack,
        BrowserFavorites, BrowserForward, BrowserHome, BrowserRefresh, BrowserSearch, BrowserStop,
        Eject, LaunchApp1, LaunchApp2, LaunchMail, MediaPlayPause, MediaSelect, MediaStop,
        MediaTrackNext, MediaTrackPrevious, Power, Sleep, AudioVolumeDown, AudioVolumeMute,
        AudioVolumeUp, WakeUp, Meta, Hyper, Turbo, Abort, Resume, Suspend, Again, Copy, Cut, Find,
        Open, Paste, Props, Select, Undo, Hiragana, Katakana, F1, F2, F3, F4, F5, F6, F7, F8, F9,
        F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24, F25, F26, F27,
        F28, F29, F30, F31, F32, F33, F34, F35,
    }
);
