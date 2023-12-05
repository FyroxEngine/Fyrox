use crate::{
    absm::selection::AbsmSelection, animation::selection::AnimationSelection,
    audio::AudioBusSelection, camera::CameraController,
    interaction::navmesh::selection::NavmeshSelection, scene::clipboard::Clipboard,
    world::graph::selection::GraphSelection, Settings,
};
use fyrox::core::log::Log;
use fyrox::{
    core::{color::Color, math::aabb::AxisAlignedBoundingBox, pool::Handle, visitor::Visitor},
    engine::Engine,
    scene::{
        base::BaseBuilder,
        camera::Camera,
        debug::{Line, SceneDrawingContext},
        graph::{Graph, GraphUpdateSwitches},
        light::{point::PointLight, spot::SpotLight},
        mesh::Mesh,
        navmesh::NavigationalMesh,
        node::Node,
        pivot::PivotBuilder,
        terrain::Terrain,
        Scene,
    },
};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub mod clipboard;
pub mod dialog;
pub mod property;
pub mod selector;
pub mod settings;

#[macro_use]
pub mod commands;
pub mod container;
pub mod controller;

pub struct EditorScene {
    pub has_unsaved_changes: bool,
    pub path: Option<PathBuf>,
    pub scene: Handle<Scene>,
    // Handle to a root for all editor nodes.
    pub editor_objects_root: Handle<Node>,
    pub scene_content_root: Handle<Node>,
    pub selection: Selection,
    pub clipboard: Clipboard,
    pub camera_controller: CameraController,
    pub preview_camera: Handle<Node>,
    pub graph_switches: GraphUpdateSwitches,
}

impl EditorScene {
    pub fn from_native_scene(
        mut scene: Scene,
        engine: &mut Engine,
        path: Option<PathBuf>,
        settings: &Settings,
    ) -> Self {
        let scene_content_root = scene.graph.get_root();

        scene
            .graph
            .change_root(PivotBuilder::new(BaseBuilder::new()).build_node());

        let editor_objects_root = PivotBuilder::new(BaseBuilder::new()).build(&mut scene.graph);
        let camera_controller = CameraController::new(
            &mut scene.graph,
            editor_objects_root,
            path.as_ref()
                .and_then(|p| settings.scene_settings.get(p).map(|s| &s.camera_settings)),
        );

        // Freeze physics simulation in while editing scene by setting time step to zero.
        scene.graph.physics.integration_parameters.dt = Some(0.0);
        scene.graph.physics2d.integration_parameters.dt = Some(0.0);

        EditorScene {
            path,
            editor_objects_root,
            scene_content_root,
            camera_controller,
            scene: engine.scenes.add(scene),
            selection: Default::default(),
            clipboard: Default::default(),
            has_unsaved_changes: false,
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
        }
    }

    pub fn make_purified_scene(&self, engine: &mut Engine) -> Scene {
        let scene = &mut engine.scenes[self.scene];

        let editor_root = self.editor_objects_root;
        let (pure_scene, _) = scene.clone(
            self.scene_content_root,
            &mut |node, _| node != editor_root,
            &mut |_, _, _| {},
        );

        pure_scene
    }

    pub fn need_save(&self) -> bool {
        self.has_unsaved_changes || self.path.is_none()
    }

    pub fn name(&self) -> String {
        self.path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| String::from("Unnamed Scene"))
    }

    #[allow(clippy::redundant_clone)] // false positive
    pub fn save(
        &mut self,
        path: PathBuf,
        settings: &Settings,
        engine: &mut Engine,
    ) -> Result<String, String> {
        // Validate first.
        let valid = true;
        let mut reason = "Scene is not saved, because validation failed:\n".to_owned();

        if valid {
            self.path = Some(path.clone());

            let mut pure_scene = self.make_purified_scene(engine);

            let mut visitor = Visitor::new();
            pure_scene.save("Scene", &mut visitor).unwrap();
            if let Err(e) = visitor.save_binary(&path) {
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
        } else {
            use std::fmt::Write;
            writeln!(&mut reason, "\nPlease fix errors and try again.").unwrap();

            Err(reason)
        }
    }

    pub fn update(&mut self, engine: &mut Engine, dt: f32, settings: &mut Settings) {
        self.draw_auxiliary_geometry(engine, settings);

        let scene = &mut engine.scenes[self.scene];

        let node_overrides = self.graph_switches.node_overrides.as_mut().unwrap();
        for handle in scene.graph.traverse_handle_iter(self.editor_objects_root) {
            node_overrides.insert(handle);
        }

        let camera = scene.graph[self.camera_controller.camera].as_camera_mut();

        camera.projection_mut().set_z_near(settings.graphics.z_near);
        camera.projection_mut().set_z_far(settings.graphics.z_far);

        self.camera_controller
            .update(&mut scene.graph, settings, self.path.as_ref(), dt);
    }

    pub fn draw_auxiliary_geometry(&mut self, engine: &mut Engine, settings: &Settings) {
        let debug_settings = &settings.debugging;
        let scene = &mut engine.scenes[self.scene];

        scene.drawing_context.clear_lines();

        if let Selection::Graph(selection) = &self.selection {
            for &node in selection.nodes() {
                let node = &scene.graph[node];
                scene.drawing_context.draw_oob(
                    &node.local_bounding_box(),
                    node.global_transform(),
                    Color::GREEN,
                );
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
            editor_scene: &EditorScene,
            settings: &Settings,
        ) {
            // Ignore editor nodes.
            if node == editor_scene.editor_objects_root {
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
                if settings.debugging.show_camera_bounds {
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
                    let selection =
                        if let Selection::Navmesh(ref selection) = editor_scene.selection {
                            Some(selection)
                        } else {
                            None
                        };

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
                draw_recursively(child, graph, ctx, editor_scene, settings)
            }
        }

        // Draw pivots.
        draw_recursively(
            self.scene_content_root,
            &scene.graph,
            &mut scene.drawing_context,
            self,
            settings,
        );
    }

    /// Checks whether the current graph selection has references to the nodes outside of the selection.
    pub fn is_current_selection_has_external_refs(&self, graph: &Graph) -> bool {
        if let Selection::Graph(selection) = &self.selection {
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    None,
    Graph(GraphSelection),
    Navmesh(NavmeshSelection),
    AudioBus(AudioBusSelection),
    Absm(AbsmSelection),
    Animation(AnimationSelection),
}

impl Default for Selection {
    fn default() -> Self {
        Self::None
    }
}

impl Selection {
    pub fn is_empty(&self) -> bool {
        match self {
            Selection::None => true,
            Selection::Graph(graph) => graph.is_empty(),
            Selection::Navmesh(navmesh) => navmesh.is_empty(),
            Selection::AudioBus(effect) => effect.is_empty(),
            Selection::Absm(absm) => absm.is_empty(),
            Selection::Animation(animation) => animation.is_empty(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Selection::None => 0,
            Selection::Graph(graph) => graph.len(),
            Selection::Navmesh(navmesh) => navmesh.len(),
            Selection::AudioBus(effect) => effect.len(),
            Selection::Absm(absm) => absm.len(),
            Selection::Animation(animation) => animation.len(),
        }
    }

    pub fn is_single_selection(&self) -> bool {
        self.len() == 1
    }
}
