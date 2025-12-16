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
    asset::item::AssetItem,
    audio::AudioBusSelection,
    camera::{CameraController, PickingOptions},
    command::{Command, CommandGroup, CommandStack},
    fyrox::{
        asset::manager::ResourceManager,
        core::{
            algebra::{Matrix4, Vector2, Vector3},
            color::Color,
            define_as_any_trait,
            futures::executor::block_on,
            log::Log,
            make_relative_path,
            math::{aabb::AxisAlignedBoundingBox, plane::Plane, Rect},
            pool::{ErasedHandle, Handle},
            reflect::Reflect,
            visitor::Visitor,
            Uuid,
        },
        engine::{Engine, SerializationContext},
        fxhash::FxHashSet,
        graph::{BaseSceneGraph, SceneGraph, SceneGraphNode},
        gui::{
            inspector::PropertyChanged,
            message::UiMessage,
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
            terrain::Terrain,
            Scene, SceneContainer,
        },
    },
    highlight::HighlightRenderPass,
    interaction::navmesh::selection::NavmeshSelection,
    message::MessageSender,
    plugins::{
        absm::selection::AbsmSelection,
        animation::selection::AnimationSelection,
        inspector::editors::handle::{
            HandlePropertyEditorHierarchyMessage, HandlePropertyEditorNameMessage,
        },
        inspector::handlers::node::SceneNodePropertyChangedHandler,
    },
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
    world::selection::GraphSelection,
    Message, Settings,
};
use fyrox::engine::GraphicsContext;
use fyrox::gui::file_browser::FileType;
use fyrox::scene::collider::BitMask;
use std::{
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
mod nullscene;
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
    pub grid_material: MaterialResource,
    pub settings_receiver: Receiver<SettingsMessage>,
}

lazy_static! {
    static ref GRID_SHADER: ShaderResource = {
        ShaderResource::from_str(
            Uuid::new_v4(),
            include_str!("../../resources/shaders/grid.shader"),
            Default::default(),
        )
        .unwrap()
    };
}

fn make_grid_material() -> MaterialResource {
    let material = Material::from_shader(GRID_SHADER.clone());
    MaterialResource::new_embedded(material)
}

impl GameScene {
    pub const EDITOR_OBJECTS_MASK: BitMask = BitMask(0b1000_0000_0000_0000);

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

