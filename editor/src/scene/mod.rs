use crate::{
    absm::selection::AbsmSelection,
    animation::selection::AnimationSelection,
    audio::EffectSelection,
    camera::CameraController,
    interaction::navmesh::{
        data_model::{Navmesh, NavmeshContainer, NavmeshTriangle, NavmeshVertex},
        selection::NavmeshSelection,
    },
    scene::clipboard::Clipboard,
    settings::debugging::DebuggingSettings,
    world::graph::selection::GraphSelection,
    GameEngine, Settings,
};
use fyrox::{
    core::{
        algebra::{Matrix4, Point3, UnitQuaternion, Vector3},
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, frustum::Frustum, Matrix4Ext, TriangleDefinition},
        pool::Handle,
        visitor::Visitor,
    },
    engine::Engine,
    scene::{
        base::BaseBuilder,
        camera::Camera,
        debug::{Line, SceneDrawingContext},
        graph::{Graph, GraphUpdateSwitches},
        light::{point::PointLight, spot::SpotLight},
        mesh::{
            buffer::{VertexAttributeUsage, VertexReadTrait},
            Mesh,
        },
        node::Node,
        particle_system::ParticleSystem,
        pivot::PivotBuilder,
        Scene,
    },
};
use std::{collections::HashMap, fmt::Write, path::PathBuf};

pub mod clipboard;
pub mod property;
pub mod selector;
pub mod settings;

#[macro_use]
pub mod commands;

pub struct EditorScene {
    pub has_unsaved_changes: bool,
    pub path: Option<PathBuf>,
    pub scene: Handle<Scene>,
    // Handle to a root for all editor nodes.
    pub editor_objects_root: Handle<Node>,
    pub selection: Selection,
    pub clipboard: Clipboard,
    pub camera_controller: CameraController,
    pub navmeshes: NavmeshContainer,
    pub preview_camera: Handle<Node>,
    pub graph_switches: GraphUpdateSwitches,
}

pub fn is_scene_needs_to_be_saved(editor_scene: Option<&EditorScene>) -> bool {
    editor_scene
        .as_ref()
        .map_or(false, |s| s.has_unsaved_changes || s.path.is_none())
}

impl EditorScene {
    pub fn from_native_scene(
        mut scene: Scene,
        engine: &mut Engine,
        path: Option<PathBuf>,
        settings: &Settings,
    ) -> Self {
        let root = PivotBuilder::new(BaseBuilder::new()).build(&mut scene.graph);
        let camera_controller = CameraController::new(
            &mut scene.graph,
            root,
            path.as_ref()
                .and_then(|p| settings.camera.camera_settings.get(p)),
        );

        // Freeze physics simulation in while editing scene by setting time step to zero.
        scene.graph.physics.integration_parameters.dt = Some(0.0);
        scene.graph.physics2d.integration_parameters.dt = Some(0.0);

        let mut navmeshes = NavmeshContainer::default();

        for navmesh in scene.navmeshes.iter() {
            let _ = navmeshes.spawn(Navmesh {
                vertices: navmesh
                    .vertices()
                    .iter()
                    .map(|vertex| NavmeshVertex {
                        position: vertex.position,
                    })
                    .collect(),
                triangles: navmesh
                    .triangles()
                    .iter()
                    .map(|triangle| NavmeshTriangle {
                        a: Handle::new(triangle[0], 1),
                        b: Handle::new(triangle[1], 1),
                        c: Handle::new(triangle[2], 1),
                    })
                    .collect(),
            });
        }

        EditorScene {
            path,
            editor_objects_root: root,
            camera_controller,
            navmeshes,
            scene: engine.scenes.add(scene),
            selection: Default::default(),
            clipboard: Default::default(),
            has_unsaved_changes: false,
            preview_camera: Default::default(),
            graph_switches: GraphUpdateSwitches {
                physics2d: true,
                physics: true,
                sound: false,
                // Update only editor's camera.
                node_overrides: Some(Default::default()),
            },
        }
    }

    pub fn make_purified_scene(&self, engine: &mut GameEngine) -> Scene {
        let scene = &mut engine.scenes[self.scene];

        let editor_root = self.editor_objects_root;
        let (mut pure_scene, _) = scene.clone(&mut |node, _| node != editor_root);

        // Reset state of nodes. For some nodes (such as particles systems) we use scene as preview
        // so before saving scene, we have to reset state of such nodes.
        for node in pure_scene.graph.linear_iter_mut() {
            if let Some(particle_system) = node.cast_mut::<ParticleSystem>() {
                // Particle system must not save generated vertices.
                particle_system.clear_particles();
            }
        }

        pure_scene.navmeshes.clear();

        for navmesh in self.navmeshes.iter() {
            // Sparse-to-dense mapping - handle to index.
            let mut vertex_map = HashMap::new();

            let vertices = navmesh
                .vertices
                .pair_iter()
                .enumerate()
                .map(|(i, (handle, vertex))| {
                    vertex_map.insert(handle, i);
                    vertex.position
                })
                .collect::<Vec<_>>();

            let triangles = navmesh
                .triangles
                .iter()
                .map(|triangle| {
                    TriangleDefinition([
                        vertex_map[&triangle.a] as u32,
                        vertex_map[&triangle.b] as u32,
                        vertex_map[&triangle.c] as u32,
                    ])
                })
                .collect::<Vec<_>>();

            pure_scene
                .navmeshes
                .add(fyrox::utils::navmesh::Navmesh::new(&triangles, &vertices));
        }

        pure_scene
    }

