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
    camera::PickingOptions,
    command::SetPropertyCommand,
    fyrox::{
        core::{
            algebra::{Vector2, Vector3},
            pool::Handle,
            some_or_return,
            type_traits::prelude::*,
            Uuid,
        },
        engine::Engine,
        graph::SceneGraph,
        gui::{
            button::{ButtonBuilder, ButtonMessage},
            grid::{Column, GridBuilder, Row},
            message::UiMessage,
            widget::{WidgetBuilder, WidgetMessage},
            BuildContext, Thickness, UiNode, UserInterface, VerticalAlignment,
        },
        scene::{probe::ReflectionProbe, Scene},
    },
    interaction::{
        calculate_gizmo_distance_scaling, gizmo::move_gizmo::MoveGizmo,
        make_interaction_mode_button, plane::PlaneKind, InteractionMode,
    },
    message::MessageSender,
    plugin::EditorPlugin,
    plugins::inspector::InspectorPlugin,
    scene::{commands::GameSceneContext, controller::SceneController, GameScene, Selection},
    settings::Settings,
    Editor, Message,
};
use fyrox::core::reflect::Reflect;

pub struct ReflectionProbePreviewControlPanel {
    pub root_widget: Handle<UiNode>,
    update: Handle<UiNode>,
    adjust: Handle<UiNode>,
}

impl ReflectionProbePreviewControlPanel {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let update;
        let adjust;
        let root_widget = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
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
                })
                .with_child({
                    adjust = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(1)
                            .with_vertical_alignment(VerticalAlignment::Center)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .with_text("Adjust")
                    .build(ctx);
                    adjust
                }),
        )
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        Self {
            root_widget,
            update,
            adjust,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_selection: &Selection,
        game_scene: &mut GameScene,
        engine: &mut Engine,
        sender: &MessageSender,
    ) {
        if let Some(selection) = editor_selection.as_graph() {
            if let Some(ButtonMessage::Click) = message.data() {
                if message.destination == self.update {
                    let scene = &mut engine.scenes[game_scene.scene];

                    for &node in &selection.nodes {
                        if let Ok(particle_system) =
                            scene.graph.try_get_mut_of_type::<ReflectionProbe>(node)
                        {
                            particle_system.force_update();
                        }
                    }
                } else if message.destination == self.adjust {
                    sender.send(Message::SetInteractionMode(
                        ReflectionProbeInteractionMode::type_uuid(),
                    ));
                }
            }
        }
    }

    fn destroy(self, ui: &UserInterface) {
        ui.send(self.root_widget, WidgetMessage::Remove);
    }
}

struct DragContext {
    new_position: Vector3<f32>,
    plane_kind: PlaneKind,
}

#[derive(TypeUuidProvider)]
#[type_uuid(id = "d8fd164c-523c-447a-93ab-e86f2d71eed6")]
pub struct ReflectionProbeInteractionMode {
    probe: Handle<ReflectionProbe>,
    move_gizmo: MoveGizmo,
    message_sender: MessageSender,
    drag_context: Option<DragContext>,
}

impl ReflectionProbeInteractionMode {
    fn destroy(self, scene: &mut Scene) {
        self.move_gizmo.destroy(&mut scene.graph)
    }

    fn set_visible(&self, controller: &dyn SceneController, engine: &mut Engine, visible: bool) {
        let game_scene = some_or_return!(controller.downcast_ref::<GameScene>());
        let scene = &mut engine.scenes[game_scene.scene];
        self.move_gizmo.set_visible(&mut scene.graph, visible);
    }
}

