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
    fyrox::core::some_or_return,
    fyrox::{graph::SceneGraph, gui::message::UiMessage, scene::navmesh::NavigationalMesh},
    game_scene::{GameScene, Selection},
    plugin::EditorPlugin,
};
use fyrox::core::dyntype::DynTypeConstructorContainer;
use fyrox::gui::inspector::editors::inspectable::InspectablePropertyEditorDefinition;

use interaction::NavmeshInteractionMode;
use panel::NavmeshPanel;
use selection::NavmeshSelection;
use settings::NavmeshSettings;

mod command;
mod interaction;
mod panel;
pub mod selection;
pub mod settings;

fn fetch_selection(editor_selection: &Selection) -> Option<NavmeshSelection> {
    if let Some(selection) = editor_selection.as_navmesh() {
        Some(selection.clone())
    } else {
        editor_selection.as_graph().map(|selection| {
            NavmeshSelection::new(selection.nodes.first().cloned().unwrap_or_default(), vec![])
        })
    }
}

#[derive(Default)]
pub struct NavmeshPlugin {
    panel: Option<NavmeshPanel>,
}

impl EditorPlugin for NavmeshPlugin {
    fn on_register(&mut self, dyn_type_constructors: &DynTypeConstructorContainer) {
        dyn_type_constructors.add::<NavmeshSettings, ()>("Navmesh Settings");
    }

    fn on_start(&mut self, editor: &mut Editor) {
        editor
            .property_editors
            .insert(InspectablePropertyEditorDefinition::<NavmeshSettings>::new());
    }

    fn on_sync_to_model(&mut self, editor: &mut Editor) {
        let entry = editor.scenes.current_scene_entry_mut();
        let game_scene = some_or_return!(entry.controller.downcast_mut::<GameScene>());
        let panel = some_or_return!(self.panel.as_mut());
        panel.sync_to_model(&editor.engine, &entry.selection, game_scene);
    }

    fn on_mode_changed(&mut self, editor: &mut Editor) {
        let panel = some_or_return!(self.panel.as_mut());
        let ui = editor.engine.user_interfaces.first();
        panel.on_mode_changed(ui, &editor.mode);
    }

    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        let entry = editor.scenes.current_scene_entry_mut();
        let panel = some_or_return!(self.panel.as_mut());
        panel.handle_ui_message(message, &editor.message_sender, &entry.selection);
    }

    fn on_message(&mut self, message: &Message, editor: &mut Editor) {
        let entry = editor.scenes.current_scene_entry_mut();
        let game_scene = some_or_return!(entry.controller.downcast_mut::<GameScene>());
        let scene = &mut editor.engine.scenes[game_scene.scene];

        if let Message::SelectionChanged { .. } = message {
            let selected_navmesh = entry
                .selection
                .as_graph()
                .and_then(|s| {
                    s.nodes()
                        .iter()
                        .find(|h| scene.graph.is_or_has_field::<NavigationalMesh>(**h))
                        .cloned()
                })
                .or_else(|| entry.selection.as_navmesh().map(|s| s.navmesh_node()));

            if selected_navmesh.is_some() {
                if entry
                    .interaction_modes
                    .of_type::<NavmeshInteractionMode>()
                    .is_none()
                {
                    entry.interaction_modes.add(NavmeshInteractionMode::new(
                        game_scene,
                        &mut editor.engine,
                        editor.message_sender.clone(),
                    ));
                }

                if self.panel.is_none() {
                    let ui = editor.engine.user_interfaces.first_mut();
                    let panel = NavmeshPanel::new(
                        editor.scene_viewer.frame(),
                        &mut ui.build_ctx(),
                        editor.message_sender.clone(),
                    );
                    self.panel = Some(panel);
                }
            } else {
                if let Some(panel) = self.panel.take() {
                    let ui = editor.engine.user_interfaces.first();
                    panel.destroy(ui);
                }
                if let Some(mode) = entry
                    .interaction_modes
                    .remove_typed::<NavmeshInteractionMode>()
                {
                    mode.destroy(scene);
                }
            }
        }
    }
}
