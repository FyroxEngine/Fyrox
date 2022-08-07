//! For efficient rendering each mesh is split into sets of triangles that use the same texture,
//! such sets are called surfaces.
//!
//! Surfaces can use the same data source across many instances, this is a memory optimization for
//! being able to re-use data when you need to draw the same mesh in many places.

use crate::{
    core::{
        algebra::{Matrix4, Point3, Vector2, Vector3, Vector4},
        hash_combine,
        inspect::{Inspect, PropertyInfo},
        math::TriangleDefinition,
        parking_lot::Mutex,
        pool::{ErasedHandle, Handle},
        reflect::Reflect,
        sparse::AtomicIndex,
        visitor::{Visit, VisitResult, Visitor},
    },
    material::Material,
    renderer::{cache::CacheEntry, framework},
    scene::{
        mesh::{
            buffer::{
                TriangleBuffer, VertexAttributeDescriptor, VertexAttributeUsage, VertexBuffer,
                VertexFetchError, VertexReadTrait, VertexWriteTrait,
            },
            vertex::StaticVertex,
        },
        node::Node,
    },
    utils::raw_mesh::{RawMesh, RawMeshBuilder},
};
use fxhash::FxHasher;
use std::{hash::Hasher, sync::Arc};

/// Data source of a surface. Each surface can share same data source, this is used
/// in instancing technique to render multiple instances of same model at different
/// places.
#[derive(Debug, Clone, Default)]
pub struct SurfaceData {
    /// Current vertex buffer.
    pub vertex_buffer: VertexBuffer,
    /// Current geometry buffer.
    pub geometry_buffer: TriangleBuffer,
    // If true - indicates that surface was generated and does not have reference
    // resource. Procedural data will be serialized.
    is_procedural: bool,
    pub(crate) cache_entry: AtomicIndex<CacheEntry<framework::geometry_buffer::GeometryBuffer>>,
}

impl SurfaceData {
    /// Creates new data source using given vertices and indices.
    pub fn new(
        vertex_buffer: VertexBuffer,
        triangles: TriangleBuffer,
        is_procedural: bool,
    ) -> Self {
        Self {
            vertex_buffer,
            geometry_buffer: triangles,
            is_procedural,
            cache_entry: AtomicIndex::unassigned(),
        }
    }

    /// Applies given transform for every spatial part of the data (vertex position, normal, tangent).
    pub fn transform_geometry(&mut self, transform: &Matrix4<f32>) -> Result<(), VertexFetchError> {
        // Discard scale by inverse and transpose given transform (M^-1)^T
        let normal_matrix = transform.try_inverse().unwrap_or_default().transpose();

        let mut vertex_buffer_mut = self.vertex_buffer.modify();
        for mut view in vertex_buffer_mut.iter_mut() {
            let position = view.read_3_f32(VertexAttributeUsage::Position)?;
            view.write_3_f32(
                VertexAttributeUsage::Position,
                transform.transform_point(&Point3::from(position)).coords,
            )?;
            let normal = view.read_3_f32(VertexAttributeUsage::Normal)?;
            view.write_3_f32(
                VertexAttributeUsage::Normal,
                normal_matrix.transform_vector(&normal),
            )?;
            let tangent = view.read_4_f32(VertexAttributeUsage::Tangent)?;
            let new_tangent = normal_matrix.transform_vector(&tangent.xyz());
            // Keep sign (W).
            view.write_4_f32(
                VertexAttributeUsage::Tangent,
                Vector4::new(new_tangent.x, new_tangent.y, new_tangent.z, tangent.w),
            )?;
        }

        Ok(())
    }

    /// Converts raw mesh into "renderable" mesh. It is useful to build procedural
    /// meshes.
    pub fn from_raw_mesh<T: Copy>(
        raw: RawMesh<T>,
        layout: &[VertexAttributeDescriptor],
        is_procedural: bool,
    ) -> Self {
        Self {
            vertex_buffer: VertexBuffer::new(raw.vertices.len(), layout, raw.vertices).unwrap(),
            geometry_buffer: TriangleBuffer::new(raw.triangles),
            is_procedural,
            cache_entry: AtomicIndex::unassigned(),
        }
    }

