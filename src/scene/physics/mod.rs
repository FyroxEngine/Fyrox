//! Contains all structures and methods to operate with physics world.

use crate::{
    core::{
        algebra::Vector2, color::Color, math::aabb::AxisAlignedBoundingBox, pool::Handle,
        visitor::prelude::*,
    },
    engine::PhysicsBinder,
    physics3d::{
        body::RigidBodyContainer,
        collider::ColliderContainer,
        desc::{ColliderDesc, ColliderShapeDesc, JointDesc, RigidBodyDesc},
        joint::JointContainer,
        rapier::{
            dynamics::{JointSet, RigidBodyBuilder, RigidBodySet, RigidBodyType},
            geometry::{Collider, ColliderBuilder, ColliderSet},
            na::{
                DMatrix, Dynamic, Isometry3, Point3, Translation, UnitQuaternion, VecStorage,
                Vector3,
            },
            parry::shape::{SharedShape, TriMesh},
        },
        ColliderHandle, JointHandle, PhysicsWorld, RigidBodyHandle,
    },
    resource::model::Model,
    scene::{
        debug::SceneDrawingContext,
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
use std::{
    collections::HashMap,
    fmt::Debug,
    ops::{Deref, DerefMut},
};

/// A set of data that has all associations with physics from resource.
/// It is used to embedding physics from resource to a scene during
/// the instantiation process.
#[derive(Default, Clone, Debug)]
pub struct ResourceLink {
    model: Model,
    // HandleInResource->HandleInInstance mappings
    bodies: HashMap<RigidBodyHandle, RigidBodyHandle>,
    colliders: HashMap<ColliderHandle, ColliderHandle>,
    joints: HashMap<JointHandle, JointHandle>,
}

impl Visit for ResourceLink {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.model.visit("Model", visitor)?;
        self.bodies.visit("Bodies", visitor)?;
        self.colliders.visit("Colliders", visitor)?;
        self.joints.visit("Visit", visitor)?;

        visitor.leave_region()
    }
}

/// Physics world.
#[derive(Debug)]
pub struct Physics {
    /// The physics world.
    pub world: PhysicsWorld,

    /// A list of external resources that were embedded in the physics during
    /// instantiation process.
    pub embedded_resources: Vec<ResourceLink>,
}

impl Deref for Physics {
    type Target = PhysicsWorld;

    fn deref(&self) -> &Self::Target {
        &self.world
    }
}

impl DerefMut for Physics {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.world
    }
}

impl Default for Physics {
    fn default() -> Self {
        Self::new()
    }
}

impl Physics {
    pub(in crate) fn new() -> Self {
        Self {
            world: PhysicsWorld::new(),
            embedded_resources: Default::default(),
        }
    }

    // Deep copy is performed using descriptors.
    pub(in crate) fn deep_copy(
        &self,
        binder: &PhysicsBinder<Node, RigidBodyHandle>,
        graph: &Graph,
    ) -> Self {
        let mut phys = Self::new();
        phys.embedded_resources = self.embedded_resources.clone();
        phys.desc = Some(self.generate_desc());
        phys.resolve(binder, graph);
        phys
    }

