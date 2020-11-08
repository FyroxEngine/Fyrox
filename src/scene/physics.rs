//! Contains all structures and methods to operate with physics world.

use crate::{
    core::{
        math::ray::Ray,
        visitor::{Visit, VisitResult, Visitor},
    },
    physics::math::AngVector,
    scene::{graph::Graph, mesh::Mesh, node::Node, ColliderHandle, PhysicsBinder, RigidBodyHandle},
    utils::{
        log::Log,
        raw_mesh::{RawMeshBuilder, RawVertex},
    },
};
use rapier3d::na::Translation3;
use rapier3d::{
    data::arena::Index,
    dynamics::{
        BodyStatus, IntegrationParameters, JointSet, RigidBody, RigidBodyBuilder, RigidBodySet,
    },
    geometry::{
        BroadPhase, Collider, ColliderBuilder, ColliderSet, ColliderShape, InteractionGroups,
        NarrowPhase, Segment, Shape,
    },
    na::{DMatrix, Dynamic, Isometry3, Point3, Translation, UnitQuaternion, VecStorage, Vector3},
    ncollide::{query, shape::FeatureId},
    pipeline::{EventHandler, PhysicsPipeline, QueryPipeline},
};
use std::{
    cmp::Ordering,
    fmt::{Debug, Formatter},
};

/// A ray intersection result.
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

/// Physics world.
pub struct Physics {
    /// Current physics pipeline.
    pub pipeline: PhysicsPipeline,
    /// Current gravity vector. Default is (0.0, -9.81, 0.0)
    pub gravity: Vector3<f32>,
    /// A set of parameters that define behavior of every rigid body.
    pub integration_parameters: IntegrationParameters,
    /// Broad phase performs rough intersection checks.
    pub broad_phase: BroadPhase,
    /// Narrow phase is responsible for precise contact generation.
    pub narrow_phase: NarrowPhase,
    /// Set of bodies in the physics world.
    pub bodies: RigidBodySet,
    /// Set of colliders in the physics world.
    pub colliders: ColliderSet,
    /// Set of joints in the physics world.
    pub joints: JointSet,
    /// Event handler collects info about contacts and proximity events.
    pub event_handler: Box<dyn EventHandler>,

    query: QueryPipeline,

    /// Descriptors have two purposes:
    /// 1) Defer deserialization to resolve stage - the stage where all meshes
    ///    were loaded and there is a possibility to obtain data for trimeshes.
    ///    Resolve stage will drain these vectors. This is normal use case.
    /// 2) Save data from editor: when descriptors are set, only they will be
    ///    written to output. This is a HACK, but I don't know better solution
    ///    yet.
    pub desc: Option<PhysicsDesc>,
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

impl Physics {
    pub(in crate) fn new() -> Self {
        Self {
            pipeline: PhysicsPipeline::new(),
            gravity: Vector3::new(0.0, -9.81, 0.0),
            integration_parameters: IntegrationParameters::default(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            joints: JointSet::new(),
            event_handler: Box::new(()),
            query: Default::default(),
            desc: Default::default(),
        }
    }

    // Deep copy is performed using descriptors.
    pub(in crate) fn deep_copy(&self, binder: &PhysicsBinder, graph: &Graph) -> Self {
        let mut phys = Self::new();
        phys.desc = Some(self.generate_desc());
        phys.resolve(binder, graph);
        phys
    }

    pub(in crate) fn step(&mut self) {
        self.query.update(&self.bodies, &self.colliders);

        self.pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.joints,
            None,
            None,
            &*self.event_handler,
        );
    }

    #[doc(hidden)]
    pub fn generate_desc(&self) -> PhysicsDesc {
        PhysicsDesc {
            integration_parameters: self.integration_parameters.clone().into(),

            bodies: self
                .bodies
                .iter()
                .map(|(_, b)| RigidBodyDesc::from_body(b))
                .collect::<Vec<_>>(),

            colliders: self
                .colliders
                .iter()
                .map(|(_, c)| ColliderDesc::from_collider(c))
                .collect::<Vec<_>>(),

            gravity: self.gravity,
        }
    }

