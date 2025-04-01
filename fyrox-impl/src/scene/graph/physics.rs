// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Scene physics module.

use crate::{
    core::{
        algebra::{
            DMatrix, Dyn, Isometry3, Matrix4, Point3, Translation, Translation3, UnitQuaternion,
            UnitVector3, VecStorage, Vector2, Vector3,
        },
        arrayvec::ArrayVec,
        instant,
        log::{Log, MessageKind},
        math::Matrix4Ext,
        parking_lot::Mutex,
        pool::Handle,
        reflect::prelude::*,
        uuid_provider,
        variable::{InheritableVariable, VariableFlags},
        visitor::prelude::*,
        BiDirHashMap,
    },
    scene::{
        self,
        collider::{self, ColliderShape, GeometrySource},
        debug::SceneDrawingContext,
        graph::{isometric_global_transform, Graph, NodePool},
        joint::{JointLocalFrames, JointParams},
        mesh::{
            buffer::{VertexAttributeUsage, VertexReadTrait},
            Mesh,
        },
        node::{Node, NodeTrait},
        rigidbody::{self, ApplyAction, RigidBodyMassPropertiesType},
        terrain::{Chunk, Terrain},
    },
    utils::raw_mesh::{RawMeshBuilder, RawVertex},
};
use rapier3d::{
    dynamics::{
        CCDSolver, GenericJoint, GenericJointBuilder, ImpulseJointHandle, ImpulseJointSet,
        IslandManager, JointAxesMask, MultibodyJointHandle, MultibodyJointSet, RigidBody,
        RigidBodyActivation, RigidBodyBuilder, RigidBodyHandle, RigidBodySet, RigidBodyType,
    },
    geometry::{
        Collider, ColliderBuilder, ColliderHandle, ColliderSet, Cuboid, DefaultBroadPhase,
        InteractionGroups, NarrowPhase, Ray, SharedShape,
    },
    parry::{query::ShapeCastOptions, shape::HeightField},
    pipeline::{DebugRenderPipeline, EventHandler, PhysicsPipeline, QueryPipeline},
    prelude::{HeightFieldCellStatus, JointAxis, MassProperties},
};
use std::{
    cell::{Cell, RefCell},
    cmp::Ordering,
    fmt::{Debug, Formatter},
    hash::Hash,
    num::NonZeroUsize,
    sync::Arc,
    time::Duration,
};
use strum_macros::{AsRefStr, EnumString, VariantNames};

use fyrox_graph::{BaseSceneGraph, SceneGraphNode};
pub use rapier3d::geometry::shape::*;

/// Shape-dependent identifier.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum FeatureId {
    /// Shape-dependent identifier of a vertex.
    Vertex(u32),
    /// Shape-dependent identifier of an edge.
    Edge(u32),
    /// Shape-dependent identifier of a face.
    Face(u32),
    /// Unknown identifier.
    Unknown,
}

impl From<rapier3d::geometry::FeatureId> for FeatureId {
    fn from(v: rapier3d::geometry::FeatureId) -> Self {
        match v {
            rapier3d::geometry::FeatureId::Vertex(v) => FeatureId::Vertex(v),
            rapier3d::geometry::FeatureId::Edge(v) => FeatureId::Edge(v),
            rapier3d::geometry::FeatureId::Face(v) => FeatureId::Face(v),
            rapier3d::geometry::FeatureId::Unknown => FeatureId::Unknown,
        }
    }
}

impl From<rapier2d::geometry::FeatureId> for FeatureId {
    fn from(v: rapier2d::geometry::FeatureId) -> Self {
        match v {
            rapier2d::geometry::FeatureId::Vertex(v) => FeatureId::Vertex(v),
            rapier2d::geometry::FeatureId::Face(v) => FeatureId::Face(v),
            rapier2d::geometry::FeatureId::Unknown => FeatureId::Unknown,
        }
    }
}

/// Rules used to combine two coefficients.
///
/// # Notes
///
/// This is used to determine the effective restitution and friction coefficients for a contact
/// between two colliders. Each collider has its combination rule of type `CoefficientCombineRule`,
/// the rule actually used is given by `max(first_combine_rule, second_combine_rule)`.
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Visit, Reflect, VariantNames, EnumString, AsRefStr, Default,
)]
#[repr(u32)]
pub enum CoefficientCombineRule {
    /// The two coefficients are averaged.
    #[default]
    Average = 0,
    /// The smallest coefficient is chosen.
    Min,
    /// The two coefficients are multiplied.
    Multiply,
    /// The greatest coefficient is chosen.
    Max,
}

uuid_provider!(CoefficientCombineRule = "775d5598-c283-4b44-9cc0-2e23dc8936f4");

impl From<rapier3d::dynamics::CoefficientCombineRule> for CoefficientCombineRule {
    fn from(v: rapier3d::dynamics::CoefficientCombineRule) -> Self {
        match v {
            rapier3d::dynamics::CoefficientCombineRule::Average => CoefficientCombineRule::Average,
            rapier3d::dynamics::CoefficientCombineRule::Min => CoefficientCombineRule::Min,
            rapier3d::dynamics::CoefficientCombineRule::Multiply => {
                CoefficientCombineRule::Multiply
            }
            rapier3d::dynamics::CoefficientCombineRule::Max => CoefficientCombineRule::Max,
        }
    }
}

impl Into<rapier3d::dynamics::CoefficientCombineRule> for CoefficientCombineRule {
    fn into(self) -> rapier3d::dynamics::CoefficientCombineRule {
        match self {
            CoefficientCombineRule::Average => rapier3d::dynamics::CoefficientCombineRule::Average,
            CoefficientCombineRule::Min => rapier3d::dynamics::CoefficientCombineRule::Min,
            CoefficientCombineRule::Multiply => {
                rapier3d::dynamics::CoefficientCombineRule::Multiply
            }
            CoefficientCombineRule::Max => rapier3d::dynamics::CoefficientCombineRule::Max,
        }
    }
}

