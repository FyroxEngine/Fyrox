use crate::{
    camera::{CameraController, PickingOptions},
    command::{Command, CommandGroup},
    fyrox::{
        core::{
            algebra::{Matrix4, Point3, Vector2, Vector3},
            math::plane::Plane,
            pool::Handle,
            uuid::{uuid, Uuid},
            TypeUuidProvider,
        },
        fxhash::FxHashSet,
        graph::SceneGraph,
        gui::{BuildContext, UiNode},
        scene::{
            camera::{Camera, Projection},
            graph::Graph,
            node::Node,
            Scene,
        },
    },
    interaction::{
        calculate_gizmo_distance_scaling, gizmo::move_gizmo::MoveGizmo,
        make_interaction_mode_button, plane::PlaneKind, InteractionMode,
    },
    message::MessageSender,
    scene::{
        commands::{graph::MoveNodeCommand, ChangeSelectionCommand},
        controller::SceneController,
        GameScene, Selection,
    },
    settings::Settings,
    world::graph::selection::GraphSelection,
    Engine, Message,
};

struct Entry {
    node: Handle<Node>,
    initial_offset_gizmo_space: Vector3<f32>,
    initial_local_position: Vector3<f32>,
    initial_parent_inv_global_transform: Matrix4<f32>,
    new_local_position: Vector3<f32>,
}

struct MoveContext {
    plane: Option<Plane>,
    objects: Vec<Entry>,
    plane_kind: PlaneKind,
    gizmo_inv_transform: Matrix4<f32>,
    gizmo_local_transform: Matrix4<f32>,
}

impl MoveContext {
    pub fn from_filler<F>(
        scene: &Scene,
        move_gizmo: &MoveGizmo,
        camera_controller: &CameraController,
        plane_kind: PlaneKind,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
        mut fill: F,
    ) -> Self
    where
        F: FnMut(Vector3<f32>, Matrix4<f32>, Vector3<f32>) -> Vec<Entry>,
    {
        let graph = &scene.graph;

        let gizmo_origin = &graph[move_gizmo.origin];

        let gizmo_inv_transform = gizmo_origin
            .global_transform()
            .try_inverse()
            .unwrap_or_default();

        let look_direction =
            gizmo_inv_transform.transform_vector(&graph[camera_controller.camera].look_vector());

        let plane = plane_kind.make_plane_from_view(look_direction);

        let plane_point = if let Some(plane) = plane {
            plane_kind.project_point(
                camera_controller
                    .pick_on_plane(plane, graph, mouse_pos, frame_size, gizmo_inv_transform)
                    .unwrap_or_default(),
            )
        } else {
            Default::default()
        };

        Self {
            plane,
            objects: fill(
                plane_point,
                gizmo_inv_transform,
                gizmo_origin.global_position(),
            ),
            gizmo_local_transform: gizmo_origin.local_transform().matrix(),
            gizmo_inv_transform,
            plane_kind,
        }
    }

    pub fn from_graph_selection(
        selection: &GraphSelection,
        scene: &Scene,
        move_gizmo: &MoveGizmo,
        camera_controller: &CameraController,
        plane_kind: PlaneKind,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
    ) -> Self {
        Self::from_filler(
            scene,
            move_gizmo,
            camera_controller,
            plane_kind,
            mouse_pos,
            frame_size,
            |plane_point, gizmo_inv_transform, gizmo_origin| {
                let graph = &scene.graph;
                selection
                    .root_nodes(graph)
                    .iter()
                    .map(|&node_handle| {
                        let node = &graph[node_handle];
                        Entry {
                            node: node_handle,
                            initial_offset_gizmo_space: gizmo_inv_transform
                                .transform_point(&Point3::from(node.global_position()))
                                .coords
                                - plane_point
                                - gizmo_inv_transform
                                    .transform_vector(&(node.global_position() - gizmo_origin)),
                            new_local_position: **node.local_transform().position(),
                            initial_local_position: **node.local_transform().position(),
                            initial_parent_inv_global_transform: if node.parent().is_some() {
                                graph[node.parent()]
                                    .global_transform()
                                    .try_inverse()
                                    .unwrap_or_default()
                            } else {
                                Matrix4::identity()
                            },
                        }
                    })
                    .collect()
            },
        )
    }

