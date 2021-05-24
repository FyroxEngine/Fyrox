use crate::{
    physics::{Collider, Joint, Physics, RigidBody},
    scene::GraphSelection,
    GameEngine,
};
use rg3d::{
    core::pool::Handle,
    scene::{graph::Graph, node::Node, Scene},
};
use std::collections::HashMap;

pub struct Clipboard {
    graph: Graph,
    physics: Physics,
    empty: bool,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self {
            graph: Graph::new(),
            physics: Default::default(),
            empty: true,
        }
    }
}

#[derive(Default, Debug)]
pub struct DeepCloneResult {
    pub root_nodes: Vec<Handle<Node>>,
    pub colliders: Vec<Handle<Collider>>,
    pub bodies: Vec<Handle<RigidBody>>,
    pub joints: Vec<Handle<Joint>>,
    pub binder: HashMap<Handle<Node>, Handle<RigidBody>>,
}

fn deep_clone_nodes(
    root_nodes: &[Handle<Node>],
    source_graph: &Graph,
    source_physics: &Physics,
    dest_graph: &mut Graph,
    dest_physics: &mut Physics,
) -> DeepCloneResult {
    let mut result = DeepCloneResult::default();

    let mut old_new_mapping = HashMap::new();

    for &root_node in root_nodes.iter() {
        let (_, old_to_new) = source_graph.copy_node(root_node, dest_graph, &mut |_, _| true);
        // Merge mappings.
        for (old, new) in old_to_new {
            old_new_mapping.insert(old, new);
        }
    }

    result.root_nodes = root_nodes
        .iter()
        .map(|n| *old_new_mapping.get(n).unwrap())
        .collect::<Vec<_>>();

    // Copy associated bodies, colliders, joints.
    for &root_node in root_nodes.iter() {
        for descendant in source_graph.traverse_handle_iter(root_node) {
            // Copy body too if we have any.
            if let Some(&body) = source_physics.binder.value_of(&descendant) {
                let body = &source_physics.bodies[body];
                let mut body_clone = body.clone();
                body_clone.colliders.clear();
                let body_clone_handle = dest_physics.bodies.spawn(body_clone);

                result.bodies.push(body_clone_handle);

                // Also copy colliders.
                for &collider in body.colliders.iter() {
                    let mut collider_clone = source_physics.colliders[collider.into()].clone();
                    collider_clone.parent = body_clone_handle.into();
                    let collider_clone_handle = dest_physics.colliders.spawn(collider_clone);
                    dest_physics.bodies[body_clone_handle]
                        .colliders
                        .push(collider_clone_handle.into());

                    result.colliders.push(collider_clone_handle);
                }

                let new_node = *old_new_mapping.get(&descendant).unwrap();
                result.binder.insert(new_node, body_clone_handle);
                dest_physics.binder.insert(new_node, body_clone_handle);
            }
        }
    }

    // TODO: Add joints.
    // Joint will be copied only if both of its associated bodies are copied too.

    result
}

impl Clipboard {
    pub fn fill_from_selection(
        &mut self,
        selection: &GraphSelection,
        scene_handle: Handle<Scene>,
        physics: &Physics,
        engine: &GameEngine,
    ) {
        self.clear();

        let scene = &engine.scenes[scene_handle];

        let root_nodes = selection.root_nodes(&scene.graph);

        deep_clone_nodes(
            &root_nodes,
            &scene.graph,
            physics,
            &mut self.graph,
            &mut self.physics,
        );

        self.empty = false;
    }

    pub fn paste(&mut self, dest_graph: &mut Graph, dest_physics: &mut Physics) -> DeepCloneResult {
        assert_ne!(self.empty, true);

        deep_clone_nodes(
            self.graph[self.graph.get_root()].children(),
            &self.graph,
            &self.physics,
            dest_graph,
            dest_physics,
        )
    }

    pub fn is_empty(&self) -> bool {
        self.empty
    }

    pub fn clear(&mut self) {
        self.empty = true;
        self.graph = Graph::new();
        self.physics = Default::default();
    }
}