    /// Creates new trimesh collider shape from given mesh node. It also bakes global transform into
    /// vertices of trimesh.
    pub fn make_trimesh(mesh: &Mesh) -> ColliderShape {
        let mut mesh_builder = RawMeshBuilder::new(0, 0);

        let mut global_transform = mesh.global_transform();

        global_transform.data[12] = 0.0;
        global_transform.data[13] = 0.0;
        global_transform.data[14] = 0.0;

        for surface in mesh.surfaces() {
            let shared_data = surface.data();
            let shared_data = shared_data.lock().unwrap();

            let vertices = shared_data.get_vertices();
            for triangle in shared_data.triangles() {
                let a = RawVertex::from(
                    global_transform
                        .transform_point(&Point3::from(vertices[triangle[0] as usize].position))
                        .coords,
                );
                let b = RawVertex::from(
                    global_transform
                        .transform_point(&Point3::from(vertices[triangle[1] as usize].position))
                        .coords,
                );
                let c = RawVertex::from(
                    global_transform
                        .transform_point(&Point3::from(vertices[triangle[2] as usize].position))
                        .coords,
                );

                mesh_builder.insert(a);
                mesh_builder.insert(b);
                mesh_builder.insert(c);
            }
        }

        let raw_mesh = mesh_builder.build();

        let vertices = raw_mesh
            .vertices
            .into_iter()
            .map(|v| Point3::new(v.x, v.y, v.z))
            .collect();

        let indices = raw_mesh
            .triangles
            .into_iter()
            .map(|t| Point3::new(t.0[0], t.0[1], t.0[2]))
            .collect();

        ColliderShape::trimesh(vertices, indices)
    }

    /// Small helper that creates static physics geometry from given mesh.
    ///
    /// # Notes
    ///
    /// This method *bakes* global transform of given mesh into static geometry
    /// data. So if given mesh was at some position with any rotation and scale
    /// resulting static geometry will have vertices that exactly matches given
    /// mesh.
    pub fn mesh_to_trimesh(&mut self, mesh: &Mesh) -> RigidBodyHandle {
        let shape = Self::make_trimesh(mesh);
        let tri_mesh = ColliderBuilder::new(shape).build();
        let position = mesh.global_position();
        let body = RigidBodyBuilder::new(BodyStatus::Static)
            .translation(position.x, position.y, position.z)
            .build();
        let handle = self.bodies.insert(body);
        self.colliders.insert(tri_mesh, handle, &mut self.bodies);
        handle.into()
    }

