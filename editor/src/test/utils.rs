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

use crate::{
    fyrox::{
        core::{algebra::Vector2, info, Uuid},
        engine::ApplicationLoopController,
        graph::SceneGraph,
        gui::message::{ButtonState, MouseButton, OsEvent},
    },
    plugin::EditorPlugin,
    test::macros::Macro,
    Editor, StartupData,
};
use std::path::PathBuf;

/// Initializes the editor as is a user would open it, adds the specified plugin that runs the test
/// logic.
pub fn run_editor_test(name: &str, plugin: impl EditorPlugin) {
    let path = PathBuf::from(format!("./AutomatedTests/{name}TestData"));
    if !path.exists() {
        std::fs::create_dir_all(&path).unwrap();
    }
    let path = path.canonicalize().unwrap();
    info!("Running {name}. Working dir: {}", path.display());

    let mut editor = Editor::new(Some(StartupData {
        working_directory: path.clone(),
        scenes: vec![],
        named_objects: false,
    }));

    editor.add_editor_plugin(plugin);

    editor.run_headless()
}

pub trait EditorTestingExtension {
    /// Clicks at the given position.
    fn click(&mut self, position: Vector2<f32>);

    /// Tries to find a widget with the given unique id and clicks at its center.
    fn click_at(&mut self, name: Uuid);
}

impl EditorTestingExtension for Editor {
    fn click(&mut self, position: Vector2<f32>) {
        let ui = self.engine.user_interfaces.first_mut();
        ui.process_os_event(&OsEvent::CursorMoved { position });
        ui.process_os_event(&OsEvent::MouseInput {
            button: MouseButton::Left,
            state: ButtonState::Pressed,
        });
        ui.process_os_event(&OsEvent::MouseInput {
            button: MouseButton::Left,
            state: ButtonState::Released,
        });
    }

    fn click_at(&mut self, uuid: Uuid) {
        let ui = self.engine.user_interfaces.first();
        if let Some((handle, n)) = ui.find_from_root(&mut |n| n.id == uuid) {
            assert!(n.is_globally_visible());
            let center = n.local_to_screen(n.center());
            self.click(center);
            info!(
                "Clicked at {uuid}({}:{}) at [{};{}] coords.",
                handle.index(),
                handle.generation(),
                center.x,
                center.y
            );
        } else {
            panic!("There's no widget {uuid}!")
        }
    }
}

/// Checks the main menu strip and its sub-menu items. Work-in-progress.
pub struct TestPlugin {
    test_macro: Macro,
}

impl TestPlugin {
    /// Creates a new editor plugin for tests.
    pub fn new(test_macro: Macro) -> Self {
        Self { test_macro }
    }
}

impl EditorPlugin for TestPlugin {
    /// Creates a new editor plugin for tests.
    fn on_update(&mut self, editor: &mut Editor, loop_controller: ApplicationLoopController) {
        self.test_macro.execute_next(editor, loop_controller)
    }
}
