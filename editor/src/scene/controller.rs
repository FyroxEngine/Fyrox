use crate::{
    asset::item::AssetItem,
    camera::PickingOptions,
    scene::{
        commands::{
            graph::AddModelCommand, mesh::SetMeshTextureCommand, ChangeSelectionCommand,
            CommandGroup, SceneCommand,
        },
        container::{EditorSceneEntry, PreviewInstance},
        Selection,
    },
    settings::{keys::KeyBindings, Settings},
    world::graph::selection::GraphSelection,
};
use fyrox::{
    core::{
        algebra::{Vector2, Vector3},
        futures::executor::block_on,
        make_relative_path,
        math::{plane::Plane, Rect},
        pool::Handle,
    },
    engine::Engine,
    fxhash::FxHashSet,
    gui::{
        message::{KeyCode, MouseButton},
        UiNode,
    },
    resource::{
        model::{Model, ModelResourceExtension},
        texture::Texture,
    },
    scene::{
        camera::{Camera, Projection},
        node::Node,
    },
};

pub trait SceneController {
    fn on_key_up(&mut self, key: KeyCode, engine: &mut Engine, key_bindings: &KeyBindings) -> bool;

    fn on_key_down(
        &mut self,
        key: KeyCode,
        engine: &mut Engine,
        key_bindings: &KeyBindings,
    ) -> bool;

    fn on_mouse_move(
        &mut self,
        pos: Vector2<f32>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    );

    fn on_mouse_up(
        &mut self,
        button: MouseButton,
        pos: Vector2<f32>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    );

    fn on_mouse_down(
        &mut self,
        button: MouseButton,
        pos: Vector2<f32>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    );

    fn on_mouse_wheel(&mut self, amount: f32, engine: &mut Engine, settings: &Settings);

    fn on_mouse_leave(&mut self, engine: &mut Engine, settings: &Settings);

    fn on_drag_over(
        &mut self,
        handle: Handle<UiNode>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    );

    fn on_drop(
        &mut self,
        handle: Handle<UiNode>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    );
}

impl SceneController for EditorSceneEntry {
    #[must_use]
    fn on_key_up(&mut self, key: KeyCode, engine: &mut Engine, key_bindings: &KeyBindings) -> bool {
        if self
            .editor_scene
            .camera_controller
            .on_key_up(key_bindings, key)
        {
            return true;
        }

        if let Some(interaction_mode) = self
            .current_interaction_mode
            .and_then(|id| self.interaction_modes.map.get_mut(&id))
        {
            if interaction_mode.on_key_up(key, &mut self.editor_scene, engine) {
                return true;
            }
        }

        false
    }

    #[must_use]
    fn on_key_down(
        &mut self,
        key: KeyCode,
        engine: &mut Engine,
        key_bindings: &KeyBindings,
    ) -> bool {
        if self
            .editor_scene
            .camera_controller
            .on_key_down(key_bindings, key)
        {
            return true;
        }

        if let Some(interaction_mode) = self
            .current_interaction_mode
            .and_then(|id| self.interaction_modes.map.get_mut(&id))
        {
            if interaction_mode.on_key_down(key, &self.selection, &mut self.editor_scene, engine) {
                return true;
            }
        }

        false
    }

    fn on_mouse_move(
        &mut self,
        pos: Vector2<f32>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
        let last_pos = *self.last_mouse_pos.get_or_insert(pos);
        let mouse_offset = pos - last_pos;
        self.editor_scene
            .camera_controller
            .on_mouse_move(mouse_offset, &settings.camera);
        let rel_pos = pos - screen_bounds.position;

        if let Some(interaction_mode) = self
            .current_interaction_mode
            .and_then(|id| self.interaction_modes.map.get_mut(&id))
        {
            interaction_mode.on_mouse_move(
                mouse_offset,
                rel_pos,
                &self.selection,
                &mut self.editor_scene,
                engine,
                screen_bounds.size,
                settings,
            );
        }

        self.last_mouse_pos = Some(pos);
    }

