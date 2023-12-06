use crate::scene::controller::SceneController;
use crate::{
    camera::PickingOptions,
    interaction::{
        calculate_gizmo_distance_scaling, gizmo::scale_gizmo::ScaleGizmo,
        make_interaction_mode_button, InteractionMode,
    },
    message::MessageSender,
    scene::{
        commands::{graph::ScaleNodeCommand, ChangeSelectionCommand, CommandGroup, SceneCommand},
        GameScene, Selection,
    },
    settings::Settings,
    world::graph::selection::GraphSelection,
    Engine,
};
use fyrox::{
    core::{
        algebra::{Vector2, Vector3},
        pool::Handle,
        uuid::{uuid, Uuid},
        TypeUuidProvider,
    },
    gui::{BuildContext, UiNode},
};

pub struct ScaleInteractionMode {
    initial_scales: Vec<Vector3<f32>>,
    scale_gizmo: ScaleGizmo,
    interacting: bool,
    message_sender: MessageSender,
}

impl ScaleInteractionMode {
    pub fn new(game_scene: &GameScene, engine: &mut Engine, message_sender: MessageSender) -> Self {
        Self {
            initial_scales: Default::default(),
            scale_gizmo: ScaleGizmo::new(game_scene, engine),
            interacting: false,
            message_sender,
        }
    }
}

impl TypeUuidProvider for ScaleInteractionMode {
    fn type_uuid() -> Uuid {
        uuid!("64b4da1a-5d0f-49e1-9f48-011165cd1ec5")
    }
}

impl InteractionMode for ScaleInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
        settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        if let Selection::Graph(selection) = editor_selection {
            let graph = &mut engine.scenes[game_scene.scene].graph;

            // Pick gizmo nodes.
            let camera = game_scene.camera_controller.camera;
            let camera_pivot = game_scene.camera_controller.pivot;
            if let Some(result) = game_scene.camera_controller.pick(PickingOptions {
                cursor_pos: mouse_pos,
                graph,
                editor_objects_root: game_scene.editor_objects_root,
                scene_content_root: game_scene.scene_content_root,
                screen_size: frame_size,
                editor_only: true,
                filter: |handle, _| handle != camera && handle != camera_pivot,
                ignore_back_faces: settings.selection.ignore_back_faces,
                use_picking_loop: true,
                only_meshes: false,
            }) {
                if self
                    .scale_gizmo
                    .handle_pick(result.node, game_scene, engine)
                {
                    let graph = &mut engine.scenes[game_scene.scene].graph;
                    self.interacting = true;
                    self.initial_scales = selection.local_scales(graph);
                }
            }
        }
    }

    fn on_left_mouse_button_up(
        &mut self,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
        settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        let graph = &mut engine.scenes[game_scene.scene].graph;

        self.scale_gizmo.reset_state(graph);

        if self.interacting {
            if let Selection::Graph(selection) = editor_selection {
                if !selection.is_empty() {
                    self.interacting = false;
                    let current_scales = selection.local_scales(graph);
                    if current_scales != self.initial_scales {
                        // Commit changes.
                        let commands = CommandGroup::from(
                            selection
                                .nodes()
                                .iter()
                                .zip(self.initial_scales.iter().zip(current_scales.iter()))
                                .map(|(&node, (&old_scale, &new_scale))| {
                                    SceneCommand::new(ScaleNodeCommand::new(
                                        node, old_scale, new_scale,
                                    ))
                                })
                                .collect::<Vec<_>>(),
                        );
                        self.message_sender.do_scene_command(commands);
                    }
                }
            }
        } else {
            let new_selection = game_scene
                .camera_controller
                .pick(PickingOptions {
                    cursor_pos: mouse_pos,
                    graph,
                    editor_objects_root: game_scene.editor_objects_root,
                    scene_content_root: game_scene.scene_content_root,
                    screen_size: frame_size,
                    editor_only: false,
                    filter: |_, _| true,
                    ignore_back_faces: settings.selection.ignore_back_faces,
                    use_picking_loop: true,
                    only_meshes: false,
                })
                .map(|result| {
                    if let (Selection::Graph(selection), true) = (
                        editor_selection,
                        engine.user_interface.keyboard_modifiers().control,
                    ) {
                        let mut selection = selection.clone();
                        selection.insert_or_exclude(result.node);
                        Selection::Graph(selection)
                    } else {
                        Selection::Graph(GraphSelection::single_or_empty(result.node))
                    }
                })
                .unwrap_or_else(|| Selection::Graph(GraphSelection::default()));

            if &new_selection != editor_selection {
                self.message_sender
                    .do_scene_command(ChangeSelectionCommand::new(
                        new_selection,
                        editor_selection.clone(),
                    ));
            }
        }
    }

    fn on_mouse_move(
        &mut self,
        mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        if let Selection::Graph(selection) = editor_selection {
            if self.interacting {
                let scale_delta = self.scale_gizmo.calculate_scale_delta(
                    game_scene,
                    game_scene.camera_controller.camera,
                    mouse_offset,
                    mouse_position,
                    engine,
                    frame_size,
                );
                for &node in selection.nodes().iter() {
                    let transform =
                        engine.scenes[game_scene.scene].graph[node].local_transform_mut();
                    let initial_scale = transform.scale();
                    let sx = (initial_scale.x * (1.0 + scale_delta.x)).max(std::f32::EPSILON);
                    let sy = (initial_scale.y * (1.0 + scale_delta.y)).max(std::f32::EPSILON);
                    let sz = (initial_scale.z * (1.0 + scale_delta.z)).max(std::f32::EPSILON);
                    transform.set_scale(Vector3::new(sx, sy, sz));
                }
            }
        }
    }

    fn update(
        &mut self,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        _settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        if let Selection::Graph(selection) = editor_selection {
            let graph = &mut engine.scenes[game_scene.scene].graph;
            if editor_selection.is_empty() || game_scene.preview_camera.is_some() {
                self.scale_gizmo.set_visible(graph, false);
            } else {
                let scale = calculate_gizmo_distance_scaling(
                    graph,
                    game_scene.camera_controller.camera,
                    self.scale_gizmo.origin,
                );
                self.scale_gizmo.sync_transform(graph, selection, scale);
                self.scale_gizmo.set_visible(graph, true);
            }
        }
    }

    fn deactivate(&mut self, controller: &dyn SceneController, engine: &mut Engine) {
        let Some(game_scene) = controller.downcast_ref::<GameScene>() else {
            return;
        };

        let graph = &mut engine.scenes[game_scene.scene].graph;
        self.scale_gizmo.set_visible(graph, false);
    }

    fn make_button(&mut self, ctx: &mut BuildContext, selected: bool) -> Handle<UiNode> {
        let scale_mode_tooltip =
            "Scale Object(s) - Shortcut: [4]\n\nScaling interaction mode allows you to scale selected \
        objects. Keep in mind that scaling always works in local coordinates!\n\n\
        This also allows you to select an object or add an object to current selection using Ctrl+Click";

        make_interaction_mode_button(
            ctx,
            include_bytes!("../../resources/scale_arrow.png"),
            scale_mode_tooltip,
            selected,
        )
    }

    fn uuid(&self) -> Uuid {
        Self::type_uuid()
    }
}