        let grid_material = make_grid_material();
        let grid = MeshBuilder::new(
            BaseBuilder::new()
                .with_frustum_culling(false)
                .with_visibility(settings.graphics.draw_grid),
        )
        .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_embedded(
            SurfaceData::make_quad(&Matrix4::new_scaling(2.0)),
        ))
        .with_material(grid_material.clone())
        .build()])
        .with_render_path(RenderPath::Forward)
        .build(&mut scene.graph);

        let (settings_sender, settings_receiver) = mpsc::channel();
        settings.subscribers.push(settings_sender);

        let camera_controller = CameraController::new(
            &mut scene.graph,
            editor_objects_root,
            settings,
            path.as_ref()
                .and_then(|p| {
                    settings
                        .scene_settings
                        .get(*p)
                        .map(|s| s.camera_settings.clone())
                })
                .unwrap_or_default(),
            grid,
            editor_objects_root,
            scene_content_root,
        );

        GameScene {
            editor_objects_root,
            scene_content_root,
            camera_controller,
            preview_instance: None,
            scene: engine.scenes.add(scene),
            clipboard: Default::default(),
            preview_camera: Default::default(),
            graph_switches: GraphUpdateSwitches {
                // Freeze physics simulation while editing scene by setting time step to zero.
                physics_dt: false,
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
            grid_material,
            settings_receiver,
        }
    }

    pub fn make_purified_scene(&self, engine: &mut Engine) -> Scene {
        let scene = &mut engine.scenes[self.scene];

        let editor_root = self.editor_objects_root;
        let (pure_scene, _) = scene.clone_ex(
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

        if let Err(e) = visitor.save_ascii_to_file(path) {
            Err(format!(
                "Failed to save scene {}! Reason: {e}",
                path.display()
            ))
        } else {
            if settings.debugging.save_scene_in_text_form {
                let text = visitor.save_ascii_to_string();
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
                if let Some(node) = scene.graph.try_get_node(node) {
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
            } else if node.component_ref::<Camera>().is_some() {
                if settings.debugging.show_camera_bounds {
                    node.debug_draw(ctx);
                }
            } else if node.component_ref::<PointLight>().is_some()
                || node.component_ref::<SpotLight>().is_some()
            {
                if settings.debugging.show_light_bounds {
                    node.debug_draw(ctx);
                }
            } else if node.component_ref::<Terrain>().is_some() {
                if settings.debugging.show_terrains {
                    node.debug_draw(ctx);
                }
            } else if let Some(navmesh) = node.component_ref::<NavigationalMesh>() {
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
                for (descendant_handle, _) in graph.traverse_iter(*node) {
                    for reference in graph.find_references_to(descendant_handle) {
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
                Err(e) => Log::err(format!("Failed to save selection as prefab! Reason: {e:?}")),
                Ok(_) => {
                    if let Err(e) = visitor.save_ascii_to_file(path) {
                        Log::err(format!("Failed to save selection as prefab! Reason: {e:?}"));
                    } else {
                        Log::info(format!(
                            "Selection was successfully saved as prefab to {path:?}!"
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
        pos: Vector2<f32>,
        offset: Vector2<f32>,
        _screen_bounds: Rect<f32>,
        engine: &mut Engine,
        _settings: &Settings,
    ) {
        self.camera_controller.on_mouse_move(
            &mut engine.scenes[self.scene].graph,
            pos,
            _screen_bounds.size,
            offset,
            _settings,
        );
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
        pos: Vector2<f32>,
        _screen_bounds: Rect<f32>,
        engine: &mut Engine,
        _settings: &Settings,
    ) {
        self.camera_controller.on_mouse_button_down(
            pos,
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
                                .traverse_iter(instance)
                                .map(|(handle, _)| handle)
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
                        method: Default::default(),
                        settings: &settings.selection,
                    },
                ) {
                    Some(result.position)
                } else {
                    // In case of empty space, check intersection with oXZ plane (3D) or oXY (2D).
                    let camera = graph[self.camera_controller.camera]
                        .component_ref::<Camera>()
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
                            method: Default::default(),
                            settings: &settings.selection,
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

    fn file_type(&self) -> FileType {
        FileType {
            description: "Game Scene".to_string(),
            extension: "rgs".to_string(),
        }
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
        let scene = &mut engine.scenes[self.scene];

        // Apply render mask to all editor objects.
        for handle in scene
            .graph
            .traverse_handle_iter(self.editor_objects_root)
            .collect::<Vec<_>>()
        {
            scene.graph[handle]
                .render_mask
                .set_value_and_mark_modified(Self::EDITOR_OBJECTS_MASK);
        }

        // Temporarily disable cameras in currently edited scene. This is needed to prevent any
        // scene camera to interfere with the editor camera.

        for (handle, camera) in scene.graph.pair_iter_mut().filter_map(|(h, n)| {
            if h == self.camera_controller.camera || h == self.preview_camera {
                None
            } else {
                n.cast_mut::<Camera>().map(|c| (h, c))
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

                if let GraphicsContext::Initialized(ref graphics_context) = engine.graphics_context
                {
                    if let Some(highlighter) = self.highlighter.as_ref() {
                        highlighter.borrow_mut().resize(
                            &*graphics_context.renderer.server,
                            frame_size.x as usize,
                            frame_size.y as usize,
                        );
                    }
                }
            }
        }

        let node_overrides = self.graph_switches.node_overrides.as_mut().unwrap();
        for (handle, _) in scene.graph.traverse_iter(self.editor_objects_root) {
            node_overrides.insert(handle);
        }

        let camera = scene.graph[self.camera_controller.camera].as_camera_mut();

        let projection = camera.projection_mut();
        projection.set_z_near(settings.graphics.z_near);
        projection.set_z_far(settings.graphics.z_far);

        let mut grid_material = self.grid_material.data_ref();
        grid_material.set_property(
            "orientation",
            match projection {
                Projection::Perspective(_) => 0i32,
                Projection::Orthographic(_) => 1i32,
            },
        );
        grid_material.set_property("isPerspective", projection.is_perspective());

        let scale = if settings.move_mode_settings.grid_snapping {
            fn div_safe(a: f32, b: f32) -> f32 {
                if b == 0.0 {
                    a
                } else {
                    a / b
                }
            }

            match projection {
                Projection::Perspective(_) => Vector2::new(
                    div_safe(1.0, settings.move_mode_settings.x_snap_step),
                    div_safe(1.0, settings.move_mode_settings.z_snap_step),
                ),
                Projection::Orthographic(_) => Vector2::new(
                    div_safe(1.0, settings.move_mode_settings.x_snap_step),
                    div_safe(1.0, settings.move_mode_settings.y_snap_step),
                ),
            }
        } else {
            Vector2::repeat(1.0)
        };

        grid_material.set_property("scale", scale);

        let grid_offset = projection.z_far() - projection.z_near();
        scene.graph[self.grid]
            .local_transform_mut()
            .set_position(Vector3::new(0.0, 0.0, grid_offset));

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
                self.camera_controller.fit_object(scene, *handle, Some(2.0));
                false
            }
            Message::SyncNodeHandleName { view, handle } => {
                let scene = &engine.scenes[self.scene];
                engine.user_interfaces.first_mut().send_message(
                    UiMessage::with_data(HandlePropertyEditorNameMessage(
                        scene
                            .graph
                            .try_get_node((*handle).into())
                            .map(|n| n.name_owned()),
                    ))
                    .with_destination(*view)
                    .with_direction(MessageDirection::ToWidget),
                );
                false
            }
            Message::ProvideSceneHierarchy { view } => {
                let scene = &engine.scenes[self.scene];

                engine.user_interfaces.first_mut().send_message(
                    UiMessage::with_data(HandlePropertyEditorHierarchyMessage(
                        HierarchyNode::from_scene_node(
                            self.scene_content_root,
                            Handle::NONE,
                            &scene.graph,
                        ),
                    ))
                    .with_destination(*view)
                    .with_direction(MessageDirection::ToWidget),
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
}

define_as_any_trait!(SelectionContainerAsAny => SelectionContainer);

pub trait BaseSelectionContainer: SelectionContainerAsAny + Debug {
    fn clone_boxed(&self) -> Box<dyn SelectionContainer>;
    fn eq_ref(&self, other: &dyn SelectionContainer) -> bool;
}

impl<T> BaseSelectionContainer for T
where
    T: Clone + SelectionContainer + PartialEq + 'static,
{
    fn clone_boxed(&self) -> Box<dyn SelectionContainer> {
        Box::new(self.clone())
    }

    fn eq_ref(&self, other: &dyn SelectionContainer) -> bool {
        if let Some(other) = other.downcast_ref::<T>() {
            self == other
        } else {
            false
        }
    }
}

pub struct EntityInfo<'a> {
    pub entity: &'a dyn Reflect,
    pub has_inheritance_parent: bool,
    pub read_only: bool,
}

impl<'a> EntityInfo<'a> {
    pub fn with_no_parent(entity: &'a dyn Reflect) -> Self {
        Self {
            entity,
            has_inheritance_parent: false,
            read_only: false,
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

    fn first_selected_entity(
        &self,
        controller: &dyn SceneController,
        scenes: &SceneContainer,
        callback: &mut dyn FnMut(EntityInfo),
    );

    fn on_property_changed(
        &mut self,
        controller: &mut dyn SceneController,
        args: &PropertyChanged,
        engine: &mut Engine,
        sender: &MessageSender,
    );

    fn paste_property(&mut self, path: &str, value: &dyn Reflect, sender: &MessageSender);

    fn provide_docs(&self, controller: &dyn SceneController, engine: &Engine) -> Option<String>;
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
        self.0.as_ref().is_some_and(|s| s.is_single_selection())
    }

    pub fn is_multi_selection(&self) -> bool {
        self.0.as_ref().is_some_and(|s| s.is_multi_selection())
    }

    pub fn as_ref<N: SelectionContainer>(&self) -> Option<&N> {
        self.0.as_ref().and_then(|v| v.downcast_ref::<N>())
    }

    pub fn as_mut<N: SelectionContainer>(&mut self) -> Option<&mut N> {
        self.0.as_mut().and_then(|v| v.downcast_mut::<N>())
    }

    define_downcast!(GraphSelection, as_graph, as_graph_mut, is_graph);

    define_downcast!(NavmeshSelection, as_navmesh, as_navmesh_mut, is_navmesh);

    define_downcast!(
        AudioBusSelection,
        as_audio_bus,
        as_audio_bus_mut,
        is_audio_bus
    );

    pub fn as_absm<N: Reflect>(&self) -> Option<&AbsmSelection<N>> {
        self.0.as_ref().and_then(|s| s.downcast_ref())
    }
    pub fn as_absm_mut<N: Reflect>(&mut self) -> Option<&mut AbsmSelection<N>> {
        self.0.as_mut().and_then(|s| s.downcast_mut())
    }
    pub fn is_absm<N: Reflect>(&mut self) -> bool {
        self.as_absm::<N>().is_some()
    }

    pub fn as_animation<N: Reflect>(&self) -> Option<&AnimationSelection<N>> {
        self.0.as_ref().and_then(|s| s.downcast_ref())
    }
    pub fn as_animation_mut<N: Reflect>(&mut self) -> Option<&mut AnimationSelection<N>> {
        self.0.as_mut().and_then(|s| s.downcast_mut())
    }
    pub fn is_animation<N: Reflect>(&mut self) -> bool {
        self.as_animation::<N>().is_some()
    }

    define_downcast!(UiSelection, as_ui, as_ui_mut, is_ui);

    pub fn first_selected_entity(
        &self,
        controller: &dyn SceneController,
        scenes: &SceneContainer,
        callback: &mut dyn FnMut(EntityInfo),
    ) {
        if let Some(container) = self.0.as_ref() {
            container.first_selected_entity(controller, scenes, callback);
        }
    }

    pub fn on_property_changed(
        &mut self,
        controller: &mut dyn SceneController,
        args: &PropertyChanged,
        engine: &mut Engine,
        sender: &MessageSender,
    ) {
        if let Some(container) = self.0.as_mut() {
            container.on_property_changed(controller, args, engine, sender);
        }
    }

    pub fn paste_property(&mut self, path: &str, value: &dyn Reflect, sender: &MessageSender) {
        if let Some(container) = self.0.as_mut() {
            container.paste_property(path, value, sender);
        }
    }

    pub fn provide_docs(
        &self,
        controller: &dyn SceneController,
        engine: &Engine,
    ) -> Option<String> {
        self.0
            .as_ref()
            .and_then(|c| c.provide_docs(controller, engine))
    }
}