    /// Calculates tangents of surface. Tangents are needed for correct lighting, you will
    /// get incorrect lighting if tangents of your surface are invalid! When engine loads
    /// a mesh from "untrusted" source, it automatically calculates tangents for you, so
    /// there is no need to call this manually in this case. However if you making your
    /// mesh procedurally, you have to use this method!
    pub fn calculate_tangents(&mut self) -> Result<(), VertexFetchError> {
        let mut tan1 = vec![Vector3::default(); self.vertex_buffer.vertex_count() as usize];
        let mut tan2 = vec![Vector3::default(); self.vertex_buffer.vertex_count() as usize];

        for triangle in self.geometry_buffer.iter() {
            let i1 = triangle[0] as usize;
            let i2 = triangle[1] as usize;
            let i3 = triangle[2] as usize;

            let view1 = &self.vertex_buffer.get(i1).unwrap();
            let view2 = &self.vertex_buffer.get(i2).unwrap();
            let view3 = &self.vertex_buffer.get(i3).unwrap();

            let v1 = view1.read_3_f32(VertexAttributeUsage::Position)?;
            let v2 = view2.read_3_f32(VertexAttributeUsage::Position)?;
            let v3 = view3.read_3_f32(VertexAttributeUsage::Position)?;

            let w1 = view1.read_3_f32(VertexAttributeUsage::TexCoord0)?;
            let w2 = view2.read_3_f32(VertexAttributeUsage::TexCoord0)?;
            let w3 = view3.read_3_f32(VertexAttributeUsage::TexCoord0)?;

            let x1 = v2.x - v1.x;
            let x2 = v3.x - v1.x;
            let y1 = v2.y - v1.y;
            let y2 = v3.y - v1.y;
            let z1 = v2.z - v1.z;
            let z2 = v3.z - v1.z;

            let s1 = w2.x - w1.x;
            let s2 = w3.x - w1.x;
            let t1 = w2.y - w1.y;
            let t2 = w3.y - w1.y;

            let r = 1.0 / (s1 * t2 - s2 * t1);

            let sdir = Vector3::new(
                (t2 * x1 - t1 * x2) * r,
                (t2 * y1 - t1 * y2) * r,
                (t2 * z1 - t1 * z2) * r,
            );

            tan1[i1] += sdir;
            tan1[i2] += sdir;
            tan1[i3] += sdir;

            let tdir = Vector3::new(
                (s1 * x2 - s2 * x1) * r,
                (s1 * y2 - s2 * y1) * r,
                (s1 * z2 - s2 * z1) * r,
            );
            tan2[i1] += tdir;
            tan2[i2] += tdir;
            tan2[i3] += tdir;
        }

        let mut vertex_buffer_mut = self.vertex_buffer.modify();
        for (mut view, (t1, t2)) in vertex_buffer_mut.iter_mut().zip(tan1.into_iter().zip(tan2)) {
            let normal = view.read_3_f32(VertexAttributeUsage::Normal)?;

            // Gram-Schmidt orthogonalize
            let tangent = (t1 - normal.scale(normal.dot(&t1)))
                .try_normalize(f32::EPSILON)
                .unwrap_or_else(|| Vector3::new(0.0, 1.0, 0.0));
            let handedness = normal.cross(&t1).dot(&t2).signum();
            view.write_4_f32(
                VertexAttributeUsage::Tangent,
                Vector4::new(tangent.x, tangent.y, tangent.z, handedness),
            )?;
        }

        Ok(())
    }

    /// Creates a quad oriented on oXY plane with unit width and height.
    pub fn make_unit_xy_quad() -> Self {
        let vertices = vec![
            StaticVertex {
                position: Vector3::default(),
                normal: Vector3::z(),
                tex_coord: Vector2::y(),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::x(),
                normal: Vector3::z(),
                tex_coord: Vector2::new(1.0, 1.0),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(1.0, 1.0, 0.0),
                normal: Vector3::z(),
                tex_coord: Vector2::x(),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::y(),
                normal: Vector3::z(),
                tex_coord: Vector2::default(),
                tangent: Vector4::default(),
            },
        ];

        let triangles = vec![TriangleDefinition([0, 1, 2]), TriangleDefinition([0, 2, 3])];

        Self::new(
            VertexBuffer::new(vertices.len(), StaticVertex::layout(), vertices).unwrap(),
            TriangleBuffer::new(triangles),
            true,
        )
    }

    /// Creates a degenerated quad which collapsed in a point. This is very special method for
    /// sprite renderer - shader will automatically "push" corners in correct sides so sprite
    /// will always face camera.
    pub fn make_collapsed_xy_quad() -> Self {
        let vertices = vec![
            StaticVertex {
                position: Vector3::default(),
                normal: Vector3::z(),
                tex_coord: Vector2::default(),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::default(),
                normal: Vector3::z(),
                tex_coord: Vector2::x(),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::default(),
                normal: Vector3::z(),
                tex_coord: Vector2::new(1.0, 1.0),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::default(),
                normal: Vector3::z(),
                tex_coord: Vector2::y(),
                tangent: Vector4::default(),
            },
        ];

        let triangles = vec![TriangleDefinition([0, 1, 2]), TriangleDefinition([0, 2, 3])];

        Self::new(
            VertexBuffer::new(vertices.len(), StaticVertex::layout(), vertices).unwrap(),
            TriangleBuffer::new(triangles),
            true,
        )
    }