    /// Casts a ray with given options.
    pub fn cast_ray(&self, opts: RayCastOptions, query_buffer: &mut Vec<Intersection>) {
        query_buffer.clear();
        let ray = query::Ray::new(
            Point3::from(opts.ray.origin),
            opts.ray
                .dir
                .try_normalize(std::f32::EPSILON)
                .unwrap_or_default(),
        );
        self.query.interferences_with_ray(
            &self.colliders,
            &ray,
            opts.max_len,
            opts.groups,
            |handle, _, intersection| {
                query_buffer.push(Intersection {
                    collider: handle.into(),
                    normal: intersection.normal,
                    position: ray.point_at(intersection.toi),
                    feature: intersection.feature,
                    toi: intersection.toi,
                });
                true
            },
        );
        if opts.sort_results {
            query_buffer.sort_by(|a, b| {
                if a.toi > b.toi {
                    Ordering::Greater
                } else if a.toi < b.toi {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            })
        }
    }

    pub(in crate) fn resolve(&mut self, binder: &PhysicsBinder, graph: &Graph) {
        assert_eq!(self.bodies.len(), 0);
        assert_eq!(self.colliders.len(), 0);

        let mut phys_desc = self.desc.take().unwrap();

        for desc in phys_desc.bodies.drain(..) {
            self.bodies.insert(desc.convert_to_body());
        }

        for desc in phys_desc.colliders.drain(..) {
            if let ColliderShapeDesc::Trimesh(_) = desc.shape {
                // Trimeshes are special: we never store data for them, but only getting correct
                // one from associated mesh in the scene.
                if let Some(associated_node) = binder.node_of(desc.parent) {
                    if graph.is_valid_handle(associated_node) {
                        if let Node::Mesh(mesh) = &graph[associated_node] {
                            // Restore data only for trimeshes.
                            let collider = ColliderBuilder::new(Self::make_trimesh(mesh)).build();
                            self.colliders
                                .insert(collider, desc.parent.into(), &mut self.bodies);

                            Log::writeln(format!(
                                "Geometry for trimesh {:?} was restored from node at handle {:?}!",
                                desc.parent, associated_node
                            ))
                        } else {
                            Log::writeln(format!("Unable to get geometry for trimesh, node at handle {:?} is not a mesh!", associated_node))
                        }
                    } else {
                        Log::writeln(format!("Unable to get geometry for trimesh, node at handle {:?} does not exists!", associated_node))
                    }
                }
            } else {
                let (collider, parent) = desc.convert_to_collider();
                self.colliders
                    .insert(collider, parent.into(), &mut self.bodies);
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(u32)]
#[doc(hidden)]
pub enum BodyStatusDesc {
    Dynamic = 0,
    Static = 1,
    Kinematic = 2,
}

impl Default for BodyStatusDesc {
    fn default() -> Self {
        Self::Dynamic
    }
}

impl BodyStatusDesc {
    fn id(self) -> u32 {
        self as u32
    }

    fn from_id(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(Self::Dynamic),
            1 => Ok(Self::Static),
            2 => Ok(Self::Kinematic),
            _ => Err(format!("Invalid body status id {}!", id)),
        }
    }
}

impl Visit for BodyStatusDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut id = self.id();
        id.visit(name, visitor)?;
        if visitor.is_reading() {
            *self = Self::from_id(id)?;
        }
        Ok(())
    }
}

impl From<BodyStatus> for BodyStatusDesc {
    fn from(s: BodyStatus) -> Self {
        match s {
            BodyStatus::Dynamic => Self::Dynamic,
            BodyStatus::Static => Self::Static,
            BodyStatus::Kinematic => Self::Kinematic,
        }
    }
}

impl Into<BodyStatus> for BodyStatusDesc {
    fn into(self) -> BodyStatus {
        match self {
            BodyStatusDesc::Dynamic => BodyStatus::Dynamic,
            BodyStatusDesc::Static => BodyStatus::Static,
            BodyStatusDesc::Kinematic => BodyStatus::Kinematic,
        }
    }
}

#[derive(Default, Clone, Debug)]
#[doc(hidden)]
pub struct RigidBodyDesc<C> {
    pub position: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
    pub linvel: Vector3<f32>,
    pub angvel: Vector3<f32>,
    pub sleeping: bool,
    pub status: BodyStatusDesc,
    pub colliders: Vec<C>,
}

impl<C: From<Index>> RigidBodyDesc<C> {
    #[doc(hidden)]
    pub fn from_body(body: &RigidBody) -> Self {
        Self {
            position: body.position.translation.vector,
            rotation: body.position.rotation,
            linvel: body.linvel,
            angvel: body.angvel,
            status: body.body_status.into(),
            sleeping: body.is_sleeping(),
            colliders: body.colliders().iter().map(|&c| C::from(c)).collect(),
        }
    }

