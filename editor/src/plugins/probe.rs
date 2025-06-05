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
        core::{pool::Handle, some_or_return},
        engine::Engine,
        graph::SceneGraph,
        gui::{
            button::{ButtonBuilder, ButtonMessage},
            grid::{Column, GridBuilder, Row},
            message::{MessageDirection, UiMessage},
            widget::{WidgetBuilder, WidgetMessage},
            BuildContext, Thickness, UiNode, UserInterface, VerticalAlignment,
        },
        scene::probe::ReflectionProbe,
    },
    plugin::EditorPlugin,
    plugins::inspector::InspectorPlugin,
    scene::{GameScene, Selection},
    Editor, Message,
};

pub struct ReflectionProbePreviewControlPanel {
    pub root_widget: Handle<UiNode>,
    update: Handle<UiNode>,
}

impl ReflectionProbePreviewControlPanel {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let update;
        let root_widget = GridBuilder::new(WidgetBuilder::new().with_child({
            update = ButtonBuilder::new(
                WidgetBuilder::new()
                    .on_row(0)
                    .on_column(0)
                    .with_vertical_alignment(VerticalAlignment::Center)
                    .with_margin(Thickness::uniform(1.0)),
            )
            .with_text("Update")
            .build(ctx);
            update
        }))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        Self {
            root_widget,
            update,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_selection: &Selection,
        game_scene: &mut GameScene,
        engine: &mut Engine,
    ) {
        if let Some(selection) = editor_selection.as_graph() {
            if let Some(ButtonMessage::Click) = message.data() {
                if message.destination == self.update {
                    let scene = &mut engine.scenes[game_scene.scene];

                    for &node in &selection.nodes {
                        if let Some(particle_system) =
                            scene.graph.try_get_mut_of_type::<ReflectionProbe>(node)
                        {
                            particle_system.force_update();
                        }
                    }
                }
            }
        }
    }

    fn destroy(self, ui: &UserInterface) {
        ui.send_message(WidgetMessage::remove(
            self.root_widget,
            MessageDirection::ToWidget,
        ));
    }
}

#[derive(Default)]
pub struct ReflectionProbePlugin {
    panel: Option<ReflectionProbePreviewControlPanel>,
}

impl EditorPlugin for ReflectionProbePlugin {
    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        let entry = some_or_return!(editor.scenes.current_scene_entry_mut());
        let game_scene = some_or_return!(entry.controller.downcast_mut::<GameScene>());
        let panel = some_or_return!(self.panel.as_mut());
        panel.handle_ui_message(message, &entry.selection, game_scene, &mut editor.engine);
    }

    fn on_message(&mut self, message: &Message, editor: &mut Editor) {
        let entry = some_or_return!(editor.scenes.current_scene_entry_mut());
        let selection = some_or_return!(entry.selection.as_graph());
        let game_scene = some_or_return!(entry.controller.downcast_mut::<GameScene>());

        let scene = &mut editor.engine.scenes[game_scene.scene];

        if let Message::SelectionChanged { .. } = message {
            let has_selected_reflection_probe = selection
                .nodes()
                .iter()
                .any(|h| scene.graph.has_component::<ReflectionProbe>(*h));

            if has_selected_reflection_probe {
                if self.panel.is_none() {
                    let inspector = editor.plugins.get::<InspectorPlugin>();
                    let ui = editor.engine.user_interfaces.first_mut();
                    let panel = ReflectionProbePreviewControlPanel::new(&mut ui.build_ctx());
                    ui.send_message(WidgetMessage::link(
                        panel.root_widget,
                        MessageDirection::ToWidget,
                        inspector.head,
                    ));
                    self.panel = Some(panel);
                }
            } else if let Some(panel) = self.panel.take() {
                let ui = editor.engine.user_interfaces.first();
                panel.destroy(ui);
            }
        }
    }
}
