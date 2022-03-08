use crate::{
    audio::EffectSelection,
    camera::CameraController,
    interaction::navmesh::{
        data_model::{Navmesh, NavmeshTriangle, NavmeshVertex},
        selection::NavmeshSelection,
    },
    scene::clipboard::Clipboard,
    settings::debugging::DebuggingSettings,
    world::graph::selection::GraphSelection,
    GameEngine,
};
use fyrox::{
    core::{
        algebra::Point3,
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, TriangleDefinition},
        pool::{Handle, Pool},
        visitor::Visitor,
    },
    engine::Engine,
    scene::{
        base::BaseBuilder,
        debug::{Line, SceneDrawingContext},
        graph::Graph,
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

#[macro_use]
pub mod commands;

pub struct EditorScene {
    pub path: Option<PathBuf>,
    pub scene: Handle<Scene>,
    // Handle to a root for all editor nodes.
    pub root: Handle<Node>,
    pub selection: Selection,
    pub clipboard: Clipboard,
    pub camera_controller: CameraController,
    pub navmeshes: Pool<Navmesh>,
}

impl EditorScene {
    pub fn from_native_scene(mut scene: Scene, engine: &mut Engine, path: Option<PathBuf>) -> Self {
        let root = PivotBuilder::new(BaseBuilder::new()).build(&mut scene.graph);
        let camera_controller = CameraController::new(&mut scene.graph, root);

        // Prevent physics simulation in while editing scene.
        scene.graph.physics.enabled = false;
        scene.graph.physics2d.enabled = false;

        let mut navmeshes = Pool::new();

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
            root,
            camera_controller,
            navmeshes,
            scene: engine.scenes.add(scene),
            selection: Default::default(),
            clipboard: Default::default(),
        }
    }

    pub fn make_purified_scene(&self, engine: &mut GameEngine) -> Scene {
        let scene = &mut engine.scenes[self.scene];

        let editor_root = self.root;
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

    pub fn draw_debug(&mut self, engine: &mut Engine, settings: &DebuggingSettings) {
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

        if settings.show_physics {
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
            if node == editor_scene.root {
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
            settings,
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    None,
    SoundContext,
    Graph(GraphSelection),
    Navmesh(NavmeshSelection),
    Effect(EffectSelection),
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
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Selection::None => 0,
            Selection::Graph(graph) => graph.len(),
            Selection::Navmesh(navmesh) => navmesh.len(),
            Selection::SoundContext => 1,
            Selection::Effect(effect) => effect.len(),
        }
    }

    pub fn is_single_selection(&self) -> bool {
        self.len() == 1
    }
}

#[macro_export]
macro_rules! define_vec_add_remove_commands {
    (struct $add_name:ident, $remove_name:ident<$model_ty:ty, $value_ty:ty> ($self:ident, $context:ident)$get_container:block) => {
        #[derive(Debug)]
        pub struct $add_name {
            pub handle: Handle<$model_ty>,
            pub value: $value_ty,
        }

        impl Command for $add_name {
            fn name(&mut self, _: &SceneContext) -> String {
                stringify!($add_name).to_owned()
            }

            fn execute(&mut $self, $context: &mut SceneContext) {
                $get_container.push(std::mem::take(&mut $self.value));
            }

            fn revert(&mut $self, $context: &mut SceneContext) {
                $self.value = $get_container.pop().unwrap();
            }
        }

        #[derive(Debug)]
        pub struct $remove_name {
            pub handle: Handle<$model_ty>,
            pub index: usize,
            pub value: Option<$value_ty>,
        }

        impl Command for $remove_name {
            fn name(&mut self, _: &SceneContext) -> String {
                stringify!($remove_name).to_owned()
            }

            fn execute(&mut $self, $context: &mut SceneContext) {
                $self.value = Some($get_container.remove($self.index));
            }

            fn revert(&mut $self, $context: &mut SceneContext) {
                $get_container.insert($self.index, $self.value.take().unwrap());
            }
        }
    };
}