    fn convert_to_body(self) -> RigidBody {
        let mut body = RigidBodyBuilder::new(self.status.into())
            .position(Isometry3 {
                translation: Translation {
                    vector: self.position,
                },
                rotation: self.rotation,
            })
            .linvel(self.linvel.x, self.linvel.y, self.linvel.z)
            .angvel(AngVector::new(self.angvel.x, self.angvel.y, self.angvel.z))
            .build();
        if self.sleeping {
            body.sleep();
        }
        body
    }
}

impl<C: Visit + Default + 'static> Visit for RigidBodyDesc<C> {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.position.visit("Position", visitor)?;
        self.rotation.visit("Rotation", visitor)?;
        self.linvel.visit("LinVel", visitor)?;
        self.angvel.visit("AngVel", visitor)?;
        self.sleeping.visit("Sleeping", visitor)?;
        self.status.visit("Status", visitor)?;
        self.colliders.visit("Colliders", visitor)?;

        visitor.leave_region()
    }
}

#[derive(Default, Copy, Clone, Debug)]
#[doc(hidden)]
pub struct BallDesc {
    pub radius: f32,
}

impl Visit for BallDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.radius.visit("Radius", visitor)?;

        visitor.leave_region()
    }
}

#[derive(Default, Copy, Clone, Debug)]
#[doc(hidden)]
pub struct CylinderDesc {
    pub half_height: f32,
    pub radius: f32,
}

impl Visit for CylinderDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.radius.visit("Radius", visitor)?;
        self.half_height.visit("HalfHeight", visitor)?;

        visitor.leave_region()
    }
}

#[derive(Default, Copy, Clone, Debug)]
#[doc(hidden)]
pub struct RoundCylinderDesc {
    pub half_height: f32,
    pub radius: f32,
    pub border_radius: f32,
}

impl Visit for RoundCylinderDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.radius.visit("Radius", visitor)?;
        self.half_height.visit("HalfHeight", visitor)?;
        self.border_radius.visit("BorderRadius", visitor)?;

        visitor.leave_region()
    }
}

#[derive(Default, Copy, Clone, Debug)]
#[doc(hidden)]
pub struct ConeDesc {
    pub half_height: f32,
    pub radius: f32,
}

impl Visit for ConeDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.radius.visit("Radius", visitor)?;
        self.half_height.visit("HalfHeight", visitor)?;

        visitor.leave_region()
    }
}

#[derive(Default, Copy, Clone, Debug)]
#[doc(hidden)]
pub struct CuboidDesc {
    pub half_extents: Vector3<f32>,
}

impl Visit for CuboidDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.half_extents.visit("HalfExtents", visitor)?;

        visitor.leave_region()
    }
}

#[derive(Default, Copy, Clone, Debug)]
#[doc(hidden)]
pub struct CapsuleDesc {
    pub begin: Vector3<f32>,
    pub end: Vector3<f32>,
    pub radius: f32,
}

impl Visit for CapsuleDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.begin.visit("Begin", visitor)?;
        self.end.visit("End", visitor)?;
        self.radius.visit("Radius", visitor)?;

        visitor.leave_region()
    }
}

#[derive(Default, Copy, Clone, Debug)]
#[doc(hidden)]
pub struct SegmentDesc {
    pub begin: Vector3<f32>,
    pub end: Vector3<f32>,
}

impl Visit for SegmentDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.begin.visit("Begin", visitor)?;
        self.end.visit("End", visitor)?;

        visitor.leave_region()
    }
}

#[derive(Default, Copy, Clone, Debug)]
#[doc(hidden)]
pub struct TriangleDesc {
    pub a: Vector3<f32>,
    pub b: Vector3<f32>,
    pub c: Vector3<f32>,
}

impl Visit for TriangleDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.a.visit("A", visitor)?;
        self.b.visit("B", visitor)?;
        self.c.visit("C", visitor)?;

        visitor.leave_region()
    }
}

// TODO: for now data of trimesh and heightfield is not serializable.
//  In most cases it is ok, because PhysicsBinder allows to automatically
//  obtain data from associated mesh.
#[derive(Default, Copy, Clone, Debug)]
#[doc(hidden)]
pub struct TrimeshDesc;