impl Into<rapier2d::dynamics::CoefficientCombineRule> for CoefficientCombineRule {
    fn into(self) -> rapier2d::dynamics::CoefficientCombineRule {
        match self {
            CoefficientCombineRule::Average => rapier2d::dynamics::CoefficientCombineRule::Average,
            CoefficientCombineRule::Min => rapier2d::dynamics::CoefficientCombineRule::Min,
            CoefficientCombineRule::Multiply => {
                rapier2d::dynamics::CoefficientCombineRule::Multiply
            }
            CoefficientCombineRule::Max => rapier2d::dynamics::CoefficientCombineRule::Max,
        }
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

impl PhysicsPerformanceStatistics {
    /// Resets performance statistics to default values.
    pub fn reset(&mut self) {
        *self = Default::default();
    }

    /// Returns total amount of time for every part of statistics.
    pub fn total(&self) -> Duration {
        self.step_time + self.total_ray_cast_time.get()
    }
}

/// A ray intersection result.
#[derive(Debug, Clone, PartialEq)]
pub struct Intersection {
    /// A handle of the collider with which intersection was detected.
    pub collider: Handle<Node>,

    /// A normal at the intersection position.
    pub normal: Vector3<f32>,

    /// A position of the intersection in world coordinates.
    pub position: Point3<f32>,

    /// Additional data that contains a kind of the feature with which
    /// intersection was detected as well as its index.
    ///
    /// # Important notes.
    ///
    /// FeatureId::Face might have index that is greater than amount of triangles in
    /// a triangle mesh, this means that intersection was detected from "back" side of
    /// a face. To "fix" that index, simply subtract amount of triangles of a triangle
    /// mesh from the value.
    pub feature: FeatureId,

    /// Distance from the ray origin.
    pub toi: f32,
}

/// A set of options for the ray cast.
pub struct RayCastOptions {
    /// A ray origin.
    pub ray_origin: Point3<f32>,

    /// A ray direction. Can be non-normalized.
    pub ray_direction: Vector3<f32>,

    /// Maximum distance of cast.
    pub max_len: f32,

    /// Groups to check.
    pub groups: collider::InteractionGroups,

    /// Whether to sort intersections from closest to farthest.
    pub sort_results: bool,
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

/// Data of the contact.
#[derive(Debug, Clone, PartialEq)]
pub struct ContactData {
    /// The contact point in the local-space of the first shape.
    pub local_p1: Vector3<f32>,
    /// The contact point in the local-space of the second shape.
    pub local_p2: Vector3<f32>,
    /// The distance between the two contact points.
    pub dist: f32,
    /// The impulse, along the contact normal, applied by this contact to the first collider's rigid-body.
    /// The impulse applied to the second collider's rigid-body is given by `-impulse`.
    pub impulse: f32,
    /// The friction impulses along the basis orthonormal to the contact normal, applied to the first
    /// collider's rigid-body.
    pub tangent_impulse: Vector2<f32>,
}

/// A contact manifold between two colliders.
#[derive(Debug, Clone, PartialEq)]
pub struct ContactManifold {
    /// The contacts points.
    pub points: Vec<ContactData>,
    /// The contact normal of all the contacts of this manifold, expressed in the local space of the first shape.
    pub local_n1: Vector3<f32>,
    /// The contact normal of all the contacts of this manifold, expressed in the local space of the second shape.
    pub local_n2: Vector3<f32>,
    /// The first rigid-body involved in this contact manifold.
    pub rigid_body1: Handle<Node>,
    /// The second rigid-body involved in this contact manifold.
    pub rigid_body2: Handle<Node>,
    /// The world-space contact normal shared by all the contact in this contact manifold.
    pub normal: Vector3<f32>,
}

/// Contact info for pair of colliders.
#[derive(Debug, Clone, PartialEq)]
pub struct ContactPair {
    /// The first collider involved in the contact pair.
    pub collider1: Handle<Node>,
    /// The second collider involved in the contact pair.
    pub collider2: Handle<Node>,
    /// The set of contact manifolds between the two colliders.
    /// All contact manifold contain themselves contact points between the colliders.
    pub manifolds: Vec<ContactManifold>,
    /// Is there any active contact in this contact pair?
    pub has_any_active_contact: bool,
}

impl ContactPair {
    fn from_native(c: &rapier3d::geometry::ContactPair, physics: &PhysicsWorld) -> Option<Self> {
        Some(ContactPair {
            collider1: Handle::decode_from_u128(physics.colliders.get(c.collider1)?.user_data),
            collider2: Handle::decode_from_u128(physics.colliders.get(c.collider2)?.user_data),
            manifolds: c
                .manifolds
                .iter()
                .filter_map(|m| {
                    Some(ContactManifold {
                        points: m
                            .points
                            .iter()
                            .map(|p| ContactData {
                                local_p1: p.local_p1.coords,
                                local_p2: p.local_p2.coords,
                                dist: p.dist,
                                impulse: p.data.impulse,
                                tangent_impulse: p.data.tangent_impulse,
                            })
                            .collect(),
                        local_n1: m.local_n1,
                        local_n2: m.local_n2,
                        rigid_body1: m.data.rigid_body1.and_then(|h| {
                            physics
                                .bodies
                                .get(h)
                                .map(|b| Handle::decode_from_u128(b.user_data))
                        })?,
                        rigid_body2: m.data.rigid_body2.and_then(|h| {
                            physics
                                .bodies
                                .get(h)
                                .map(|b| Handle::decode_from_u128(b.user_data))
                        })?,
                        normal: m.data.normal,
                    })
                })
                .collect(),
            has_any_active_contact: c.has_any_active_contact,
        })
    }
}

/// Intersection info for pair of colliders.
#[derive(Debug, Clone, PartialEq)]
pub struct IntersectionPair {
    /// The first collider involved in the contact pair.
    pub collider1: Handle<Node>,
    /// The second collider involved in the contact pair.
    pub collider2: Handle<Node>,
    /// Is there any active contact in this contact pair?
    pub has_any_active_contact: bool,
}

pub(super) struct Container<S, A>
where
    A: Hash + Eq + Clone,
{
    set: S,
    map: BiDirHashMap<A, Handle<Node>>,
}

fn convert_joint_params(
    params: scene::joint::JointParams,
    local_frame1: Isometry3<f32>,
    local_frame2: Isometry3<f32>,
) -> GenericJoint {
    let locked_axis = match params {
        JointParams::BallJoint(_) => JointAxesMask::LOCKED_SPHERICAL_AXES,
        JointParams::FixedJoint(_) => JointAxesMask::LOCKED_FIXED_AXES,
        JointParams::PrismaticJoint(_) => JointAxesMask::LOCKED_PRISMATIC_AXES,
        JointParams::RevoluteJoint(_) => JointAxesMask::LOCKED_REVOLUTE_AXES,
    };

    let mut joint = GenericJointBuilder::new(locked_axis)
        .local_frame1(local_frame1)
        .local_frame2(local_frame2)
        .build();

    match params {
        scene::joint::JointParams::BallJoint(v) => {
            if v.x_limits_enabled {
                joint.set_limits(
                    JointAxis::AngX,
                    [v.x_limits_angles.start, v.x_limits_angles.end],
                );
            }
            if v.y_limits_enabled {
                joint.set_limits(
                    JointAxis::AngY,
                    [v.y_limits_angles.start, v.y_limits_angles.end],
                );
            }
            if v.z_limits_enabled {
                joint.set_limits(
                    JointAxis::AngZ,
                    [v.z_limits_angles.start, v.z_limits_angles.end],
                );
            }
        }
        scene::joint::JointParams::FixedJoint(_) => {}
        scene::joint::JointParams::PrismaticJoint(v) => {
            if v.limits_enabled {
                joint.set_limits(JointAxis::LinX, [v.limits.start, v.limits.end]);
            }
        }
        scene::joint::JointParams::RevoluteJoint(v) => {
            if v.limits_enabled {
                joint.set_limits(JointAxis::AngX, [v.limits.start, v.limits.end]);
            }
        }
    }

    joint
}

/// Creates new trimesh collider shape from given mesh node. It also bakes scale into
/// vertices of trimesh because rapier does not support collider scaling yet.
fn make_trimesh(
    owner_inv_transform: Matrix4<f32>,
    owner: Handle<Node>,
    sources: &[GeometrySource],
    nodes: &NodePool,
) -> Option<SharedShape> {
    let mut mesh_builder = RawMeshBuilder::new(0, 0);

    // Create inverse transform that will discard rotation and translation, but leave scaling and
    // other parameters of global transform.
    // When global transform of node is combined with this transform, we'll get relative transform
    // with scale baked in. We need to do this because root's transform will be synced with body's
    // but we don't want to bake entire transform including root's transform.
    let root_inv_transform = owner_inv_transform;

    for &source in sources {
        if let Some(mesh) = nodes.try_borrow(source.0).and_then(|n| n.cast::<Mesh>()) {
            let global_transform = root_inv_transform * mesh.global_transform();

            for surface in mesh.surfaces() {
                let shared_data = surface.data();
                let shared_data = shared_data.data_ref();

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

        SharedShape::trimesh(vec![Point3::new(0.0, 0.0, 0.0)], vec![[0, 0, 0]]).ok()
    } else {
        SharedShape::trimesh(vertices, indices).ok()
    }
}

/// Creates new convex polyhedron collider shape from given mesh node. It also bakes scale into
/// vertices of trimesh because rapier does not support collider scaling yet.
fn make_polyhedron_shape(owner_inv_transform: Matrix4<f32>, mesh: &Mesh) -> SharedShape {
    let mut mesh_builder = RawMeshBuilder::new(0, 0);

    // Create inverse transform that will discard rotation and translation, but leave scaling and
    // other parameters of global transform.
    // When global transform of node is combined with this transform, we'll get relative transform
    // with scale baked in. We need to do this because root's transform will be synced with body's
    // but we don't want to bake entire transform including root's transform.
    let root_inv_transform = owner_inv_transform;

    let global_transform = root_inv_transform * mesh.global_transform();

    for surface in mesh.surfaces() {
        let shared_data = surface.data();
        let shared_data = shared_data.data_ref();

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

    SharedShape::convex_decomposition(&vertices, &indices)
}

/// Creates height field shape from given terrain.
fn make_heightfield(terrain: &Terrain) -> Option<SharedShape> {
    assert!(!terrain.chunks_ref().is_empty());

    // HACK: Temporary solution for https://github.com/FyroxEngine/Fyrox/issues/365
    let scale = terrain.local_transform().scale();

    // Count rows and columns.
    let height_map_size = terrain.height_map_size();
    let chunk_size = height_map_size.map(|x| x - 3);
    let chunk_min = terrain
        .chunks_ref()
        .iter()
        .map(Chunk::grid_position)
        .reduce(|a, b| a.inf(&b));
    let chunk_max = terrain
        .chunks_ref()
        .iter()
        .map(Chunk::grid_position)
        .reduce(|a, b| a.sup(&b));
    let (Some(chunk_min), Some(chunk_max)) = (chunk_min, chunk_max) else {
        return None;
    };
    let row_range = chunk_max.y - chunk_min.y + 1;
    let col_range = chunk_max.x - chunk_min.x + 1;
    let nrows = chunk_size.y * row_range as u32 + 1;
    let ncols = chunk_size.x * col_range as u32 + 1;

    // Combine height map of each chunk into bigger one.
    let mut data = vec![0.0; (nrows * ncols) as usize];
    for chunk in terrain.chunks_ref() {
        let texture = chunk.heightmap().data_ref();
        let height_map = texture.data_of_type::<f32>().unwrap();
        let pos = (chunk.grid_position() - chunk_min).map(|x| x as u32);
        let (ox, oy) = (pos.x * chunk_size.x, pos.y * chunk_size.y);
        for iy in 0..height_map_size.y - 2 {
            for ix in 0..height_map_size.x - 2 {
                let (x, y) = (ix + 1, iy + 1);
                let value = height_map[(y * height_map_size.x + x) as usize] * scale.y;
                data[((ox + ix) * nrows + oy + iy) as usize] = value;
            }
        }
    }
    let x_scale = terrain.chunk_size().x * scale.x * col_range as f32;
    let z_scale = terrain.chunk_size().y * scale.z * row_range as f32;
    let x_pos = terrain.chunk_size().x * scale.x * chunk_min.x as f32;
    let z_pos = terrain.chunk_size().y * scale.z * chunk_min.y as f32;
    let mut hf = HeightField::new(
        DMatrix::from_data(VecStorage::new(
            Dyn(nrows as usize),
            Dyn(ncols as usize),
            data,
        )),
        Vector3::new(x_scale, 1.0, z_scale),
    );
    hf.cells_statuses_mut()
        .fill(HeightFieldCellStatus::CELL_REMOVED);
    let hole_mask_size = terrain.hole_mask_size();
    for chunk in terrain.chunks_ref() {
        let Some(texture) = chunk.hole_mask().map(|t| t.data_ref()) else {
            continue;
        };
        let hole_mask = texture.data_of_type::<u8>().unwrap();
        let pos = (chunk.grid_position() - chunk_min).map(|x| x as u32);
        let (ox, oy) = (pos.x * chunk_size.x, pos.y * chunk_size.y);

        for iy in 0..hole_mask_size.y {
            for ix in 0..hole_mask_size.x {
                let is_hole = hole_mask[(iy * hole_mask_size.x + ix) as usize] < 128;
                let (x, y) = (ox + ix, oy + iy);
                if !is_hole {
                    hf.set_cell_status(y as usize, x as usize, HeightFieldCellStatus::empty());
                }
            }
        }
    }
    // HeightField colliders naturally have their origin at their centers,
    // so to position the collider correctly we must add half of the size to x and z.
    Some(SharedShape::compound(vec![(
        Isometry3::translation(x_scale * 0.5 + x_pos, 0.0, z_scale * 0.5 + z_pos),
        SharedShape::new(hf),
    )]))
}

// Converts descriptor in a shared shape.
fn collider_shape_into_native_shape(
    shape: &ColliderShape,
    owner_inv_global_transform: Matrix4<f32>,
    owner_collider: Handle<Node>,
    pool: &NodePool,
) -> Option<SharedShape> {
    match shape {
        ColliderShape::Ball(ball) => Some(SharedShape::ball(ball.radius)),

        ColliderShape::Cylinder(cylinder) => {
            Some(SharedShape::cylinder(cylinder.half_height, cylinder.radius))
        }
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
                make_trimesh(
                    owner_inv_global_transform,
                    owner_collider,
                    &trimesh.sources,
                    pool,
                )
            }
        }
        ColliderShape::Heightfield(heightfield) => pool
            .try_borrow(heightfield.geometry_source.0)
            .and_then(|n| n.cast::<Terrain>())
            .and_then(make_heightfield),
        ColliderShape::Polyhedron(polyhedron) => pool
            .try_borrow(polyhedron.geometry_source.0)
            .and_then(|n| n.cast::<Mesh>())
            .map(|mesh| make_polyhedron_shape(owner_inv_global_transform, mesh)),
    }
}

/// Parameters for a time-step of the physics engine.
///
/// # Notes
///
/// This is almost one-to-one copy of Rapier's integration parameters with custom attributes for
/// each parameter.
#[derive(Copy, Clone, Visit, Reflect, Debug, PartialEq)]
#[visit(optional)]
pub struct IntegrationParameters {
    /// The time step length, default is None - this means that physics simulation will use engine's
    /// time step.
    #[reflect(min_value = 0.0, description = "The time step length (default: None)")]
    pub dt: Option<f32>,

    /// Minimum timestep size when using CCD with multiple substeps (default `1.0 / 60.0 / 100.0`)
    ///
    /// When CCD with multiple substeps is enabled, the timestep is subdivided into smaller pieces.
    /// This timestep subdivision won't generate timestep lengths smaller than `min_ccd_dt`.
    ///
    /// Setting this to a large value will reduce the opportunity to performing CCD substepping,
    /// resulting in potentially more time dropped by the motion-clamping mechanism. Setting this
    /// to an very small value may lead to numerical instabilities.
    #[reflect(
        min_value = 0.0,
        description = "Minimum timestep size when using CCD with multiple\
         substeps (default `1.0 / 60.0 / 100.0`)"
    )]
    pub min_ccd_dt: f32,

    /// The damping ratio used by the springs for contact constraint stabilization.
    /// Larger values make the constraints more compliant (allowing more visible penetrations
    /// before stabilization). Default `5.0`.
    #[reflect(
        min_value = 0.0,
        description = "The damping ratio used by the springs for contact constraint stabilization.
Larger values make the constraints more compliant (allowing more visible penetrations
before stabilization). Default `5.0`."
    )]
    pub contact_damping_ratio: f32,

