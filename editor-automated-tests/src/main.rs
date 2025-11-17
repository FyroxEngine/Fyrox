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

#![allow(deprecated)]

//! Automated tests for the entire editor.
//! WARNING: This is experimental functionality and currently in development.

use fyrox::core::{info, Uuid};
use fyrox::{
    core::{algebra::Vector2, log::Log},
    event_loop::{ControlFlow, EventLoopBuilder},
    graph::SceneGraph,
    gui::message::{ButtonState, MouseButton, OsEvent},
};
use fyroxed_base::menu::file::FileMenu;
use fyroxed_base::{plugin::EditorPlugin, Editor, StartupData};
use std::{panic, path::PathBuf};

/// Initializes the editor as is a user would open it, adds the specified plugin that runs the test
/// logic.
fn run_editor_test(name: &str, plugin: impl EditorPlugin) {
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
        automated_testing: true,
    }));

    let event_loop = EventLoopBuilder::new().build().unwrap();

    // Keep updating the editor at full speed.
    event_loop.set_control_flow(ControlFlow::Poll);

    editor.add_editor_plugin(plugin);
    editor.run(event_loop)
}

trait EditorExt {
    fn click(&mut self, position: Vector2<f32>);

    fn click_at(&mut self, name: Uuid);
}

impl EditorExt for Editor {
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
#[derive(Default)]
struct MenuTestPlugin {
    frame: usize,
}

impl EditorPlugin for MenuTestPlugin {
    fn on_update(&mut self, editor: &mut Editor) {
        match self.frame {
            0 => editor.click_at(FileMenu::FILE),
            1 => editor.click_at(FileMenu::NEW_SCENE),
            2 => assert_eq!(editor.scenes.len(), 1),
            _ => editor.exit = true,
        }
        self.frame += 1;
    }
}

fn main() {
    Log::set_file_name("fyrox.log");
    run_editor_test("MenuTest", MenuTestPlugin::default());
}
