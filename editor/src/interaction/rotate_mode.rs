use crate::scene::controller::SceneController;
use crate::{
    camera::PickingOptions,
    interaction::{
        calculate_gizmo_distance_scaling, gizmo::rotate_gizmo::RotationGizmo,
        make_interaction_mode_button, InteractionMode,
    },
    message::MessageSender,
    scene::{
        commands::{graph::RotateNodeCommand, ChangeSelectionCommand, CommandGroup, SceneCommand},
        EditorScene, Selection,
    },
    settings::Settings,
    world::graph::selection::GraphSelection,
    Engine,
};
use fyrox::core::TypeUuidProvider;
use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector2},
        math::round_to_step,
        pool::Handle,
        uuid::{uuid, Uuid},
    },
    gui::{BuildContext, UiNode},
};

pub struct RotateInteractionMode {
    initial_rotations: Vec<UnitQuaternion<f32>>,
    rotation_gizmo: RotationGizmo,
    interacting: bool,
    message_sender: MessageSender,
}

impl RotateInteractionMode {
    pub fn new(
        editor_scene: &EditorScene,
        engine: &mut Engine,
        message_sender: MessageSender,
    ) -> Self {
        Self {
            initial_rotations: Default::default(),
            rotation_gizmo: RotationGizmo::new(editor_scene, engine),
            interacting: false,
            message_sender,
        }
    }
}

impl TypeUuidProvider for RotateInteractionMode {
    fn type_uuid() -> Uuid {
        uuid!("37f20364-feb5-4731-8c19-c3df922818d6")
    }
}

impl InteractionMode for RotateInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
        settings: &Settings,
    ) {
        let Some(editor_scene) = controller.downcast_mut::<EditorScene>() else {
            return;
        };

        let graph = &mut engine.scenes[editor_scene.scene].graph;

        // Pick gizmo nodes.
        let camera = editor_scene.camera_controller.camera;
        let camera_pivot = editor_scene.camera_controller.pivot;
        if let Some(result) = editor_scene.camera_controller.pick(PickingOptions {
            cursor_pos: mouse_pos,
            graph,
            editor_objects_root: editor_scene.editor_objects_root,
            scene_content_root: editor_scene.scene_content_root,
            screen_size: frame_size,
            editor_only: true,
            filter: |handle, _| {
                handle != camera && handle != camera_pivot && handle != self.rotation_gizmo.origin
            },
            ignore_back_faces: settings.selection.ignore_back_faces,
            use_picking_loop: true,
            only_meshes: false,
        }) {
            if self
                .rotation_gizmo
                .handle_pick(result.node, editor_scene, engine)
            {
                let graph = &mut engine.scenes[editor_scene.scene].graph;
                if let Selection::Graph(selection) = editor_selection {
                    self.interacting = true;
                    self.initial_rotations = selection.local_rotations(graph);
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
        let Some(editor_scene) = controller.downcast_mut::<EditorScene>() else {
            return;
        };

        let graph = &mut engine.scenes[editor_scene.scene].graph;

        self.rotation_gizmo.reset_state(graph);

        if self.interacting {
            if let Selection::Graph(selection) = editor_selection {
                if !selection.is_empty() {
                    self.interacting = false;
                    let current_rotation = selection.local_rotations(graph);
                    if current_rotation != self.initial_rotations {
                        let commands = CommandGroup::from(
                            selection
                                .nodes()
                                .iter()
                                .zip(self.initial_rotations.iter().zip(current_rotation.iter()))
                                .map(|(&node, (&old_rotation, &new_rotation))| {
                                    SceneCommand::new(RotateNodeCommand::new(
                                        node,
                                        old_rotation,
                                        new_rotation,
                                    ))
                                })
                                .collect::<Vec<SceneCommand>>(),
                        );
                        // Commit changes.
                        self.message_sender.do_scene_command(commands);
                    }
                }
            }
        } else {
            let new_selection = editor_scene
                .camera_controller
                .pick(PickingOptions {
                    cursor_pos: mouse_pos,
                    graph,
                    editor_objects_root: editor_scene.editor_objects_root,
                    scene_content_root: editor_scene.scene_content_root,
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
        settings: &Settings,
    ) {
        let Some(editor_scene) = controller.downcast_mut::<EditorScene>() else {
            return;
        };

        if let Selection::Graph(selection) = editor_selection {
            if self.interacting {
                let rotation_delta = self.rotation_gizmo.calculate_rotation_delta(
                    editor_scene,
                    editor_scene.camera_controller.camera,
                    mouse_offset,
                    mouse_position,
                    engine,
                    frame_size,
                );
                for &node in selection.nodes().iter() {
                    let transform =
                        engine.scenes[editor_scene.scene].graph[node].local_transform_mut();
                    let rotation = **transform.rotation();
                    let final_rotation = rotation * rotation_delta;
                    let (mut roll, mut pitch, mut yaw) = final_rotation.euler_angles();
                    if settings.rotate_mode_settings.angle_snapping {
                        pitch = round_to_step(
                            pitch,
                            settings.rotate_mode_settings.x_snap_step.to_radians(),
                        );
                        yaw = round_to_step(
                            yaw,
                            settings.rotate_mode_settings.y_snap_step.to_radians(),
                        );
                        roll = round_to_step(
                            roll,
                            settings.rotate_mode_settings.z_snap_step.to_radians(),
                        );
                    }
                    transform.set_rotation(UnitQuaternion::from_euler_angles(roll, pitch, yaw));
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
        let Some(editor_scene) = controller.downcast_mut::<EditorScene>() else {
            return;
        };

        if let Selection::Graph(selection) = editor_selection {
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            if editor_selection.is_empty() || editor_scene.preview_camera.is_some() {
                self.rotation_gizmo.set_visible(graph, false);
            } else {
                let scale = calculate_gizmo_distance_scaling(
                    graph,
                    editor_scene.camera_controller.camera,
                    self.rotation_gizmo.origin,
                );
                self.rotation_gizmo.sync_transform(graph, selection, scale);
                self.rotation_gizmo.set_visible(graph, true);
            }
        }
    }

    fn deactivate(&mut self, controller: &dyn SceneController, engine: &mut Engine) {
        let Some(editor_scene) = controller.downcast_ref::<EditorScene>() else {
            return;
        };

        let graph = &mut engine.scenes[editor_scene.scene].graph;
        self.rotation_gizmo.set_visible(graph, false);
    }

    fn make_button(&mut self, ctx: &mut BuildContext, selected: bool) -> Handle<UiNode> {
        let rotate_mode_tooltip =
            "Rotate Object(s) - Shortcut: [3]\n\nRotation interaction mode allows you to rotate selected \
        objects. Keep in mind that rotation always works in local coordinates!\n\n\
        This also allows you to select an object or add an object to current selection using Ctrl+Click";

        make_interaction_mode_button(
            ctx,
            include_bytes!("../../resources/rotate_arrow.png"),
            rotate_mode_tooltip,
            selected,
        )
    }

    fn uuid(&self) -> Uuid {
        Self::type_uuid()
    }
}