    pub fn update(
        &mut self,
        graph: &Graph,
        game_scene: &mut GameScene,
        settings: &Settings,
        mouse_position: Vector2<f32>,
        frame_size: Vector2<f32>,
    ) {
        match self.plane_kind {
            PlaneKind::SMART => {
                self.update_smart_move(graph, game_scene, settings, mouse_position, frame_size);
            }
            _ => self.update_plane_move(
                graph,
                &game_scene.camera_controller,
                settings,
                mouse_position,
                frame_size,
            ),
        }
    }

    fn update_smart_move(
        &mut self,
        graph: &Graph,
        game_scene: &mut GameScene,
        settings: &Settings,
        mouse_position: Vector2<f32>,
        frame_size: Vector2<f32>,
    ) {
        let preview_nodes = self
            .objects
            .iter()
            .map(|f| f.node)
            .flat_map(|node| graph.traverse_handle_iter(node))
            .collect::<FxHashSet<Handle<Node>>>();

        let new_position = if let Some(result) = game_scene.camera_controller.pick(
            graph,
            PickingOptions {
                cursor_pos: mouse_position,
                editor_only: false,
                filter: Some(&mut |handle, _| !preview_nodes.contains(&handle)),
                ignore_back_faces: settings.selection.ignore_back_faces,
                // We need info only about closest intersection.
                use_picking_loop: false,
                only_meshes: false,
            },
        ) {
            Some(result.position)
        } else {
            // In case of empty space, check intersection with oXZ plane (3D) or oXY (2D).
            if let Some(camera) = graph[game_scene.camera_controller.camera].cast::<Camera>() {
                let normal = match camera.projection() {
                    Projection::Perspective(_) => Vector3::new(0.0, 1.0, 0.0),
                    Projection::Orthographic(_) => Vector3::new(0.0, 0.0, 1.0),
                };

                let plane =
                    Plane::from_normal_and_point(&normal, &Default::default()).unwrap_or_default();

                let ray = camera.make_ray(mouse_position, frame_size);

                ray.plane_intersection_point(&plane)
            } else {
                None
            }
        };

        if let Some(new_position) = new_position {
            for entry in self.objects.iter_mut() {
                let n2 = entry
                    .initial_parent_inv_global_transform
                    .transform_point(&(Point3::from(new_position)));
                entry.new_local_position = Vector3::new(n2.x, n2.y, n2.z);
            }
        }
    }

    pub fn update_plane_move(
        &mut self,
        graph: &Graph,
        camera_controller: &CameraController,
        settings: &Settings,
        mouse_position: Vector2<f32>,
        frame_size: Vector2<f32>,
    ) {
        if let Some(picked_position_gizmo_space) = camera_controller
            .pick_on_plane(
                self.plane.unwrap(),
                graph,
                mouse_position,
                frame_size,
                self.gizmo_inv_transform,
            )
            .map(|p| self.plane_kind.project_point(p))
        {
            for entry in self.objects.iter_mut() {
                entry.new_local_position = settings.move_mode_settings.try_snap_vector_to_grid(
                    entry.initial_local_position
                        + entry.initial_parent_inv_global_transform.transform_vector(
                            &self.gizmo_local_transform.transform_vector(
                                &(picked_position_gizmo_space + entry.initial_offset_gizmo_space),
                            ),
                        ),
                );
            }
        }
    }
}

pub struct MoveInteractionMode {
    move_context: Option<MoveContext>,
    move_gizmo: MoveGizmo,
    message_sender: MessageSender,
}

impl MoveInteractionMode {
    pub fn new(game_scene: &GameScene, engine: &mut Engine, message_sender: MessageSender) -> Self {
        Self {
            move_context: None,
            move_gizmo: MoveGizmo::new(game_scene, engine),
            message_sender,
        }
    }
}

impl TypeUuidProvider for MoveInteractionMode {
    fn type_uuid() -> Uuid {
        uuid!("067c67e2-865b-4112-8d60-133ee1e1883a")
    }
}