    fn on_mouse_up(
        &mut self,
        button: MouseButton,
        pos: Vector2<f32>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
        if button == MouseButton::Left {
            self.click_mouse_pos = None;
            if let Some(interaction_mode) = self
                .current_interaction_mode
                .and_then(|id| self.interaction_modes.map.get_mut(&id))
            {
                let rel_pos = pos - screen_bounds.position;
                interaction_mode.on_left_mouse_button_up(
                    &self.selection,
                    &mut self.editor_scene,
                    engine,
                    rel_pos,
                    screen_bounds.size,
                    settings,
                );
            }
        }

        self.editor_scene
            .camera_controller
            .on_mouse_button_up(button, &mut engine.scenes[self.editor_scene.scene].graph);
    }

    fn on_mouse_down(
        &mut self,
        button: MouseButton,
        pos: Vector2<f32>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
        if button == MouseButton::Left {
            if let Some(interaction_mode) = self
                .current_interaction_mode
                .and_then(|id| self.interaction_modes.map.get_mut(&id))
            {
                let rel_pos = pos - screen_bounds.position;

                self.click_mouse_pos = Some(rel_pos);

                interaction_mode.on_left_mouse_button_down(
                    &self.selection,
                    &mut self.editor_scene,
                    engine,
                    rel_pos,
                    screen_bounds.size,
                    settings,
                );
            }
        }

        self.editor_scene.camera_controller.on_mouse_button_down(
            button,
            engine.user_interface.keyboard_modifiers(),
            &mut engine.scenes[self.editor_scene.scene].graph,
        );
    }

    fn on_mouse_wheel(&mut self, amount: f32, engine: &mut Engine, settings: &Settings) {
        self.editor_scene.camera_controller.on_mouse_wheel(
            amount * settings.camera.zoom_speed,
            &mut engine.scenes[self.editor_scene.scene].graph,
            settings,
        );
    }

    fn on_mouse_leave(&mut self, engine: &mut Engine, _settings: &Settings) {
        if let Some(preview) = self.preview_instance.take() {
            let scene = &mut engine.scenes[self.editor_scene.scene];

            scene.graph.remove_node(preview.instance);
        }
    }

    fn on_drag_over(
        &mut self,
        handle: Handle<UiNode>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
        match self.preview_instance.as_ref() {
            None => {
                if let Some(item) = engine.user_interface.node(handle).cast::<AssetItem>() {
                    // Make sure all resources loaded with relative paths only.
                    // This will make scenes portable.
                    if let Ok(relative_path) = make_relative_path(&item.path) {
                        // No model was loaded yet, do it.
                        if let Some(model) = engine
                            .resource_manager
                            .try_request::<Model>(relative_path)
                            .and_then(|m| block_on(m).ok())
                        {
                            let scene = &mut engine.scenes[self.editor_scene.scene];

                            // Instantiate the model.
                            let instance = model.instantiate(scene);

                            scene
                                .graph
                                .link_nodes(instance, self.editor_scene.scene_content_root);

                            scene.graph[instance]
                                .local_transform_mut()
                                .set_scale(settings.model.instantiation_scale);

                            let nodes = scene
                                .graph
                                .traverse_handle_iter(instance)
                                .collect::<FxHashSet<Handle<Node>>>();

                            self.preview_instance = Some(PreviewInstance { instance, nodes });
                        }
                    }
                }
            }
            Some(preview) => {
                let frame_size = screen_bounds.size;
                let cursor_pos = engine.user_interface.cursor_position();
                let rel_pos = cursor_pos - screen_bounds.position;
                let graph = &mut engine.scenes[self.editor_scene.scene].graph;

                let position = if let Some(result) =
                    self.editor_scene.camera_controller.pick(PickingOptions {
                        cursor_pos: rel_pos,
                        graph,
                        editor_objects_root: self.editor_scene.editor_objects_root,
                        scene_content_root: self.editor_scene.scene_content_root,
                        screen_size: frame_size,
                        editor_only: false,
                        filter: |handle, _| !preview.nodes.contains(&handle),
                        ignore_back_faces: settings.selection.ignore_back_faces,
                        // We need info only about closest intersection.
                        use_picking_loop: false,
                        only_meshes: false,
                    }) {
                    Some(result.position)
                } else {
                    // In case of empty space, check intersection with oXZ plane (3D) or oXY (2D).
                    let camera = graph[self.editor_scene.camera_controller.camera]
                        .query_component_ref::<Camera>()
                        .unwrap();

                    let normal = match camera.projection() {
                        Projection::Perspective(_) => Vector3::new(0.0, 1.0, 0.0),
                        Projection::Orthographic(_) => Vector3::new(0.0, 0.0, 1.0),
                    };

                    let plane = Plane::from_normal_and_point(&normal, &Default::default())
                        .unwrap_or_default();

                    let ray = camera.make_ray(rel_pos, frame_size);

                    ray.plane_intersection_point(&plane)
                };

                if let Some(position) = position {
                    graph[preview.instance].local_transform_mut().set_position(
                        settings
                            .move_mode_settings
                            .try_snap_vector_to_grid(position),
                    );
                }
            }
        }
    }

