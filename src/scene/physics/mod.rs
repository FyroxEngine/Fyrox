//! Contains all structures and methods to operate with physics world.

use crate::core::algebra::Vector2;
use crate::{
    core::{
        arrayvec::ArrayVec,
        color::Color,
        instant,
        math::{aabb::AxisAlignedBoundingBox, ray::Ray},
        pool::Handle,
        visitor::prelude::*,
        BiDirHashMap,
    },
    engine::{ColliderHandle, JointHandle, PhysicsBinder, RigidBodyHandle},
    resource::model::Model,
    scene::{
        graph::Graph,
        mesh::buffer::{VertexAttributeUsage, VertexReadTrait},
        node::Node,
        physics::{
            body::RigidBodyContainer,
            collider::ColliderContainer,
            desc::{ColliderDesc, ColliderShapeDesc, JointDesc, PhysicsDesc, RigidBodyDesc},
            joint::JointContainer,
        },
        terrain::Terrain,
        SceneDrawingContext,
    },
    utils::{
        log::{Log, MessageKind},
        raw_mesh::{RawMeshBuilder, RawVertex},
    },
};
use rapier3d::{
    dynamics::{
        CCDSolver, IntegrationParameters, IslandManager, Joint, JointParams, RigidBody,
        RigidBodyBuilder, RigidBodyType,
    },
    geometry::{BroadPhase, Collider, ColliderBuilder, InteractionGroups, NarrowPhase},
    na::{DMatrix, Dynamic, Isometry3, Point3, Translation, UnitQuaternion, VecStorage, Vector3},
    parry::shape::{FeatureId, SharedShape, TriMesh},
    pipeline::{EventHandler, PhysicsPipeline, QueryPipeline},
};
use std::{
    cell::{Cell, RefCell},
    cmp::Ordering,
    collections::HashMap,
    fmt::{Debug, Display, Formatter},
    time::Duration,
};

pub mod body;
pub mod collider;
pub mod desc;
pub mod joint;

/// A ray intersection result.
#[derive(Debug, Clone)]
pub struct Intersection {
    /// A handle of the collider with which intersection was detected.
    pub collider: ColliderHandle,

    /// A normal at the intersection position.
    pub normal: Vector3<f32>,

    /// A position of the intersection in world coordinates.
    pub position: Point3<f32>,

    /// Additional data that contains a kind of the feature with which
    /// intersection was detected as well as its index.    
    pub feature: FeatureId,

    /// Distance from the ray origin.
    pub toi: f32,
}

/// A set of options for the ray cast.
pub struct RayCastOptions {
    /// A ray for cast.
    pub ray: Ray,

    /// Maximum distance of cast.
    pub max_len: f32,

    /// Groups to check.
    pub groups: InteractionGroups,

    /// Whether to sort intersections from closest to farthest.
    pub sort_results: bool,
}

/// A set of data that has all associations with physics from resource.
/// It is used to embedding physics from resource to a scene during
/// the instantiation process.
#[derive(Default, Clone)]
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
pub struct Physics {
    /// Current physics pipeline.
    pipeline: PhysicsPipeline,
    /// Current gravity vector. Default is (0.0, -9.81, 0.0)
    pub gravity: Vector3<f32>,
    /// A set of parameters that define behavior of every rigid body.
    pub integration_parameters: IntegrationParameters,
    /// Broad phase performs rough intersection checks.
    pub broad_phase: BroadPhase,
    /// Narrow phase is responsible for precise contact generation.
    pub narrow_phase: NarrowPhase,
    /// A continuous collision detection solver.
    pub ccd_solver: CCDSolver,
    /// Structure responsible for maintaining the set of active rigid-bodies, and putting non-moving
    /// rigid-bodies to sleep to save computation times.
    pub islands: IslandManager,

    /// A container of rigid bodies.
    pub bodies: RigidBodyContainer,

    /// A container of colliders.
    pub colliders: ColliderContainer,

    /// A container of joints.
    pub joints: JointContainer,

    /// Event handler collects info about contacts and proximity events.
    pub event_handler: Box<dyn EventHandler>,

    /// Descriptors have two purposes:
    /// 1) Defer deserialization to resolve stage - the stage where all meshes
    ///    were loaded and there is a possibility to obtain data for trimeshes.
    ///    Resolve stage will drain these vectors. This is normal use case.
    /// 2) Save data from editor: when descriptors are set, only they will be
    ///    written to output. This is a HACK, but I don't know better solution
    ///    yet.
    pub desc: Option<PhysicsDesc>,

