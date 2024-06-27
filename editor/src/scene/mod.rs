use crate::{
    absm::{
        command::fetch_machine,
        selection::{AbsmSelection, SelectedEntity},
    },
    animation::{self, command::fetch_animations_container, selection::AnimationSelection},
    asset::item::AssetItem,
    audio::AudioBusSelection,
    camera::{CameraController, PickingOptions},
    command::{make_command, Command, CommandGroup, CommandStack},
    fyrox::{
        asset::manager::ResourceManager,
        core::{
            algebra::{Matrix4, Vector2, Vector3},
            color::Color,
            futures::executor::block_on,
            log::Log,
            make_relative_path,
            math::{aabb::AxisAlignedBoundingBox, plane::Plane, Rect},
            pool::{ErasedHandle, Handle},
            reflect::Reflect,
            visitor::Visitor,
        },
        engine::{Engine, SerializationContext},
        fxhash::FxHashSet,
        graph::{BaseSceneGraph, SceneGraph},
        gui::{
            inspector::PropertyChanged,
            message::{KeyCode, MessageDirection, MouseButton},
            UiNode,
        },
        material::{
            shader::ShaderResource, shader::ShaderResourceExtension, Material, MaterialResource,
        },
        resource::{
            model::{Model, ModelResourceExtension},
            texture::{Texture, TextureKind, TextureResource, TextureResourceExtension},
        },
        scene::{
            animation::{absm::AnimationBlendingStateMachine, AnimationPlayer},
            base::BaseBuilder,
            camera::{Camera, Projection},
            debug::{Line, SceneDrawingContext},
            graph::{Graph, GraphUpdateSwitches},
            light::{point::PointLight, spot::SpotLight},
            mesh::RenderPath,
            mesh::{
                surface::{SurfaceBuilder, SurfaceData, SurfaceResource},
                Mesh, MeshBuilder,
            },
            navmesh::NavigationalMesh,
            node::Node,
            pivot::PivotBuilder,
            sound::AudioBus,
            terrain::Terrain,
            Scene, SceneContainer,
        },
    },
    highlight::HighlightRenderPass,
    inspector::{
        editors::handle::HandlePropertyEditorMessage,
        handlers::node::SceneNodePropertyChangedHandler,
    },
    interaction::navmesh::selection::NavmeshSelection,
    message::MessageSender,
    scene::{
        clipboard::Clipboard,
        commands::{
            graph::AddModelCommand, mesh::SetMeshTextureCommand, ChangeSelectionCommand,
            GameSceneContext,
        },
        controller::SceneController,
        selector::HierarchyNode,
    },
    settings::{keys::KeyBindings, SettingsMessage},
    ui_scene::selection::UiSelection,
    world::graph::selection::GraphSelection,
    Message, Settings,
};
use fyrox::asset::untyped::ResourceKind;
use std::{
    any::Any,
    cell::RefCell,
    fmt::Debug,
    fs::File,
    io::Write,
    path::Path,
    rc::Rc,
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
};

pub mod clipboard;
pub mod dialog;
pub mod property;
pub mod selector;
pub mod settings;

#[macro_use]
pub mod commands;
pub mod container;
pub mod controller;

pub struct PreviewInstance {
    pub instance: Handle<Node>,
    pub nodes: FxHashSet<Handle<Node>>,
}

pub struct GameScene {
    pub scene: Handle<Scene>,
    // Handle to a root for all editor nodes.
    pub editor_objects_root: Handle<Node>,
    pub scene_content_root: Handle<Node>,
    pub clipboard: Clipboard,
    pub camera_controller: CameraController,
    pub preview_camera: Handle<Node>,
    pub graph_switches: GraphUpdateSwitches,
    pub preview_instance: Option<PreviewInstance>,
    pub sender: MessageSender,
    pub camera_state: Vec<(Handle<Node>, bool)>,
    pub node_property_changed_handler: SceneNodePropertyChangedHandler,
    pub highlighter: Option<Rc<RefCell<HighlightRenderPass>>>,
    pub resource_manager: ResourceManager,
    pub serialization_context: Arc<SerializationContext>,
    pub grid: Handle<Node>,
    pub settings_receiver: Receiver<SettingsMessage>,
}

lazy_static! {
    static ref GRID_SHADER: ShaderResource = {
        ShaderResource::from_str(
            include_str!("../../resources/shaders/grid.shader",),
            Default::default(),
        )
        .unwrap()
    };
}

fn make_grid_material() -> MaterialResource {
    let material = Material::from_shader(GRID_SHADER.clone(), None);
    MaterialResource::new_ok(Default::default(), material)
}

