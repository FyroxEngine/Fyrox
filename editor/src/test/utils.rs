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
        core::{algebra::Vector2, info, pool::Handle, Uuid},
        engine::ApplicationLoopController,
        gui::{message::UiMessage, test::UserInterfaceTestingExtension, Control, UiNode},
    },
    plugin::EditorPlugin,
    settings::Settings,
    test::macros::Macro,
    Editor, StartupData,
};
use std::path::PathBuf;

/// Initializes the editor as is a user would open it, adds the specified plugin that runs the test
/// logic.
pub fn run_editor_test(name: &str, plugin: impl EditorPlugin) {
    let path = PathBuf::from(format!("./AutomatedTests/{name}TestData"));
    if path.exists() {
        std::fs::remove_dir_all(&path).unwrap();
    }
    std::fs::create_dir_all(&path).unwrap();

    let path = path.canonicalize().unwrap();
    info!("Running {name}. Working dir: {}", path.display());

    let mut settings = Settings::default();
    settings.windows.window_position = Default::default();
    settings.windows.window_size = Vector2::repeat(1000.0);
    let mut editor = Editor::new_with_settings(
        Some(StartupData {
            working_directory: path.clone(),
            scenes: vec![],
            named_objects: false,
        }),
        settings,
    );

    editor.add_editor_plugin(plugin);

    editor.run_headless()
}

pub trait EditorTestingExtension {
    /// Clicks at the given position.
    fn click(&mut self, position: Vector2<f32>);

    /// Tries to find a widget with the given unique id and clicks at its center.
    fn click_at(&mut self, name: Uuid) -> Handle<UiNode>;

    fn click_at_text(&mut self, uuid: Uuid, text: &str);

    fn find(&self, uuid: Uuid) -> Option<&UiNode>;

    fn find_of<T: Control>(&self, uuid: Uuid) -> Option<&T>;

    fn is_visible(&self, uuid: Uuid) -> bool {
        if let Some(node) = self.find(uuid) {
            node.is_globally_visible()
        } else {
            panic!("Widget {uuid} does not exist!")
        }
    }
}

impl EditorTestingExtension for Editor {
    fn click(&mut self, position: Vector2<f32>) {
        self.engine.user_interfaces.first_mut().click(position)
    }

    fn click_at(&mut self, uuid: Uuid) -> Handle<UiNode> {
        self.engine.user_interfaces.first_mut().click_at(uuid)
    }

    fn click_at_text(&mut self, uuid: Uuid, text: &str) {
        self.engine
            .user_interfaces
            .first_mut()
            .click_at_text(uuid, text)
    }

    fn find(&self, uuid: Uuid) -> Option<&UiNode> {
        self.engine.user_interfaces.first().find_by_uuid(uuid)
    }

    fn find_of<T: Control>(&self, uuid: Uuid) -> Option<&T> {
        self.engine.user_interfaces.first().find_by_uuid_of(uuid)
    }
}

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
    fn on_ui_message(&mut self, message: &mut UiMessage, _editor: &mut Editor) {
        info!("{message:?}")
    }

    fn on_post_update(&mut self, editor: &mut Editor, loop_controller: ApplicationLoopController) {
        self.test_macro.execute_next(editor, loop_controller)
    }
}
