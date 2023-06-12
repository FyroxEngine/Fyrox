use fyrox::{
    core::reflect::prelude::*,
    gui::{
        key::{HotKey, KeyBinding},
        message::KeyCode,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Reflect)]
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
    pub enable_navmesh_mode: HotKey,
    pub enable_terrain_mode: HotKey,
    pub save_scene: HotKey,
    pub load_scene: HotKey,
    pub copy_selection: HotKey,
    pub paste: HotKey,
    pub new_scene: HotKey,
    pub close_scene: HotKey,
    pub remove_selection: HotKey,
    #[serde(default = "default_focus_hotkey")]
    pub focus: HotKey,
}

fn default_focus_hotkey() -> HotKey {
    HotKey::from_key_code(KeyCode::F)
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            move_forward: KeyBinding::from_key_code(KeyCode::W),
            move_back: KeyBinding::from_key_code(KeyCode::S),
            move_left: KeyBinding::from_key_code(KeyCode::A),
            move_right: KeyBinding::from_key_code(KeyCode::D),
            move_up: KeyBinding::from_key_code(KeyCode::Q),
            move_down: KeyBinding::from_key_code(KeyCode::E),
            speed_up: KeyBinding::from_key_code(KeyCode::LControl),
            slow_down: KeyBinding::from_key_code(KeyCode::LShift),

            undo: HotKey::ctrl_key(KeyCode::Z),
            redo: HotKey::ctrl_key(KeyCode::Y),
            enable_select_mode: HotKey::from_key_code(KeyCode::Key1),
            enable_move_mode: HotKey::from_key_code(KeyCode::Key2),
            enable_rotate_mode: HotKey::from_key_code(KeyCode::Key3),
            enable_scale_mode: HotKey::from_key_code(KeyCode::Key4),
            enable_navmesh_mode: HotKey::from_key_code(KeyCode::Key5),
            enable_terrain_mode: HotKey::from_key_code(KeyCode::Key6),
            save_scene: HotKey::ctrl_key(KeyCode::S),
            load_scene: HotKey::ctrl_key(KeyCode::L),
            copy_selection: HotKey::ctrl_key(KeyCode::C),
            paste: HotKey::ctrl_key(KeyCode::V),
            new_scene: HotKey::ctrl_key(KeyCode::N),
            close_scene: HotKey::ctrl_key(KeyCode::Q),
            remove_selection: HotKey::from_key_code(KeyCode::Delete),
            focus: default_focus_hotkey(),
        }
    }
}