    /// Creates new quad at oXY plane with given transform.
    pub fn make_quad(transform: &Matrix4<f32>) -> Self {
        let vertices = vec![
            StaticVertex {
                position: Vector3::new(-0.5, 0.5, 0.0),
                normal: -Vector3::z(),
                tex_coord: Vector2::new(1.0, 1.0),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(0.5, 0.5, 0.0),
                normal: -Vector3::z(),
                tex_coord: Vector2::new(0.0, 1.0),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(0.5, -0.5, 0.0),
                normal: -Vector3::z(),
                tex_coord: Vector2::new(0.0, 0.0),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(-0.5, -0.5, 0.0),
                normal: -Vector3::z(),
                tex_coord: Vector2::new(1.0, 0.0),
                tangent: Vector4::default(),
            },
        ];

        let mut data = Self::new(
            VertexBuffer::new(vertices.len(), StaticVertex::layout(), vertices).unwrap(),
            TriangleBuffer::new(vec![
                TriangleDefinition([0, 1, 2]),
                TriangleDefinition([0, 2, 3]),
            ]),
            true,
        );
        data.calculate_tangents().unwrap();
        data.transform_geometry(transform).unwrap();
        data
    }

    /// Calculates per-face normals. This method is fast, but have very poor quality, and surface
    /// will look facet.
    pub fn calculate_normals(&mut self) -> Result<(), VertexFetchError> {
        let mut vertex_buffer_mut = self.vertex_buffer.modify();
        for triangle in self.geometry_buffer.iter() {
            let ia = triangle[0] as usize;
            let ib = triangle[1] as usize;
            let ic = triangle[2] as usize;

            let a = vertex_buffer_mut
                .get(ia)
                .unwrap()
                .read_3_f32(VertexAttributeUsage::Position)?;
            let b = vertex_buffer_mut
                .get(ib)
                .unwrap()
                .read_3_f32(VertexAttributeUsage::Position)?;
            let c = vertex_buffer_mut
                .get(ic)
                .unwrap()
                .read_3_f32(VertexAttributeUsage::Position)?;

            let normal = (b - a).cross(&(c - a)).normalize();

            vertex_buffer_mut
                .get_mut(ia)
                .unwrap()
                .write_3_f32(VertexAttributeUsage::Normal, normal)?;
            vertex_buffer_mut
                .get_mut(ib)
                .unwrap()
                .write_3_f32(VertexAttributeUsage::Normal, normal)?;
            vertex_buffer_mut
                .get_mut(ic)
                .unwrap()
                .write_3_f32(VertexAttributeUsage::Normal, normal)?;
        }

        Ok(())
    }

    /// Creates sphere of specified radius with given slices and stacks.
    pub fn make_sphere(slices: usize, stacks: usize, r: f32, transform: &Matrix4<f32>) -> Self {
        let mut builder = RawMeshBuilder::<StaticVertex>::new(stacks * slices, stacks * slices * 3);

        let d_theta = std::f32::consts::PI / slices as f32;
        let d_phi = 2.0 * std::f32::consts::PI / stacks as f32;
        let d_tc_y = 1.0 / stacks as f32;
        let d_tc_x = 1.0 / slices as f32;

        for i in 0..stacks {
            for j in 0..slices {
                let nj = j + 1;
                let ni = i + 1;

                let k0 = r * (d_theta * i as f32).sin();
                let k1 = (d_phi * j as f32).cos();
                let k2 = (d_phi * j as f32).sin();
                let k3 = r * (d_theta * i as f32).cos();

                let k4 = r * (d_theta * ni as f32).sin();
                let k5 = (d_phi * nj as f32).cos();
                let k6 = (d_phi * nj as f32).sin();
                let k7 = r * (d_theta * ni as f32).cos();

                if i != (stacks - 1) {
                    let v0 = Vector3::new(k0 * k1, k0 * k2, k3);
                    let t0 = Vector2::new(d_tc_x * j as f32, d_tc_y * i as f32);

                    let v1 = Vector3::new(k4 * k1, k4 * k2, k7);
                    let t1 = Vector2::new(d_tc_x * j as f32, d_tc_y * ni as f32);

                    let v2 = Vector3::new(k4 * k5, k4 * k6, k7);
                    let t2 = Vector2::new(d_tc_x * nj as f32, d_tc_y * ni as f32);

                    builder.insert(StaticVertex::from_pos_uv_normal(v0, t0, v0));
                    builder.insert(StaticVertex::from_pos_uv_normal(v1, t1, v1));
                    builder.insert(StaticVertex::from_pos_uv_normal(v2, t2, v2));
                }

                if i != 0 {
                    let v0 = Vector3::new(k4 * k5, k4 * k6, k7);
                    let t0 = Vector2::new(d_tc_x * nj as f32, d_tc_y * ni as f32);

                    let v1 = Vector3::new(k0 * k5, k0 * k6, k3);
                    let t1 = Vector2::new(d_tc_x * nj as f32, d_tc_y * i as f32);

                    let v2 = Vector3::new(k0 * k1, k0 * k2, k3);
                    let t2 = Vector2::new(d_tc_x * j as f32, d_tc_y * i as f32);

                    builder.insert(StaticVertex::from_pos_uv_normal(v0, t0, v0));
                    builder.insert(StaticVertex::from_pos_uv_normal(v1, t1, v1));
                    builder.insert(StaticVertex::from_pos_uv_normal(v2, t2, v2));
                }
            }
        }

        let mut data = Self::from_raw_mesh(builder.build(), StaticVertex::layout(), true);
        data.calculate_tangents().unwrap();
        data.transform_geometry(transform).unwrap();
        data
    }