impl Visit for TrimeshDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;
        visitor.leave_region()
    }
}

#[derive(Default, Copy, Clone, Debug)]
#[doc(hidden)]
pub struct HeightfieldDesc;

impl Visit for HeightfieldDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;
        visitor.leave_region()
    }
}

#[derive(Copy, Clone, Debug)]
#[doc(hidden)]
pub enum ColliderShapeDesc {
    Ball(BallDesc),
    Cylinder(CylinderDesc),
    RoundCylinder(RoundCylinderDesc),
    Cone(ConeDesc),
    Cuboid(CuboidDesc),
    Capsule(CapsuleDesc),
    Segment(SegmentDesc),
    Triangle(TriangleDesc),
    Trimesh(TrimeshDesc),
    Heightfield(HeightfieldDesc),
}

impl Default for ColliderShapeDesc {
    fn default() -> Self {
        Self::Ball(Default::default())
    }
}

impl ColliderShapeDesc {
    #[doc(hidden)]
    pub fn id(&self) -> u32 {
        match self {
            ColliderShapeDesc::Ball(_) => 0,
            ColliderShapeDesc::Cylinder(_) => 1,
            ColliderShapeDesc::RoundCylinder(_) => 2,
            ColliderShapeDesc::Cone(_) => 3,
            ColliderShapeDesc::Cuboid(_) => 4,
            ColliderShapeDesc::Capsule(_) => 5,
            ColliderShapeDesc::Segment(_) => 6,
            ColliderShapeDesc::Triangle(_) => 7,
            ColliderShapeDesc::Trimesh(_) => 8,
            ColliderShapeDesc::Heightfield(_) => 9,
        }
    }