    /// The natural frequency used by the springs for contact constraint regularization.
    /// Increasing this value will make it so that penetrations get fixed more quickly at the
    /// expense of potential jitter effects due to overshooting. In order to make the simulation
    /// look stiffer, it is recommended to increase the `contact_damping_ratio` instead of this
    /// value. Default: `30.0`
    #[reflect(
        min_value = 0.0,
        description = "The natural frequency used by the springs for contact constraint regularization.
Increasing this value will make it so that penetrations get fixed more quickly at the
expense of potential jitter effects due to overshooting. In order to make the simulation
look stiffer, it is recommended to increase the `contact_damping_ratio` instead of this
value. Default: `30.0`"
    )]
    pub contact_natural_frequency: f32,

    /// The natural frequency used by the springs for joint constraint regularization.
    /// Increasing this value will make it so that penetrations get fixed more quickly.
    /// Default: `1.0e6`
    #[reflect(
        min_value = 0.0,
        description = "The natural frequency used by the springs for joint constraint regularization.
Increasing this value will make it so that penetrations get fixed more quickly. Default: `1.0e6`."
    )]
    pub joint_natural_frequency: f32,

    /// The fraction of critical damping applied to the joint for constraints regularization.
    /// (default `0.8`).
    #[reflect(
        min_value = 0.0,
        description = "The fraction of critical damping applied to the joint for \
        constraints regularization (default: `0.8`)."
    )]
    pub joint_damping_ratio: f32,

    /// Amount of penetration the engine wont attempt to correct (default: `0.002m`).
    #[reflect(
        min_value = 0.0,
        description = "Amount of penetration the engine wont attempt to correct (default: `0.002m`)."
    )]
    pub allowed_linear_error: f32,

    /// Maximum amount of penetration the solver will attempt to resolve in one timestep (default: `10.0`).
    #[reflect(
        min_value = 0.0,
        description = "Maximum amount of penetration the solver will attempt to resolve in one timestep (default: `10.0`)."
    )]
    pub normalized_max_corrective_velocity: f32,

    /// The maximal distance separating two objects that will generate predictive contacts (default: `0.002`).
    #[reflect(
        min_value = 0.0,
        description = "The maximal distance separating two objects that will generate \
        predictive contacts (default: `0.002`)."
    )]
    pub prediction_distance: f32,

    /// The number of solver iterations run by the constraints solver for calculating forces (default: `4`).
    #[reflect(
        min_value = 0.0,
        description = "The number of solver iterations run by the constraints solver for calculating forces (default: `4`)."
    )]
    pub num_solver_iterations: usize,

    /// Number of addition friction resolution iteration run during the last solver sub-step (default: `4`).
    #[reflect(
        min_value = 0.0,
        description = "Number of addition friction resolution iteration run during the last solver sub-step (default: `4`)."
    )]
    pub num_additional_friction_iterations: usize,

    /// Number of internal Project Gauss Seidel (PGS) iterations run at each solver iteration (default: `1`).
    #[reflect(
        min_value = 0.0,
        description = "Number of internal Project Gauss Seidel (PGS) iterations run at each solver iteration (default: `1`)."
    )]
    pub num_internal_pgs_iterations: usize,

    /// Minimum number of dynamic bodies in each active island (default: `128`).
    #[reflect(
        min_value = 0.0,
        description = "Minimum number of dynamic bodies in each active island (default: `128`)."
    )]
    pub min_island_size: u32,

    /// Maximum number of substeps performed by the  solver (default: `4`).
    #[reflect(
        min_value = 0.0,
        description = "Maximum number of substeps performed by the  solver (default: `4`)."
    )]
    pub max_ccd_substeps: u32,

    /// The coefficient in `[0, 1]` applied to warmstart impulses, i.e., impulses that are used as the
    /// initial solution (instead of 0) at the next simulation step. Default `1.0`.
    pub warmstart_coefficient: f32,

    /// The approximate size of most dynamic objects in the scene.
    ///
    /// This value can be understood as the number of units-per-meter in your physical world compared
    /// to a human-sized world in meter. For example, in a 2d game, if your typical object size is 100
    /// pixels, set the `[`Self::length_unit`]` parameter to 100.0. The physics engine will interpret
    /// it as if 100 pixels is equivalent to 1 meter in its various internal threshold.
    /// (default `1.0`).
    pub length_unit: f32,

    /// The number of stabilization iterations run at each solver iterations (default: `2`).
    pub num_internal_stabilization_iterations: usize,
}

