use crate::interaction::navmesh::data_model::{NavmeshTriangle, NavmeshVertex};
use crate::{
    camera::CameraController,
    interaction::navmesh::{data_model::Navmesh, selection::NavmeshSelection},
    scene::clipboard::Clipboard,
    world::{graph::selection::GraphSelection, sound::selection::SoundSelection},
    GameEngine,
};
use rg3d::engine::Engine;
use rg3d::scene::base::BaseBuilder;
use rg3d::{
    core::{
        pool::{Handle, Pool},
        visitor::{Visit, Visitor},
    },
    scene::{node::Node, Scene},
    sound::math::TriangleDefinition,
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
        let root = BaseBuilder::new().build(&mut scene.graph);
        let camera_controller = CameraController::new(&mut scene.graph, root);

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

    pub fn save(&mut self, path: PathBuf, engine: &mut GameEngine) -> Result<String, String> {
        let scene = &mut engine.scenes[self.scene];

        // Validate first.
        let valid = true;
        let mut reason = "Scene is not saved, because validation failed:\n".to_owned();

        if valid {
            self.path = Some(path.clone());

            let editor_root = self.root;
            let (mut pure_scene, _) = scene.clone(&mut |node, _| node != editor_root);

            // Reset state of nodes. For some nodes (such as particles systems) we use scene as preview
            // so before saving scene, we have to reset state of such nodes.
            for node in pure_scene.graph.linear_iter_mut() {
                if let Node::ParticleSystem(particle_system) = node {
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
                    .add(rg3d::utils::navmesh::Navmesh::new(&triangles, &vertices));
            }

            let mut visitor = Visitor::new();
            pure_scene.visit("Scene", &mut visitor).unwrap();
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    None,
    Graph(GraphSelection),
    Navmesh(NavmeshSelection),
    Sound(SoundSelection),
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
            Selection::Sound(sound) => sound.sources().is_empty(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Selection::None => 0,
            Selection::Graph(graph) => graph.len(),
            Selection::Navmesh(navmesh) => navmesh.len(),
            Selection::Sound(sound) => sound.len(),
        }
    }

    pub fn is_single_selection(&self) -> bool {
        self.len() == 1
    }
}