    /// Creates vertical cone - it has its vertex higher than base.
    pub fn make_cone(sides: usize, r: f32, h: f32, transform: &Matrix4<f32>) -> Self {
        let mut builder = RawMeshBuilder::<StaticVertex>::new(3 * sides, 3 * sides);

        let d_phi = 2.0 * std::f32::consts::PI / sides as f32;
        let d_theta = 1.0 / sides as f32;

        for i in 0..sides {
            let nx0 = (d_phi * i as f32).cos();
            let ny0 = (d_phi * i as f32).sin();
            let nx1 = (d_phi * (i + 1) as f32).cos();
            let ny1 = (d_phi * (i + 1) as f32).sin();

            let x0 = r * nx0;
            let z0 = r * ny0;
            let x1 = r * nx1;
            let z1 = r * ny1;
            let tx0 = d_theta * i as f32;
            let tx1 = d_theta * (i + 1) as f32;

            // back cap
            let (t_cap_y_curr, t_cap_x_curr) = (d_phi * i as f32).sin_cos();
            let (t_cap_y_next, t_cap_x_next) = (d_phi * (i + 1) as f32).sin_cos();

            let t_cap_x_curr = t_cap_x_curr * 0.5 + 0.5;
            let t_cap_y_curr = t_cap_y_curr * 0.5 + 0.5;

            let t_cap_x_next = t_cap_x_next * 0.5 + 0.5;
            let t_cap_y_next = t_cap_y_next * 0.5 + 0.5;

            builder.insert(StaticVertex::from_pos_uv_normal(
                Vector3::new(0.0, 0.0, 0.0),
                Vector2::new(0.5, 0.5),
                Vector3::new(0.0, -1.0, 0.0),
            ));
            builder.insert(StaticVertex::from_pos_uv_normal(
                Vector3::new(x0, 0.0, z0),
                Vector2::new(t_cap_x_curr, t_cap_y_curr),
                Vector3::new(0.0, -1.0, 0.0),
            ));
            builder.insert(StaticVertex::from_pos_uv_normal(
                Vector3::new(x1, 0.0, z1),
                Vector2::new(t_cap_x_next, t_cap_y_next),
                Vector3::new(0.0, -1.0, 0.0),
            ));

            // sides
            let tip = Vector3::new(0.0, h, 0.0);
            let v_curr = Vector3::new(x0, 0.0, z0);
            let v_next = Vector3::new(x1, 0.0, z1);
            let n_next = (tip - v_next).cross(&(v_next - v_curr));
            let n_curr = (tip - v_curr).cross(&(v_next - v_curr));

            builder.insert(StaticVertex::from_pos_uv_normal(
                tip,
                Vector2::new(0.5, 0.0),
                n_curr,
            ));
            builder.insert(StaticVertex::from_pos_uv_normal(
                v_next,
                Vector2::new(tx1, 1.0),
                n_next,
            ));
            builder.insert(StaticVertex::from_pos_uv_normal(
                v_curr,
                Vector2::new(tx0, 1.0),
                n_curr,
            ));
        }

        let mut data = Self::from_raw_mesh(builder.build(), StaticVertex::layout(), true);
        data.calculate_tangents().unwrap();
        data.transform_geometry(transform).unwrap();
        data
    }