impl Default for IntegrationParameters {
    fn default() -> Self {
        Self {
            dt: None,
            min_ccd_dt: 1.0 / 60.0 / 100.0,
            contact_damping_ratio: 5.0,
            contact_natural_frequency: 30.0,
            joint_natural_frequency: 1.0e6,
            joint_damping_ratio: 1.0,
            warmstart_coefficient: 1.0,
            allowed_linear_error: 0.002,
            normalized_max_corrective_velocity: 10.0,
            prediction_distance: 0.002,
            num_internal_pgs_iterations: 1,
            num_additional_friction_iterations: 4,
            num_solver_iterations: 4,
            min_island_size: 128,
            max_ccd_substeps: 4,
            length_unit: 1.0,
            num_internal_stabilization_iterations: 4,
        }
    }
}

/// Physics world is responsible for physics simulation in the engine. There is a very few public
/// methods, mostly for ray casting. You should add physical entities using scene graph nodes, such
/// as RigidBody, Collider, Joint.
#[derive(Visit, Reflect)]
pub struct PhysicsWorld {
    /// A flag that defines whether physics simulation is enabled or not.
    pub enabled: InheritableVariable<bool>,

    /// A set of parameters that define behavior of every rigid body.
    #[visit(optional)]
    pub integration_parameters: InheritableVariable<IntegrationParameters>,

    /// Current gravity vector. Default is (0.0, -9.81, 0.0)
    pub gravity: InheritableVariable<Vector3<f32>>,

    /// Performance statistics of a single simulation step.
    #[visit(skip)]
    #[reflect(hidden)]
    pub performance_statistics: PhysicsPerformanceStatistics,

    // Current physics pipeline.
    #[visit(skip)]
    #[reflect(hidden)]
    pipeline: PhysicsPipeline,
    // Broad phase performs rough intersection checks.
    #[visit(skip)]
    #[reflect(hidden)]
    broad_phase: DefaultBroadPhase,
    // Narrow phase is responsible for precise contact generation.
    #[visit(skip)]
    #[reflect(hidden)]
    narrow_phase: NarrowPhase,
    // A continuous collision detection solver.
    #[visit(skip)]
    #[reflect(hidden)]
    ccd_solver: CCDSolver,
    // Structure responsible for maintaining the set of active rigid-bodies, and putting non-moving
    // rigid-bodies to sleep to save computation times.
    #[visit(skip)]
    #[reflect(hidden)]
    islands: IslandManager,
    // A container of rigid bodies.
    #[visit(skip)]
    #[reflect(hidden)]
    bodies: RigidBodySet,
    // A container of colliders.
    #[visit(skip)]
    #[reflect(hidden)]
    pub(crate) colliders: ColliderSet,
    // A container of impulse joints.
    #[visit(skip)]
    #[reflect(hidden)]
    joints: Container<ImpulseJointSet, ImpulseJointHandle>,
    // A container of multibody joints.
    #[visit(skip)]
    #[reflect(hidden)]
    multibody_joints: Container<MultibodyJointSet, MultibodyJointHandle>,
    // Event handler collects info about contacts and proximity events.
    #[visit(skip)]
    #[reflect(hidden)]
    event_handler: Box<dyn EventHandler>,
    #[visit(skip)]
    #[reflect(hidden)]
    query: RefCell<QueryPipeline>,
    #[visit(skip)]
    #[reflect(hidden)]
    debug_render_pipeline: Mutex<DebugRenderPipeline>,
}

impl Clone for PhysicsWorld {
    fn clone(&self) -> Self {
        PhysicsWorld {
            enabled: self.enabled.clone(),
            integration_parameters: self.integration_parameters.clone(),
            gravity: self.gravity.clone(),
            ..Default::default()
        }
    }
}