    /// A list of external resources that were embedded in the physics during
    /// instantiation process.
    pub embedded_resources: Vec<ResourceLink>,

    query: RefCell<QueryPipeline>,

    pub(in crate) performance_statistics: PhysicsPerformanceStatistics,
}

impl Debug for Physics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Physics")
    }
}

impl Default for Physics {
    fn default() -> Self {
        Self::new()
    }
}

/// A trait for ray cast results storage. It has two implementations: Vec and ArrayVec.
/// Latter is needed for the cases where you need to avoid runtime memory allocations
/// and do everything on stack.
pub trait QueryResultsStorage {
    /// Pushes new intersection in the storage. Returns true if intersection was
    /// successfully inserted, false otherwise.
    fn push(&mut self, intersection: Intersection) -> bool;

    /// Clears the storage.
    fn clear(&mut self);

    /// Sorts intersections by given compare function.
    fn sort_intersections_by<C: FnMut(&Intersection, &Intersection) -> Ordering>(&mut self, cmp: C);
}

impl QueryResultsStorage for Vec<Intersection> {
    fn push(&mut self, intersection: Intersection) -> bool {
        self.push(intersection);
        true
    }

    fn clear(&mut self) {
        self.clear()
    }

    fn sort_intersections_by<C>(&mut self, cmp: C)
    where
        C: FnMut(&Intersection, &Intersection) -> Ordering,
    {
        self.sort_by(cmp);
    }
}

impl<const CAP: usize> QueryResultsStorage for ArrayVec<Intersection, CAP> {
    fn push(&mut self, intersection: Intersection) -> bool {
        self.try_push(intersection).is_ok()
    }

    fn clear(&mut self) {
        self.clear()
    }

    fn sort_intersections_by<C>(&mut self, cmp: C)
    where
        C: FnMut(&Intersection, &Intersection) -> Ordering,
    {
        self.sort_by(cmp);
    }
}

/// Performance statistics for the physics part of the engine.
#[derive(Debug, Default, Clone)]
pub struct PhysicsPerformanceStatistics {
    /// A time that was needed to perform a single simulation step.
    pub step_time: Duration,

    /// A time that was needed to perform all ray casts.
    pub total_ray_cast_time: Cell<Duration>,
}

impl Display for PhysicsPerformanceStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Physics Step Time: {:?}\nPhysics Ray Cast Time: {:?}",
            self.step_time,
            self.total_ray_cast_time.get(),
        )
    }
}

impl PhysicsPerformanceStatistics {
    pub(in crate) fn reset(&mut self) {
        *self = Default::default();
    }
}

