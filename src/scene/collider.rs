#![allow(missing_docs)]

use crate::scene::graph::physics::{ContactPair, PhysicsWorld};
use crate::{
    core::{
        algebra::{DMatrix, Dynamic, Matrix4, Point3, VecStorage, Vector2, Vector3},
        inspect::{Inspect, PropertyInfo},
        pool::{Handle, Pool},
        visitor::prelude::*,
    },
    physics3d::rapier::geometry::{
        ColliderHandle, Cuboid, InteractionGroups, Segment, Shape, SharedShape,
    },
    scene::{
        base::{Base, BaseBuilder},
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
use bitflags::bitflags;
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
    sync::Arc,
};

bitflags! {
    pub(crate) struct ColliderChanges: u32 {
        const NONE = 0;
        const SHAPE = 0b0000_0001;
        const RESTITUTION = 0b0000_0010;
        const COLLISION_GROUPS = 0b0000_0100;
        const FRICTION = 0b0000_1000;
        const FRICTION_COMBINE_RULE = 0b0001_0000;
        const RESTITUTION_COMBINE_RULE = 0b0010_0000;
        const IS_SENSOR = 0b0100_0000;
        const SOLVER_GROUPS = 0b1000_0000;
        const DENSITY = 0b0001_0000_0000;
    }
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct BallShape {
    #[inspect(min_value = 0.0, step = 0.05)]
    pub radius: f32,
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct CylinderShape {
    #[inspect(min_value = 0.0, step = 0.05)]
    pub half_height: f32,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub radius: f32,
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct RoundCylinderShape {
    #[inspect(min_value = 0.0, step = 0.05)]
    pub half_height: f32,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub radius: f32,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub border_radius: f32,
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct ConeShape {
    #[inspect(min_value = 0.0, step = 0.05)]
    pub half_height: f32,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub radius: f32,
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct CuboidShape {
    pub half_extents: Vector3<f32>,
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct CapsuleShape {
    pub begin: Vector3<f32>,
    pub end: Vector3<f32>,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub radius: f32,
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct SegmentShape {
    pub begin: Vector3<f32>,
    pub end: Vector3<f32>,
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct TriangleShape {
    pub a: Vector3<f32>,
    pub b: Vector3<f32>,
    pub c: Vector3<f32>,
}

#[derive(Default, Clone, Copy, PartialEq, Hash, Debug, Visit, Inspect)]
pub struct GeometrySource(pub Handle<Node>);

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct TrimeshShape {
    pub sources: Vec<GeometrySource>,
}

#[derive(Default, Clone, Debug, Visit, Inspect)]
pub struct HeightfieldShape {
    pub geometry_source: GeometrySource,
}

#[doc(hidden)]
#[derive(Visit, Debug, Clone, Copy, Inspect)]
pub struct InteractionGroupsDesc {
    pub memberships: u32,
    pub filter: u32,
}

impl InteractionGroupsDesc {
    pub fn new(memberships: u32, filter: u32) -> Self {
        Self {
            memberships,
            filter,
        }
    }
}

impl Default for InteractionGroupsDesc {
    fn default() -> Self {
        Self {
            memberships: u32::MAX,
            filter: u32::MAX,
        }
    }
}

impl From<InteractionGroups> for InteractionGroupsDesc {
    fn from(g: InteractionGroups) -> Self {
        Self {
            memberships: g.memberships,
            filter: g.filter,
        }
    }
}

impl Inspect for ColliderShape {
    fn properties(&self) -> Vec<PropertyInfo<'_>> {
        match self {
            ColliderShape::Ball(v) => v.properties(),
            ColliderShape::Cylinder(v) => v.properties(),
            ColliderShape::RoundCylinder(v) => v.properties(),
            ColliderShape::Cone(v) => v.properties(),
            ColliderShape::Cuboid(v) => v.properties(),
            ColliderShape::Capsule(v) => v.properties(),
            ColliderShape::Segment(v) => v.properties(),
            ColliderShape::Triangle(v) => v.properties(),
            ColliderShape::Trimesh(v) => v.properties(),
            ColliderShape::Heightfield(v) => v.properties(),
        }
    }
}

/// Creates new trimesh collider shape from given mesh node. It also bakes scale into
/// vertices of trimesh because rapier does not support collider scaling yet.
pub(crate) fn make_trimesh(
    owner_inv_transform: Matrix4<f32>,
    owner: Handle<Node>,
    sources: Vec<GeometrySource>,
    nodes: &Pool<Node>,
) -> SharedShape {
    let mut mesh_builder = RawMeshBuilder::new(0, 0);

    // Create inverse transform that will discard rotation and translation, but leave scaling and
    // other parameters of global transform.
    // When global transform of node is combined with this transform, we'll get relative transform
    // with scale baked in. We need to do this because root's transform will be synced with body's
    // but we don't want to bake entire transform including root's transform.
    let root_inv_transform = owner_inv_transform;

    for source in sources {
        if let Some(Node::Mesh(mesh)) = nodes.try_borrow(source.0) {
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
                nodes[owner].name()
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

#[derive(Clone, Debug, Visit)]
pub enum ColliderShape {
    Ball(BallShape),
    Cylinder(CylinderShape),
    RoundCylinder(RoundCylinderShape),
    Cone(ConeShape),
    Cuboid(CuboidShape),
    Capsule(CapsuleShape),
    Segment(SegmentShape),
    Triangle(TriangleShape),
    Trimesh(TrimeshShape),
    Heightfield(HeightfieldShape),
}

impl Default for ColliderShape {
    fn default() -> Self {
        Self::Ball(Default::default())
    }
}

impl ColliderShape {
    pub(crate) fn from_collider_shape(shape: &dyn Shape) -> Self {
        if let Some(ball) = shape.as_ball() {
            ColliderShape::Ball(BallShape {
                radius: ball.radius,
            })
        } else if let Some(cuboid) = shape.as_cuboid() {
            ColliderShape::Cuboid(CuboidShape {
                half_extents: cuboid.half_extents,
            })
        } else if let Some(capsule) = shape.as_capsule() {
            ColliderShape::Capsule(CapsuleShape {
                begin: capsule.segment.a.coords,
                end: capsule.segment.b.coords,
                radius: capsule.radius,
            })
        } else if let Some(segment) = shape.downcast_ref::<Segment>() {
            ColliderShape::Segment(SegmentShape {
                begin: segment.a.coords,
                end: segment.b.coords,
            })
        } else if let Some(triangle) = shape.as_triangle() {
            ColliderShape::Triangle(TriangleShape {
                a: triangle.a.coords,
                b: triangle.b.coords,
                c: triangle.c.coords,
            })
        } else if shape.as_trimesh().is_some() {
            ColliderShape::Trimesh(TrimeshShape {
                sources: Default::default(),
            })
        } else if shape.as_heightfield().is_some() {
            ColliderShape::Heightfield(HeightfieldShape {
                geometry_source: Default::default(),
            })
        } else if let Some(cylinder) = shape.as_cylinder() {
            ColliderShape::Cylinder(CylinderShape {
                half_height: cylinder.half_height,
                radius: cylinder.radius,
            })
        } else if let Some(round_cylinder) = shape.as_round_cylinder() {
            ColliderShape::RoundCylinder(RoundCylinderShape {
                half_height: round_cylinder.base_shape.half_height,
                radius: round_cylinder.base_shape.radius,
                border_radius: round_cylinder.border_radius,
            })
        } else if let Some(cone) = shape.as_cone() {
            ColliderShape::Cone(ConeShape {
                half_height: cone.half_height,
                radius: cone.radius,
            })
        } else {
            unreachable!()
        }
    }

    // Converts descriptor in a shared shape.
    pub(crate) fn into_native_shape(
        self,
        owner_inv_global_transform: Matrix4<f32>,
        owner_collider: Handle<Node>,
        pool: &Pool<Node>,
    ) -> Option<SharedShape> {
        match self {
            ColliderShape::Ball(ball) => Some(SharedShape::ball(ball.radius)),

            ColliderShape::Cylinder(cylinder) => {
                Some(SharedShape::cylinder(cylinder.half_height, cylinder.radius))
            }
            ColliderShape::RoundCylinder(rcylinder) => Some(SharedShape::round_cylinder(
                rcylinder.half_height,
                rcylinder.radius,
                rcylinder.border_radius,
            )),
            ColliderShape::Cone(cone) => Some(SharedShape::cone(cone.half_height, cone.radius)),
            ColliderShape::Cuboid(cuboid) => {
                Some(SharedShape(Arc::new(Cuboid::new(cuboid.half_extents))))
            }
            ColliderShape::Capsule(capsule) => Some(SharedShape::capsule(
                Point3::from(capsule.begin),
                Point3::from(capsule.end),
                capsule.radius,
            )),
            ColliderShape::Segment(segment) => Some(SharedShape::segment(
                Point3::from(segment.begin),
                Point3::from(segment.end),
            )),
            ColliderShape::Triangle(triangle) => Some(SharedShape::triangle(
                Point3::from(triangle.a),
                Point3::from(triangle.b),
                Point3::from(triangle.c),
            )),
            ColliderShape::Trimesh(trimesh) => {
                if trimesh.sources.is_empty() {
                    None
                } else {
                    Some(make_trimesh(
                        owner_inv_global_transform,
                        owner_collider,
                        trimesh.sources,
                        pool,
                    ))
                }
            }
            ColliderShape::Heightfield(heightfield) => {
                if let Some(Node::Terrain(terrain)) = pool.try_borrow(heightfield.geometry_source.0)
                {
                    Some(make_heightfield(terrain))
                } else {
                    None
                }
            }
        }
    }

    /// Initializes a ball shape defined by its radius.
    pub fn ball(radius: f32) -> Self {
        Self::Ball(BallShape { radius })
    }

    /// Initializes a cylindrical shape defined by its half-height (along along the y axis) and its
    /// radius.
    pub fn cylinder(half_height: f32, radius: f32) -> Self {
        Self::Cylinder(CylinderShape {
            half_height,
            radius,
        })
    }

    /// Initializes a rounded cylindrical shape defined by its half-height (along along the y axis),
    /// its radius, and its roundness (the radius of the sphere used for dilating the cylinder).
    pub fn round_cylinder(half_height: f32, radius: f32, border_radius: f32) -> Self {
        Self::RoundCylinder(RoundCylinderShape {
            half_height,
            radius,
            border_radius,
        })
    }

    /// Initializes a cone shape defined by its half-height (along along the y axis) and its basis
    /// radius.
    pub fn cone(half_height: f32, radius: f32) -> Self {
        Self::Cone(ConeShape {
            half_height,
            radius,
        })
    }

    /// Initializes a cuboid shape defined by its half-extents.
    pub fn cuboid(hx: f32, hy: f32, hz: f32) -> Self {
        Self::Cuboid(CuboidShape {
            half_extents: Vector3::new(hx, hy, hz),
        })
    }

    /// Initializes a capsule shape from its endpoints and radius.
    pub fn capsule(begin: Vector3<f32>, end: Vector3<f32>, radius: f32) -> Self {
        Self::Capsule(CapsuleShape { begin, end, radius })
    }

    /// Initializes a new collider builder with a capsule shape aligned with the `x` axis.
    pub fn capsule_x(half_height: f32, radius: f32) -> Self {
        let p = Vector3::x() * half_height;
        Self::capsule(-p, p, radius)
    }

    /// Initializes a new collider builder with a capsule shape aligned with the `y` axis.
    pub fn capsule_y(half_height: f32, radius: f32) -> Self {
        let p = Vector3::y() * half_height;
        Self::capsule(-p, p, radius)
    }

    /// Initializes a new collider builder with a capsule shape aligned with the `z` axis.
    pub fn capsule_z(half_height: f32, radius: f32) -> Self {
        let p = Vector3::z() * half_height;
        Self::capsule(-p, p, radius)
    }

    /// Initializes a segment shape from its endpoints.
    pub fn segment(begin: Vector3<f32>, end: Vector3<f32>) -> Self {
        Self::Segment(SegmentShape { begin, end })
    }

    /// Initializes a triangle shape.
    pub fn triangle(a: Vector3<f32>, b: Vector3<f32>, c: Vector3<f32>) -> Self {
        Self::Triangle(TriangleShape { a, b, c })
    }

    /// Initializes a triangle mesh shape defined by a set of handles to mesh nodes that will be
    /// used to create physical shape.
    pub fn trimesh(geometry_sources: Vec<GeometrySource>) -> Self {
        Self::Trimesh(TrimeshShape {
            sources: geometry_sources,
        })
    }

    /// Initializes a heightfield shape defined by a handle to terrain node.
    pub fn heightfield(geometry_source: GeometrySource) -> Self {
        Self::Heightfield(HeightfieldShape { geometry_source })
    }
}

#[derive(Inspect, Visit, Debug)]
pub struct Collider {
    base: Base,
    shape: ColliderShape,
    #[inspect(min_value = 0.0, step = 0.05)]
    friction: f32,
    density: Option<f32>,
    #[inspect(min_value = 0.0, step = 0.05)]
    restitution: f32,
    is_sensor: bool,
    collision_groups: InteractionGroupsDesc,
    solver_groups: InteractionGroupsDesc,
    #[visit(skip)]
    #[inspect(skip)]
    pub(in crate) native: Cell<ColliderHandle>,
    #[visit(skip)]
    #[inspect(skip)]
    pub(in crate) changes: Cell<ColliderChanges>,
}

impl Default for Collider {
    fn default() -> Self {
        Self {
            base: Default::default(),
            shape: Default::default(),
            friction: 0.0,
            density: None,
            restitution: 0.0,
            is_sensor: false,
            collision_groups: Default::default(),
            solver_groups: Default::default(),
            native: Cell::new(ColliderHandle::invalid()),
            changes: Cell::new(ColliderChanges::NONE),
        }
    }
}

impl Deref for Collider {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Collider {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

pub struct ColliderShapeRefMut<'a> {
    parent: &'a mut Collider,
}

impl<'a> Drop for ColliderShapeRefMut<'a> {
    fn drop(&mut self) {
        self.parent.changes.get_mut().insert(ColliderChanges::SHAPE);
    }
}

impl<'a> Deref for ColliderShapeRefMut<'a> {
    type Target = ColliderShape;

    fn deref(&self) -> &Self::Target {
        &self.parent.shape
    }
}

impl<'a> DerefMut for ColliderShapeRefMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.parent.shape
    }
}

impl Collider {
    pub fn raw_copy(&self) -> Self {
        Self {
            base: self.base.raw_copy(),
            shape: self.shape.clone(),
            friction: self.friction,
            density: self.density,
            restitution: self.restitution,
            is_sensor: self.is_sensor,
            collision_groups: self.collision_groups,
            solver_groups: self.solver_groups,
            // Do not copy.
            native: Cell::new(ColliderHandle::invalid()),
            changes: Cell::new(ColliderChanges::NONE),
        }
    }

    pub fn set_shape(&mut self, shape: ColliderShape) {
        self.shape = shape;
        self.changes.get_mut().insert(ColliderChanges::SHAPE);
    }

    pub fn shape(&self) -> &ColliderShape {
        &self.shape
    }

    pub fn shape_value(&self) -> ColliderShape {
        self.shape.clone()
    }

    pub fn shape_mut(&mut self) -> ColliderShapeRefMut {
        ColliderShapeRefMut { parent: self }
    }

    pub fn set_restitution(&mut self, restitution: f32) {
        self.restitution = restitution;
        self.changes.get_mut().insert(ColliderChanges::RESTITUTION);
    }

    pub fn restitution(&self) -> f32 {
        self.restitution
    }

    pub fn set_density(&mut self, density: Option<f32>) {
        self.density = density;
        self.changes.get_mut().insert(ColliderChanges::DENSITY);
    }

    pub fn density(&self) -> Option<f32> {
        self.density
    }

    pub fn set_friction(&mut self, friction: f32) {
        self.friction = friction;
        self.changes.get_mut().insert(ColliderChanges::FRICTION);
    }

    pub fn friction(&self) -> f32 {
        self.friction
    }

    pub fn set_collision_groups(&mut self, groups: InteractionGroupsDesc) {
        self.collision_groups = groups;
        self.changes
            .get_mut()
            .insert(ColliderChanges::COLLISION_GROUPS);
    }

    pub fn collision_groups(&self) -> InteractionGroupsDesc {
        self.collision_groups
    }

    pub fn set_solver_groups(&mut self, groups: InteractionGroupsDesc) {
        self.solver_groups = groups;
        self.changes
            .get_mut()
            .insert(ColliderChanges::SOLVER_GROUPS);
    }

    pub fn solver_groups(&self) -> InteractionGroupsDesc {
        self.solver_groups
    }

    pub fn set_is_sensor(&mut self, is_sensor: bool) {
        self.is_sensor = is_sensor;
        self.changes.get_mut().insert(ColliderChanges::IS_SENSOR);
    }

    pub fn is_sensor(&self) -> bool {
        self.is_sensor
    }

    pub fn contacts<'a>(
        &self,
        physics: &'a PhysicsWorld,
    ) -> impl Iterator<Item = ContactPair> + 'a {
        physics.contacts_with(self.native.get())
    }
}

pub struct ColliderBuilder {
    base_builder: BaseBuilder,
    shape: ColliderShape,
    friction: f32,
    density: Option<f32>,
    restitution: f32,
    is_sensor: bool,
    collision_groups: InteractionGroupsDesc,
    solver_groups: InteractionGroupsDesc,
}

impl ColliderBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            shape: Default::default(),
            friction: 0.0,
            density: None,
            restitution: 0.0,
            is_sensor: false,
            collision_groups: Default::default(),
            solver_groups: Default::default(),
        }
    }

    pub fn with_shape(mut self, shape: ColliderShape) -> Self {
        self.shape = shape;
        self
    }

    pub fn build_node(self) -> Node {
        let collider = Collider {
            base: self.base_builder.build_base(),
            shape: self.shape,
            friction: self.friction,
            density: self.density,
            restitution: self.restitution,
            is_sensor: self.is_sensor,
            collision_groups: self.collision_groups,
            solver_groups: self.solver_groups,
            native: Cell::new(ColliderHandle::invalid()),
            changes: Cell::new(ColliderChanges::NONE),
        };
        Node::Collider(collider)
    }

    pub fn with_density(mut self, density: Option<f32>) -> Self {
        self.density = density;
        self
    }

    pub fn with_restitution(mut self, restitution: f32) -> Self {
        self.restitution = restitution;
        self
    }

    pub fn with_friction(mut self, friction: f32) -> Self {
        self.friction = friction;
        self
    }

    pub fn with_sensor(mut self, sensor: bool) -> Self {
        self.is_sensor = sensor;
        self
    }

    pub fn with_solver_groups(mut self, solver_groups: InteractionGroupsDesc) -> Self {
        self.solver_groups = solver_groups;
        self
    }

    pub fn with_collision_groups(mut self, collision_groups: InteractionGroupsDesc) -> Self {
        self.collision_groups = collision_groups;
        self
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
