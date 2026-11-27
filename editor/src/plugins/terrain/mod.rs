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

use crate::game_scene::GameScene;
use crate::plugin::EditorPlugin;
use crate::plugins::terrain::interaction::TerrainInteractionMode;
use crate::{Editor, Message};
use fyrox::core::some_or_return;
use fyrox::graph::SceneGraph;
use fyrox::scene::terrain::Terrain;

pub mod command;
pub mod gizmo;
pub mod interaction;
pub mod panel;

#[derive(Default)]
pub struct TerrainPlugin {}

impl EditorPlugin for TerrainPlugin {
    fn on_message(&mut self, message: &Message, editor: &mut Editor) {
        let entry = editor.scenes.current_scene_entry_mut();
        let game_scene = some_or_return!(entry.controller.downcast_mut::<GameScene>());
        let scene = &mut editor.engine.scenes[game_scene.scene];

        if let Message::SelectionChanged { .. } = message {
            let selected_terrain = entry
                .selection
                .as_graph()
                .and_then(|s| {
                    s.nodes()
                        .iter()
                        .find(|h| scene.graph.is_or_has_field::<Terrain>(**h))
                        .cloned()
                })
                .or_else(|| entry.selection.as_navmesh().map(|s| s.navmesh_node()));

            if selected_terrain.is_some() {
                if entry
                    .interaction_modes
                    .of_type::<TerrainInteractionMode>()
                    .is_none()
                {
                    entry.interaction_modes.add(TerrainInteractionMode::new(
                        game_scene,
                        &mut editor.engine,
                        editor.message_sender.clone(),
                        editor.scene_viewer.frame(),
                    ));
                }
            } else {
                if let Some(mode) = entry
                    .interaction_modes
                    .remove_typed::<TerrainInteractionMode>()
                {
                    let ui = editor.engine.user_interfaces.first_mut();
                    mode.destroy(&mut scene.graph, ui);
                }
            }
        }
    }
}