    /// Creates torus.
    pub fn make_torus(
        r1: f32,
        r2: f32,
        num_rings: usize,
        num_segments: usize,
        transform: &Matrix4<f32>,
    ) -> Self {
        let mut vertices = Vec::new();
        for j in 0..=num_rings {
            for i in 0..=num_segments {
                let u = i as f32 / num_segments as f32 * std::f32::consts::TAU;
                let v = j as f32 / num_rings as f32 * std::f32::consts::TAU;

                let center = Vector3::new(r1 * u.cos(), r1 * u.sin(), 0.0);

                let position = Vector3::new(
                    (r1 + r2 * v.cos()) * u.cos(),
                    r2 * v.sin(),
                    (r1 + r2 * v.cos()) * u.sin(),
                );

                let uv = Vector2::new(i as f32 / num_segments as f32, j as f32 / num_rings as f32);

                let normal = (position - center)
                    .try_normalize(f32::EPSILON)
                    .unwrap_or_default();

                vertices.push(StaticVertex::from_pos_uv_normal(position, uv, normal));
            }
        }

        let mut triangles = Vec::new();
        for j in 1..=num_rings {
            for i in 1..=num_segments {
                let a = ((num_segments + 1) * j + i - 1) as u32;
                let b = ((num_segments + 1) * (j - 1) + i - 1) as u32;
                let c = ((num_segments + 1) * (j - 1) + i) as u32;
                let d = ((num_segments + 1) * j + i) as u32;

                triangles.push(TriangleDefinition([a, b, d]));
                triangles.push(TriangleDefinition([b, c, d]));
            }
        }

        let mut data = Self::new(
            VertexBuffer::new(vertices.len(), StaticVertex::layout(), vertices).unwrap(),
            TriangleBuffer::new(triangles),
            true,
        );
        data.calculate_tangents().unwrap();
        data.transform_geometry(transform).unwrap();
        data
    }

    /// Creates vertical cylinder.
    pub fn make_cylinder(
        sides: usize,
        r: f32,
        h: f32,
        caps: bool,
        transform: &Matrix4<f32>,
    ) -> Self {
        let mut builder = RawMeshBuilder::<StaticVertex>::new(3 * sides, 3 * sides);

        let d_phi = 2.0 * std::f32::consts::PI / sides as f32;
        let d_theta = 1.0 / sides as f32;

        for i in 0..sides {
            let nx0 = (d_phi * i as f32).cos();
            let ny0 = (d_phi * i as f32).sin();
            let nx1 = (d_phi * (i + 1) as f32).cos();
            let ny1 = (d_phi * (i + 1) as f32).sin();

            let x0 = r * nx0;
            let z0 = r * ny0;
            let x1 = r * nx1;
            let z1 = r * ny1;

            if caps {
                let (t_cap_y_curr, t_cap_x_curr) = (d_phi * i as f32).sin_cos();
                let (t_cap_y_next, t_cap_x_next) = (d_phi * (i + 1) as f32).sin_cos();

                let t_cap_x_curr = t_cap_x_curr * 0.5 + 0.5;
                let t_cap_y_curr = t_cap_y_curr * 0.5 + 0.5;

                let t_cap_x_next = t_cap_x_next * 0.5 + 0.5;
                let t_cap_y_next = t_cap_y_next * 0.5 + 0.5;

                // front cap
                builder.insert(StaticVertex::from_pos_uv_normal(
                    Vector3::new(x1, h, z1),
                    Vector2::new(t_cap_x_next, t_cap_y_next),
                    Vector3::new(0.0, 1.0, 0.0),
                ));
                builder.insert(StaticVertex::from_pos_uv_normal(
                    Vector3::new(x0, h, z0),
                    Vector2::new(t_cap_x_curr, t_cap_y_curr),
                    Vector3::new(0.0, 1.0, 0.0),
                ));
                builder.insert(StaticVertex::from_pos_uv_normal(
                    Vector3::new(0.0, h, 0.0),
                    Vector2::new(0.5, 0.5),
                    Vector3::new(0.0, 1.0, 0.0),
                ));

                // back cap
                builder.insert(StaticVertex::from_pos_uv_normal(
                    Vector3::new(x0, 0.0, z0),
                    Vector2::new(t_cap_x_curr, t_cap_y_curr),
                    Vector3::new(0.0, -1.0, 0.0),
                ));
                builder.insert(StaticVertex::from_pos_uv_normal(
                    Vector3::new(x1, 0.0, z1),
                    Vector2::new(t_cap_x_next, t_cap_y_next),
                    Vector3::new(0.0, -1.0, 0.0),
                ));
                builder.insert(StaticVertex::from_pos_uv_normal(
                    Vector3::new(0.0, 0.0, 0.0),
                    Vector2::new(0.5, 0.5),
                    Vector3::new(0.0, -1.0, 0.0),
                ));
            }

            let t_side_curr = d_theta * i as f32;
            let t_side_next = d_theta * (i + 1) as f32;

            // sides
            builder.insert(StaticVertex::from_pos_uv_normal(
                Vector3::new(x0, 0.0, z0),
                Vector2::new(t_side_curr, 0.0),
                Vector3::new(x0, 0.0, z0),
            ));
            builder.insert(StaticVertex::from_pos_uv_normal(
                Vector3::new(x0, h, z0),
                Vector2::new(t_side_curr, 1.0),
                Vector3::new(x0, 0.0, z0),
            ));
            builder.insert(StaticVertex::from_pos_uv_normal(
                Vector3::new(x1, 0.0, z1),
                Vector2::new(t_side_next, 0.0),
                Vector3::new(x1, 0.0, z1),
            ));

            builder.insert(StaticVertex::from_pos_uv_normal(
                Vector3::new(x1, 0.0, z1),
                Vector2::new(t_side_next, 0.0),
                Vector3::new(x1, 0.0, z1),
            ));
            builder.insert(StaticVertex::from_pos_uv_normal(
                Vector3::new(x0, h, z0),
                Vector2::new(t_side_curr, 1.0),
                Vector3::new(x0, 0.0, z0),
            ));
            builder.insert(StaticVertex::from_pos_uv_normal(
                Vector3::new(x1, h, z1),
                Vector2::new(t_side_next, 1.0),
                Vector3::new(x1, 0.0, z1),
            ));
        }

        let mut data = Self::from_raw_mesh(builder.build(), StaticVertex::layout(), true);
        data.calculate_tangents().unwrap();
        data.transform_geometry(transform).unwrap();
        data
    }

