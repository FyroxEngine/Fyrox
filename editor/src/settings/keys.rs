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

use crate::fyrox::{
    core::reflect::prelude::*,
    core::visitor::prelude::*,
    gui::{
        key::{HotKey, KeyBinding},
        message::KeyCode,
    },
};
use fyrox::gui::message::KeyboardModifiers;

#[derive(Visit, PartialEq, Clone, Debug, Reflect)]
#[reflect(type_uuid = "fbcdc91c-ba63-4d24-b604-c8ffd58834bf")]
pub struct TerrainKeyBindings {
    pub modify_height_map_mode: HotKey,
    pub draw_on_mask_mode: HotKey,
    pub flatten_slopes_mode: HotKey,
    pub increase_brush_size: HotKey,
    pub decrease_brush_size: HotKey,
    pub increase_brush_opacity: HotKey,
    pub decrease_brush_opacity: HotKey,
    pub prev_layer: HotKey,
    pub next_layer: HotKey,
}

#[derive(Visit, PartialEq, Clone, Debug, Reflect)]
#[reflect(type_uuid = "82d67946-8b7b-41f0-9520-5d4a927aa68f")]
pub struct KeyBindings {
    pub move_forward: KeyBinding,
    pub move_back: KeyBinding,
    pub move_left: KeyBinding,
    pub move_right: KeyBinding,
    pub move_up: KeyBinding,
    pub move_down: KeyBinding,
    pub speed_up: KeyBinding,
    pub slow_down: KeyBinding,
    pub undo: HotKey,
    pub redo: HotKey,
    pub enable_select_mode: HotKey,
    pub enable_move_mode: HotKey,
    pub enable_rotate_mode: HotKey,
    pub enable_scale_mode: HotKey,
    pub enable_terrain_mode: HotKey,
    pub save_scene: HotKey,
    pub save_scene_as: HotKey,
    pub save_all_scenes: HotKey,
    pub load_scene: HotKey,
    pub copy_selection: HotKey,
    pub paste: HotKey,
    pub new_scene: HotKey,
    pub close_scene: HotKey,
    pub remove_selection: HotKey,
    pub focus: HotKey,
    pub terrain_key_bindings: TerrainKeyBindings,
    pub run_game: HotKey,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            move_forward: KeyBinding::from_key_code(KeyCode::KeyW),
            move_back: KeyBinding::from_key_code(KeyCode::KeyS),
            move_left: KeyBinding::from_key_code(KeyCode::KeyA),
            move_right: KeyBinding::from_key_code(KeyCode::KeyD),
            move_up: KeyBinding::from_key_code(KeyCode::KeyE),
            move_down: KeyBinding::from_key_code(KeyCode::KeyQ),
            speed_up: KeyBinding::from_key_code(KeyCode::ControlLeft),
            slow_down: KeyBinding::from_key_code(KeyCode::ShiftLeft),

            undo: HotKey::ctrl_key(KeyCode::KeyZ),
            redo: HotKey::ctrl_key(KeyCode::KeyY),
            enable_select_mode: HotKey::from_key_code(KeyCode::Digit1),
            enable_move_mode: HotKey::from_key_code(KeyCode::Digit2),
            enable_rotate_mode: HotKey::from_key_code(KeyCode::Digit3),
            enable_scale_mode: HotKey::from_key_code(KeyCode::Digit4),
            enable_terrain_mode: HotKey::from_key_code(KeyCode::Digit5),
            save_scene: HotKey::ctrl_key(KeyCode::KeyS),
            save_scene_as: HotKey::Some {
                code: KeyCode::KeyS,
                modifiers: KeyboardModifiers {
                    shift: true,
                    control: true,
                    ..Default::default()
                },
            },
            save_all_scenes: HotKey::Some {
                code: KeyCode::KeyS,
                modifiers: KeyboardModifiers {
                    alt: true,
                    control: true,
                    ..Default::default()
                },
            },
            load_scene: HotKey::ctrl_key(KeyCode::KeyL),
            copy_selection: HotKey::ctrl_key(KeyCode::KeyC),
            paste: HotKey::ctrl_key(KeyCode::KeyV),
            new_scene: HotKey::ctrl_key(KeyCode::KeyN),
            close_scene: HotKey::ctrl_key(KeyCode::KeyQ),
            remove_selection: HotKey::from_key_code(KeyCode::Delete),
            focus: HotKey::from_key_code(KeyCode::KeyF),
            terrain_key_bindings: TerrainKeyBindings {
                modify_height_map_mode: HotKey::from_key_code(KeyCode::F1),
                draw_on_mask_mode: HotKey::from_key_code(KeyCode::F2),
                flatten_slopes_mode: HotKey::from_key_code(KeyCode::F3),
                increase_brush_size: HotKey::from_key_code(KeyCode::BracketRight),
                decrease_brush_size: HotKey::from_key_code(KeyCode::BracketLeft),
                increase_brush_opacity: HotKey::from_key_code(KeyCode::Period),
                decrease_brush_opacity: HotKey::from_key_code(KeyCode::Comma),
                prev_layer: HotKey::from_key_code(KeyCode::Semicolon),
                next_layer: HotKey::from_key_code(KeyCode::Quote),
            },
            run_game: HotKey::from_key_code(KeyCode::F5),
        }
    }
}