    fn from_id(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(ColliderShapeDesc::Ball(Default::default())),
            1 => Ok(ColliderShapeDesc::Cylinder(Default::default())),
            2 => Ok(ColliderShapeDesc::RoundCylinder(Default::default())),
            3 => Ok(ColliderShapeDesc::Cone(Default::default())),
            4 => Ok(ColliderShapeDesc::Cuboid(Default::default())),
            5 => Ok(ColliderShapeDesc::Capsule(Default::default())),
            6 => Ok(ColliderShapeDesc::Segment(Default::default())),
            7 => Ok(ColliderShapeDesc::Triangle(Default::default())),
            8 => Ok(ColliderShapeDesc::Trimesh(Default::default())),
            9 => Ok(ColliderShapeDesc::Heightfield(Default::default())),
            _ => Err(format!("Invalid collider shape desc id {}!", id)),
        }
    }

    #[doc(hidden)]
    pub fn from_collider_shape(shape: &dyn Shape) -> Self {
        if let Some(ball) = shape.as_ball() {
            ColliderShapeDesc::Ball(BallDesc {
                radius: ball.radius,
            })
        } else if let Some(cylinder) = shape.as_cylinder() {
            ColliderShapeDesc::Cylinder(CylinderDesc {
                half_height: cylinder.half_height,
                radius: cylinder.half_height,
            })
        } else if let Some(round_cylinder) = shape.as_round_cylinder() {
            ColliderShapeDesc::RoundCylinder(RoundCylinderDesc {
                half_height: round_cylinder.cylinder.half_height,
                radius: round_cylinder.cylinder.radius,
                border_radius: round_cylinder.border_radius,
            })
        } else if let Some(cone) = shape.as_cone() {
            ColliderShapeDesc::Cone(ConeDesc {
                half_height: cone.half_height,
                radius: cone.radius,
            })
        } else if let Some(cuboid) = shape.as_cuboid() {
            ColliderShapeDesc::Cuboid(CuboidDesc {
                half_extents: cuboid.half_extents,
            })
        } else if let Some(capsule) = shape.as_capsule() {
            ColliderShapeDesc::Capsule(CapsuleDesc {
                begin: capsule.segment.a.coords,
                end: capsule.segment.b.coords,
                radius: capsule.radius,
            })
        } else if let Some(segment) = shape.downcast_ref::<Segment>() {
            ColliderShapeDesc::Segment(SegmentDesc {
                begin: segment.a.coords,
                end: segment.b.coords,
            })
        } else if let Some(triangle) = shape.as_triangle() {
            ColliderShapeDesc::Triangle(TriangleDesc {
                a: triangle.a.coords,
                b: triangle.b.coords,
                c: triangle.c.coords,
            })
        } else if shape.as_trimesh().is_some() {
            ColliderShapeDesc::Trimesh(TrimeshDesc)
        } else if shape.as_heightfield().is_some() {
            ColliderShapeDesc::Heightfield(HeightfieldDesc)
        } else {
            unreachable!()
        }
    }

    fn into_collider_shape(self) -> ColliderShape {
        match self {
            ColliderShapeDesc::Ball(ball) => ColliderShape::ball(ball.radius),
            ColliderShapeDesc::Cylinder(cylinder) => {
                ColliderShape::cylinder(cylinder.half_height, cylinder.radius)
            }
            ColliderShapeDesc::RoundCylinder(rcylinder) => ColliderShape::round_cylinder(
                rcylinder.half_height,
                rcylinder.radius,
                rcylinder.border_radius,
            ),
            ColliderShapeDesc::Cone(cone) => ColliderShape::cone(cone.half_height, cone.radius),
            ColliderShapeDesc::Cuboid(cuboid) => ColliderShape::cuboid(cuboid.half_extents),
            ColliderShapeDesc::Capsule(capsule) => ColliderShape::capsule(
                Point3::from(capsule.begin),
                Point3::from(capsule.end),
                capsule.radius,
            ),
            ColliderShapeDesc::Segment(segment) => {
                ColliderShape::segment(Point3::from(segment.begin), Point3::from(segment.end))
            }
            ColliderShapeDesc::Triangle(triangle) => ColliderShape::triangle(
                Point3::from(triangle.a),
                Point3::from(triangle.b),
                Point3::from(triangle.c),
            ),
            ColliderShapeDesc::Trimesh(_) => {
                // Create fake trimesh. It will be filled with actual data on resolve stage later on.
                let a = Point3::new(0.0, 0.0, 1.0);
                let b = Point3::new(1.0, 0.0, 1.0);
                let c = Point3::new(1.0, 0.0, 0.0);
                ColliderShape::trimesh(vec![a, b, c], vec![Point3::new(0, 1, 2)])
            }
            ColliderShapeDesc::Heightfield(_) => ColliderShape::heightfield(
                DMatrix::from_data(VecStorage::new(
                    Dynamic::new(2),
                    Dynamic::new(2),
                    vec![0.0, 1.0, 0.0, 0.0],
                )),
                Default::default(),
            ),
        }
    }
}

impl Visit for ColliderShapeDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut id: u32 = if visitor.is_reading() { 0 } else { self.id() };
        id.visit("Id", visitor)?;
        if visitor.is_reading() {
            *self = Self::from_id(id)?;
        }
        match self {
            ColliderShapeDesc::Ball(v) => v.visit(name, visitor)?,
            ColliderShapeDesc::Cylinder(v) => v.visit(name, visitor)?,
            ColliderShapeDesc::RoundCylinder(v) => v.visit(name, visitor)?,
            ColliderShapeDesc::Cone(v) => v.visit(name, visitor)?,
            ColliderShapeDesc::Cuboid(v) => v.visit(name, visitor)?,
            ColliderShapeDesc::Capsule(v) => v.visit(name, visitor)?,
            ColliderShapeDesc::Segment(v) => v.visit(name, visitor)?,
            ColliderShapeDesc::Triangle(v) => v.visit(name, visitor)?,
            ColliderShapeDesc::Trimesh(v) => v.visit(name, visitor)?,
            ColliderShapeDesc::Heightfield(v) => v.visit(name, visitor)?,
        }

        visitor.leave_region()
    }
}