    /// Creates unit cube with given transform.
    pub fn make_cube(transform: Matrix4<f32>) -> Self {
        let vertices = vec![
            // Front
            StaticVertex {
                position: Vector3::new(-0.5, -0.5, 0.5),
                normal: Vector3::z(),
                tex_coord: Vector2::default(),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(-0.5, 0.5, 0.5),
                normal: Vector3::z(),
                tex_coord: Vector2::y(),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(0.5, 0.5, 0.5),
                normal: Vector3::z(),
                tex_coord: Vector2::new(1.0, 1.0),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(0.5, -0.5, 0.5),
                normal: Vector3::z(),
                tex_coord: Vector2::x(),
                tangent: Vector4::default(),
            },
            // Back
            StaticVertex {
                position: Vector3::new(-0.5, -0.5, -0.5),
                normal: -Vector3::z(),
                tex_coord: Vector2::default(),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(-0.5, 0.5, -0.5),
                normal: -Vector3::z(),
                tex_coord: Vector2::y(),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(0.5, 0.5, -0.5),
                normal: -Vector3::z(),
                tex_coord: Vector2::new(1.0, 1.0),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(0.5, -0.5, -0.5),
                normal: -Vector3::z(),
                tex_coord: Vector2::x(),
                tangent: Vector4::default(),
            },
            // Left
            StaticVertex {
                position: Vector3::new(-0.5, -0.5, -0.5),
                normal: -Vector3::x(),
                tex_coord: Vector2::default(),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(-0.5, 0.5, -0.5),
                normal: -Vector3::x(),
                tex_coord: Vector2::y(),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(-0.5, 0.5, 0.5),
                normal: -Vector3::x(),
                tex_coord: Vector2::new(1.0, 1.0),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(-0.5, -0.5, 0.5),
                normal: -Vector3::x(),
                tex_coord: Vector2::x(),
                tangent: Vector4::default(),
            },
            // Right
            StaticVertex {
                position: Vector3::new(0.5, -0.5, -0.5),
                normal: Vector3::x(),
                tex_coord: Vector2::default(),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(0.5, 0.5, -0.5),
                normal: Vector3::x(),
                tex_coord: Vector2::y(),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(0.5, 0.5, 0.5),
                normal: Vector3::x(),
                tex_coord: Vector2::new(1.0, 1.0),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(0.5, -0.5, 0.5),
                normal: Vector3::x(),
                tex_coord: Vector2::x(),
                tangent: Vector4::default(),
            },
            // Top
            StaticVertex {
                position: Vector3::new(-0.5, 0.5, 0.5),
                normal: Vector3::y(),
                tex_coord: Vector2::default(),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(-0.5, 0.5, -0.5),
                normal: Vector3::y(),
                tex_coord: Vector2::y(),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(0.5, 0.5, -0.5),
                normal: Vector3::y(),
                tex_coord: Vector2::new(1.0, 1.0),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(0.5, 0.5, 0.5),
                normal: Vector3::y(),
                tex_coord: Vector2::x(),
                tangent: Vector4::default(),
            },
            // Bottom
            StaticVertex {
                position: Vector3::new(-0.5, -0.5, 0.5),
                normal: -Vector3::y(),
                tex_coord: Vector2::default(),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(-0.5, -0.5, -0.5),
                normal: -Vector3::y(),
                tex_coord: Vector2::y(),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(0.5, -0.5, -0.5),
                normal: -Vector3::y(),
                tex_coord: Vector2::new(1.0, 1.0),
                tangent: Vector4::default(),
            },
            StaticVertex {
                position: Vector3::new(0.5, -0.5, 0.5),
                normal: -Vector3::y(),
                tex_coord: Vector2::x(),
                tangent: Vector4::default(),
            },
        ];

        let triangles = vec![
            TriangleDefinition([2, 1, 0]),
            TriangleDefinition([3, 2, 0]),
            TriangleDefinition([4, 5, 6]),
            TriangleDefinition([4, 6, 7]),
            TriangleDefinition([10, 9, 8]),
            TriangleDefinition([11, 10, 8]),
            TriangleDefinition([12, 13, 14]),
            TriangleDefinition([12, 14, 15]),
            TriangleDefinition([18, 17, 16]),
            TriangleDefinition([19, 18, 16]),
            TriangleDefinition([20, 21, 22]),
            TriangleDefinition([20, 22, 23]),
        ];

        let mut data = Self::new(
            VertexBuffer::new(vertices.len(), StaticVertex::layout(), vertices).unwrap(),
            TriangleBuffer::new(triangles),
            true,
        );
        data.calculate_tangents().unwrap();
        data.transform_geometry(&transform).unwrap();
        data
    }