fn isometry_from_global_transform(transform: &Matrix4<f32>) -> Isometry3<f32> {
    Isometry3 {
        translation: Translation3::new(transform[12], transform[13], transform[14]),
        rotation: UnitQuaternion::from_matrix_eps(
            &transform.basis(),
            f32::EPSILON,
            16,
            UnitQuaternion::identity(),
        ),
    }
}

fn calculate_local_frames(
    joint: &dyn NodeTrait,
    body1: &dyn NodeTrait,
    body2: &dyn NodeTrait,
) -> (Isometry3<f32>, Isometry3<f32>) {
    let joint_isometry = isometry_from_global_transform(&joint.global_transform());

    (
        isometry_from_global_transform(&body1.global_transform()).inverse() * joint_isometry,
        isometry_from_global_transform(&body2.global_transform()).inverse() * joint_isometry,
    )
}

fn u32_to_group(v: u32) -> rapier3d::geometry::Group {
    rapier3d::geometry::Group::from_bits(v).unwrap_or_else(rapier3d::geometry::Group::all)
}

/// A filter tha describes what collider should be included or excluded from a scene query.
#[derive(Copy, Clone, Default)]
#[allow(clippy::type_complexity)]
pub struct QueryFilter<'a> {
    /// Flags indicating what particular type of colliders should be excluded from the scene query.
    pub flags: collider::QueryFilterFlags,
    /// If set, only colliders with collision groups compatible with this one will
    /// be included in the scene query.
    pub groups: Option<collider::InteractionGroups>,
    /// If set, this collider will be excluded from the scene query.
    pub exclude_collider: Option<Handle<Node>>,
    /// If set, any collider attached to this rigid-body will be excluded from the scene query.
    pub exclude_rigid_body: Option<Handle<Node>>,
    /// If set, any collider for which this closure returns false will be excluded from the scene query.
    pub predicate: Option<&'a dyn Fn(Handle<Node>, &collider::Collider) -> bool>,
}

/// The result of a time-of-impact (TOI) computation.
#[derive(Copy, Clone, Debug)]
pub struct TOI {
    /// The time at which the objects touch.
    pub toi: f32,
    /// The local-space closest point on the first shape at the time of impact.
    ///
    /// Undefined if `status` is `Penetrating`.
    pub witness1: Point3<f32>,
    /// The local-space closest point on the second shape at the time of impact.
    ///
    /// Undefined if `status` is `Penetrating`.
    pub witness2: Point3<f32>,
    /// The local-space outward normal on the first shape at the time of impact.
    ///
    /// Undefined if `status` is `Penetrating`.
    pub normal1: UnitVector3<f32>,
    /// The local-space outward normal on the second shape at the time of impact.
    ///
    /// Undefined if `status` is `Penetrating`.
    pub normal2: UnitVector3<f32>,
    /// The way the time-of-impact computation algorithm terminated.
    pub status: collider::TOIStatus,
}

