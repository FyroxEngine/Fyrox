//! Contains all structures and methods to operate with physics world.

use crate::{
    core::{pool::Handle, visitor::prelude::*},
    engine::PhysicsBinder,
    physics3d::{
        legacy::body::RigidBodyContainer,
        legacy::collider::ColliderContainer,
        legacy::desc::{ColliderShapeDesc, PhysicsDesc},
        legacy::joint::JointContainer,
        legacy::PhysicsWorld,
        legacy::RigidBodyHandle,
        rapier::{
            dynamics::{JointSet, RigidBodySet},
            geometry::{Collider, ColliderBuilder, ColliderSet},
            na::{
                DMatrix, Dynamic, Isometry3, Point3, Translation, UnitQuaternion, VecStorage,
                Vector3,
            },
            parry::shape::SharedShape,
        },
    },
    scene::{graph::Graph, node::Node},
    utils::log::{Log, MessageKind},
};
use fxhash::FxHashMap;
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

/// Physics world.
#[derive(Debug)]
pub(crate) struct LegacyPhysics {
    /// The physics world.
    pub world: PhysicsWorld,

    /// Legacy physics descriptor.
    pub desc: Option<PhysicsDesc>,
}

impl Visit for LegacyPhysics {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        if !visitor.is_reading() {
            return VisitResult::Err(VisitError::User(
                "Serialization of legacy physics is prohibited!".to_string(),
            ));
        }

        visitor.enter_region(name)?;

        let mut desc = PhysicsDesc::default();
        desc.visit("Desc", visitor)?;

        // Save descriptors for resolve stage.
        self.desc = Some(desc);

        visitor.leave_region()
    }
}

impl Deref for LegacyPhysics {
    type Target = PhysicsWorld;

    fn deref(&self) -> &Self::Target {
        &self.world
    }
}

impl DerefMut for LegacyPhysics {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.world
    }
}

impl Default for LegacyPhysics {
    fn default() -> Self {
        Self::new()
    }
}

impl LegacyPhysics {
    pub(in crate) fn new() -> Self {
        Self {
            world: PhysicsWorld::new(),
            desc: None,
        }
    }

    /// Creates new height field collider from given terrain scene node.
    pub fn terrain_to_heightfield_collider(
        &mut self,
        terrain_handle: Handle<Node>,
        graph: &Graph,
    ) -> Collider {
        let terrain = graph[terrain_handle].as_terrain();
        let shape = SharedShape::heightfield(
            DMatrix::from_data(VecStorage::new(Dynamic::new(1), Dynamic::new(1), vec![0.0])),
            Vector3::new(1.0, 1.0, 1.0),
        );
        ColliderBuilder::new(shape)
            .position(Isometry3 {
                rotation: UnitQuaternion::default(),
                translation: Translation {
                    vector: Vector3::new(terrain.width() * 0.5, 0.0, terrain.length() * 0.5),
                },
            })
            .friction(0.0)
            .build()
    }