    pub fn save(&mut self, path: PathBuf, engine: &mut GameEngine) -> Result<String, String> {
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
                Ok(format!("Scene {} was successfully saved!", path.display()))
            }
        } else {
            writeln!(&mut reason, "\nPlease fix errors and try again.").unwrap();

            Err(reason)
        }
    }

    pub fn update(&mut self, engine: &mut Engine, dt: f32, settings: &Settings) {
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
            .update(&mut scene.graph, &settings.camera, dt);
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
            settings: &DebuggingSettings,
        ) {
            // Ignore editor nodes.
            if node == editor_scene.editor_objects_root {
                return;
            }

            let node = &graph[node];

            if settings.show_bounds {
                ctx.draw_oob(
                    &AxisAlignedBoundingBox::unit(),
                    node.global_transform(),
                    Color::opaque(255, 127, 39),
                );
            }

            if let Some(mesh) = node.cast::<Mesh>() {
                if settings.show_tbn {
                    // TODO: Add switch to settings to turn this on/off
                    let transform = node.global_transform();

                    for surface in mesh.surfaces() {
                        for vertex in surface.data().lock().vertex_buffer.iter() {
                            let len = 0.025;
                            let position = transform
                                .transform_point(&Point3::from(
                                    vertex.read_3_f32(VertexAttributeUsage::Position).unwrap(),
                                ))
                                .coords;
                            let vertex_tangent =
                                vertex.read_4_f32(VertexAttributeUsage::Tangent).unwrap();
                            let tangent = transform
                                .transform_vector(&vertex_tangent.xyz())
                                .normalize()
                                .scale(len);
                            let normal = transform
                                .transform_vector(
                                    &vertex
                                        .read_3_f32(VertexAttributeUsage::Normal)
                                        .unwrap()
                                        .xyz(),
                                )
                                .normalize()
                                .scale(len);
                            let binormal = tangent
                                .xyz()
                                .cross(&normal)
                                .scale(vertex_tangent.w)
                                .normalize()
                                .scale(len);

                            ctx.add_line(Line {
                                begin: position,
                                end: position + tangent,
                                color: Color::RED,
                            });

                            ctx.add_line(Line {
                                begin: position,
                                end: position + normal,
                                color: Color::BLUE,
                            });

                            ctx.add_line(Line {
                                begin: position,
                                end: position + binormal,
                                color: Color::GREEN,
                            });
                        }
                    }
                }
            } else if let Some(camera) = node.query_component_ref::<Camera>() {
                ctx.draw_frustum(
                    &Frustum::from(camera.view_projection_matrix()).unwrap_or_default(),
                    Color::ORANGE,
                );
            } else if let Some(light) = node.query_component_ref::<PointLight>() {
                ctx.draw_wire_sphere(light.global_position(), light.radius(), 30, Color::GREEN);
            } else if let Some(light) = node.query_component_ref::<SpotLight>() {
                ctx.draw_cone(
                    16,
                    (light.full_cone_angle() * 0.5).tan() * light.distance(),
                    light.distance(),
                    Matrix4::new_translation(&light.global_position())
                        * UnitQuaternion::from_matrix_eps(
                            &light.global_transform().basis(),
                            f32::EPSILON,
                            16,
                            UnitQuaternion::identity(),
                        )
                        .to_homogeneous()
                        * Matrix4::new_translation(&Vector3::new(
                            0.0,
                            -light.distance() * 0.5,
                            0.0,
                        )),
                    Color::GREEN,
                    false,
                );
            }

            for &child in node.children() {
                draw_recursively(child, graph, ctx, editor_scene, settings)
            }
        }

        // Draw pivots.
        draw_recursively(
            scene.graph.get_root(),
            &scene.graph,
            &mut scene.drawing_context,
            self,
            debug_settings,
        );

        let selection = if let Selection::Navmesh(ref selection) = self.selection {
            Some(selection)
        } else {
            None
        };
        self.navmeshes
            .draw(&mut scene.drawing_context, selection, &settings.navmesh);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    None,
    SoundContext,
    Graph(GraphSelection),
    Navmesh(NavmeshSelection),
    Effect(EffectSelection),
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
            Selection::SoundContext => false,
            Selection::Effect(effect) => effect.is_empty(),
            Selection::Absm(absm) => absm.is_empty(),
            Selection::Animation(animation) => animation.is_empty(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Selection::None => 0,
            Selection::Graph(graph) => graph.len(),
            Selection::Navmesh(navmesh) => navmesh.len(),
            Selection::SoundContext => 1,
            Selection::Effect(effect) => effect.len(),
            Selection::Absm(absm) => absm.len(),
            Selection::Animation(animation) => animation.len(),
        }
    }

    pub fn is_single_selection(&self) -> bool {
        self.len() == 1
    }
}
