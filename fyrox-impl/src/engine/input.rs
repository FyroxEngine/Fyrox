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

//! This module contains a set of "shortcuts" that allows getting the state of a mouse and a keyboard
//! in a simplified manner (read - without using "verbose" event-based approach). It may be useful
//! in simple scenarios where you just need to know if a button (on keyboard, mouse) was pressed
//! and do something. You should always prefer the event-based approach when possible.

use fxhash::FxHashMap;
use fyrox_core::algebra::Vector2;
use winit::event::{ButtonId, ElementState};
use winit::keyboard::{KeyCode, PhysicalKey};

/// Represents the mouse state in the current frame. The contents of this structure is a simplified
/// version of event-based approach. **Important:** this structure does not track from which mouse the
/// corresponding event has come from, if you have more than one mouse use event-based approach
/// instead!
#[derive(Default, Clone)]
pub struct MouseState {
    /// Coordinates in pixels relative to the top-left corner of the window.
    pub position: Vector2<f32>,
    /// Speed of the mouse in some units.
    pub speed: Vector2<f32>,
    /// Physical state of mouse buttons. Usually, the button indices are the following:
    ///
    /// - 0 - left mouse button
    /// - 1 - right mouse button
    /// - 2 - middle mouse button
    /// - 3 - additional mouse button (could back or forward)
    /// - 4 - additional mouse button (could back or forward)
    /// - 5 and higher - device-specific buttons
    pub buttons_state: FxHashMap<ButtonId, ElementState>,
}

/// Represents the keyboard state in the current frame. The contents of this structure is a simplified
/// version of event-based approach. **Important:** this structure does not track from which keyboard the
/// corresponding event has come from, if you have more than one keyboard use event-based approach
/// instead!
#[derive(Default, Clone)]
pub struct KeyboardState {
    /// Represents the keyboard state in the current frame.
    pub keys: FxHashMap<PhysicalKey, ElementState>,
}

/// A stored state of most common input events. It is used a "shortcut" in cases where event-based
/// approach is too verbose. **Important:** this structure does not track from which device the
/// corresponding event has come from, if you have more than one keyboard and/or mouse, use
/// event-based approach instead! You should always prefer the event-based approach when possible.
#[derive(Default, Clone)]
pub struct InputState {
    /// Represents the mouse state in the current frame.
    pub mouse: MouseState,
    /// Represents the keyboard state in the current frame.
    pub keyboard: KeyboardState,
}

impl InputState {
    /// Returns `true` if the specified key was pressed in the current frame, `false` - otherwise.
    #[inline]
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keyboard
            .keys
            .get(&PhysicalKey::Code(key))
            .is_some_and(|state| *state == ElementState::Pressed)
    }

    /// Returns `true` if the specified mouse button pressed in the current frame, `false` -
    /// otherwise. Usually, the button indices are the following:
    ///
    /// - 0 - left mouse button
    /// - 1 - right mouse button
    /// - 2 - middle mouse button
    /// - 3 - additional mouse button (could back or forward)
    /// - 4 - additional mouse button (could back or forward)
    /// - 5 and higher - device-specific buttons
    #[inline]
    pub fn is_mouse_button_pressed(&self, button_id: ButtonId) -> bool {
        self.mouse
            .buttons_state
            .get(&button_id)
            .is_some_and(|state| *state == ElementState::Pressed)
    }

    /// Returns `true` if the left mouse button pressed in the current frame, `false` - otherwise.
    #[inline]
    pub fn is_left_mouse_button_pressed(&self) -> bool {
        self.is_mouse_button_pressed(0)
    }

    /// Returns `true` if the right mouse button pressed in the current frame, `false` - otherwise.
    #[inline]
    pub fn is_right_mouse_button_pressed(&self) -> bool {
        self.is_mouse_button_pressed(1)
    }

    /// Returns `true` if the middle mouse button pressed in the current frame, `false` - otherwise.
    #[inline]
    pub fn is_middle_mouse_button_pressed(&self) -> bool {
        self.is_mouse_button_pressed(2)
    }

    /// Returns mouse speed in the current frame, the speed expressed in some arbitrary units.
    #[inline]
    pub fn mouse_speed(&self) -> Vector2<f32> {
        self.mouse.speed
    }

    /// Returns mouse position in pixels relative to the top-left corner of the main window.
    #[inline]
    pub fn mouse_position(&self) -> Vector2<f32> {
        self.mouse.position
    }
}