#[derive(Default, Clone, Debug)]
#[doc(hidden)]
pub struct ColliderDesc<R> {
    pub shape: ColliderShapeDesc,
    pub parent: R,
    pub friction: f32,
    pub density: f32,
    pub restitution: f32,
    pub is_sensor: bool,
    pub translation: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
    pub collision_groups: u32,
    pub solver_groups: u32,
}

impl<R: From<Index>> ColliderDesc<R> {
    fn from_collider(collider: &Collider) -> Self {
        Self {
            shape: ColliderShapeDesc::from_collider_shape(collider.shape()),
            parent: R::from(collider.parent()),
            friction: collider.friction,
            density: collider.density(),
            restitution: collider.restitution,
            is_sensor: collider.is_sensor(),
            translation: collider.position().translation.vector,
            rotation: collider.position().rotation,
            collision_groups: collider.collision_groups().0,
            solver_groups: collider.solver_groups().0,
        }
    }

    fn convert_to_collider(self) -> (Collider, R) {
        (
            ColliderBuilder::new(self.shape.into_collider_shape())
                .friction(self.friction)
                .restitution(self.restitution)
                .density(self.density)
                .position(Isometry3 {
                    translation: Translation3 {
                        vector: self.translation,
                    },
                    rotation: self.rotation,
                })
                .solver_groups(InteractionGroups(self.solver_groups))
                .collision_groups(InteractionGroups(self.collision_groups))
                .sensor(self.is_sensor)
                .build(),
            self.parent,
        )
    }
}

impl<R: 'static + Visit + Default> Visit for ColliderDesc<R> {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.shape.visit("Shape", visitor)?;
        self.parent.visit("Parent", visitor)?;
        self.friction.visit("Friction", visitor)?;
        self.density.visit("Density", visitor)?;
        self.restitution.visit("Restitution", visitor)?;
        self.is_sensor.visit("IsSensor", visitor)?;
        self.translation.visit("Translation", visitor)?;
        self.rotation.visit("Rotation", visitor)?;
        self.collision_groups.visit("CollisionGroups", visitor)?;
        self.solver_groups.visit("SolverGroups", visitor)?;

        visitor.leave_region()
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

        // Save descriptors for resolve stage.
        if visitor.is_reading() {
            self.desc = Some(desc);
        }

        visitor.leave_region()
    }
}

// Almost full copy of rapier's IntegrationParameters
#[derive(Default, Clone, Debug)]
#[doc(hidden)]
pub struct IntegrationParametersDesc {
    pub dt: f32,
    pub return_after_ccd_substep: bool,
    pub erp: f32,
    pub joint_erp: f32,
    pub warmstart_coeff: f32,
    pub restitution_velocity_threshold: f32,
    pub allowed_linear_error: f32,
    pub prediction_distance: f32,
    pub allowed_angular_error: f32,
    pub max_linear_correction: f32,
    pub max_angular_correction: f32,
    pub max_stabilization_multiplier: f32,
    pub max_velocity_iterations: u32,
    pub max_position_iterations: u32,
    pub min_island_size: u32,
    pub max_ccd_position_iterations: u32,
    pub max_ccd_substeps: u32,
    pub multiple_ccd_substep_sensor_events_enabled: bool,
    pub ccd_on_penetration_enabled: bool,
}