impl PhysicsWorld {
    /// Creates a new instance of the physics world.
    pub(super) fn new() -> Self {
        Self {
            enabled: true.into(),
            pipeline: PhysicsPipeline::new(),
            gravity: Vector3::new(0.0, -9.81, 0.0).into(),
            integration_parameters: IntegrationParameters::default().into(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            ccd_solver: CCDSolver::new(),
            islands: IslandManager::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            joints: Container {
                set: ImpulseJointSet::new(),
                map: Default::default(),
            },
            multibody_joints: Container {
                set: MultibodyJointSet::new(),
                map: Default::default(),
            },
            event_handler: Box::new(()),
            query: RefCell::new(Default::default()),
            performance_statistics: Default::default(),
            debug_render_pipeline: Default::default(),
        }
    }

    pub(super) fn update(&mut self, dt: f32) {
        let time = instant::Instant::now();

        if *self.enabled {
            let integration_parameters = rapier3d::dynamics::IntegrationParameters {
                dt: self.integration_parameters.dt.unwrap_or(dt),
                min_ccd_dt: self.integration_parameters.min_ccd_dt,
                contact_damping_ratio: self.integration_parameters.contact_damping_ratio,
                contact_natural_frequency: self.integration_parameters.contact_natural_frequency,
                joint_natural_frequency: self.integration_parameters.joint_natural_frequency,
                joint_damping_ratio: self.integration_parameters.joint_damping_ratio,
                warmstart_coefficient: self.integration_parameters.warmstart_coefficient,
                length_unit: self.integration_parameters.length_unit,
                normalized_allowed_linear_error: self.integration_parameters.allowed_linear_error,
                normalized_max_corrective_velocity: self
                    .integration_parameters
                    .normalized_max_corrective_velocity,
                normalized_prediction_distance: self.integration_parameters.prediction_distance,
                num_solver_iterations: NonZeroUsize::new(
                    self.integration_parameters.num_solver_iterations,
                )
                .unwrap(),
                num_additional_friction_iterations: self
                    .integration_parameters
                    .num_additional_friction_iterations,
                num_internal_pgs_iterations: self
                    .integration_parameters
                    .num_internal_pgs_iterations,
                num_internal_stabilization_iterations: self
                    .integration_parameters
                    .num_internal_stabilization_iterations,
                min_island_size: self.integration_parameters.min_island_size as usize,
                max_ccd_substeps: self.integration_parameters.max_ccd_substeps as usize,
            };

            let mut query = self.query.borrow_mut();

            self.pipeline.step(
                &self.gravity,
                &integration_parameters,
                &mut self.islands,
                &mut self.broad_phase,
                &mut self.narrow_phase,
                &mut self.bodies,
                &mut self.colliders,
                &mut self.joints.set,
                &mut self.multibody_joints.set,
                &mut self.ccd_solver,
                Some(&mut query),
                &(),
                &*self.event_handler,
            );
        }

        self.performance_statistics.step_time += instant::Instant::now() - time;
    }

    pub(super) fn add_body(&mut self, owner: Handle<Node>, mut body: RigidBody) -> RigidBodyHandle {
        body.user_data = owner.encode_to_u128();
        self.bodies.insert(body)
    }

    pub(crate) fn remove_body(&mut self, handle: RigidBodyHandle) {
        self.bodies.remove(
            handle,
            &mut self.islands,
            &mut self.colliders,
            &mut self.joints.set,
            &mut self.multibody_joints.set,
            true,
        );
    }

    pub(super) fn add_collider(
        &mut self,
        owner: Handle<Node>,
        parent_body: RigidBodyHandle,
        mut collider: Collider,
    ) -> ColliderHandle {
        collider.user_data = owner.encode_to_u128();
        self.colliders
            .insert_with_parent(collider, parent_body, &mut self.bodies)
    }

    pub(crate) fn remove_collider(&mut self, handle: ColliderHandle) -> bool {
        self.colliders
            .remove(handle, &mut self.islands, &mut self.bodies, false)
            .is_some()
    }

    pub(super) fn add_joint(
        &mut self,
        owner: Handle<Node>,
        body1: RigidBodyHandle,
        body2: RigidBodyHandle,
        joint: GenericJoint,
    ) -> ImpulseJointHandle {
        let handle = self.joints.set.insert(body1, body2, joint, false);
        self.joints.map.insert(handle, owner);
        handle
    }

    pub(crate) fn remove_joint(&mut self, handle: ImpulseJointHandle) {
        if self.joints.set.remove(handle, false).is_some() {
            assert!(self.joints.map.remove_by_key(&handle).is_some());
        }
    }

    /// Draws physics world. Very useful for debugging, it allows you to see where are
    /// rigid bodies, which colliders they have and so on.
    pub fn draw(&self, context: &mut SceneDrawingContext) {
        self.debug_render_pipeline.lock().render(
            context,
            &self.bodies,
            &self.colliders,
            &self.joints.set,
            &self.multibody_joints.set,
            &self.narrow_phase,
        );
    }

    /// Casts a ray with given options.
    pub fn cast_ray<S: QueryResultsStorage>(&self, opts: RayCastOptions, query_buffer: &mut S) {
        let time = instant::Instant::now();

        let query = self.query.borrow_mut();

        query_buffer.clear();
        let ray = Ray::new(
            opts.ray_origin,
            opts.ray_direction
                .try_normalize(f32::EPSILON)
                .unwrap_or_default(),
        );
        query.intersections_with_ray(
            &self.bodies,
            &self.colliders,
            &ray,
            opts.max_len,
            true,
            rapier3d::pipeline::QueryFilter::new().groups(InteractionGroups::new(
                u32_to_group(opts.groups.memberships.0),
                u32_to_group(opts.groups.filter.0),
            )),
            |handle, intersection| {
                query_buffer.push(Intersection {
                    collider: Handle::decode_from_u128(
                        self.colliders.get(handle).unwrap().user_data,
                    ),
                    normal: intersection.normal,
                    position: ray.point_at(intersection.time_of_impact),
                    feature: intersection.feature.into(),
                    toi: intersection.time_of_impact,
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

    /// Casts a shape at a constant linear velocity and retrieve the first collider it hits.
    ///
    /// This is similar to ray-casting except that we are casting a whole shape instead of just a
    /// point (the ray origin). In the resulting `TOI`, witness and normal 1 refer to the world
    /// collider, and are in world space.
    ///
    /// # Parameters
    ///
    /// * `graph` - a reference to the scene graph.
    /// * `shape` - The shape to cast.
    /// * `shape_pos` - The initial position of the shape to cast.
    /// * `shape_vel` - The constant velocity of the shape to cast (i.e. the cast direction).
    /// * `max_toi` - The maximum time-of-impact that can be reported by this cast. This effectively
    ///   limits the distance traveled by the shape to `shapeVel.norm() * maxToi`.
    /// * `stop_at_penetration` - If set to `false`, the linear shape-cast won’t immediately stop if
    ///   the shape is penetrating another shape at its starting point **and** its trajectory is such
    ///   that it’s on a path to exist that penetration state.
    /// * `filter`: set of rules used to determine which collider is taken into account by this scene
    ///   query.
    pub fn cast_shape(
        &self,
        graph: &Graph,
        shape: &dyn Shape,
        shape_pos: &Isometry3<f32>,
        shape_vel: &Vector3<f32>,
        max_toi: f32,
        stop_at_penetration: bool,
        filter: QueryFilter,
    ) -> Option<(Handle<Node>, TOI)> {
        let predicate = |handle: ColliderHandle, _: &Collider| -> bool {
            if let Some(pred) = filter.predicate {
                let h = Handle::decode_from_u128(self.colliders.get(handle).unwrap().user_data);
                pred(
                    h,
                    graph.node(h).component_ref::<collider::Collider>().unwrap(),
                )
            } else {
                true
            }
        };

        let filter = rapier3d::pipeline::QueryFilter {
            flags: rapier3d::pipeline::QueryFilterFlags::from_bits(filter.flags.bits()).unwrap(),
            groups: filter.groups.map(|g| {
                InteractionGroups::new(u32_to_group(g.memberships.0), u32_to_group(g.filter.0))
            }),
            exclude_collider: filter
                .exclude_collider
                .and_then(|h| graph.try_get(h))
                .and_then(|n| n.component_ref::<collider::Collider>())
                .map(|c| c.native.get()),
            exclude_rigid_body: filter
                .exclude_collider
                .and_then(|h| graph.try_get(h))
                .and_then(|n| n.component_ref::<rigidbody::RigidBody>())
                .map(|c| c.native.get()),
            predicate: Some(&predicate),
        };

        let query = self.query.borrow_mut();

        let opts = ShapeCastOptions {
            max_time_of_impact: max_toi,
            target_distance: 0.0,
            stop_at_penetration,
            compute_impact_geometry_on_penetration: true,
        };

        query
            .cast_shape(
                &self.bodies,
                &self.colliders,
                shape_pos,
                shape_vel,
                shape,
                opts,
                filter,
            )
            .map(|(handle, toi)| {
                (
                    Handle::decode_from_u128(self.colliders.get(handle).unwrap().user_data),
                    TOI {
                        toi: toi.time_of_impact,
                        witness1: toi.witness1,
                        witness2: toi.witness2,
                        normal1: toi.normal1,
                        normal2: toi.normal2,
                        status: toi.status.into(),
                    },
                )
            })
    }

    pub(crate) fn set_rigid_body_position(
        &mut self,
        rigid_body: &scene::rigidbody::RigidBody,
        new_global_transform: &Matrix4<f32>,
    ) {
        if let Some(native) = self.bodies.get_mut(rigid_body.native.get()) {
            native.set_position(
                isometry_from_global_transform(new_global_transform),
                // Do not wake up body, it is too expensive and must be done **only** by explicit
                // `wake_up` call!
                false,
            );
        }
    }

    pub(crate) fn sync_rigid_body_node(
        &mut self,
        rigid_body: &mut scene::rigidbody::RigidBody,
        parent_transform: Matrix4<f32>,
    ) {
        if *self.enabled {
            if let Some(native) = self.bodies.get(rigid_body.native.get()) {
                if native.body_type() != RigidBodyType::Fixed {
                    let local_transform: Matrix4<f32> = parent_transform
                        .try_inverse()
                        .unwrap_or_else(Matrix4::identity)
                        * native.position().to_homogeneous();

                    let new_local_rotation = UnitQuaternion::from_matrix_eps(
                        &local_transform.basis(),
                        f32::EPSILON,
                        16,
                        UnitQuaternion::identity(),
                    );
                    let new_local_position = Vector3::new(
                        local_transform[12],
                        local_transform[13],
                        local_transform[14],
                    );

                    // Do not touch local transform if position/rotation is not changing. This will
                    // prevent redundant update of its global transform, which in its turn save some
                    // CPU cycles.
                    let local_transform = rigid_body.local_transform();
                    if **local_transform.position() != new_local_position
                        || **local_transform.rotation() != new_local_rotation
                    {
                        rigid_body
                            .local_transform_mut()
                            .set_position(new_local_position)
                            .set_rotation(new_local_rotation);
                    }

                    rigid_body
                        .lin_vel
                        .set_value_with_flags(*native.linvel(), VariableFlags::MODIFIED);
                    rigid_body
                        .ang_vel
                        .set_value_with_flags(*native.angvel(), VariableFlags::MODIFIED);
                    rigid_body.sleeping = native.is_sleeping();
                }
            }
        }
    }

    pub(crate) fn sync_to_rigid_body_node(
        &mut self,
        handle: Handle<Node>,
        rigid_body_node: &scene::rigidbody::RigidBody,
    ) {
        if !rigid_body_node.is_globally_enabled() {
            self.remove_body(rigid_body_node.native.get());
            rigid_body_node.native.set(Default::default());
            return;
        }

        // Important notes!
        // 1) `get_mut` is **very** expensive because it forces physics engine to recalculate contacts
        //    and a lot of other stuff, this is why we need `anything_changed` flag.
        if rigid_body_node.native.get() != RigidBodyHandle::invalid() {
            let mut actions = rigid_body_node.actions.lock();
            if rigid_body_node.need_sync_model() || !actions.is_empty() {
                if let Some(native) = self.bodies.get_mut(rigid_body_node.native.get()) {
                    // Sync native rigid body's properties with scene node's in case if they
                    // were changed by user.
                    rigid_body_node
                        .body_type
                        .try_sync_model(|v| native.set_body_type(v.into(), false));
                    rigid_body_node
                        .lin_vel
                        .try_sync_model(|v| native.set_linvel(v, false));
                    rigid_body_node
                        .ang_vel
                        .try_sync_model(|v| native.set_angvel(v, false));
                    rigid_body_node.mass.try_sync_model(|v| {
                        match *rigid_body_node.mass_properties_type {
                            RigidBodyMassPropertiesType::Default => {
                                native.set_additional_mass(v, false);
                            }
                            RigidBodyMassPropertiesType::Additional {
                                center_of_mass,
                                principal_inertia,
                            } => {
                                native.set_additional_mass_properties(
                                    MassProperties::new(
                                        Point3::from(center_of_mass),
                                        v,
                                        principal_inertia,
                                    ),
                                    false,
                                );
                            }
                        };
                    });
                    rigid_body_node.mass_properties_type.try_sync_model(|v| {
                        match v {
                            RigidBodyMassPropertiesType::Default => {
                                native.set_additional_mass(*rigid_body_node.mass, false);
                            }
                            RigidBodyMassPropertiesType::Additional {
                                center_of_mass,
                                principal_inertia,
                            } => {
                                native.set_additional_mass_properties(
                                    MassProperties::new(
                                        Point3::from(center_of_mass),
                                        *rigid_body_node.mass,
                                        principal_inertia,
                                    ),
                                    false,
                                );
                            }
                        };
                    });
                    rigid_body_node
                        .lin_damping
                        .try_sync_model(|v| native.set_linear_damping(v));
                    rigid_body_node
                        .ang_damping
                        .try_sync_model(|v| native.set_angular_damping(v));
                    rigid_body_node
                        .ccd_enabled
                        .try_sync_model(|v| native.enable_ccd(v));
                    rigid_body_node.can_sleep.try_sync_model(|v| {
                        let activation = native.activation_mut();
                        if v {
                            activation.normalized_linear_threshold =
                                RigidBodyActivation::default_normalized_linear_threshold();
                            activation.angular_threshold =
                                RigidBodyActivation::default_angular_threshold();
                        } else {
                            activation.sleeping = false;
                            activation.normalized_linear_threshold = -1.0;
                            activation.angular_threshold = -1.0;
                        };
                    });
                    rigid_body_node
                        .translation_locked
                        .try_sync_model(|v| native.lock_translations(v, false));
                    rigid_body_node.x_rotation_locked.try_sync_model(|v| {
                        native.set_enabled_rotations(
                            !v,
                            !native.is_rotation_locked()[1],
                            !native.is_rotation_locked()[2],
                            false,
                        );
                    });
                    rigid_body_node.y_rotation_locked.try_sync_model(|v| {
                        native.set_enabled_rotations(
                            !native.is_rotation_locked()[0],
                            !v,
                            !native.is_rotation_locked()[2],
                            false,
                        );
                    });
                    rigid_body_node.z_rotation_locked.try_sync_model(|v| {
                        native.set_enabled_rotations(
                            !native.is_rotation_locked()[0],
                            !native.is_rotation_locked()[1],
                            !v,
                            false,
                        );
                    });
                    rigid_body_node
                        .dominance
                        .try_sync_model(|v| native.set_dominance_group(v));
                    rigid_body_node
                        .gravity_scale
                        .try_sync_model(|v| native.set_gravity_scale(v, false));

                    // We must reset any forces applied at previous update step, otherwise physics engine
                    // will keep pushing the rigid body infinitely.
                    if rigid_body_node.reset_forces.replace(false) {
                        native.reset_forces(false);
                        native.reset_torques(false);
                    }

                    while let Some(action) = actions.pop_front() {
                        match action {
                            ApplyAction::Force(force) => {
                                native.add_force(force, false);
                                rigid_body_node.reset_forces.set(true);
                            }
                            ApplyAction::Torque(torque) => {
                                native.add_torque(torque, false);
                                rigid_body_node.reset_forces.set(true);
                            }
                            ApplyAction::ForceAtPoint { force, point } => {
                                native.add_force_at_point(force, Point3::from(point), false);
                                rigid_body_node.reset_forces.set(true);
                            }
                            ApplyAction::Impulse(impulse) => native.apply_impulse(impulse, false),
                            ApplyAction::TorqueImpulse(impulse) => {
                                native.apply_torque_impulse(impulse, false)
                            }
                            ApplyAction::ImpulseAtPoint { impulse, point } => {
                                native.apply_impulse_at_point(impulse, Point3::from(point), false)
                            }
                            ApplyAction::WakeUp => native.wake_up(true),
                            ApplyAction::NextTranslation(position) => {
                                native.set_next_kinematic_translation(position)
                            }
                            ApplyAction::NextRotation(rotation) => {
                                native.set_next_kinematic_rotation(rotation)
                            }
                            ApplyAction::NextPosition(position) => {
                                native.set_next_kinematic_position(position)
                            }
                        }
                    }
                }
            }
        } else {
            let mut builder = RigidBodyBuilder::new(rigid_body_node.body_type().into())
                .position(isometry_from_global_transform(
                    &rigid_body_node.global_transform(),
                ))
                .ccd_enabled(rigid_body_node.is_ccd_enabled())
                .additional_mass(rigid_body_node.mass())
                .angvel(*rigid_body_node.ang_vel)
                .linvel(*rigid_body_node.lin_vel)
                .linear_damping(*rigid_body_node.lin_damping)
                .angular_damping(*rigid_body_node.ang_damping)
                .can_sleep(rigid_body_node.is_can_sleep())
                .sleeping(rigid_body_node.is_sleeping())
                .dominance_group(rigid_body_node.dominance())
                .gravity_scale(rigid_body_node.gravity_scale())
                .enabled_rotations(
                    !rigid_body_node.is_x_rotation_locked(),
                    !rigid_body_node.is_y_rotation_locked(),
                    !rigid_body_node.is_z_rotation_locked(),
                );

            match *rigid_body_node.mass_properties_type {
                RigidBodyMassPropertiesType::Default => {
                    builder = builder.additional_mass(*rigid_body_node.mass);
                }
                RigidBodyMassPropertiesType::Additional {
                    center_of_mass,
                    principal_inertia,
                } => {
                    builder = builder.additional_mass_properties(MassProperties::new(
                        Point3::from(center_of_mass),
                        *rigid_body_node.mass,
                        principal_inertia,
                    ));
                }
            };
            if rigid_body_node.is_translation_locked() {
                builder = builder.lock_translations();
            }

            rigid_body_node
                .native
                .set(self.add_body(handle, builder.build()));

            Log::writeln(
                MessageKind::Information,
                format!(
                    "Native rigid body was created for node {}",
                    rigid_body_node.name()
                ),
            );
        }
    }

    pub(crate) fn sync_to_collider_node(
        &mut self,
        nodes: &NodePool,
        handle: Handle<Node>,
        collider_node: &scene::collider::Collider,
    ) {
        if !collider_node.is_globally_enabled() {
            self.remove_collider(collider_node.native.get());
            collider_node.native.set(Default::default());
            return;
        }

        let anything_changed = collider_node.needs_sync_model();

        // Important notes!
        // 1) The collider node may lack backing native physics collider in case if it
        //    is not attached to a rigid body.
        // 2) `get_mut` is **very** expensive because it forces physics engine to recalculate contacts
        //    and a lot of other stuff, this is why we need `anything_changed` flag.
        if collider_node.native.get() != ColliderHandle::invalid() {
            if anything_changed {
                if let Some(native) = self.colliders.get_mut(collider_node.native.get()) {
                    collider_node
                        .restitution
                        .try_sync_model(|v| native.set_restitution(v));
                    collider_node.collision_groups.try_sync_model(|v| {
                        native.set_collision_groups(InteractionGroups::new(
                            u32_to_group(v.memberships.0),
                            u32_to_group(v.filter.0),
                        ))
                    });
                    collider_node.solver_groups.try_sync_model(|v| {
                        native.set_solver_groups(InteractionGroups::new(
                            u32_to_group(v.memberships.0),
                            u32_to_group(v.filter.0),
                        ))
                    });
                    collider_node
                        .friction
                        .try_sync_model(|v| native.set_friction(v));
                    collider_node
                        .is_sensor
                        .try_sync_model(|v| native.set_sensor(v));
                    collider_node
                        .friction_combine_rule
                        .try_sync_model(|v| native.set_friction_combine_rule(v.into()));
                    collider_node
                        .restitution_combine_rule
                        .try_sync_model(|v| native.set_restitution_combine_rule(v.into()));
                    let mut remove_collider = false;
                    collider_node.shape.try_sync_model(|v| {
                        let inv_global_transform = isometric_global_transform(nodes, handle)
                            .try_inverse()
                            .unwrap_or_default();

                        if let Some(shape) = collider_shape_into_native_shape(
                            &v,
                            inv_global_transform,
                            handle,
                            nodes,
                        ) {
                            native.set_shape(shape);
                        } else {
                            remove_collider = true;
                        }
                    });
                    if remove_collider {
                        self.remove_collider(collider_node.native.get());
                        collider_node.native.set(ColliderHandle::invalid());
                    }
                }
            }
        } else if let Some(parent_body) = nodes
            .try_borrow(collider_node.parent())
            .and_then(|n| n.cast::<scene::rigidbody::RigidBody>())
        {
            if parent_body.native.get() != RigidBodyHandle::invalid() {
                let inv_global_transform = isometric_global_transform(nodes, handle)
                    .try_inverse()
                    .unwrap();
                let rigid_body_native = parent_body.native.get();
                if let Some(shape) = collider_shape_into_native_shape(
                    collider_node.shape(),
                    inv_global_transform,
                    handle,
                    nodes,
                ) {
                    let mut builder = ColliderBuilder::new(shape)
                        .position(Isometry3 {
                            rotation: **collider_node.local_transform().rotation(),
                            translation: Translation3 {
                                vector: **collider_node.local_transform().position(),
                            },
                        })
                        .friction(collider_node.friction())
                        .restitution(collider_node.restitution())
                        .collision_groups(InteractionGroups::new(
                            u32_to_group(collider_node.collision_groups().memberships.0),
                            u32_to_group(collider_node.collision_groups().filter.0),
                        ))
                        .friction_combine_rule(collider_node.friction_combine_rule().into())
                        .restitution_combine_rule(collider_node.restitution_combine_rule().into())
                        .solver_groups(InteractionGroups::new(
                            u32_to_group(collider_node.solver_groups().memberships.0),
                            u32_to_group(collider_node.solver_groups().filter.0),
                        ))
                        .sensor(collider_node.is_sensor());

                    if let Some(density) = collider_node.density() {
                        builder = builder.density(density);
                    }

                    let native_handle =
                        self.add_collider(handle, rigid_body_native, builder.build());

                    collider_node.native.set(native_handle);

                    Log::writeln(
                        MessageKind::Information,
                        format!(
                            "Native collider was created for node {}",
                            collider_node.name()
                        ),
                    );
                }
            }
        }
    }

    pub(crate) fn sync_to_joint_node(
        &mut self,
        nodes: &NodePool,
        handle: Handle<Node>,
        joint: &scene::joint::Joint,
    ) {
        if !joint.is_globally_enabled() {
            self.remove_joint(joint.native.get());
            joint.native.set(ImpulseJointHandle(Default::default()));
            return;
        }

        if let Some(native) = self.joints.set.get_mut(joint.native.get(), false) {
            joint.body1.try_sync_model(|v| {
                if let Some(rigid_body_node) = nodes.typed_ref(v) {
                    native.body1 = rigid_body_node.native.get();
                }
            });
            joint.body2.try_sync_model(|v| {
                if let Some(rigid_body_node) = nodes.typed_ref(v) {
                    native.body2 = rigid_body_node.native.get();
                }
            });
            joint.params.try_sync_model(|v| {
                native.data =
                    // Preserve local frames.
                    convert_joint_params(v, native.data.local_frame1, native.data.local_frame2)
            });
            joint.contacts_enabled.try_sync_model(|v| {
                native.data.set_contacts_enabled(v);
            });

            let mut local_frames = joint.local_frames.borrow_mut();
            if local_frames.is_none() {
                if let (Some(body1), Some(body2)) = (
                    nodes.typed_ref(joint.body1()),
                    nodes.typed_ref(joint.body2()),
                ) {
                    let (local_frame1, local_frame2) = calculate_local_frames(joint, body1, body2);
                    native.data =
                        convert_joint_params((*joint.params).clone(), local_frame1, local_frame2);
                    *local_frames = Some(JointLocalFrames::new(&local_frame1, &local_frame2));
                }
            }
        } else {
            let body1_handle = joint.body1();
            let body2_handle = joint.body2();
            let params = joint.params().clone();

            // A native joint can be created iff both rigid bodies are correctly assigned and their respective
            // native bodies exists.
            if let (Some(body1), Some(body2)) = (
                nodes
                    .typed_ref(body1_handle)
                    .filter(|b| self.bodies.get(b.native.get()).is_some()),
                nodes
                    .typed_ref(body2_handle)
                    .filter(|b| self.bodies.get(b.native.get()).is_some()),
            ) {
                // Calculate local frames first (if needed).
                let mut local_frames = joint.local_frames.borrow_mut();
                let (local_frame1, local_frame2) = local_frames
                    .clone()
                    .map(|frames| {
                        (
                            Isometry3 {
                                rotation: frames.body1.rotation,
                                translation: Translation {
                                    vector: frames.body1.position,
                                },
                            },
                            Isometry3 {
                                rotation: frames.body2.rotation,
                                translation: Translation {
                                    vector: frames.body2.position,
                                },
                            },
                        )
                    })
                    .unwrap_or_else(|| calculate_local_frames(joint, body1, body2));

                let native_body1 = body1.native.get();
                let native_body2 = body2.native.get();

                let mut native_joint = convert_joint_params(params, local_frame1, local_frame2);
                native_joint.contacts_enabled = joint.is_contacts_enabled();
                let native_handle =
                    self.add_joint(handle, native_body1, native_body2, native_joint);

                joint.native.set(native_handle);
                *local_frames = Some(JointLocalFrames::new(&local_frame1, &local_frame2));

                Log::writeln(
                    MessageKind::Information,
                    format!("Native joint was created for node {}", joint.name()),
                );
            }
        }
    }

    /// Intersections checks between regular colliders and sensor colliders
    pub(crate) fn intersections_with(
        &self,
        collider: ColliderHandle,
    ) -> impl Iterator<Item = IntersectionPair> + '_ {
        self.narrow_phase
            .intersection_pairs_with(collider)
            .filter_map(|(collider1, collider2, intersecting)| {
                Some(IntersectionPair {
                    collider1: Handle::decode_from_u128(self.colliders.get(collider1)?.user_data),
                    collider2: Handle::decode_from_u128(self.colliders.get(collider2)?.user_data),
                    has_any_active_contact: intersecting,
                })
            })
    }

    /// Contacts checks between two regular colliders
    pub(crate) fn contacts_with(
        &self,
        collider: ColliderHandle,
    ) -> impl Iterator<Item = ContactPair> + '_ {
        self.narrow_phase
            // Note: contacts with will only return the interaction between 2 non-sensor nodes
            // https://rapier.rs/docs/user_guides/rust/advanced_collision_detection/#the-contact-graph
            .contact_pairs_with(collider)
            .filter_map(|c| ContactPair::from_native(c, self))
    }

    /// Returns an iterator over all contact pairs generated in this frame.
    pub fn contacts(&self) -> impl Iterator<Item = ContactPair> + '_ {
        self.narrow_phase
            .contact_pairs()
            .filter_map(|c| ContactPair::from_native(c, self))
    }
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for PhysicsWorld {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "PhysicsWorld")
    }
}
