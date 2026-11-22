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
        core::some_or_return,
        engine::{ApplicationLoopController, Engine},
        gui::{UserInterface, message::UiMessage, widget::WidgetMessage},
        scene::mesh::{Mesh, surface::SurfaceResource},
    },
    game_scene::GameScene,
    plugin::EditorPlugin,
    plugins::{inspector::InspectorPlugin, mesh::surface::SurfaceDataPropertyEditorDefinition},
};
use data_viewer::SurfaceDataViewer;
use panel::MeshControlPanel;

mod data_viewer;
mod panel;
mod surface;

struct ViewSurfaceDataMessage(pub SurfaceResource);

#[derive(Default)]
pub struct MeshPlugin {
    panel: Option<MeshControlPanel>,
    surface_data_viewer: Option<SurfaceDataViewer>,
}

impl MeshPlugin {
    fn destroy_panel(&mut self, ui: &UserInterface) {
        if let Some(panel) = self.panel.take() {
            panel.destroy(ui);
        }
    }

    fn destroy_surface_viewer(&mut self, engine: &mut Engine) {
        if let Some(surface_data_viewer) = self.surface_data_viewer.take() {
            surface_data_viewer.destroy(engine)
        }
    }
}

impl EditorPlugin for MeshPlugin {
    fn on_start(&mut self, editor: &mut Editor) {
        editor
            .property_editors
            .insert(SurfaceDataPropertyEditorDefinition {
                sender: editor.message_sender.clone(),
            });
    }

    fn on_sync_to_model(&mut self, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();
        let entry = editor.scenes.current_scene_entry_ref();
        let selection = some_or_return!(entry.selection.as_graph());
        let game_scene = some_or_return!(entry.controller.downcast_ref::<GameScene>());
        let scene = &editor.engine.scenes[game_scene.scene];
        if let Some(panel) = self.panel.as_ref() {
            panel.sync_to_model(selection, scene, ui);
        }
    }

    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        if let Some(surface_data_viewer) = self.surface_data_viewer.take() {
            self.surface_data_viewer =
                surface_data_viewer.handle_ui_message(message, &mut editor.engine);
        }

        let entry = editor.scenes.current_scene_entry_mut();
        let game_scene = some_or_return!(entry.controller.downcast_mut::<GameScene>());
        let panel = some_or_return!(self.panel.as_mut());
        panel.handle_ui_message(
            message,
            &entry.selection,
            game_scene,
            &mut editor.engine,
            &editor.message_sender,
        );
    }

    fn on_update(&mut self, editor: &mut Editor, _loop_controller: ApplicationLoopController) {
        if let Some(viewer) = self.surface_data_viewer.as_mut() {
            viewer.update(&mut editor.engine);
        }
    }

    fn on_message(&mut self, message: &Message, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();
        let entry = editor.scenes.current_scene_entry_mut();

        let (Some(game_scene), Some(selection)) = (
            entry.controller.downcast_mut::<GameScene>(),
            entry.selection.as_graph(),
        ) else {
            self.destroy_panel(ui);
            self.destroy_surface_viewer(&mut editor.engine);
            return;
        };

        let scene = &mut editor.engine.scenes[game_scene.scene];

        if let Message::SelectionChanged { .. } = message {
            if selection.contains_type::<Mesh>(&scene.graph) {
                if self.panel.is_none() {
                    let inspector = editor.plugins.get::<InspectorPlugin>();
                    let panel = MeshControlPanel::new(selection, scene, &mut ui.build_ctx());
                    ui.send(panel.root_widget, WidgetMessage::link_with(inspector.head));
                    self.panel = Some(panel);
                }
            } else {
                self.destroy_panel(ui);
            }
        } else if let Some(data) = message.try_cast_any::<ViewSurfaceDataMessage>() {
            let mut viewer = SurfaceDataViewer::new(&mut editor.engine);
            viewer.open(data.0.clone(), &mut editor.engine);
            self.surface_data_viewer = Some(viewer);
        }
    }
}
