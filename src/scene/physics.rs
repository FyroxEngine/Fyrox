//! Contains all structures and methods to operate with physics world.

use crate::scene::collider::GeometrySource;
use crate::{
    core::{algebra::Vector2, pool::Handle, visitor::prelude::*},
    engine::PhysicsBinder,
    physics3d::{
        body::RigidBodyContainer,
        collider::ColliderContainer,
        desc::{ColliderShapeDesc, PhysicsDesc},
        joint::JointContainer,
        rapier::{
            dynamics::{JointSet, RigidBodyBuilder, RigidBodySet, RigidBodyType},
            geometry::{Collider, ColliderBuilder, ColliderSet},
            na::{
                DMatrix, Dynamic, Isometry3, Point3, Translation, UnitQuaternion, VecStorage,
                Vector3,
            },
            parry::shape::SharedShape,
        },
        PhysicsWorld, RigidBodyHandle,
    },
    scene::{
        graph::Graph,
        mesh::buffer::{VertexAttributeUsage, VertexReadTrait},
        node::Node,
        terrain::Terrain,
    },
    utils::{
        log::{Log, MessageKind},
        raw_mesh::{RawMeshBuilder, RawVertex},
    },
};
use fxhash::FxHashMap;
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

/// Physics world.
#[derive(Debug)]
pub struct LegacyPhysics {
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

    /// Creates new trimesh collider shape from given mesh node. It also bakes scale into
    /// vertices of trimesh because rapier does not support collider scaling yet.
    pub fn make_trimesh(
        owner: Handle<Node>,
        nodes: Vec<GeometrySource>,
        graph: &Graph,
    ) -> SharedShape {
        let mut mesh_builder = RawMeshBuilder::new(0, 0);

        // Create inverse transform that will discard rotation and translation, but leave scaling and
        // other parameters of global transform.
        // When global transform of node is combined with this transform, we'll get relative transform
        // with scale baked in. We need to do this because root's transform will be synced with body's
        // but we don't want to bake entire transform including root's transform.
        let root_inv_transform = graph
            .isometric_global_transform(owner)
            .try_inverse()
            .unwrap();

        for source in nodes {
            if let Some(Node::Mesh(mesh)) = graph.try_get(source.0) {
                let global_transform = root_inv_transform * mesh.global_transform();

                for surface in mesh.surfaces() {
                    let shared_data = surface.data();
                    let shared_data = shared_data.lock();

                    let vertices = &shared_data.vertex_buffer;
                    for triangle in shared_data.geometry_buffer.iter() {
                        let a = RawVertex::from(
                            global_transform
                                .transform_point(&Point3::from(
                                    vertices
                                        .get(triangle[0] as usize)
                                        .unwrap()
                                        .read_3_f32(VertexAttributeUsage::Position)
                                        .unwrap(),
                                ))
                                .coords,
                        );
                        let b = RawVertex::from(
                            global_transform
                                .transform_point(&Point3::from(
                                    vertices
                                        .get(triangle[1] as usize)
                                        .unwrap()
                                        .read_3_f32(VertexAttributeUsage::Position)
                                        .unwrap(),
                                ))
                                .coords,
                        );
                        let c = RawVertex::from(
                            global_transform
                                .transform_point(&Point3::from(
                                    vertices
                                        .get(triangle[2] as usize)
                                        .unwrap()
                                        .read_3_f32(VertexAttributeUsage::Position)
                                        .unwrap(),
                                ))
                                .coords,
                        );

                        mesh_builder.insert(a);
                        mesh_builder.insert(b);
                        mesh_builder.insert(c);
                    }
                }
            }
        }

        let raw_mesh = mesh_builder.build();

        let vertices: Vec<Point3<f32>> = raw_mesh
            .vertices
            .into_iter()
            .map(|v| Point3::new(v.x, v.y, v.z))
            .collect();

        let indices = raw_mesh
            .triangles
            .into_iter()
            .map(|t| [t.0[0], t.0[1], t.0[2]])
            .collect::<Vec<_>>();

        if indices.is_empty() {
            Log::writeln(
                MessageKind::Warning,
                format!(
                    "Failed to create triangle mesh collider for {}, it has no vertices!",
                    graph[owner].name()
                ),
            );

            SharedShape::trimesh(vec![Point3::new(0.0, 0.0, 0.0)], vec![[0, 0, 0]])
        } else {
            SharedShape::trimesh(vertices, indices)
        }
    }

    /// Creates height field shape from given terrain.
    pub fn make_heightfield(terrain: &Terrain) -> SharedShape {
        assert!(!terrain.chunks_ref().is_empty());

        // Count rows and columns.
        let first_chunk = terrain.chunks_ref().first().unwrap();
        let chunk_size = Vector2::new(
            first_chunk.width_point_count(),
            first_chunk.length_point_count(),
        );
        let nrows = chunk_size.y * terrain.length_chunk_count() as u32;
        let ncols = chunk_size.x * terrain.width_chunk_count() as u32;

        // Combine height map of each chunk into bigger one.
        let mut ox = 0;
        let mut oz = 0;
        let mut data = vec![0.0; (nrows * ncols) as usize];
        for cz in 0..terrain.length_chunk_count() {
            for cx in 0..terrain.width_chunk_count() {
                let chunk = &terrain.chunks_ref()[cz * terrain.width_chunk_count() + cx];

                for z in 0..chunk.length_point_count() {
                    for x in 0..chunk.width_point_count() {
                        let value = chunk.heightmap()[(z * chunk.width_point_count() + x) as usize];
                        data[((ox + x) * nrows + oz + z) as usize] = value;
                    }
                }

                ox += chunk_size.x;
            }

            ox = 0;
            oz += chunk_size.y;
        }

        SharedShape::heightfield(
            DMatrix::from_data(VecStorage::new(
                Dynamic::new(nrows as usize),
                Dynamic::new(ncols as usize),
                data,
            )),
            Vector3::new(terrain.width(), 1.0, terrain.length()),
        )
    }

    /// Creates new height field collider from given terrain scene node.
    pub fn terrain_to_heightfield_collider(
        &mut self,
        terrain_handle: Handle<Node>,
        graph: &Graph,
    ) -> Collider {
        let terrain = graph[terrain_handle].as_terrain();
        let shape = Self::make_heightfield(terrain);
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

    /// Creates new height field rigid body from given terrain scene node.
    pub fn terrain_to_heightfield(
        &mut self,
        terrain_handle: Handle<Node>,
        graph: &Graph,
    ) -> RigidBodyHandle {
        let heightfield = self.terrain_to_heightfield_collider(terrain_handle, graph);
        let (global_rotation, global_position) =
            graph.isometric_global_rotation_position(terrain_handle);
        let body = RigidBodyBuilder::new(RigidBodyType::Static)
            .position(Isometry3 {
                rotation: global_rotation,
                translation: Translation {
                    vector: global_position,
                },
            })
            .build();
        let handle = self.add_body(body);
        self.add_collider(heightfield, &handle);
        handle
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