    pub(in crate) fn resolve(
        &mut self,
        binder: &PhysicsBinder<Node, RigidBodyHandle>,
        graph: &Graph,
        old_to_new_mapping: Option<&FxHashMap<Handle<Node>, Handle<Node>>>,
    ) {
        assert_eq!(self.bodies.len(), 0);
        assert_eq!(self.colliders.len(), 0);
        assert_eq!(self.joints.len(), 0);

        let mut phys_desc = if let Some(old) = self.desc.take() {
            old
        } else {
            return;
        };

        assert_eq!(phys_desc.bodies.len(), phys_desc.body_handle_map.len());
        assert_eq!(
            phys_desc.colliders.len(),
            phys_desc.collider_handle_map.len()
        );
        assert_eq!(phys_desc.joints.len(), phys_desc.joint_handle_map.len());

        self.integration_parameters = phys_desc.integration_parameters.into();

        let mut bodies = RigidBodySet::new();
        let mut colliders = ColliderSet::new();
        let mut joints = JointSet::new();

        for desc in phys_desc.bodies.drain(..) {
            bodies.insert(desc.convert_to_body());
        }

        for desc in phys_desc.colliders.drain(..) {
            match desc.shape {
                ColliderShapeDesc::Trimesh(_) => {
                    // Trimeshes are special: we never store data for them, but only getting correct
                    // one from associated mesh in the scene.
                    if let Some(mut associated_node) = binder.node_of(desc.parent) {
                        if let Some(old_to_new_mapping) = old_to_new_mapping {
                            associated_node = *old_to_new_mapping
                                .get(&associated_node)
                                .expect("Old to new mapping must have corresponding node!");
                        }

                        if graph.is_valid_handle(associated_node) {
                            // Restore data only for trimeshes.
                            let collider = ColliderBuilder::new(SharedShape::trimesh(
                                vec![Point3::new(0.0, 0.0, 0.0)],
                                vec![[0, 0, 0]],
                            ))
                            .build();
                            colliders.insert_with_parent(
                                collider,
                                phys_desc
                                    .body_handle_map
                                    .value_of(&desc.parent)
                                    .cloned()
                                    .unwrap(),
                                &mut bodies,
                            );

                            Log::writeln(
                                MessageKind::Information,
                                format!(
                                    "Geometry for trimesh {:?} was restored from node at handle {:?}!",
                                    desc.parent, associated_node
                                ),
                            )
                        } else {
                            Log::writeln(
                                MessageKind::Error,
                                format!(
                                    "Unable to get geometry for trimesh,\
                             node at handle {:?} does not exists!",
                                    associated_node
                                ),
                            )
                        }
                    }
                }
                ColliderShapeDesc::Heightfield(_) => {
                    // Height fields are special: we never store data for them, but only getting correct
                    // one from associated terrain in the scene.
                    if let Some(mut associated_node) = binder.node_of(desc.parent) {
                        if let Some(old_to_new_mapping) = old_to_new_mapping {
                            associated_node = *old_to_new_mapping
                                .get(&associated_node)
                                .expect("Old to new mapping must have corresponding node!");
                        }

                        if graph.is_valid_handle(associated_node) {
                            if let Node::Terrain(_) = &graph[associated_node] {
                                let collider =
                                    self.terrain_to_heightfield_collider(associated_node, graph);

                                colliders.insert_with_parent(
                                    collider,
                                    phys_desc
                                        .body_handle_map
                                        .value_of(&desc.parent)
                                        .cloned()
                                        .unwrap(),
                                    &mut bodies,
                                );

                                Log::writeln(
                                    MessageKind::Information,
                                    format!(
                                        "Geometry for height field {:?} was restored from node at handle {:?}!",
                                        desc.parent, associated_node
                                    ),
                                )
                            } else {
                                Log::writeln(
                                    MessageKind::Error,
                                    format!(
                                        "Unable to get geometry for height field,\
                                 node at handle {:?} is not a terrain!",
                                        associated_node
                                    ),
                                )
                            }
                        } else {
                            Log::writeln(
                                MessageKind::Error,
                                format!(
                                    "Unable to get geometry for height field,\
                            node at handle {:?} does not exists!",
                                    associated_node
                                ),
                            )
                        }
                    }
                }
                // Rest of colliders are independent.
                _ => {
                    let (collider, parent) = desc.convert_to_collider();
                    colliders.insert_with_parent(
                        collider,
                        phys_desc
                            .body_handle_map
                            .value_of(&parent)
                            .cloned()
                            .unwrap(),
                        &mut bodies,
                    );
                }
            }
        }

        for desc in phys_desc.joints.drain(..) {
            let b1 = phys_desc
                .body_handle_map
                .value_of(&desc.body1)
                .cloned()
                .unwrap();
            let b2 = phys_desc
                .body_handle_map
                .value_of(&desc.body2)
                .cloned()
                .unwrap();
            joints.insert(b1, b2, desc.params);
        }

        self.bodies =
            RigidBodyContainer::from_raw_parts(bodies, phys_desc.body_handle_map).unwrap();
        self.colliders =
            ColliderContainer::from_raw_parts(colliders, phys_desc.collider_handle_map).unwrap();
        self.joints = JointContainer::from_raw_parts(joints, phys_desc.joint_handle_map).unwrap();
    }
}