    /// Calculates hash based on contents of surface shared data.
    pub fn content_hash(&self) -> u64 {
        hash_combine(
            self.geometry_buffer.data_hash(),
            self.vertex_buffer.data_hash(),
        )
    }

    /// Clears both vertex and index buffers.
    pub fn clear(&mut self) {
        self.geometry_buffer.modify().clear();
        self.vertex_buffer.modify().clear();
    }
}

impl Visit for SurfaceData {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.is_procedural.visit("IsProcedural", &mut region)?;

        if self.is_procedural {
            self.vertex_buffer.visit("VertexBuffer", &mut region)?;
            self.geometry_buffer.visit("GeometryBuffer", &mut region)?
        }

        Ok(())
    }
}

/// Vertex weight is a pair of (bone; weight) that affects vertex.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct VertexWeight {
    /// Exact weight value in [0; 1] range
    pub value: f32,
    /// Handle to an entity that affects this vertex. It has double meaning
    /// relative to context:
    /// 1. When converting fbx model to engine node it points to FbxModel
    ///    that control this vertex via sub deformer.
    /// 2. After conversion is done, on resolve stage it points to a Node
    ///    in a scene to which converter put all the nodes.
    pub effector: ErasedHandle,
}

impl Default for VertexWeight {
    fn default() -> Self {
        Self {
            value: 0.0,
            effector: ErasedHandle::none(),
        }
    }
}

/// Weight set contains up to four pairs of (bone; weight).
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct VertexWeightSet {
    weights: [VertexWeight; 4],
    count: usize,
}

impl VertexWeightSet {
    /// Pushes new weight in the set and returns true if vertex was pushed,
    /// false - otherwise.
    pub fn push(&mut self, weight: VertexWeight) -> bool {
        if self.count < self.weights.len() {
            self.weights[self.count] = weight;
            self.count += 1;
            true
        } else {
            false
        }
    }

    /// Returns exact amount of weights in the set.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns true if set is empty.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Returns shared iterator.
    pub fn iter(&self) -> std::slice::Iter<VertexWeight> {
        self.weights[0..self.count].iter()
    }

    /// Returns mutable iterator.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<VertexWeight> {
        self.weights[0..self.count].iter_mut()
    }

    /// Normalizes weights in the set so they form unit 4-d vector. This method is useful
    /// when mesh has more than 4 weights per vertex. Engine supports only 4 weights per
    /// vertex so when there are more than 4 weights, first four weights may not give sum
    /// equal to 1.0, we must fix that to prevent weirdly looking results.
    pub fn normalize(&mut self) {
        let len = self.iter().fold(0.0, |qs, w| qs + w.value * w.value).sqrt();
        if len >= f32::EPSILON {
            let k = 1.0 / len;
            for w in self.iter_mut() {
                w.value *= k;
            }
        }
    }
}