impl From<IntegrationParameters> for IntegrationParametersDesc {
    fn from(params: IntegrationParameters) -> Self {
        Self {
            dt: params.dt(),
            return_after_ccd_substep: params.return_after_ccd_substep,
            erp: params.erp,
            joint_erp: params.joint_erp,
            warmstart_coeff: params.warmstart_coeff,
            restitution_velocity_threshold: params.restitution_velocity_threshold,
            allowed_linear_error: params.allowed_linear_error,
            prediction_distance: params.prediction_distance,
            allowed_angular_error: params.allowed_angular_error,
            max_linear_correction: params.max_linear_correction,
            max_angular_correction: params.max_angular_correction,
            max_stabilization_multiplier: params.max_stabilization_multiplier,
            max_velocity_iterations: params.max_velocity_iterations as u32,
            max_position_iterations: params.max_position_iterations as u32,
            min_island_size: params.min_island_size as u32,
            max_ccd_position_iterations: params.max_ccd_position_iterations as u32,
            max_ccd_substeps: params.max_ccd_substeps as u32,
            multiple_ccd_substep_sensor_events_enabled: params
                .multiple_ccd_substep_sensor_events_enabled,
            ccd_on_penetration_enabled: params.ccd_on_penetration_enabled,
        }
    }
}

impl Into<IntegrationParameters> for IntegrationParametersDesc {
    fn into(self) -> IntegrationParameters {
        IntegrationParameters::new(
            self.dt,
            self.erp,
            self.joint_erp,
            self.warmstart_coeff,
            self.restitution_velocity_threshold,
            self.allowed_linear_error,
            self.allowed_angular_error,
            self.max_linear_correction,
            self.max_angular_correction,
            self.prediction_distance,
            self.max_stabilization_multiplier,
            self.max_velocity_iterations as usize,
            self.max_position_iterations as usize,
            self.max_ccd_position_iterations as usize,
            self.max_ccd_substeps as usize,
            self.return_after_ccd_substep,
            self.multiple_ccd_substep_sensor_events_enabled,
            self.ccd_on_penetration_enabled,
        )
    }
}

impl Visit for IntegrationParametersDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.dt.visit("DeltaTime", visitor)?;
        self.return_after_ccd_substep
            .visit("ReturnAfterCcdSubstep", visitor)?;
        self.erp.visit("Erp", visitor)?;
        self.joint_erp.visit("JointErp", visitor)?;
        self.warmstart_coeff.visit("WarmstartCoeff", visitor)?;
        self.restitution_velocity_threshold
            .visit("RestitutionVelocityThreshold", visitor)?;
        self.allowed_linear_error
            .visit("AllowedLinearError", visitor)?;
        self.max_linear_correction
            .visit("MaxLinearCorrection", visitor)?;
        self.max_angular_correction
            .visit("MaxAngularCorrection", visitor)?;
        self.max_stabilization_multiplier
            .visit("MaxStabilizationMultiplier", visitor)?;
        self.max_velocity_iterations
            .visit("MaxVelocityIterations", visitor)?;
        self.max_position_iterations
            .visit("MaxPositionIterations", visitor)?;
        self.min_island_size.visit("MinIslandSize", visitor)?;
        self.max_ccd_position_iterations
            .visit("MaxCcdPositionIterations", visitor)?;
        self.max_ccd_substeps.visit("MaxCcdSubsteps", visitor)?;
        self.multiple_ccd_substep_sensor_events_enabled
            .visit("MultipleCcdSubstepSensorEventsEnabled", visitor)?;
        self.ccd_on_penetration_enabled
            .visit("CcdOnPenetrationEnabled", visitor)?;

        visitor.leave_region()
    }
}

#[derive(Default, Clone, Debug)]
#[doc(hidden)]
pub struct PhysicsDesc {
    pub integration_parameters: IntegrationParametersDesc,
    pub colliders: Vec<ColliderDesc<RigidBodyHandle>>,
    pub bodies: Vec<RigidBodyDesc<ColliderHandle>>,
    pub gravity: Vector3<f32>,
}

impl Visit for PhysicsDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.integration_parameters
            .visit("IntegrationParameters", visitor)?;
        self.gravity.visit("Gravity", visitor)?;
        self.colliders.visit("Colliders", visitor)?;
        self.bodies.visit("Bodies", visitor)?;

        visitor.leave_region()
    }
}
