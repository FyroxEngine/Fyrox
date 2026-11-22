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
    Editor, Message,
    fyrox::{
        core::some_or_return, engine::ApplicationLoopController, gui::UserInterface,
        gui::message::UiMessage, gui::widget::WidgetMessage, scene::sound::Sound,
    },
    game_scene::GameScene,
    plugin::EditorPlugin,
    plugins::inspector::InspectorPlugin,
};
use fyrox::gui::dock::{TileContent, TileMessage};

use panel::AudioPanel;
use preview::AudioPreviewPanel;

mod bus;
mod command;
mod panel;
pub mod preview;
mod selection;

#[derive(Default)]
pub struct AudioPlugin {
    sound_preview_panel: Option<AudioPreviewPanel>,
    audio_panel: AudioPanel,
}

impl AudioPlugin {
    fn destroy_panel(&mut self, ui: &UserInterface) {
        if let Some(panel) = self.sound_preview_panel.take() {
            panel.destroy(ui);
        }
    }
}

impl EditorPlugin for AudioPlugin {
    fn on_start(&mut self, editor: &mut Editor) {
        self.audio_panel = AudioPanel::new(
            &mut editor.engine,
            editor.message_sender.clone(),
            editor.asset_browser.preview_sender.clone(),
        );
        editor.engine.user_interfaces.first().send(
            editor.default_tiles.audio_tile,
            TileMessage::Content(TileContent::Window(self.audio_panel.window)),
        );
    }

    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        let entry = editor.scenes.current_scene_entry_mut();
        let game_scene = some_or_return!(entry.controller.downcast_mut::<GameScene>());
        if let Some(panel) = self.sound_preview_panel.as_mut() {
            panel.handle_ui_message(message, &entry.selection, game_scene, &mut editor.engine);
        }
        self.audio_panel.handle_ui_message(
            message,
            &entry.selection,
            &editor.message_sender,
            &editor.engine,
        );
    }

    fn on_sync_to_model(&mut self, editor: &mut Editor) {
        let entry = editor.scenes.current_scene_entry_mut();
        let game_scene = some_or_return!(entry.controller.downcast_mut::<GameScene>());
        self.audio_panel
            .sync_to_model(&entry.selection, game_scene, &mut editor.engine);
    }

    fn on_mode_changed(&mut self, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first();
        self.audio_panel.on_mode_changed(ui, &editor.mode);
    }

    fn on_leave_preview_mode(&mut self, editor: &mut Editor) {
        let entry = editor.scenes.current_scene_entry_mut();
        let game_scene = some_or_return!(entry.controller.downcast_mut::<GameScene>());
        if let Some(panel) = self.sound_preview_panel.as_mut() {
            panel.leave_preview_mode(game_scene, &mut editor.engine);
        }
    }

    fn on_update(&mut self, editor: &mut Editor, _loop_controller: ApplicationLoopController) {
        let entry = editor.scenes.current_scene_entry_mut();
        let game_scene = some_or_return!(entry.controller.downcast_mut::<GameScene>());
        if let Some(panel) = self.sound_preview_panel.as_mut() {
            panel.update(&entry.selection, game_scene, &editor.engine);
        }
    }

    fn is_in_preview_mode(&self, _editor: &Editor) -> bool {
        self.sound_preview_panel
            .as_ref()
            .is_some_and(|p| p.is_in_preview_mode())
    }

    fn on_message(&mut self, message: &Message, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();
        let entry = editor.scenes.current_scene_entry_mut();
        let Some(game_scene) = entry.controller.downcast_mut::<GameScene>() else {
            self.destroy_panel(ui);
            return;
        };

        if let Message::SelectionChanged { .. } = message {
            let scene = &mut editor.engine.scenes[game_scene.scene];
            let is_sound_selected = entry
                .selection
                .as_graph()
                .is_some_and(|s| s.contains_type::<Sound>(&scene.graph));
            if is_sound_selected {
                if self.sound_preview_panel.is_none() {
                    let inspector = editor.plugins.get::<InspectorPlugin>();
                    let panel = AudioPreviewPanel::new(&mut ui.build_ctx());
                    ui.send(panel.root_widget, WidgetMessage::link_with(inspector.head));
                    self.sound_preview_panel = Some(panel);
                }
            } else {
                self.destroy_panel(ui);
            }
        } else if let Message::DoCommand(_)
        | Message::UndoCurrentSceneCommand
        | Message::RedoCurrentSceneCommand = message
            && let Some(panel) = self.sound_preview_panel.as_mut()
        {
            panel.leave_preview_mode(game_scene, &mut editor.engine);
        }
    }
}