impl Physics {
    pub(in crate) fn new() -> Self {
        Self {
            pipeline: PhysicsPipeline::new(),
            gravity: Vector3::new(0.0, -9.81, 0.0),
            integration_parameters: IntegrationParameters::default(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            ccd_solver: CCDSolver::new(),
            islands: IslandManager::new(),
            bodies: RigidBodyContainer::new(),
            colliders: ColliderContainer::new(),
            joints: JointContainer::new(),
            event_handler: Box::new(()),
            query: Default::default(),
            desc: Default::default(),
            embedded_resources: Default::default(),
            performance_statistics: Default::default(),
        }
    }

    // Deep copy is performed using descriptors.
    pub(in crate) fn deep_copy(&self, binder: &PhysicsBinder<Node>, graph: &Graph) -> Self {
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

    /// Tries to get a parent of collider.
    pub fn collider_parent(&self, collider: &ColliderHandle) -> Option<&RigidBodyHandle> {
        self.colliders
            .get(collider)
            .and_then(|c| self.bodies.handle_map().key_of(&c.parent().unwrap()))
    }

    pub(in crate) fn step(&mut self) {
        let time = instant::Instant::now();

        self.pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies.set,
            &mut self.colliders.set,
            &mut self.joints.set,
            &mut self.ccd_solver,
            &(),
            &*self.event_handler,
        );

        self.performance_statistics.step_time += instant::Instant::now() - time;
    }

    #[doc(hidden)]
    pub fn generate_desc(&self) -> PhysicsDesc {
        let body_dense_map = self
            .bodies
            .set
            .iter()
            .enumerate()
            .map(|(i, (h, _))| {
                (
                    h,
                    rapier3d::dynamics::RigidBodyHandle::from_raw_parts(i as u32, 0),
                )
            })
            .collect::<HashMap<_, _>>();

        let mut body_handle_map = BiDirHashMap::default();
        for (engine_handle, rapier_handle) in self.bodies.handle_map().forward_map() {
            body_handle_map.insert(*engine_handle, body_dense_map[rapier_handle]);
        }

        let collider_dense_map = self
            .colliders
            .set
            .iter()
            .enumerate()
            .map(|(i, (h, _))| {
                (
                    h,
                    rapier3d::geometry::ColliderHandle::from_raw_parts(i as u32, 0),
                )
            })
            .collect::<HashMap<_, _>>();

        let mut collider_handle_map = BiDirHashMap::default();
        for (engine_handle, rapier_handle) in self.colliders.handle_map().forward_map() {
            collider_handle_map.insert(*engine_handle, collider_dense_map[rapier_handle]);
        }

        let joint_dense_map = self
            .joints
            .set
            .iter()
            .enumerate()
            .map(|(i, (h, _))| {
                (
                    h,
                    rapier3d::dynamics::JointHandle::from_raw_parts(i as u32, 0),
                )
            })
            .collect::<HashMap<_, _>>();

        let mut joint_handle_map = BiDirHashMap::default();
        for (engine_handle, rapier_handle) in self.joints.handle_map.forward_map() {
            joint_handle_map.insert(*engine_handle, joint_dense_map[rapier_handle]);
        }

        PhysicsDesc {
            integration_parameters: self.integration_parameters.into(),

            bodies: self
                .bodies
                .iter()
                .map(|b| RigidBodyDesc::from_body(b, &self.colliders.handle_map()))
                .collect::<Vec<_>>(),

            colliders: self
                .colliders
                .iter()
                .map(|c| ColliderDesc::from_collider(c, &self.bodies.handle_map()))
                .collect::<Vec<_>>(),

            gravity: self.gravity,

            joints: self
                .joints
                .iter()
                .map(|j| JointDesc::from_joint(j, &self.bodies.handle_map()))
                .collect::<Vec<_>>(),

            body_handle_map,
            collider_handle_map,
            joint_handle_map,
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

    /// Creates new heightfield rigid body from given terrain scene node.
    pub fn terrain_to_heightfield(
        &mut self,
        terrain_handle: Handle<Node>,
        graph: &Graph,
    ) -> RigidBodyHandle {
        let terrain = graph[terrain_handle].as_terrain();
        let shape = Self::make_heightfield(terrain);
        let heightfield = ColliderBuilder::new(shape)
            .position(Isometry3 {
                rotation: UnitQuaternion::default(),
                translation: Translation {
                    vector: Vector3::new(terrain.width() * 0.5, 0.0, terrain.length() * 0.5),
                },
            })
            .friction(0.0)
            .build();
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

    /// Casts a ray with given options.
    pub fn cast_ray<S: QueryResultsStorage>(&self, opts: RayCastOptions, query_buffer: &mut S) {
        let time = instant::Instant::now();

        let mut query = self.query.borrow_mut();

        // TODO: Ideally this must be called once per frame, but it seems to be impossible because
        // a body can be deleted during the consecutive calls of this method which will most
        // likely end up in panic because of invalid handle stored in internal acceleration
        // structure. This could be fixed by delaying deleting of bodies/collider to the end
        // of the frame.
        query.update(&self.islands, &self.bodies.set, &self.colliders.set);

        query_buffer.clear();
        let ray = rapier3d::geometry::Ray::new(
            Point3::from(opts.ray.origin),
            opts.ray
                .dir
                .try_normalize(std::f32::EPSILON)
                .unwrap_or_default(),
        );
        query.intersections_with_ray(
            &self.colliders.set,
            &ray,
            opts.max_len,
            true,
            opts.groups,
            None, // TODO
            |handle, intersection| {
                query_buffer.push(Intersection {
                    collider: self
                        .colliders
                        .handle_map()
                        .key_of(&handle)
                        .cloned()
                        .unwrap(),
                    normal: intersection.normal,
                    position: ray.point_at(intersection.toi),
                    feature: intersection.feature,
                    toi: intersection.toi,
                })
            },
        );
        if opts.sort_results {
            query_buffer.sort_intersections_by(|a, b| {
                if a.toi > b.toi {
                    Ordering::Greater
                } else if a.toi < b.toi {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            })
        }

        self.performance_statistics.total_ray_cast_time.set(
            self.performance_statistics.total_ray_cast_time.get()
                + (instant::Instant::now() - time),
        );
    }

    pub(in crate) fn resolve(&mut self, binder: &PhysicsBinder<Node>, graph: &Graph) {
        assert_eq!(self.bodies.len(), 0);
        assert_eq!(self.colliders.len(), 0);

        let mut phys_desc = self.desc.take().unwrap();

        self.bodies.handle_map = phys_desc.body_handle_map;
        self.colliders.handle_map = phys_desc.collider_handle_map;
        self.joints.handle_map = phys_desc.joint_handle_map;

        self.integration_parameters = phys_desc.integration_parameters.into();

        for desc in phys_desc.bodies.drain(..) {
            self.bodies.set.insert(desc.convert_to_body());
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
                            self.colliders.set.insert_with_parent(
                                collider,
                                self.bodies
                                    .handle_map()
                                    .value_of(&desc.parent)
                                    .cloned()
                                    .unwrap(),
                                &mut self.bodies.set,
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
                            if let Node::Terrain(terrain) = &graph[associated_node] {
                                let heightfield = Self::make_heightfield(terrain);

                                let collider = ColliderBuilder::new(heightfield).build();

                                self.colliders.set.insert_with_parent(
                                    collider,
                                    self.bodies
                                        .handle_map()
                                        .value_of(&desc.parent)
                                        .cloned()
                                        .unwrap(),
                                    &mut self.bodies.set,
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
                    self.colliders.set.insert_with_parent(
                        collider,
                        self.bodies.handle_map().value_of(&parent).cloned().unwrap(),
                        &mut self.bodies.set,
                    );
                }
            }
        }

        for desc in phys_desc.joints.drain(..) {
            let b1 = self
                .bodies
                .handle_map
                .value_of(&desc.body1)
                .cloned()
                .unwrap();
            let b2 = self
                .bodies
                .handle_map
                .value_of(&desc.body2)
                .cloned()
                .unwrap();
            self.joints
                .set
                .insert(&mut self.bodies.set, b1, b2, desc.params);
        }
    }

    pub(in crate) fn embed_resource(
        &mut self,
        target_binder: &mut PhysicsBinder<Node>,
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
        for (resource_handle, body) in resource_physics.bodies.set.iter() {
            let desc = RigidBodyDesc::<ColliderHandle>::from_body(
                body,
                &resource_physics.colliders.handle_map(),
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
        for (resource_handle, collider) in resource_physics.colliders.set.iter() {
            let desc = ColliderDesc::from_collider(collider, &resource_physics.bodies.handle_map());
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
                        if let Some(Node::Terrain(terrain)) = target_graph.try_get(associated_node)
                        {
                            let collider =
                                ColliderBuilder::new(Self::make_heightfield(terrain)).build();
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
        for (resource_handle, joint) in resource_physics.joints.set.iter() {
            let desc = JointDesc::<RigidBodyHandle>::from_joint(
                joint,
                &resource_physics.bodies.handle_map(),
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
                    .handle_map
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

    /// Adds new rigid body.
    pub fn add_body(&mut self, rigid_body: RigidBody) -> RigidBodyHandle {
        self.bodies.add(rigid_body)
    }

    /// Removes a rigid body.
    pub fn remove_body(&mut self, rigid_body: &RigidBodyHandle) -> Option<RigidBody> {
        self.bodies.remove(
            rigid_body,
            &mut self.colliders,
            &mut self.joints,
            &mut self.islands,
        )
    }

    /// Adds new collider.
    pub fn add_collider(
        &mut self,
        collider: Collider,
        rigid_body: &RigidBodyHandle,
    ) -> ColliderHandle {
        self.colliders.add(collider, rigid_body, &mut self.bodies)
    }

    /// Removes a collider.
    pub fn remove_collider(&mut self, collider_handle: &ColliderHandle) -> Option<Collider> {
        self.colliders
            .remove(collider_handle, &mut self.bodies, &mut self.islands)
    }

    /// Adds new joint.
    pub fn add_joint<J>(
        &mut self,
        body1: &RigidBodyHandle,
        body2: &RigidBodyHandle,
        joint_params: J,
    ) -> JointHandle
    where
        J: Into<JointParams>,
    {
        self.joints
            .add(body1, body2, joint_params, &mut self.bodies)
    }

    /// Removes a joint.
    pub fn remove_joint(&mut self, joint_handle: &JointHandle, wake_up: bool) -> Option<Joint> {
        self.joints
            .remove(joint_handle, &mut self.bodies, &mut self.islands, wake_up)
    }
}

impl Visit for Physics {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut desc = if visitor.is_reading() {
            Default::default()
        } else if let Some(desc) = self.desc.as_ref() {
            desc.clone()
        } else {
            self.generate_desc()
        };
        desc.visit("Desc", visitor)?;

        self.embedded_resources
            .visit("EmbeddedResources", visitor)?;

        // Save descriptors for resolve stage.
        if visitor.is_reading() {
            self.desc = Some(desc);
        }

        visitor.leave_region()
    }
}