impl GameScene {
    pub fn from_native_scene(
        mut scene: Scene,
        engine: &mut Engine,
        path: Option<&Path>,
        settings: &mut Settings,
        sender: MessageSender,
        highlighter: Option<Rc<RefCell<HighlightRenderPass>>>,
    ) -> Self {
        scene.rendering_options.render_target = Some(TextureResource::new_render_target(0, 0));

        let scene_content_root = scene.graph.get_root();

        scene
            .graph
            .change_root_node(PivotBuilder::new(BaseBuilder::new()).build_node());

        let editor_objects_root = PivotBuilder::new(BaseBuilder::new()).build(&mut scene.graph);

        let grid = MeshBuilder::new(
            BaseBuilder::new()
                .with_frustum_culling(false)
                .with_visibility(settings.graphics.draw_grid),
        )
        .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_ok(
            ResourceKind::Embedded,
            SurfaceData::make_quad(&Matrix4::new_scaling(2.0)),
        ))
        .with_material(make_grid_material())
        .build()])
        .with_render_path(RenderPath::Forward)
        .build(&mut scene.graph);
        scene.graph.link_nodes(grid, editor_objects_root);

        let (settings_sender, settings_receiver) = mpsc::channel();
        settings.subscribers.push(settings_sender);

        let camera_controller = CameraController::new(
            &mut scene.graph,
            editor_objects_root,
            path.as_ref()
                .and_then(|p| settings.scene_settings.get(*p).map(|s| &s.camera_settings)),
            grid,
            editor_objects_root,
            scene_content_root,
        );

        // Freeze physics simulation in while editing scene by setting time step to zero.
        scene.graph.physics.integration_parameters.dt = Some(0.0);
        scene.graph.physics2d.integration_parameters.dt = Some(0.0);

        GameScene {
            editor_objects_root,
            scene_content_root,
            camera_controller,
            preview_instance: None,
            scene: engine.scenes.add(scene),
            clipboard: Default::default(),
            preview_camera: Default::default(),
            graph_switches: GraphUpdateSwitches {
                physics2d: true,
                physics: true,
                // Prevent engine to update lifetime of the nodes and to delete "dead" nodes. Otherwise
                // the editor will crash if some node is "dead".
                delete_dead_nodes: false,
                // Update only editor's camera.
                node_overrides: Some(Default::default()),
                paused: false,
            },
            sender,
            camera_state: Default::default(),
            node_property_changed_handler: SceneNodePropertyChangedHandler,
            highlighter,
            resource_manager: engine.resource_manager.clone(),
            serialization_context: engine.serialization_context.clone(),
            grid,
            settings_receiver,
        }
    }

    pub fn make_purified_scene(&self, engine: &mut Engine) -> Scene {
        let scene = &mut engine.scenes[self.scene];

        let editor_root = self.editor_objects_root;
        let (pure_scene, _) = scene.clone(
            self.scene_content_root,
            &mut |node, _| node != editor_root,
            &mut |_, _| {},
            &mut |_, _, _| {},
        );

        pure_scene
    }

    pub fn save(
        &mut self,
        path: &Path,
        settings: &Settings,
        engine: &mut Engine,
    ) -> Result<String, String> {
        let mut pure_scene = self.make_purified_scene(engine);

        let mut visitor = Visitor::new();
        pure_scene.save("Scene", &mut visitor).unwrap();

        if let Err(e) = visitor.save_binary(path) {
            Err(format!("Failed to save scene! Reason: {}", e))
        } else {
            if settings.debugging.save_scene_in_text_form {
                let text = visitor.save_text();
                let mut path = path.to_path_buf();
                path.set_extension("txt");
                if let Ok(mut file) = File::create(path) {
                    Log::verify(file.write_all(text.as_bytes()));
                }
            }

            Ok(format!("Scene {} was successfully saved!", path.display()))
        }
    }

    pub fn draw_auxiliary_geometry(
        &mut self,
        editor_selection: &Selection,
        engine: &mut Engine,
        settings: &Settings,
    ) {
        let debug_settings = &settings.debugging;
        let scene = &mut engine.scenes[self.scene];

        scene.drawing_context.clear_lines();

        if let Some(selection) = editor_selection.as_graph() {
            for &node in selection.nodes() {
                if let Some(node) = scene.graph.try_get(node) {
                    scene.drawing_context.draw_oob(
                        &node.local_bounding_box(),
                        node.global_transform(),
                        Color::GREEN,
                    );
                }
            }
        }

        if debug_settings.show_physics {
            scene.graph.physics.draw(&mut scene.drawing_context);
            scene.graph.physics2d.draw(&mut scene.drawing_context);
        }

        fn draw_recursively(
            node: Handle<Node>,
            graph: &Graph,
            ctx: &mut SceneDrawingContext,
            editor_selection: &Selection,
            game_scene: &GameScene,
            settings: &Settings,
        ) {
            // Ignore editor nodes.
            if node == game_scene.editor_objects_root {
                return;
            }

            let node = &graph[node];

            if settings.debugging.show_bounds {
                ctx.draw_oob(
                    &AxisAlignedBoundingBox::unit(),
                    node.global_transform(),
                    Color::opaque(255, 127, 39),
                );
            }

            if node.cast::<Mesh>().is_some() {
                if settings.debugging.show_tbn {
                    node.debug_draw(ctx);
                }
            } else if node.query_component_ref::<Camera>().is_some() {
                if settings.debugging.show_camera_bounds
                    && game_scene.preview_camera == Handle::NONE
                {
                    node.debug_draw(ctx);
                }
            } else if node.query_component_ref::<PointLight>().is_some()
                || node.query_component_ref::<SpotLight>().is_some()
            {
                if settings.debugging.show_light_bounds {
                    node.debug_draw(ctx);
                }
            } else if node.query_component_ref::<Terrain>().is_some() {
                if settings.debugging.show_terrains {
                    node.debug_draw(ctx);
                }
            } else if let Some(navmesh) = node.query_component_ref::<NavigationalMesh>() {
                if settings.navmesh.draw_all {
                    let selection = editor_selection.as_navmesh();

                    for (index, vertex) in navmesh.navmesh_ref().vertices().iter().enumerate() {
                        ctx.draw_sphere(
                            *vertex,
                            10,
                            10,
                            settings.navmesh.vertex_radius,
                            selection.map_or(Color::GREEN, |s| {
                                if s.unique_vertices().contains(&index) {
                                    Color::RED
                                } else {
                                    Color::GREEN
                                }
                            }),
                        );
                    }

                    for triangle in navmesh.navmesh_ref().triangles().iter() {
                        for edge in &triangle.edges() {
                            ctx.add_line(Line {
                                begin: navmesh.navmesh_ref().vertices()[edge.a as usize],
                                end: navmesh.navmesh_ref().vertices()[edge.b as usize],
                                color: selection.map_or(Color::GREEN, |s| {
                                    if s.contains_edge(*edge) {
                                        Color::RED
                                    } else {
                                        Color::GREEN
                                    }
                                }),
                            });
                        }
                    }
                }
            } else {
                node.debug_draw(ctx);
            }

            for &child in node.children() {
                draw_recursively(child, graph, ctx, editor_selection, game_scene, settings)
            }
        }

        // Draw pivots.
        draw_recursively(
            self.scene_content_root,
            &scene.graph,
            &mut scene.drawing_context,
            editor_selection,
            self,
            settings,
        );
    }

    /// Checks whether the current graph selection has references to the nodes outside of the selection.
    pub fn is_current_selection_has_external_refs(
        &self,
        editor_selection: &Selection,
        graph: &Graph,
    ) -> bool {
        if let Some(selection) = editor_selection.as_graph() {
            for node in selection.nodes() {
                for descendant in graph.traverse_handle_iter(*node) {
                    for reference in graph.find_references_to(descendant) {
                        if !selection.contains(reference) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn try_save_selection_as_prefab(&self, path: &Path, selection: &Selection, engine: &Engine) {
        let source_scene = &engine.scenes[self.scene];
        let mut dest_scene = Scene::new();
        if let Some(graph_selection) = selection.as_graph() {
            for root_node in graph_selection.root_nodes(&source_scene.graph) {
                source_scene.graph.copy_node(
                    root_node,
                    &mut dest_scene.graph,
                    &mut |_, _| true,
                    &mut |_, _| {},
                    &mut |_, _, _| {},
                );
            }

            let mut visitor = Visitor::new();
            match dest_scene.save("Scene", &mut visitor) {
                Err(e) => Log::err(format!(
                    "Failed to save selection as prefab! Reason: {:?}",
                    e
                )),
                Ok(_) => {
                    if let Err(e) = visitor.save_binary(path) {
                        Log::err(format!(
                            "Failed to save selection as prefab! Reason: {:?}",
                            e
                        ));
                    } else {
                        Log::info(format!(
                            "Selection was successfully saved as prefab to {:?}!",
                            path
                        ))
                    }
                }
            }
        } else {
            Log::warn("Unable to selection to prefab, because selection is not scene selection!");
        }
    }

    fn select_object(&mut self, handle: ErasedHandle, engine: &Engine) {
        if engine.scenes[self.scene]
            .graph
            .is_valid_handle(handle.into())
        {
            self.sender
                .do_command(ChangeSelectionCommand::new(Selection::new(
                    GraphSelection::single_or_empty(handle.into()),
                )))
        }
    }
}

impl SceneController for GameScene {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    #[must_use]
    fn on_key_up(
        &mut self,
        key: KeyCode,
        _engine: &mut Engine,
        key_bindings: &KeyBindings,
    ) -> bool {
        if self.camera_controller.on_key_up(key_bindings, key) {
            return true;
        }

        false
    }

    #[must_use]
    fn on_key_down(
        &mut self,
        key: KeyCode,
        _engine: &mut Engine,
        key_bindings: &KeyBindings,
    ) -> bool {
        if self.camera_controller.on_key_down(key_bindings, key) {
            return true;
        }

        false
    }

    fn on_mouse_move(
        &mut self,
        _pos: Vector2<f32>,
        offset: Vector2<f32>,
        _screen_bounds: Rect<f32>,
        _engine: &mut Engine,
        settings: &Settings,
    ) {
        self.camera_controller
            .on_mouse_move(offset, &settings.camera);
    }

    fn on_mouse_up(
        &mut self,
        button: MouseButton,
        _pos: Vector2<f32>,
        _screen_bounds: Rect<f32>,
        engine: &mut Engine,
        _settings: &Settings,
    ) {
        self.camera_controller
            .on_mouse_button_up(button, &mut engine.scenes[self.scene].graph);
    }

    fn on_mouse_down(
        &mut self,
        button: MouseButton,
        _pos: Vector2<f32>,
        _screen_bounds: Rect<f32>,
        engine: &mut Engine,
        _settings: &Settings,
    ) {
        self.camera_controller.on_mouse_button_down(
            button,
            engine.user_interfaces.first_mut().keyboard_modifiers(),
            &mut engine.scenes[self.scene].graph,
        );
    }

    fn on_mouse_wheel(&mut self, amount: f32, engine: &mut Engine, settings: &Settings) {
        self.camera_controller.on_mouse_wheel(
            amount * settings.camera.zoom_speed,
            &mut engine.scenes[self.scene].graph,
            settings,
        );
    }

    fn on_mouse_leave(&mut self, engine: &mut Engine, _settings: &Settings) {
        if let Some(preview) = self.preview_instance.take() {
            let scene = &mut engine.scenes[self.scene];

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
                if let Some(item) = engine
                    .user_interfaces
                    .first_mut()
                    .node(handle)
                    .cast::<AssetItem>()
                {
                    // Make sure all resources loaded with relative paths only.
                    // This will make scenes portable.
                    if let Ok(relative_path) = make_relative_path(&item.path) {
                        // No model was loaded yet, do it.
                        if let Some(model) = engine
                            .resource_manager
                            .try_request::<Model>(relative_path)
                            .and_then(|m| block_on(m).ok())
                        {
                            let scene = &mut engine.scenes[self.scene];

                            // Instantiate the model.
                            let instance = model.instantiate(scene);

                            scene.graph.link_nodes(instance, self.scene_content_root);

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
                let cursor_pos = engine.user_interfaces.first_mut().cursor_position();
                let rel_pos = cursor_pos - screen_bounds.position;
                let graph = &mut engine.scenes[self.scene].graph;

                let position = if let Some(result) = self.camera_controller.pick(
                    graph,
                    PickingOptions {
                        cursor_pos: rel_pos,
                        editor_only: false,
                        filter: Some(&mut |handle, _| !preview.nodes.contains(&handle)),
                        ignore_back_faces: settings.selection.ignore_back_faces,
                        // We need info only about closest intersection.
                        use_picking_loop: false,
                        only_meshes: false,
                    },
                ) {
                    Some(result.position)
                } else {
                    // In case of empty space, check intersection with oXZ plane (3D) or oXY (2D).
                    let camera = graph[self.camera_controller.camera]
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

        if let Some(item) = engine
            .user_interfaces
            .first_mut()
            .node(handle)
            .cast::<AssetItem>()
        {
            // Make sure all resources loaded with relative paths only.
            // This will make scenes portable.
            if let Ok(relative_path) = make_relative_path(&item.path) {
                if let Some(preview) = self.preview_instance.take() {
                    let scene = &mut engine.scenes[self.scene];

                    // Immediately after extract if from the scene to subgraph. This is required to not violate
                    // the rule of one place of execution, only commands allowed to modify the scene.
                    let sub_graph = scene.graph.take_reserve_sub_graph(preview.instance);

                    let group = vec![
                        Command::new(AddModelCommand::new(sub_graph)),
                        // We also want to select newly instantiated model.
                        Command::new(ChangeSelectionCommand::new(Selection::new(
                            GraphSelection::single_or_empty(preview.instance),
                        ))),
                    ];

                    self.sender.do_command(CommandGroup::from(group));
                } else if let Some(tex) = engine
                    .resource_manager
                    .try_request::<Texture>(relative_path)
                    .and_then(|t| block_on(t).ok())
                {
                    let cursor_pos = engine.user_interfaces.first_mut().cursor_position();
                    let rel_pos = cursor_pos - screen_bounds.position;
                    let graph = &engine.scenes[self.scene].graph;
                    if let Some(result) = self.camera_controller.pick(
                        graph,
                        PickingOptions {
                            cursor_pos: rel_pos,
                            editor_only: false,
                            filter: None,
                            ignore_back_faces: settings.selection.ignore_back_faces,
                            use_picking_loop: true,
                            only_meshes: false,
                        },
                    ) {
                        let texture = tex.clone();
                        let mut texture = texture.state();
                        if texture.data().is_some() {
                            let node = &mut engine.scenes[self.scene].graph[result.node];

                            if node.is_mesh() {
                                self.sender
                                    .do_command(SetMeshTextureCommand::new(result.node, tex));
                            }
                        }
                    }
                }
            }
        }
    }

    fn render_target(&self, engine: &Engine) -> Option<TextureResource> {
        engine.scenes[self.scene]
            .rendering_options
            .render_target
            .clone()
    }

    fn extension(&self) -> &str {
        "rgs"
    }

    fn save(
        &mut self,
        path: &Path,
        settings: &Settings,
        engine: &mut Engine,
    ) -> Result<String, String> {
        self.save(path, settings, engine)
    }

    fn do_command(
        &mut self,
        command_stack: &mut CommandStack,
        command: Command,
        selection: &mut Selection,
        engine: &mut Engine,
    ) {
        GameSceneContext::exec(
            selection,
            &mut engine.scenes[self.scene],
            &mut self.scene_content_root,
            &mut self.clipboard,
            self.sender.clone(),
            engine.resource_manager.clone(),
            engine.serialization_context.clone(),
            |ctx| command_stack.do_command(command, ctx),
        )
    }

    fn undo(
        &mut self,
        command_stack: &mut CommandStack,
        selection: &mut Selection,
        engine: &mut Engine,
    ) {
        GameSceneContext::exec(
            selection,
            &mut engine.scenes[self.scene],
            &mut self.scene_content_root,
            &mut self.clipboard,
            self.sender.clone(),
            engine.resource_manager.clone(),
            engine.serialization_context.clone(),
            |ctx| command_stack.undo(ctx),
        );
    }

    fn redo(
        &mut self,
        command_stack: &mut CommandStack,
        selection: &mut Selection,
        engine: &mut Engine,
    ) {
        GameSceneContext::exec(
            selection,
            &mut engine.scenes[self.scene],
            &mut self.scene_content_root,
            &mut self.clipboard,
            self.sender.clone(),
            engine.resource_manager.clone(),
            engine.serialization_context.clone(),
            |ctx| command_stack.redo(ctx),
        );
    }

    fn clear_command_stack(
        &mut self,
        command_stack: &mut CommandStack,
        selection: &mut Selection,
        scenes: &mut SceneContainer,
    ) {
        GameSceneContext::exec(
            selection,
            &mut scenes[self.scene],
            &mut self.scene_content_root,
            &mut self.clipboard,
            self.sender.clone(),
            self.resource_manager.clone(),
            self.serialization_context.clone(),
            |ctx| command_stack.clear(ctx),
        );
    }

    fn on_before_render(&mut self, _editor_selection: &Selection, engine: &mut Engine) {
        // Temporarily disable cameras in currently edited scene. This is needed to prevent any
        // scene camera to interfere with the editor camera.
        let scene = &mut engine.scenes[self.scene];
        let has_preview_camera = scene.graph.is_valid_handle(self.preview_camera);
        for (handle, camera) in scene.graph.pair_iter_mut().filter_map(|(h, n)| {
            if has_preview_camera && h != self.preview_camera
                || !has_preview_camera && h != self.camera_controller.camera
            {
                n.cast_mut::<Camera>().map(|c| (h, c))
            } else {
                None
            }
        }) {
            self.camera_state.push((handle, camera.is_enabled()));
            camera.set_enabled(false);
        }
    }

    fn on_after_render(&mut self, engine: &mut Engine) {
        // Revert state of the cameras.
        for (handle, enabled) in self.camera_state.drain(..) {
            engine.scenes[self.scene].graph[handle]
                .as_camera_mut()
                .set_enabled(enabled);
        }
    }

    fn update(
        &mut self,
        editor_selection: &Selection,
        engine: &mut Engine,
        dt: f32,
        path: Option<&Path>,
        settings: &mut Settings,
        screen_bounds: Rect<f32>,
    ) -> Option<TextureResource> {
        self.draw_auxiliary_geometry(editor_selection, engine, settings);

        let scene = &mut engine.scenes[self.scene];

        for message in self.settings_receiver.try_iter() {
            match message {
                SettingsMessage::Changed => {
                    scene.graph[self.grid].set_visibility(settings.graphics.draw_grid);
                }
            }
        }

        // Create new render target if preview frame has changed its size.
        let mut new_render_target = None;
        if let TextureKind::Rectangle { width, height } = scene
            .rendering_options
            .render_target
            .clone()
            .unwrap()
            .data_ref()
            .kind()
        {
            let frame_size = screen_bounds.size;
            if width != frame_size.x as u32 || height != frame_size.y as u32 {
                scene.rendering_options.render_target = Some(TextureResource::new_render_target(
                    frame_size.x as u32,
                    frame_size.y as u32,
                ));
                new_render_target.clone_from(&scene.rendering_options.render_target);

                let gc = engine.graphics_context.as_initialized_mut();

                if let Some(highlighter) = self.highlighter.as_ref() {
                    highlighter.borrow_mut().resize(
                        &gc.renderer.state,
                        frame_size.x as usize,
                        frame_size.y as usize,
                    );
                }
            }
        }

        let node_overrides = self.graph_switches.node_overrides.as_mut().unwrap();
        for handle in scene.graph.traverse_handle_iter(self.editor_objects_root) {
            node_overrides.insert(handle);
        }

        let camera = scene.graph[self.camera_controller.camera].as_camera_mut();

        camera.projection_mut().set_z_near(settings.graphics.z_near);
        camera.projection_mut().set_z_far(settings.graphics.z_far);

        self.camera_controller.update(
            &mut scene.graph,
            settings,
            path,
            self.editor_objects_root,
            self.scene_content_root,
            screen_bounds.size,
            dt,
        );

        if let Some(highlighter) = self.highlighter.as_ref() {
            let mut highlighter = highlighter.borrow_mut();
            highlighter.nodes_to_highlight.clear();

            highlighter.scene_handle = self.scene;
            if let Some(selection) = editor_selection.as_graph() {
                for &handle in selection.nodes() {
                    highlighter.nodes_to_highlight.insert(handle);
                }
            }
        }

        new_render_target
    }

    fn is_interacting(&self) -> bool {
        self.camera_controller.is_interacting()
    }

    fn on_destroy(
        &mut self,
        command_stack: &mut CommandStack,
        engine: &mut Engine,
        selection: &mut Selection,
    ) {
        GameSceneContext::exec(
            selection,
            &mut engine.scenes[self.scene],
            &mut self.scene_content_root,
            &mut self.clipboard,
            self.sender.clone(),
            engine.resource_manager.clone(),
            engine.serialization_context.clone(),
            |ctx| command_stack.clear(ctx),
        );

        engine.scenes.remove(self.scene);
    }

    fn on_message(
        &mut self,
        message: &Message,
        selection: &Selection,
        engine: &mut Engine,
    ) -> bool {
        match message {
            Message::SaveSelectionAsPrefab(path) => {
                self.try_save_selection_as_prefab(path, selection, engine);
                false
            }
            Message::SetEditorCameraProjection(projection) => {
                self.camera_controller
                    .set_projection(&mut engine.scenes[self.scene].graph, projection.clone());

                false
            }
            Message::SelectObject { handle } => {
                self.select_object(*handle, engine);
                false
            }
            Message::FocusObject(handle) => {
                let scene = &mut engine.scenes[self.scene];
                self.camera_controller.fit_object(scene, *handle);
                false
            }
            Message::SyncNodeHandleName { view, handle } => {
                let scene = &engine.scenes[self.scene];
                engine
                    .user_interfaces
                    .first_mut()
                    .send_message(HandlePropertyEditorMessage::name(
                        *view,
                        MessageDirection::ToWidget,
                        scene
                            .graph
                            .try_get((*handle).into())
                            .map(|n| n.name_owned()),
                    ));
                false
            }
            Message::ProvideSceneHierarchy { view } => {
                let scene = &engine.scenes[self.scene];
                engine.user_interfaces.first_mut().send_message(
                    HandlePropertyEditorMessage::hierarchy(
                        *view,
                        MessageDirection::ToWidget,
                        HierarchyNode::from_scene_node(
                            self.scene_content_root,
                            Handle::NONE,
                            &scene.graph,
                        ),
                    ),
                );
                false
            }
            _ => false,
        }
    }

    fn command_names(
        &mut self,
        command_stack: &mut CommandStack,
        selection: &mut Selection,
        engine: &mut Engine,
    ) -> Vec<String> {
        command_stack
            .commands
            .iter_mut()
            .map(|c| {
                let mut name = String::new();
                GameSceneContext::exec(
                    selection,
                    &mut engine.scenes[self.scene],
                    &mut self.scene_content_root,
                    &mut self.clipboard,
                    self.sender.clone(),
                    engine.resource_manager.clone(),
                    engine.serialization_context.clone(),
                    |ctx| {
                        name = c.name(ctx);
                    },
                );
                name
            })
            .collect::<Vec<_>>()
    }

    fn first_selected_entity(
        &self,
        selection: &Selection,
        scenes: &SceneContainer,
        callback: &mut dyn FnMut(&dyn Reflect),
    ) {
        let scene = &scenes[self.scene];
        if let Some(selection) = selection.as_graph() {
            if let Some(node) = selection
                .nodes
                .first()
                .and_then(|handle| scene.graph.try_get(*handle))
            {
                (callback)(node as &dyn Reflect);
            }
        } else if let Some(selection) = selection.as_audio_bus() {
            let state = scene.graph.sound_context.state();
            if let Some(effect) = selection
                .buses
                .first()
                .and_then(|handle| state.bus_graph_ref().try_get_bus_ref(*handle))
            {
                (callback)(effect as &dyn Reflect);
            }
        } else if let Some(selection) = selection.as_animation() {
            if let Some(player) = scene
                .graph
                .try_get_of_type::<AnimationPlayer>(selection.animation_player)
            {
                if let Some(animation) = player.animations().try_get(selection.animation) {
                    if let Some(animation::selection::SelectedEntity::Signal(id)) =
                        selection.entities.first()
                    {
                        if let Some(signal) = animation.signals().iter().find(|s| s.id == *id) {
                            (callback)(signal as &dyn Reflect);
                        } else {
                            (callback)(player as &dyn Reflect);
                        }
                    } else {
                        (callback)(player as &dyn Reflect);
                    }
                } else {
                    (callback)(player as &dyn Reflect);
                }
            }
        } else if let Some(selection) = selection.as_absm() {
            if let Some(node) = scene
                .graph
                .try_get_of_type::<AnimationBlendingStateMachine>(selection.absm_node_handle)
            {
                if let Some(first) = selection.entities.first() {
                    let machine = node.machine();
                    if let Some(layer_index) = selection.layer {
                        if let Some(layer) = machine.layers().get(layer_index) {
                            match first {
                                SelectedEntity::Transition(transition) => {
                                    (callback)(&layer.transitions()[*transition] as &dyn Reflect)
                                }
                                SelectedEntity::State(state) => {
                                    (callback)(&layer.states()[*state] as &dyn Reflect)
                                }
                                SelectedEntity::PoseNode(pose) => {
                                    (callback)(&layer.nodes()[*pose] as &dyn Reflect)
                                }
                            };
                        }
                    }
                }
            }
        }
    }

    fn on_property_changed(
        &mut self,
        args: &PropertyChanged,
        selection: &Selection,
        engine: &mut Engine,
    ) {
        let scene = &mut engine.scenes[self.scene];

        let group = if let Some(selection) = selection.as_graph() {
            selection
                .nodes
                .iter()
                .filter_map(|&node_handle| {
                    if scene.graph.is_valid_handle(node_handle) {
                        self.node_property_changed_handler.handle(
                            args,
                            node_handle,
                            &mut scene.graph[node_handle],
                        )
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        } else if let Some(selection) = selection.as_audio_bus() {
            selection
                .buses
                .iter()
                .filter_map(|&handle| {
                    make_command(args, move |ctx| {
                        let mut state = ctx
                            .get_mut::<GameSceneContext>()
                            .scene
                            .graph
                            .sound_context
                            .state();
                        let bus = state.bus_graph_mut().try_get_bus_mut(handle).unwrap();
                        // FIXME: HACK!
                        unsafe {
                            std::mem::transmute::<&'_ mut AudioBus, &'static mut AudioBus>(bus)
                        }
                    })
                })
                .collect::<Vec<_>>()
        } else if let Some(selection) = selection.as_animation() {
            if scene
                .graph
                .try_get_of_type::<AnimationPlayer>(selection.animation_player)
                .and_then(|player| player.animations().try_get(selection.animation))
                .is_some()
            {
                let animation_player = selection.animation_player;
                let animation = selection.animation;
                selection
                    .entities
                    .iter()
                    .filter_map(|e| {
                        if let &animation::selection::SelectedEntity::Signal(id) = e {
                            make_command(args, move |ctx| {
                                fetch_animations_container(animation_player, ctx)[animation]
                                    .signals_mut()
                                    .iter_mut()
                                    .find(|s| s.id == id)
                                    .unwrap()
                            })
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                vec![]
            }
        } else if let Some(selection) = selection.as_absm() {
            if scene
                .graph
                .try_get(selection.absm_node_handle)
                .and_then(|n| n.query_component_ref::<AnimationBlendingStateMachine>())
                .is_some()
            {
                if let Some(layer_index) = selection.layer {
                    let absm_node_handle = selection.absm_node_handle;
                    selection
                        .entities
                        .iter()
                        .filter_map(|ent| match *ent {
                            SelectedEntity::Transition(transition) => {
                                make_command(args, move |ctx| {
                                    let machine = fetch_machine(ctx, absm_node_handle);
                                    &mut machine.layers_mut()[layer_index].transitions_mut()
                                        [transition]
                                })
                            }
                            SelectedEntity::State(state) => make_command(args, move |ctx| {
                                let machine = fetch_machine(ctx, absm_node_handle);
                                &mut machine.layers_mut()[layer_index].states_mut()[state]
                            }),
                            SelectedEntity::PoseNode(pose) => make_command(args, move |ctx| {
                                let machine = fetch_machine(ctx, absm_node_handle);
                                &mut machine.layers_mut()[layer_index].nodes_mut()[pose]
                            }),
                        })
                        .collect()
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        if group.is_empty() {
            if !args.is_inheritable() {
                Log::err(format!("Failed to handle a property {}", args.path()))
            }
        } else if group.len() == 1 {
            self.sender
                .send(Message::DoCommand(group.into_iter().next().unwrap()))
        } else {
            self.sender.do_command(CommandGroup::from(group));
        }
    }

    fn provide_docs(&self, selection: &Selection, engine: &Engine) -> Option<String> {
        let scene = &engine.scenes[self.scene];

        if let Some(graph_selection) = selection.as_graph() {
            graph_selection
                .nodes
                .first()
                .map(|h| scene.graph[*h].doc().to_string())
        } else if let Some(navmesh_selection) = selection.as_navmesh() {
            Some(
                scene.graph[navmesh_selection.navmesh_node()]
                    .doc()
                    .to_string(),
            )
        } else if let Some(audio_bus_selection) = selection.as_audio_bus() {
            audio_bus_selection.buses.first().and_then(|h| {
                scene
                    .graph
                    .sound_context
                    .state()
                    .bus_graph_ref()
                    .try_get_bus_ref(*h)
                    .map(|bus| bus.doc().to_string())
            })
        } else if let Some(absm_selection) = selection.as_absm::<Node>() {
            Some(
                scene.graph[absm_selection.absm_node_handle]
                    .doc()
                    .to_string(),
            )
        } else {
            selection.as_animation::<Node>().map(|animation_selection| {
                scene.graph[animation_selection.animation_player]
                    .doc()
                    .to_string()
            })
        }
    }
}

pub trait BaseSelectionContainer: Debug + 'static {
    fn clone_boxed(&self) -> Box<dyn SelectionContainer>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn eq_ref(&self, other: &dyn SelectionContainer) -> bool;
}

impl<T> BaseSelectionContainer for T
where
    T: Clone + SelectionContainer + PartialEq + 'static,
{
    fn clone_boxed(&self) -> Box<dyn SelectionContainer> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn eq_ref(&self, other: &dyn SelectionContainer) -> bool {
        if let Some(other) = other.downcast_ref::<T>() {
            self == other
        } else {
            false
        }
    }
}

pub trait SelectionContainer: BaseSelectionContainer {
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn is_single_selection(&self) -> bool {
        self.len() == 1
    }

    fn is_multi_selection(&self) -> bool {
        self.len() > 1
    }
}

impl dyn SelectionContainer {
    fn downcast_ref<T: SelectionContainer>(&self) -> Option<&T> {
        self.as_any().downcast_ref()
    }

    fn downcast_mut<T: SelectionContainer>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut()
    }
}

#[derive(Debug, Default)]
pub struct Selection(pub Option<Box<dyn SelectionContainer>>);

impl PartialEq for Selection {
    fn eq(&self, other: &Self) -> bool {
        if let (Some(this), Some(other)) = (self.0.as_ref(), other.0.as_ref()) {
            this.eq_ref(&**other)
        } else {
            matches!((&self.0, &other.0), (None, None))
        }
    }
}

impl Clone for Selection {
    fn clone(&self) -> Self {
        match self.0.as_ref() {
            Some(inner) => Self(Some(inner.clone_boxed())),
            None => Self::default(),
        }
    }
}

macro_rules! define_downcast {
    ($ty:ty, $as_ref:ident, $as_mut:ident, $is:ident) => {
        pub fn $as_ref(&self) -> Option<&$ty> {
            self.0.as_ref().and_then(|s| s.downcast_ref())
        }
        pub fn $as_mut(&mut self) -> Option<&mut $ty> {
            self.0.as_mut().and_then(|s| s.downcast_mut())
        }
        pub fn $is(&mut self) -> bool {
            self.$as_ref().is_some()
        }
    };
}

impl Selection {
    pub fn new<T: SelectionContainer>(container: T) -> Self {
        Self(Some(Box::new(container)))
    }

    pub fn new_empty() -> Self {
        Self::default()
    }

    pub fn is_none(&self) -> bool {
        self.0.is_none()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.0.as_ref().map_or(0, |s| s.len())
    }

    pub fn is_single_selection(&self) -> bool {
        self.0.as_ref().map_or(false, |s| s.is_single_selection())
    }

    pub fn is_multi_selection(&self) -> bool {
        self.0.as_ref().map_or(false, |s| s.is_multi_selection())
    }

    define_downcast!(GraphSelection, as_graph, as_graph_mut, is_graph);

    define_downcast!(NavmeshSelection, as_navmesh, as_navmesh_mut, is_navmesh);

    define_downcast!(
        AudioBusSelection,
        as_audio_bus,
        as_audio_bus_mut,
        is_audio_bus
    );

    pub fn as_absm<N: 'static>(&self) -> Option<&AbsmSelection<N>> {
        self.0.as_ref().and_then(|s| s.downcast_ref())
    }
    pub fn as_absm_mut<N: 'static>(&mut self) -> Option<&mut AbsmSelection<N>> {
        self.0.as_mut().and_then(|s| s.downcast_mut())
    }
    pub fn is_absm<N: 'static>(&mut self) -> bool {
        self.as_absm::<N>().is_some()
    }

    pub fn as_animation<N: 'static>(&self) -> Option<&AnimationSelection<N>> {
        self.0.as_ref().and_then(|s| s.downcast_ref())
    }
    pub fn as_animation_mut<N: 'static>(&mut self) -> Option<&mut AnimationSelection<N>> {
        self.0.as_mut().and_then(|s| s.downcast_mut())
    }
    pub fn is_animation<N: 'static>(&mut self) -> bool {
        self.as_animation::<N>().is_some()
    }

    define_downcast!(UiSelection, as_ui, as_ui_mut, is_ui);
}
