use crate::{
    camera::CameraController,
    interaction::navmesh::{data_model::Navmesh, selection::NavmeshSelection},
    physics::Physics,
    scene::clipboard::Clipboard,
    sound::SoundSelection,
    utils, GameEngine,
};
use rg3d::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        math::Matrix4Ext,
        pool::{Handle, Pool},
        visitor::{Visit, Visitor},
    },
    scene::{graph::Graph, node::Node, Scene},
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

    pub fn is_single_selection(&self) -> bool {
        match self {
            Selection::None => false,
            Selection::Graph(graph) => graph.is_single_selection(),
            Selection::Navmesh(navmesh) => navmesh.is_single_selection(),
            Selection::Sound(sound) => sound.is_single_selection(),
        }
    }
}

#[derive(Debug, Default, Clone, Eq)]
pub struct GraphSelection {
    nodes: Vec<Handle<Node>>,
}

impl PartialEq for GraphSelection {
    fn eq(&self, other: &Self) -> bool {
        utils::is_slice_equal_permutation(self.nodes(), other.nodes())
    }
}

impl GraphSelection {
    pub fn from_list(nodes: Vec<Handle<Node>>) -> Self {
        Self {
            nodes: nodes.into_iter().filter(|h| h.is_some()).collect(),
        }
    }

    /// Creates new selection as single if node handle is not none, and empty if it is.
    pub fn single_or_empty(node: Handle<Node>) -> Self {
        if node.is_none() {
            Self {
                nodes: Default::default(),
            }
        } else {
            Self { nodes: vec![node] }
        }
    }

    /// Adds new selected node, or removes it if it is already in the list of selected nodes.
    pub fn insert_or_exclude(&mut self, handle: Handle<Node>) {
        if let Some(position) = self.nodes.iter().position(|&h| h == handle) {
            self.nodes.remove(position);
        } else {
            self.nodes.push(handle);
        }
    }

    pub fn contains(&self, handle: Handle<Node>) -> bool {
        self.nodes.iter().any(|&h| h == handle)
    }

    pub fn nodes(&self) -> &[Handle<Node>] {
        &self.nodes
    }

    pub fn is_multi_selection(&self) -> bool {
        self.nodes.len() > 1
    }

    pub fn is_single_selection(&self) -> bool {
        self.nodes.len() == 1
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn extend(&mut self, other: &GraphSelection) {
        self.nodes.extend_from_slice(&other.nodes)
    }

    pub fn root_nodes(&self, graph: &Graph) -> Vec<Handle<Node>> {
        // Helper function.
        fn is_descendant_of(handle: Handle<Node>, other: Handle<Node>, graph: &Graph) -> bool {
            for &child in graph[other].children() {
                if child == handle {
                    return true;
                }

                let inner = is_descendant_of(handle, child, graph);
                if inner {
                    return true;
                }
            }
            false
        }

        let mut root_nodes = Vec::new();
        for &node in self.nodes().iter() {
            let mut descendant = false;
            for &other_node in self.nodes().iter() {
                if is_descendant_of(node, other_node, graph) {
                    descendant = true;
                    break;
                }
            }
            if !descendant {
                root_nodes.push(node);
            }
        }
        root_nodes
    }

    pub fn global_rotation_position(
        &self,
        graph: &Graph,
    ) -> Option<(UnitQuaternion<f32>, Vector3<f32>)> {
        if self.is_single_selection() {
            Some(graph.global_rotation_position_no_scale(self.nodes[0]))
        } else if self.is_empty() {
            None
        } else {
            let mut position = Vector3::default();
            let mut rotation = graph.global_rotation(self.nodes[0]);
            let t = 1.0 / self.nodes.len() as f32;
            for &handle in self.nodes.iter() {
                let global_transform = graph[handle].global_transform();
                position += global_transform.position();
                rotation = rotation.slerp(&graph.global_rotation(self.nodes[0]), t);
            }
            position = position.scale(t);
            Some((rotation, position))
        }
    }

    pub fn offset(&self, graph: &mut Graph, offset: Vector3<f32>) {
        for &handle in self.nodes.iter() {
            let mut chain_scale = Vector3::new(1.0, 1.0, 1.0);
            let mut parent_handle = graph[handle].parent();
            while parent_handle.is_some() {
                let parent = &graph[parent_handle];
                let parent_scale = parent.local_transform().scale();
                chain_scale.x *= parent_scale.x;
                chain_scale.y *= parent_scale.y;
                chain_scale.z *= parent_scale.z;
                parent_handle = parent.parent();
            }

            let offset = Vector3::new(
                if chain_scale.x.abs() > 0.0 {
                    offset.x / chain_scale.x
                } else {
                    offset.x
                },
                if chain_scale.y.abs() > 0.0 {
                    offset.y / chain_scale.y
                } else {
                    offset.y
                },
                if chain_scale.z.abs() > 0.0 {
                    offset.z / chain_scale.z
                } else {
                    offset.z
                },
            );
            graph[handle].local_transform_mut().offset(offset);
        }
    }

    pub fn rotate(&self, graph: &mut Graph, rotation: UnitQuaternion<f32>) {
        for &handle in self.nodes.iter() {
            graph[handle].local_transform_mut().set_rotation(rotation);
        }
    }

    pub fn scale(&self, graph: &mut Graph, scale: Vector3<f32>) {
        for &handle in self.nodes.iter() {
            graph[handle].local_transform_mut().set_scale(scale);
        }
    }

    pub fn local_positions(&self, graph: &Graph) -> Vec<Vector3<f32>> {
        let mut positions = Vec::new();
        for &handle in self.nodes.iter() {
            positions.push(**graph[handle].local_transform().position());
        }
        positions
    }

    pub fn local_rotations(&self, graph: &Graph) -> Vec<UnitQuaternion<f32>> {
        let mut rotations = Vec::new();
        for &handle in self.nodes.iter() {
            rotations.push(**graph[handle].local_transform().rotation());
        }
        rotations
    }

    pub fn local_scales(&self, graph: &Graph) -> Vec<Vector3<f32>> {
        let mut scales = Vec::new();
        for &handle in self.nodes.iter() {
            scales.push(**graph[handle].local_transform().scale());
        }
        scales
    }
}