impl InteractionMode for ReflectionProbeInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        _editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        mouse_position: Vector2<f32>,
        _frame_size: Vector2<f32>,
        settings: &Settings,
    ) {
        let game_scene = some_or_return!(controller.downcast_mut::<GameScene>());

        let scene = &mut engine.scenes[game_scene.scene];

        if let Some(result) = game_scene.camera_controller.pick(
            &scene.graph,
            PickingOptions {
                cursor_pos: mouse_position,
                editor_only: true,
                filter: Some(&mut |handle, _| handle != self.move_gizmo.origin),
                ignore_back_faces: false,
                use_picking_loop: false,
                method: Default::default(),
                settings: &settings.selection,
            },
        ) {
            if let Some(plane_kind) = self.move_gizmo.handle_pick(result.node, &mut scene.graph) {
                self.drag_context = Some(DragContext {
                    new_position: *scene.graph[self.probe].rendering_position,
                    plane_kind,
                })
            }
        }
    }

    fn on_left_mouse_button_up(
        &mut self,
        _editor_selection: &Selection,
        _controller: &mut dyn SceneController,
        _engine: &mut Engine,
        _mouse_pos: Vector2<f32>,
        _frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let drag_context = some_or_return!(self.drag_context.take());
        let probe = self.probe;
        let command = SetPropertyCommand::new(
            ReflectionProbe::RENDERING_POSITION.into(),
            Box::new(drag_context.new_position),
            move |ctx| {
                ctx.get_mut::<GameSceneContext>()
                    .scene
                    .graph
                    .try_get_node_mut(probe.transmute())
                    .ok()
                    .map(|n| n as &mut dyn Reflect)
            },
        );
        self.message_sender.do_command(command);
    }

    fn on_mouse_move(
        &mut self,
        mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        _editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        frame_size: Vector2<f32>,
        settings: &Settings,
    ) {
        let game_scene = some_or_return!(controller.downcast_mut::<GameScene>());
        let scene = &mut engine.scenes[game_scene.scene];

        self.move_gizmo.reset_state(&mut scene.graph);
        if let Some(result) = game_scene.camera_controller.pick(
            &scene.graph,
            PickingOptions {
                cursor_pos: mouse_position,
                editor_only: true,
                filter: Some(&mut |handle, _| handle != self.move_gizmo.origin),
                ignore_back_faces: false,
                use_picking_loop: false,
                method: Default::default(),
                settings: &settings.selection,
            },
        ) {
            self.move_gizmo.handle_pick(result.node, &mut scene.graph);
        }

        if let Some(drag_context) = self.drag_context.as_mut() {
            let global_offset = self.move_gizmo.calculate_offset(
                &scene.graph,
                game_scene.camera_controller.camera,
                mouse_offset,
                mouse_position,
                frame_size,
                drag_context.plane_kind,
            );
            drag_context.new_position += global_offset;
            scene.graph[self.probe]
                .rendering_position
                .set_value_and_mark_modified(drag_context.new_position);
        }
    }

    fn update(
        &mut self,
        _editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        _settings: &Settings,
    ) {
        let game_scene = some_or_return!(controller.downcast_mut::<GameScene>());
        let scene = &mut engine.scenes[game_scene.scene];

        let scale = calculate_gizmo_distance_scaling(
            &scene.graph,
            game_scene.camera_controller.camera,
            self.move_gizmo.origin,
        );

        self.move_gizmo.set_visible(&mut scene.graph, true);

        let position = scene.graph[self.probe].global_rendering_position();
        self.move_gizmo
            .transform(&mut scene.graph)
            .set_position(position)
            .set_scale(scale);
    }

    fn activate(&mut self, controller: &dyn SceneController, engine: &mut Engine) {
        self.set_visible(controller, engine, true);
    }

    fn deactivate(&mut self, controller: &dyn SceneController, engine: &mut Engine) {
        self.set_visible(controller, engine, false);
    }

    fn make_button(&mut self, ctx: &mut BuildContext, selected: bool) -> Handle<UiNode> {
        make_interaction_mode_button(
            ctx,
            include_bytes!("../../resources/triangle.png"),
            "Edit Reflection Probe",
            selected,
        )
    }

    fn uuid(&self) -> Uuid {
        Self::type_uuid()
    }
}

#[derive(Default)]
pub struct ReflectionProbePlugin {
    panel: Option<ReflectionProbePreviewControlPanel>,
}

impl EditorPlugin for ReflectionProbePlugin {
    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
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

    fn on_message(&mut self, message: &Message, editor: &mut Editor) {
        let entry = editor.scenes.current_scene_entry_mut();
        let game_scene = some_or_return!(entry.controller.downcast_mut::<GameScene>());

        let scene = &mut editor.engine.scenes[game_scene.scene];

        if let Message::SelectionChanged { .. } = message {
            if let Some(mode) = entry
                .interaction_modes
                .remove_typed::<ReflectionProbeInteractionMode>()
            {
                mode.destroy(scene);
            }

            let selected_reflection_probe = entry.selection.as_graph().and_then(|s| {
                s.nodes()
                    .iter()
                    .find(|h| scene.graph.has_component::<ReflectionProbe>(**h))
                    .cloned()
            });

            if let Some(selected_reflection_probe) = selected_reflection_probe {
                entry.interaction_modes.add(ReflectionProbeInteractionMode {
                    probe: selected_reflection_probe.transmute(),
                    move_gizmo: MoveGizmo::new(game_scene, &mut editor.engine),
                    message_sender: editor.message_sender.clone(),
                    drag_context: None,
                });

                if self.panel.is_none() {
                    let inspector = editor.plugins.get::<InspectorPlugin>();
                    let ui = editor.engine.user_interfaces.first_mut();
                    let panel = ReflectionProbePreviewControlPanel::new(&mut ui.build_ctx());
                    ui.send(panel.root_widget, WidgetMessage::LinkWith(inspector.head));
                    self.panel = Some(panel);
                }
            } else if let Some(panel) = self.panel.take() {
                let ui = editor.engine.user_interfaces.first();
                panel.destroy(ui);
            }
        }
    }
}