    /// Draws physics world. Very useful for debugging, it allows you to see where are
    /// rigid bodies, which colliders they have and so on.
    pub fn draw(&self, context: &mut SceneDrawingContext) {
        for body in self.bodies.iter() {
            context.draw_transform(body.position().to_homogeneous());
        }

        for collider in self.colliders.iter() {
            let body = self.bodies.native_ref(collider.parent().unwrap()).unwrap();
            let collider_local_transform = collider.position_wrt_parent().unwrap().to_homogeneous();
            let transform = body.position().to_homogeneous() * collider_local_transform;
            if let Some(trimesh) = collider.shape().as_trimesh() {
                let trimesh: &TriMesh = trimesh;
                for triangle in trimesh.triangles() {
                    let a = transform.transform_point(&triangle.a);
                    let b = transform.transform_point(&triangle.b);
                    let c = transform.transform_point(&triangle.c);
                    context.draw_triangle(
                        a.coords,
                        b.coords,
                        c.coords,
                        Color::opaque(200, 200, 200),
                    );
                }
            } else if let Some(cuboid) = collider.shape().as_cuboid() {
                let min = -cuboid.half_extents;
                let max = cuboid.half_extents;
                context.draw_oob(
                    &AxisAlignedBoundingBox::from_min_max(min, max),
                    transform,
                    Color::opaque(200, 200, 200),
                );
            } else if let Some(ball) = collider.shape().as_ball() {
                context.draw_sphere(
                    body.position().translation.vector,
                    10,
                    10,
                    ball.radius,
                    Color::opaque(200, 200, 200),
                );
            } else if let Some(cone) = collider.shape().as_cone() {
                context.draw_cone(
                    10,
                    cone.radius,
                    cone.half_height * 2.0,
                    transform,
                    Color::opaque(200, 200, 200),
                );
            } else if let Some(cylinder) = collider.shape().as_cylinder() {
                context.draw_cylinder(
                    10,
                    cylinder.radius,
                    cylinder.half_height * 2.0,
                    true,
                    transform,
                    Color::opaque(200, 200, 200),
                );
            } else if let Some(round_cylinder) = collider.shape().as_round_cylinder() {
                context.draw_cylinder(
                    10,
                    round_cylinder.base_shape.radius,
                    round_cylinder.base_shape.half_height * 2.0,
                    false,
                    transform,
                    Color::opaque(200, 200, 200),
                );
            } else if let Some(triangle) = collider.shape().as_triangle() {
                context.draw_triangle(
                    triangle.a.coords,
                    triangle.b.coords,
                    triangle.c.coords,
                    Color::opaque(200, 200, 200),
                );
            } else if let Some(capsule) = collider.shape().as_capsule() {
                context.draw_segment_capsule(
                    capsule.segment.a.coords,
                    capsule.segment.b.coords,
                    capsule.radius,
                    10,
                    10,
                    transform,
                    Color::opaque(200, 200, 200),
                );
            } else if let Some(heightfield) = collider.shape().as_heightfield() {
                for triangle in heightfield.triangles() {
                    let a = transform.transform_point(&triangle.a);
                    let b = transform.transform_point(&triangle.b);
                    let c = transform.transform_point(&triangle.c);
                    context.draw_triangle(
                        a.coords,
                        b.coords,
                        c.coords,
                        Color::opaque(200, 200, 200),
                    );
                }
            }
        }
    }