/// See module docs.
#[derive(Debug, Clone, Inspect, Reflect, Visit)]
pub struct Surface {
    // Wrapped into option to be able to implement Default for serialization.
    // In normal conditions it must never be None!
    #[reflect(hidden)]
    #[inspect(skip)]
    data: Option<Arc<Mutex<SurfaceData>>>,
    #[reflect(hidden)]
    material: Arc<Mutex<Material>>,
    /// Temporal array for FBX conversion needs, it holds skinning data (weight + bone handle)
    /// and will be used to fill actual bone indices and weight in vertices that will be
    /// sent to GPU. The idea is very simple: GPU needs to know only indices of matrices of
    /// bones so we can use `bones` array as reference to get those indices. This could be done
    /// like so: iterate over all vertices and weight data and calculate index of node handle that
    /// associated with vertex in `bones` array and store it as bone index in vertex.
    #[inspect(skip)]
    #[visit(skip)]
    #[reflect(hidden)]
    pub vertex_weights: Vec<VertexWeightSet>,
    /// Array of handle to scene nodes which are used as bones.
    pub bones: Vec<Handle<Node>>,
}

impl PartialEq for Surface {
    fn eq(&self, other: &Self) -> bool {
        let data_equal = match (&self.data, &other.data) {
            (Some(data), Some(other_data)) => Arc::ptr_eq(data, other_data),
            (None, None) => true,
            _ => false,
        };

        let material_equal = Arc::ptr_eq(&self.material, &other.material);

        self.bones == other.bones
            && self.vertex_weights == other.vertex_weights
            && data_equal
            && material_equal
    }
}

impl Default for Surface {
    fn default() -> Self {
        Self {
            data: None,
            material: Arc::new(Mutex::new(Material::standard())),
            vertex_weights: Default::default(),
            bones: Default::default(),
        }
    }
}

impl Surface {
    /// Creates new surface instance with given data and without any texture.
    #[inline]
    pub fn new(data: Arc<Mutex<SurfaceData>>) -> Self {
        Self {
            data: Some(data),
            ..Default::default()
        }
    }

    /// Calculates material id.
    pub fn material_id(&self) -> u64 {
        &*self.material as *const _ as u64
    }

    /// Calculates batch id.
    pub fn batch_id(&self) -> u64 {
        let mut hasher = FxHasher::default();
        hasher.write_u64(self.material_id());
        hasher.write_u64(&**self.data.as_ref().unwrap() as *const _ as u64);
        hasher.finish()
    }

    /// Returns current data used by surface.
    #[inline]
    pub fn data(&self) -> Arc<Mutex<SurfaceData>> {
        self.data.as_ref().unwrap().clone()
    }

    /// Returns current material of the surface.
    pub fn material(&self) -> &Arc<Mutex<Material>> {
        &self.material
    }

    /// Sets new material for the surface.
    pub fn set_material(&mut self, material: Arc<Mutex<Material>>) {
        self.material = material;
    }

    /// Returns list of bones that affects the surface.
    #[inline]
    pub fn bones(&self) -> &[Handle<Node>] {
        &self.bones
    }
}

/// Surface builder allows you to create surfaces in declarative manner.
pub struct SurfaceBuilder {
    data: Arc<Mutex<SurfaceData>>,
    material: Option<Arc<Mutex<Material>>>,
    bones: Vec<Handle<Node>>,
}

impl SurfaceBuilder {
    /// Creates new builder instance with given data and no textures or bones.
    pub fn new(data: Arc<Mutex<SurfaceData>>) -> Self {
        Self {
            data,
            material: None,
            bones: Default::default(),
        }
    }

    /// Sets desired diffuse texture.
    pub fn with_material(mut self, material: Arc<Mutex<Material>>) -> Self {
        self.material = Some(material);
        self
    }

    /// Sets desired bones array. Make sure your vertices has valid indices of bones!
    pub fn with_bones(mut self, bones: Vec<Handle<Node>>) -> Self {
        self.bones = bones;
        self
    }

    /// Creates new instance of surface.
    pub fn build(self) -> Surface {
        Surface {
            data: Some(self.data),
            material: self
                .material
                .unwrap_or_else(|| Arc::new(Mutex::new(Material::standard()))),
            vertex_weights: Default::default(),
            bones: self.bones,
        }
    }
}
