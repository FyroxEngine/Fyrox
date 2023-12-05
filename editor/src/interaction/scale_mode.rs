use crate::{
    camera::PickingOptions,
    interaction::{
        calculate_gizmo_distance_scaling, gizmo::scale_gizmo::ScaleGizmo, InteractionMode,
    },
    message::MessageSender,
    scene::{
        commands::{graph::ScaleNodeCommand, ChangeSelectionCommand, CommandGroup, SceneCommand},
        EditorScene, Selection,
    },
    settings::Settings,
    world::graph::selection::GraphSelection,
    Engine,
};
use fyrox::core::algebra::{Vector2, Vector3};

pub struct ScaleInteractionMode {
    initial_scales: Vec<Vector3<f32>>,
    scale_gizmo: ScaleGizmo,
    interacting: bool,
    message_sender: MessageSender,
}

impl ScaleInteractionMode {
    pub fn new(
        editor_scene: &EditorScene,
        engine: &mut Engine,
        message_sender: MessageSender,
    ) -> Self {
        Self {
            initial_scales: Default::default(),
            scale_gizmo: ScaleGizmo::new(editor_scene, engine),
            interacting: false,
            message_sender,
        }
    }
}

impl InteractionMode for ScaleInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        editor_scene: &mut EditorScene,
        engine: &mut Engine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
        settings: &Settings,
    ) {
        if let Selection::Graph(selection) = &editor_scene.selection {
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
                filter: |handle, _| handle != camera && handle != camera_pivot,
                ignore_back_faces: settings.selection.ignore_back_faces,
                use_picking_loop: true,
                only_meshes: false,
            }) {
                if self
                    .scale_gizmo
                    .handle_pick(result.node, editor_scene, engine)
                {
                    let graph = &mut engine.scenes[editor_scene.scene].graph;
                    self.interacting = true;
                    self.initial_scales = selection.local_scales(graph);
                }
            }
        }
    }

    fn on_left_mouse_button_up(
        &mut self,
        editor_scene: &mut EditorScene,
        engine: &mut Engine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
        settings: &Settings,
    ) {
        let graph = &mut engine.scenes[editor_scene.scene].graph;

        self.scale_gizmo.reset_state(graph);

        if self.interacting {
            if let Selection::Graph(selection) = &editor_scene.selection {
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
                        &editor_scene.selection,
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

            if new_selection != editor_scene.selection {
                self.message_sender
                    .do_scene_command(ChangeSelectionCommand::new(
                        new_selection,
                        editor_scene.selection.clone(),
                    ));
            }
        }
    }

    fn on_mouse_move(
        &mut self,
        mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        editor_scene: &mut EditorScene,
        engine: &mut Engine,
        frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        if let Selection::Graph(selection) = &editor_scene.selection {
            if self.interacting {
                let scale_delta = self.scale_gizmo.calculate_scale_delta(
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
        editor_scene: &mut EditorScene,
        engine: &mut Engine,
        _settings: &Settings,
    ) {
        if let Selection::Graph(selection) = &editor_scene.selection {
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            if editor_scene.selection.is_empty() || editor_scene.preview_camera.is_some() {
                self.scale_gizmo.set_visible(graph, false);
            } else {
                let scale = calculate_gizmo_distance_scaling(
                    graph,
                    editor_scene.camera_controller.camera,
                    self.scale_gizmo.origin,
                );
                self.scale_gizmo.sync_transform(graph, selection, scale);
                self.scale_gizmo.set_visible(graph, true);
            }
        }
    }

    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut Engine) {
        let graph = &mut engine.scenes[editor_scene.scene].graph;
        self.scale_gizmo.set_visible(graph, false);
    }
}