    /// Creates new trimesh collider shape from given mesh node. It also bakes scale into
    /// vertices of trimesh because rapier does not support collider scaling yet.
    pub fn make_trimesh(root: Handle<Node>, graph: &Graph) -> SharedShape {
        let mut mesh_builder = RawMeshBuilder::new(0, 0);

        // Create inverse transform that will discard rotation and translation, but leave scaling and
        // other parameters of global transform.
        // When global transform of node is combined with this transform, we'll get relative transform
        // with scale baked in. We need to do this because root's transform will be synced with body's
        // but we don't want to bake entire transform including root's transform.
        let root_inv_transform = graph
            .isometric_global_transform(root)
            .try_inverse()
            .unwrap();

        // Iterate over hierarchy of nodes and build one single trimesh.
        let mut stack = vec![root];
        while let Some(handle) = stack.pop() {
            let node = &graph[handle];
            if let Node::Mesh(mesh) = node {
                let global_transform = root_inv_transform * mesh.global_transform();

                for surface in mesh.surfaces() {
                    let shared_data = surface.data();
                    let shared_data = shared_data.read().unwrap();

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
            stack.extend_from_slice(node.children.as_slice());
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
                    graph[root].name()
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

    /// Small helper that creates static physics geometry from given mesh.
    ///
    /// # Notes
    ///
    /// This method *bakes* global transform of given mesh into static geometry
    /// data. So if given mesh was at some position with any rotation and scale
    /// resulting static geometry will have vertices that exactly matches given
    /// mesh.
    pub fn mesh_to_trimesh(&mut self, root: Handle<Node>, graph: &Graph) -> RigidBodyHandle {
        let shape = Self::make_trimesh(root, graph);
        let tri_mesh = ColliderBuilder::new(shape).friction(0.0).build();
        let (global_rotation, global_position) = graph.isometric_global_rotation_position(root);
        let body = RigidBodyBuilder::new(RigidBodyType::Static)
            .position(Isometry3 {
                rotation: global_rotation,
                translation: Translation {
                    vector: global_position,
                },
            })
            .build();
        let handle = self.add_body(body);
        self.add_collider(tri_mesh, &handle);
        handle
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
    ) {
        assert_eq!(self.bodies.len(), 0);
        assert_eq!(self.colliders.len(), 0);
        assert_eq!(self.joints.len(), 0);

        let mut phys_desc = self.desc.take().unwrap();

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
                    if let Some(associated_node) = binder.node_of(desc.parent) {
                        if graph.is_valid_handle(associated_node) {
                            // Restore data only for trimeshes.
                            let collider =
                                ColliderBuilder::new(Self::make_trimesh(associated_node, graph))
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
                    if let Some(associated_node) = binder.node_of(desc.parent) {
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

    pub(in crate) fn embed_resource(
        &mut self,
        target_binder: &mut PhysicsBinder<Node, RigidBodyHandle>,
        target_graph: &Graph,
        old_to_new: HashMap<Handle<Node>, Handle<Node>>,
        resource: Model,
    ) {
        let data = resource.data_ref();
        let resource_scene = data.get_scene();
        let resource_binder = &resource_scene.physics_binder;
        let resource_physics = &resource_scene.physics;
        let mut link = ResourceLink::default();

        // Instantiate rigid bodies.
        for (resource_handle, body) in resource_physics.bodies.inner_ref().iter() {
            let desc = RigidBodyDesc::<ColliderHandle>::from_body(
                body,
                resource_physics.colliders.handle_map(),
            );
            let new_handle = self.add_body(desc.convert_to_body());

            link.bodies.insert(
                resource_physics
                    .bodies
                    .handle_map()
                    .key_of(&resource_handle)
                    .cloned()
                    .unwrap(),
                new_handle,
            );
        }

        // Bind instantiated nodes with their respective rigid bodies from resource.
        for (handle, body) in resource_binder.forward_map().iter() {
            let new_handle = *old_to_new.get(handle).unwrap();
            let new_body = *link.bodies.get(body).unwrap();
            target_binder.bind(new_handle, new_body);
        }

        // Instantiate colliders.
        for (resource_handle, collider) in resource_physics.colliders.inner_ref().iter() {
            let desc = ColliderDesc::from_collider(collider, resource_physics.bodies.handle_map());
            // Remap handle from resource to one that was created above.
            let remapped_parent = *link.bodies.get(&desc.parent).unwrap();
            match desc.shape {
                ColliderShapeDesc::Trimesh(_) => {
                    if let Some(associated_node) = target_binder.node_of(remapped_parent) {
                        if target_graph.is_valid_handle(associated_node) {
                            let collider = ColliderBuilder::new(Self::make_trimesh(
                                associated_node,
                                target_graph,
                            ))
                            .build();
                            let new_handle = self.add_collider(collider, &remapped_parent);
                            link.colliders.insert(
                                new_handle,
                                resource_physics
                                    .colliders
                                    .handle_map()
                                    .key_of(&resource_handle)
                                    .cloned()
                                    .unwrap(),
                            );

                            Log::writeln(
                                MessageKind::Information,
                                format!(
                                    "Geometry for trimesh {:?} was restored from node at handle {:?}!",
                                    desc.parent, associated_node
                                ),
                            )
                        } else {
                            Log::writeln(MessageKind::Error, format!("Unable to get geometry for trimesh, node at handle {:?} does not exists!", associated_node))
                        }
                    }
                }
                ColliderShapeDesc::Heightfield(_) => {
                    if let Some(associated_node) = target_binder.node_of(remapped_parent) {
                        if let Some(Node::Terrain(_)) = target_graph.try_get(associated_node) {
                            let collider =
                                self.terrain_to_heightfield_collider(associated_node, target_graph);

                            let new_handle = self.add_collider(collider, &remapped_parent);
                            link.colliders.insert(
                                new_handle,
                                resource_physics
                                    .colliders
                                    .handle_map()
                                    .key_of(&resource_handle)
                                    .cloned()
                                    .unwrap(),
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
                             node at handle {:?} does not exists!",
                                    associated_node
                                ),
                            )
                        }
                    } else {
                        Log::writeln(
                            MessageKind::Information,
                            format!(
                                "Unable to restore geometry for height field {:?} because it has no associated node in the scene!",
                                desc.parent
                            ),
                        )
                    }
                }
                _ => {
                    let (new_collider, _) = desc.convert_to_collider();
                    let new_handle = self.add_collider(new_collider, &remapped_parent);
                    link.colliders.insert(
                        resource_physics
                            .colliders
                            .handle_map()
                            .key_of(&resource_handle)
                            .cloned()
                            .unwrap(),
                        new_handle,
                    );
                }
            }
        }

        // Instantiate joints.
        for (resource_handle, joint) in resource_physics.joints.inner_ref().iter() {
            let desc = JointDesc::<RigidBodyHandle>::from_joint(
                joint,
                resource_physics.bodies.handle_map(),
            );
            let new_body1_handle = link
                .bodies
                .get(self.bodies.handle_map().key_of(&joint.body1).unwrap())
                .unwrap();
            let new_body2_handle = link
                .bodies
                .get(self.bodies.handle_map().key_of(&joint.body2).unwrap())
                .unwrap();
            let new_handle = self.add_joint(new_body1_handle, new_body2_handle, desc.params);
            link.joints.insert(
                *resource_physics
                    .joints
                    .handle_map()
                    .key_of(&resource_handle)
                    .unwrap(),
                new_handle,
            );
        }

        self.embedded_resources.push(link);

        Log::writeln(
            MessageKind::Information,
            format!(
                "Resource {} was successfully embedded into physics world!",
                data.path.display()
            ),
        );
    }
}

impl Visit for Physics {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.world.visit("Desc", visitor)?;

        self.embedded_resources
            .visit("EmbeddedResources", visitor)?;

        visitor.leave_region()
    }
}
