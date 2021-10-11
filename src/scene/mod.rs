use crate::{
    camera::CameraController,
    interaction::navmesh::{data_model::Navmesh, selection::NavmeshSelection},
    physics::Physics,
    scene::clipboard::Clipboard,
    world::{
        graph::selection::GraphSelection,
        physics::selection::{JointSelection, RigidBodySelection},
        sound::selection::SoundSelection,
    },
    GameEngine,
};
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
    // Editor uses split data model - some parts of scene are editable directly,
    // but some parts are not because of incompatible data model.
    pub physics: Physics,
    pub navmeshes: Pool<Navmesh>,
}

impl EditorScene {
    pub fn save(&mut self, path: PathBuf, engine: &mut GameEngine) -> Result<String, String> {
        let scene = &mut engine.scenes[self.scene];

        // Validate first.
        let mut valid = true;
        let mut reason = "Scene is not saved, because validation failed:\n".to_owned();

        for joint in self.physics.joints.iter() {
            if joint.body1.is_none() || joint.body2.is_none() {
                let mut associated_node = Handle::NONE;
                for (&node, &body) in self.physics.binder.forward_map().iter() {
                    if body == joint.body1.into() {
                        associated_node = node;
                        break;
                    }
                }

                writeln!(
                    &mut reason,
                    "Invalid joint on node {} ({}:{}). Associated body is missing!",
                    scene.graph[associated_node].name(),
                    associated_node.index(),
                    associated_node.generation()
                )
                .unwrap();
                valid = false;
            }
        }

        if valid {
            self.path = Some(path.clone());

            let editor_root = self.root;
            let (mut pure_scene, old_to_new) = scene.clone(&mut |node, _| node != editor_root);

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

            let (desc, binder) = self.physics.generate_engine_desc();
            pure_scene.physics.desc = Some(desc);
            pure_scene.physics_binder.enabled = true;
            pure_scene.physics_binder.clear();
            for (node, body) in binder {
                pure_scene
                    .physics_binder
                    .bind(*old_to_new.get(&node).unwrap(), body);
            }
            let mut visitor = Visitor::new();
            pure_scene.visit("Scene", &mut visitor).unwrap();
            if let Err(e) = visitor.save_binary(&path) {
                Err(format!("Failed to save scene! Reason: {}", e.to_string()))
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
    RigidBody(RigidBodySelection),
    Joint(JointSelection),
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
            Selection::RigidBody(rb) => rb.bodies().is_empty(),
            Selection::Joint(joint) => joint.joints().is_empty(),
        }
    }

    pub fn is_single_selection(&self) -> bool {
        match self {
            Selection::None => false,
            Selection::Graph(graph) => graph.is_single_selection(),
            Selection::Navmesh(navmesh) => navmesh.is_single_selection(),
            Selection::Sound(sound) => sound.is_single_selection(),
            Selection::RigidBody(rb) => rb.is_single_selection(),
            Selection::Joint(joint) => joint.is_single_selection(),
        }
    }
}