    fn on_drop(
        &mut self,
        handle: Handle<UiNode>,
        screen_bounds: Rect<f32>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
        if handle.is_none() {
            return;
        }

        let frame_size = screen_bounds.size;

        if let Some(item) = engine.user_interface.node(handle).cast::<AssetItem>() {
            // Make sure all resources loaded with relative paths only.
            // This will make scenes portable.
            if let Ok(relative_path) = make_relative_path(&item.path) {
                if let Some(preview) = self.preview_instance.take() {
                    let scene = &mut engine.scenes[self.editor_scene.scene];

                    // Immediately after extract if from the scene to subgraph. This is required to not violate
                    // the rule of one place of execution, only commands allowed to modify the scene.
                    let sub_graph = scene.graph.take_reserve_sub_graph(preview.instance);

                    let group = vec![
                        SceneCommand::new(AddModelCommand::new(sub_graph)),
                        // We also want to select newly instantiated model.
                        SceneCommand::new(ChangeSelectionCommand::new(
                            Selection::Graph(GraphSelection::single_or_empty(preview.instance)),
                            self.selection.clone(),
                        )),
                    ];

                    self.sender.do_scene_command(CommandGroup::from(group));
                } else if let Some(tex) = engine
                    .resource_manager
                    .try_request::<Texture>(relative_path)
                    .and_then(|t| block_on(t).ok())
                {
                    let cursor_pos = engine.user_interface.cursor_position();
                    let rel_pos = cursor_pos - screen_bounds.position;
                    let graph = &engine.scenes[self.editor_scene.scene].graph;
                    if let Some(result) = self.editor_scene.camera_controller.pick(PickingOptions {
                        cursor_pos: rel_pos,
                        graph,
                        editor_objects_root: self.editor_scene.editor_objects_root,
                        scene_content_root: self.editor_scene.scene_content_root,
                        screen_size: frame_size,
                        editor_only: false,
                        filter: |_, _| true,
                        ignore_back_faces: settings.selection.ignore_back_faces,
                        use_picking_loop: true,
                        only_meshes: false,
                    }) {
                        let texture = tex.clone();
                        let mut texture = texture.state();
                        if texture.data().is_some() {
                            let node =
                                &mut engine.scenes[self.editor_scene.scene].graph[result.node];

                            if node.is_mesh() {
                                self.sender
                                    .do_scene_command(SetMeshTextureCommand::new(result.node, tex));
                            }
                        }
                    }
                }
            }
        }
    }
}