impl InteractionMode for MoveInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        let scene = &mut engine.scenes[game_scene.scene];
        let graph = &mut scene.graph;

        if let Some(result) = game_scene.camera_controller.pick(
            graph,
            PickingOptions {
                cursor_pos: mouse_pos,
                editor_only: true,
                filter: Some(&mut |handle, _| handle != self.move_gizmo.origin),
                ignore_back_faces: false,
                use_picking_loop: true,
                only_meshes: false,
            },
        ) {
            if let Some(plane_kind) = self.move_gizmo.handle_pick(result.node, graph) {
                if let Some(selection) = editor_selection.as_graph() {
                    self.move_context = Some(MoveContext::from_graph_selection(
                        selection,
                        scene,
                        &self.move_gizmo,
                        &game_scene.camera_controller,
                        plane_kind,
                        mouse_pos,
                        frame_size,
                    ));
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
        _frame_size: Vector2<f32>,
        settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        let scene = &mut engine.scenes[game_scene.scene];

        self.move_gizmo.reset_state(&mut scene.graph);

        if let Some(move_context) = self.move_context.take() {
            let mut changed = false;

            for initial_state in move_context.objects.iter() {
                if **scene.graph[initial_state.node].local_transform().position()
                    != initial_state.initial_local_position
                {
                    changed = true;
                    break;
                }
            }

            if changed {
                let commands = CommandGroup::from(
                    move_context
                        .objects
                        .iter()
                        .map(|initial_state| {
                            Command::new(MoveNodeCommand::new(
                                initial_state.node,
                                initial_state.initial_local_position,
                                **scene.graph[initial_state.node].local_transform().position(),
                            ))
                        })
                        .collect::<Vec<_>>(),
                );

                // Commit changes.
                self.message_sender
                    .send(Message::DoCommand(Command::new(commands)));
            }
        } else {
            let new_selection = game_scene
                .camera_controller
                .pick(
                    &scene.graph,
                    PickingOptions {
                        cursor_pos: mouse_pos,
                        editor_only: false,
                        filter: None,
                        ignore_back_faces: settings.selection.ignore_back_faces,
                        use_picking_loop: true,
                        only_meshes: false,
                    },
                )
                .map(|result| {
                    if let (Some(selection), true) = (
                        editor_selection.as_graph(),
                        engine
                            .user_interfaces
                            .first_mut()
                            .keyboard_modifiers()
                            .control,
                    ) {
                        let mut selection = selection.clone();
                        selection.insert_or_exclude(result.node);
                        Selection::new(selection)
                    } else {
                        Selection::new(GraphSelection::single_or_empty(result.node))
                    }
                })
                .unwrap_or_else(|| Selection::new(GraphSelection::default()));

            if &new_selection != editor_selection {
                self.message_sender
                    .do_command(ChangeSelectionCommand::new(new_selection));
            }
        }
    }

    fn on_mouse_move(
        &mut self,
        _mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        _editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        frame_size: Vector2<f32>,
        settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        let scene = &mut engine.scenes[game_scene.scene];

        if let Some(move_context) = self.move_context.as_mut() {
            let graph = &mut scene.graph;

            move_context.update(graph, game_scene, settings, mouse_position, frame_size);

            for entry in move_context.objects.iter() {
                scene.graph[entry.node]
                    .local_transform_mut()
                    .set_position(entry.new_local_position);
            }
        } else {
            let picked = game_scene
                .camera_controller
                .pick(
                    &scene.graph,
                    PickingOptions {
                        cursor_pos: mouse_position,
                        editor_only: true,
                        filter: None,
                        ignore_back_faces: false,
                        use_picking_loop: false,
                        only_meshes: false,
                    },
                )
                .map(|r| r.node)
                .unwrap_or_default();
            self.move_gizmo.handle_pick(picked, &mut scene.graph);
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

        let scene = &mut engine.scenes[game_scene.scene];
        let graph = &mut scene.graph;
        if editor_selection.is_empty() || game_scene.preview_camera.is_some() {
            self.move_gizmo.set_visible(graph, false);
        } else {
            let scale = calculate_gizmo_distance_scaling(
                graph,
                game_scene.camera_controller.camera,
                self.move_gizmo.origin,
            );
            self.move_gizmo.set_visible(graph, true);
            self.move_gizmo
                .sync_transform(scene, editor_selection, scale);
        }
    }

    fn deactivate(&mut self, controller: &dyn SceneController, engine: &mut Engine) {
        let Some(game_scene) = controller.downcast_ref::<GameScene>() else {
            return;
        };

        let graph = &mut engine.scenes[game_scene.scene].graph;
        self.move_gizmo.set_visible(graph, false);
    }

    fn make_button(&mut self, ctx: &mut BuildContext, selected: bool) -> Handle<UiNode> {
        let move_mode_tooltip =
            "Move Object(s) - Shortcut: [2]\n\nMovement interaction mode allows you to move selected \
        objects. Keep in mind that movement always works in local coordinates!\n\n\
        This also allows you to select an object or add an object to current selection using Ctrl+Click";

        make_interaction_mode_button(
            ctx,
            include_bytes!("../../resources/move_arrow.png"),
            move_mode_tooltip,
            selected,
        )
    }

    fn uuid(&self) -> Uuid {
        Self::type_uuid()
    }
}
