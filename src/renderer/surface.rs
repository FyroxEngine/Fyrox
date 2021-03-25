//! For efficient rendering each mesh is split into sets of triangles that use the same texture,
//! such sets are called surfaces.
//!
//! Surfaces can use the same data source across many instances, this is a memory optimization for
//! being able to re-use data when you need to draw the same mesh in many places.

use crate::{
    core::{
        algebra::{Matrix4, Point3, Vector2, Vector3, Vector4},
        color::Color,
        math::TriangleDefinition,
        pool::{ErasedHandle, Handle},
        visitor::{Visit, VisitResult, Visitor},
    },
    resource::texture::Texture,
    scene::node::Node,
    utils::raw_mesh::{RawMesh, RawMeshBuilder},
};
use std::collections::hash_map::DefaultHasher;
use std::sync::RwLock;
use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};

/// Vertex for each mesh in engine.
///
/// # Possible optimizations
///
/// This vertex format maybe too big for some cases thus impacting performance.
/// The ability to make your own vertices is nice to have but this is still a
/// TODO.
#[derive(Copy, Clone, Debug, Default)]
#[repr(C)] // OpenGL expects this structure packed as in C
pub struct Vertex {
    /// Position of vertex in local coordinates.
    pub position: Vector3<f32>,
    /// Texture coordinates.
    pub tex_coord: Vector2<f32>,
    /// Second texture coordinates. Usually used for lightmapping.
    pub second_tex_coord: Vector2<f32>,
    /// Normal in local coordinates.
    pub normal: Vector3<f32>,
    /// Tangent vector in local coordinates.
    pub tangent: Vector4<f32>,
    /// Array of bone weights. Unused bones will have 0.0 weight so they won't
    /// impact the shape of mesh.
    pub bone_weights: [f32; 4],
    /// Array of bone indices. It has indices of bones in array of bones of a
    /// surface.
    pub bone_indices: [u8; 4],
}

impl Visit for Vertex {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.position.visit("Position", visitor)?;
        self.tex_coord.visit("TexCoord", visitor)?;
        let _ = self.second_tex_coord.visit("SecondTexCoord", visitor);
        self.normal.visit("Normal", visitor)?;
        self.tangent.visit("Tangent", visitor)?;

        self.bone_weights[0].visit("Weight0", visitor)?;
        self.bone_weights[1].visit("Weight1", visitor)?;
        self.bone_weights[2].visit("Weight2", visitor)?;
        self.bone_weights[3].visit("Weight3", visitor)?;

        self.bone_indices[0].visit("BoneIndex0", visitor)?;
        self.bone_indices[1].visit("BoneIndex1", visitor)?;
        self.bone_indices[2].visit("BoneIndex2", visitor)?;
        self.bone_indices[3].visit("BoneIndex3", visitor)?;

        visitor.leave_region()
    }
}

impl Vertex {
    /// Creates new vertex from given position and texture coordinates.
    pub fn from_pos_uv(position: Vector3<f32>, tex_coord: Vector2<f32>) -> Self {
        Self {
            position,
            tex_coord,
            second_tex_coord: Default::default(),
            normal: Vector3::new(0.0, 1.0, 0.0),
            tangent: Vector4::default(),
            bone_weights: [0.0; 4],
            bone_indices: Default::default(),
        }
    }
}

impl PartialEq for Vertex {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
            && self.tex_coord == other.tex_coord
            && self.normal == other.normal
            && self.tangent == other.tangent
            && self.bone_weights == other.bone_weights
            && self.bone_indices == other.bone_indices
    }
}

// This is safe because Vertex is tightly packed struct with C representation
// there is no padding bytes which may contain garbage data. This is strictly
// required because vertices will be directly passed on GPU.
impl Hash for Vertex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        #[allow(unsafe_code)]
        unsafe {
            let bytes = self as *const Self as *const u8;
            state.write(std::slice::from_raw_parts(
                bytes,
                std::mem::size_of::<Self>(),
            ))
        }
    }
}

/// Data source of a surface. Each surface can share same data source, this is used
/// in instancing technique to render multiple instances of same model at different
/// places.
#[derive(Debug)]
pub struct SurfaceSharedData {
    pub(in crate) vertices: Vec<Vertex>,
    pub(in crate) triangles: Vec<TriangleDefinition>,
    // If true - indicates that surface was generated and does not have reference
    // resource. Procedural data will be serialized.
    is_procedural: bool,
}

impl Default for SurfaceSharedData {
    fn default() -> Self {
        Self {
            vertices: Default::default(),
            triangles: Default::default(),
            is_procedural: false,
        }
    }
}

impl SurfaceSharedData {
    /// Creates new data source using given vertices and indices.
    pub fn new(
        vertices: Vec<Vertex>,
        triangles: Vec<TriangleDefinition>,
        is_procedural: bool,
    ) -> Self {
        Self {
            vertices,
            triangles,
            is_procedural,
        }
    }

    /// Converts raw mesh into "renderable" mesh. It is useful to build procedural
    /// meshes.
    pub fn from_raw_mesh(raw: RawMesh<Vertex>, is_procedural: bool) -> Self {
        Self {
            vertices: raw.vertices,
            triangles: raw.triangles,
            is_procedural,
        }
    }

    /// Returns shared reference to vertices array.
    #[inline]
    pub fn get_vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    #[inline]
    pub(in crate) fn get_vertices_mut(&mut self) -> &mut [Vertex] {
        &mut self.vertices
    }

    /// Return shared reference to triangles array.
    #[inline]
    pub fn triangles(&self) -> &[TriangleDefinition] {
        self.triangles.as_slice()
    }

    /// Calculates tangents of surface. Tangents are needed for correct lighting, you will
    /// get incorrect lighting if tangents of your surface are invalid! When engine loads
    /// a mesh from "untrusted" source, it automatically calculates tangents for you, so
    /// there is no need to call this manually in this case. However if you making your
    /// mesh procedurally, you have to use this method!
    pub fn calculate_tangents(&mut self) {
        let mut tan1 = vec![Vector3::default(); self.vertices.len()];
        let mut tan2 = vec![Vector3::default(); self.vertices.len()];

        for triangle in self.triangles.iter() {
            let i1 = triangle[0] as usize;
            let i2 = triangle[1] as usize;
            let i3 = triangle[2] as usize;

            let v1 = &self.vertices[i1].position;
            let v2 = &self.vertices[i2].position;
            let v3 = &self.vertices[i3].position;

            let w1 = &self.vertices[i1].tex_coord;
            let w2 = &self.vertices[i2].tex_coord;
            let w3 = &self.vertices[i3].tex_coord;

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

        for (v, (t1, t2)) in self.vertices.iter_mut().zip(tan1.into_iter().zip(tan2)) {
            // Gram-Schmidt orthogonalize
            let tangent = (t1 - v.normal.scale(v.normal.dot(&t1)))
                .try_normalize(std::f32::EPSILON)
                .unwrap_or_else(|| Vector3::new(0.0, 1.0, 0.0));
            let handedness = v.normal.cross(&t1).dot(&t2).signum();
            v.tangent = Vector4::new(tangent.x, tangent.y, tangent.z, handedness);
        }
    }

    /// Creates a quad oriented on oXY plane with unit width and height.
    pub fn make_unit_xy_quad() -> Self {
        let vertices = vec![
            Vertex {
                position: Vector3::default(),
                normal: Vector3::z(),
                tex_coord: Vector2::y(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::x(),
                normal: Vector3::z(),
                tex_coord: Vector2::new(1.0, 1.0),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(1.0, 1.0, 0.0),
                normal: Vector3::z(),
                tex_coord: Vector2::x(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::y(),
                normal: Vector3::z(),
                tex_coord: Vector2::default(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
        ];

        let indices = vec![TriangleDefinition([0, 1, 2]), TriangleDefinition([0, 2, 3])];

        Self::new(vertices, indices, true)
    }

    /// Creates a degenerated quad which collapsed in a point. This is very special method for
    /// sprite renderer - shader will automatically "push" corners in correct sides so sprite
    /// will always face camera.
    pub fn make_collapsed_xy_quad() -> Self {
        let vertices = vec![
            Vertex {
                position: Vector3::default(),
                normal: Vector3::z(),
                tex_coord: Vector2::default(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::default(),
                normal: Vector3::z(),
                tex_coord: Vector2::x(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::default(),
                normal: Vector3::z(),
                tex_coord: Vector2::new(1.0, 1.0),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::default(),
                normal: Vector3::z(),
                tex_coord: Vector2::y(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
        ];

        let indices = vec![TriangleDefinition([0, 1, 2]), TriangleDefinition([0, 2, 3])];

        Self::new(vertices, indices, true)
    }

    /// Creates new quad at oXY plane with given transform.
    pub fn make_quad(transform: Matrix4<f32>) -> Self {
        let mut vertices = vec![
            Vertex {
                position: Vector3::new(-0.5, 0.5, 0.0),
                normal: Vector3::z(),
                tex_coord: Vector2::new(1.0, 1.0),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(0.5, 0.5, 0.0),
                normal: Vector3::z(),
                tex_coord: Vector2::new(0.0, 1.0),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(0.5, -0.5, 0.0),
                normal: Vector3::z(),
                tex_coord: Vector2::new(0.0, 0.0),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(-0.5, -0.5, 0.0),
                normal: Vector3::z(),
                tex_coord: Vector2::new(1.0, 0.0),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
        ];

        for v in vertices.iter_mut() {
            v.position = transform.transform_point(&Point3::from(v.position)).coords;
        }

        let indices = vec![TriangleDefinition([0, 1, 2]), TriangleDefinition([0, 2, 3])];

        let mut data = Self::new(vertices, indices, true);

        data.calculate_normals();
        data.calculate_tangents();

        data
    }

    /// Calculates per-face normals. This method is fast, but have very poor quality, and surface
    /// will look facet.
    pub fn calculate_normals(&mut self) {
        for triangle in self.triangles.iter() {
            let ia = triangle[0] as usize;
            let ib = triangle[1] as usize;
            let ic = triangle[2] as usize;

            let a = self.vertices[ia].position;
            let b = self.vertices[ib].position;
            let c = self.vertices[ic].position;

            let normal = (b - a).cross(&(c - a)).normalize();

            self.vertices[ia].normal = normal;
            self.vertices[ib].normal = normal;
            self.vertices[ic].normal = normal;
        }
    }

    /// Creates sphere of specified radius with given slices and stacks.
    pub fn make_sphere(slices: usize, stacks: usize, r: f32) -> Self {
        let mut builder = RawMeshBuilder::<Vertex>::new(stacks * slices, stacks * slices * 3);

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
                    builder.insert(Vertex::from_pos_uv(
                        Vector3::new(k0 * k1, k0 * k2, k3),
                        Vector2::new(d_tc_x * j as f32, d_tc_y * i as f32),
                    ));
                    builder.insert(Vertex::from_pos_uv(
                        Vector3::new(k4 * k1, k4 * k2, k7),
                        Vector2::new(d_tc_x * j as f32, d_tc_y * ni as f32),
                    ));
                    builder.insert(Vertex::from_pos_uv(
                        Vector3::new(k4 * k5, k4 * k6, k7),
                        Vector2::new(d_tc_x * nj as f32, d_tc_y * ni as f32),
                    ));
                }

                if i != 0 {
                    builder.insert(Vertex::from_pos_uv(
                        Vector3::new(k4 * k5, k4 * k6, k7),
                        Vector2::new(d_tc_x * nj as f32, d_tc_y * ni as f32),
                    ));
                    builder.insert(Vertex::from_pos_uv(
                        Vector3::new(k0 * k5, k0 * k6, k3),
                        Vector2::new(d_tc_x * nj as f32, d_tc_y * i as f32),
                    ));
                    builder.insert(Vertex::from_pos_uv(
                        Vector3::new(k0 * k1, k0 * k2, k3),
                        Vector2::new(d_tc_x * j as f32, d_tc_y * i as f32),
                    ));
                }
            }
        }

        let mut data = Self::from_raw_mesh(builder.build(), true);
        data.calculate_normals();
        data.calculate_tangents();
        data
    }

    /// Creates vertical cone - it has its vertex higher than base.
    pub fn make_cone(sides: usize, r: f32, h: f32, transform: Matrix4<f32>) -> Self {
        let mut builder = RawMeshBuilder::<Vertex>::new(3 * sides, 3 * sides);

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
            builder.insert(Vertex::from_pos_uv(
                transform
                    .transform_point(&Point3::new(0.0, 0.0, 0.0))
                    .coords,
                Vector2::default(),
            ));
            builder.insert(Vertex::from_pos_uv(
                transform.transform_point(&Point3::new(x0, 0.0, z0)).coords,
                Vector2::new(tx0, 1.0),
            ));
            builder.insert(Vertex::from_pos_uv(
                transform.transform_point(&Point3::new(x1, 0.0, z1)).coords,
                Vector2::new(tx1, 0.0),
            ));

            // sides
            builder.insert(Vertex::from_pos_uv(
                transform.transform_point(&Point3::new(0.0, h, 0.0)).coords,
                Vector2::new(tx1, 0.0),
            ));
            builder.insert(Vertex::from_pos_uv(
                transform.transform_point(&Point3::new(x1, 0.0, z1)).coords,
                Vector2::new(tx0, 1.0),
            ));
            builder.insert(Vertex::from_pos_uv(
                transform.transform_point(&Point3::new(x0, 0.0, z0)).coords,
                Vector2::new(tx0, 0.0),
            ));
        }

        let mut data = Self::from_raw_mesh(builder.build(), true);
        data.calculate_normals();
        data.calculate_tangents();
        data
    }

    /// Creates vertical cylinder.
    pub fn make_cylinder(
        sides: usize,
        r: f32,
        h: f32,
        caps: bool,
        transform: Matrix4<f32>,
    ) -> Self {
        let mut builder = RawMeshBuilder::<Vertex>::new(3 * sides, 3 * sides);

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

            if caps {
                // front cap
                builder.insert(Vertex::from_pos_uv(
                    transform.transform_point(&Point3::new(x1, h, z1)).coords,
                    Vector2::new(tx1, 1.0),
                ));
                builder.insert(Vertex::from_pos_uv(
                    transform.transform_point(&Point3::new(x0, h, z0)).coords,
                    Vector2::new(tx0, 1.0),
                ));
                builder.insert(Vertex::from_pos_uv(
                    transform.transform_point(&Point3::new(0.0, h, 0.0)).coords,
                    Vector2::default(),
                ));

                // back cap
                builder.insert(Vertex::from_pos_uv(
                    transform.transform_point(&Point3::new(x0, 0.0, z0)).coords,
                    Vector2::new(tx1, 1.0),
                ));
                builder.insert(Vertex::from_pos_uv(
                    transform.transform_point(&Point3::new(x1, 0.0, z1)).coords,
                    Vector2::new(tx0, 1.0),
                ));
                builder.insert(Vertex::from_pos_uv(
                    transform
                        .transform_point(&Point3::new(0.0, 0.0, 0.0))
                        .coords,
                    Vector2::default(),
                ));
            }

            // sides
            builder.insert(Vertex::from_pos_uv(
                transform.transform_point(&Point3::new(x0, 0.0, z0)).coords,
                Vector2::new(tx0, 0.0),
            ));
            builder.insert(Vertex::from_pos_uv(
                transform.transform_point(&Point3::new(x0, h, z0)).coords,
                Vector2::new(tx0, 1.0),
            ));
            builder.insert(Vertex::from_pos_uv(
                transform.transform_point(&Point3::new(x1, 0.0, z1)).coords,
                Vector2::new(tx1, 0.0),
            ));

            builder.insert(Vertex::from_pos_uv(
                transform.transform_point(&Point3::new(x1, 0.0, z1)).coords,
                Vector2::new(tx1, 0.0),
            ));
            builder.insert(Vertex::from_pos_uv(
                transform.transform_point(&Point3::new(x0, h, z0)).coords,
                Vector2::new(tx0, 1.0),
            ));
            builder.insert(Vertex::from_pos_uv(
                transform.transform_point(&Point3::new(x1, h, z1)).coords,
                Vector2::new(tx1, 1.0),
            ));
        }

        let mut data = Self::from_raw_mesh(builder.build(), true);
        data.calculate_normals();
        data.calculate_tangents();
        data
    }

    /// Creates unit cube with given transform.
    pub fn make_cube(transform: Matrix4<f32>) -> Self {
        let mut vertices = vec![
            // Front
            Vertex {
                position: Vector3::new(-0.5, -0.5, 0.5),
                normal: Vector3::z(),
                tex_coord: Vector2::default(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(-0.5, 0.5, 0.5),
                normal: Vector3::z(),
                tex_coord: Vector2::y(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(0.5, 0.5, 0.5),
                normal: Vector3::z(),
                tex_coord: Vector2::new(1.0, 1.0),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(0.5, -0.5, 0.5),
                normal: Vector3::z(),
                tex_coord: Vector2::x(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            // Back
            Vertex {
                position: Vector3::new(-0.5, -0.5, -0.5),
                normal: -Vector3::z(),
                tex_coord: Vector2::default(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(-0.5, 0.5, -0.5),
                normal: -Vector3::z(),
                tex_coord: Vector2::y(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(0.5, 0.5, -0.5),
                normal: -Vector3::z(),
                tex_coord: Vector2::new(1.0, 1.0),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(0.5, -0.5, -0.5),
                normal: -Vector3::z(),
                tex_coord: Vector2::x(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            // Left
            Vertex {
                position: Vector3::new(-0.5, -0.5, -0.5),
                normal: -Vector3::x(),
                tex_coord: Vector2::default(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(-0.5, 0.5, -0.5),
                normal: -Vector3::x(),
                tex_coord: Vector2::y(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(-0.5, 0.5, 0.5),
                normal: -Vector3::x(),
                tex_coord: Vector2::new(1.0, 1.0),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(-0.5, -0.5, 0.5),
                normal: -Vector3::x(),
                tex_coord: Vector2::x(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            // Right
            Vertex {
                position: Vector3::new(0.5, -0.5, -0.5),
                normal: Vector3::x(),
                tex_coord: Vector2::default(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(0.5, 0.5, -0.5),
                normal: Vector3::x(),
                tex_coord: Vector2::y(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(0.5, 0.5, 0.5),
                normal: Vector3::x(),
                tex_coord: Vector2::new(1.0, 1.0),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(0.5, -0.5, 0.5),
                normal: Vector3::x(),
                tex_coord: Vector2::x(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            // Top
            Vertex {
                position: Vector3::new(-0.5, 0.5, 0.5),
                normal: Vector3::y(),
                tex_coord: Vector2::default(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(-0.5, 0.5, -0.5),
                normal: Vector3::y(),
                tex_coord: Vector2::y(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(0.5, 0.5, -0.5),
                normal: Vector3::y(),
                tex_coord: Vector2::new(1.0, 1.0),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(0.5, 0.5, 0.5),
                normal: Vector3::y(),
                tex_coord: Vector2::x(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            // Bottom
            Vertex {
                position: Vector3::new(-0.5, -0.5, 0.5),
                normal: -Vector3::y(),
                tex_coord: Vector2::default(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(-0.5, -0.5, -0.5),
                normal: -Vector3::y(),
                tex_coord: Vector2::y(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(0.5, -0.5, -0.5),
                normal: -Vector3::y(),
                tex_coord: Vector2::new(1.0, 1.0),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
            Vertex {
                position: Vector3::new(0.5, -0.5, 0.5),
                normal: -Vector3::y(),
                tex_coord: Vector2::x(),
                tangent: Vector4::default(),
                bone_weights: [0.0; 4],
                bone_indices: [0; 4],
                second_tex_coord: Default::default(),
            },
        ];

        for v in vertices.iter_mut() {
            v.position = transform.transform_point(&Point3::from(v.position)).coords;
        }

        let indices = vec![
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

        let mut data = Self::new(vertices, indices, true);
        data.calculate_tangents();
        data
    }

    /// Calculates unique id based on contents of surface shared data.
    pub fn id(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        unsafe {
            let triangles_bytes = std::slice::from_raw_parts(
                self.triangles.as_ptr() as *const u8,
                self.triangles.len() * std::mem::size_of::<TriangleDefinition>(),
            );
            triangles_bytes.hash(&mut hasher);

            let vertices_bytes = std::slice::from_raw_parts(
                self.vertices.as_ptr() as *const u8,
                self.vertices.len() * std::mem::size_of::<Vertex>(),
            );
            vertices_bytes.hash(&mut hasher);
        }
        hasher.finish()
    }
}

impl Visit for SurfaceSharedData {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        if visitor.is_reading() || (self.is_procedural && !visitor.is_reading()) {
            self.vertices.visit("Vertices", visitor)?;
            self.triangles.visit("Triangles", visitor)?;
        } else {
            let mut dummy = Vec::<Vertex>::new();
            dummy.visit("Vertices", visitor)?;
            let mut dummy = Vec::<TriangleDefinition>::new();
            dummy.visit("Triangles", visitor)?;
        }

        self.is_procedural.visit("IsProcedural", visitor)?;

        visitor.leave_region()
    }
}

/// Vertex weight is a pair of (bone; weight) that affects vertex.
#[derive(Copy, Clone, Debug)]
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
#[derive(Copy, Clone, Debug)]
pub struct VertexWeightSet {
    weights: [VertexWeight; 4],
    count: usize,
}

impl Default for VertexWeightSet {
    fn default() -> Self {
        Self {
            weights: Default::default(),
            count: 0,
        }
    }
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
        if len >= std::f32::EPSILON {
            let k = 1.0 / len;
            for w in self.iter_mut() {
                w.value *= k;
            }
        }
    }
}

/// See module docs.
#[derive(Debug, Default)]
pub struct Surface {
    // Wrapped into option to be able to implement Default for serialization.
    // In normal conditions it must never be None!
    data: Option<Arc<RwLock<SurfaceSharedData>>>,
    diffuse_texture: Option<Texture>,
    normal_texture: Option<Texture>,
    lightmap_texture: Option<Texture>,
    specular_texture: Option<Texture>,
    roughness_texture: Option<Texture>,
    /// Temporal array for FBX conversion needs, it holds skinning data (weight + bone handle)
    /// and will be used to fill actual bone indices and weight in vertices that will be
    /// sent to GPU. The idea is very simple: GPU needs to know only indices of matrices of
    /// bones so we can use `bones` array as reference to get those indices. This could be done
    /// like so: iterate over all vertices and weight data and calculate index of node handle that
    /// associated with vertex in `bones` array and store it as bone index in vertex.
    pub vertex_weights: Vec<VertexWeightSet>,
    /// Array of handle to scene nodes which are used as bones.
    pub bones: Vec<Handle<Node>>,
    color: Color,
}

/// Shallow copy of surface.
///
/// # Notes
///
/// Handles to bones must be remapped afterwards, so it is not advised
/// to use this clone to clone surfaces.
impl Clone for Surface {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            diffuse_texture: self.diffuse_texture.clone(),
            normal_texture: self.normal_texture.clone(),
            specular_texture: self.specular_texture.clone(),
            roughness_texture: self.roughness_texture.clone(),
            bones: self.bones.clone(),
            vertex_weights: Vec::new(), // Intentionally not copied.
            color: self.color,
            lightmap_texture: self.lightmap_texture.clone(),
        }
    }
}

impl Surface {
    /// Creates new surface instance with given data and without any texture.
    #[inline]
    pub fn new(data: Arc<RwLock<SurfaceSharedData>>) -> Self {
        Self {
            data: Some(data),
            diffuse_texture: None,
            normal_texture: None,
            specular_texture: None,
            roughness_texture: None,
            bones: Vec::new(),
            vertex_weights: Vec::new(),
            color: Color::WHITE,
            lightmap_texture: None,
        }
    }

    /// Calculates batch id.
    pub fn batch_id(&self) -> u64 {
        let mut hasher = DefaultHasher::new();

        let data_key = &*self.data() as *const _ as u64;
        data_key.hash(&mut hasher);

        if let Some(diffuse_texture) = self.diffuse_texture.as_ref() {
            diffuse_texture.key().hash(&mut hasher);
        }
        if let Some(normal_texture) = self.normal_texture.as_ref() {
            normal_texture.key().hash(&mut hasher);
        }
        if let Some(specular_texture) = self.specular_texture.as_ref() {
            specular_texture.key().hash(&mut hasher);
        }
        if let Some(roughness_texture) = self.roughness_texture.as_ref() {
            roughness_texture.key().hash(&mut hasher);
        }
        if let Some(lightmap_texture) = self.lightmap_texture.as_ref() {
            lightmap_texture.key().hash(&mut hasher);
        }

        hasher.finish()
    }

    /// Returns current data used by surface.
    #[inline]
    pub fn data(&self) -> Arc<RwLock<SurfaceSharedData>> {
        self.data.as_ref().unwrap().clone()
    }

    /// Sets new diffuse texture.
    #[inline]
    pub fn set_diffuse_texture(&mut self, tex: Option<Texture>) {
        self.diffuse_texture = tex;
    }

    /// Returns current diffuse texture.
    #[inline]
    pub fn diffuse_texture(&self) -> Option<Texture> {
        self.diffuse_texture.clone()
    }

    /// Sets new normal map texture.
    #[inline]
    pub fn set_normal_texture(&mut self, tex: Option<Texture>) {
        self.normal_texture = tex;
    }

    /// Returns current normal map texture.
    #[inline]
    pub fn normal_texture(&self) -> Option<Texture> {
        self.normal_texture.clone()
    }

    /// Sets new specular texture.
    #[inline]
    pub fn set_specular_texture(&mut self, tex: Option<Texture>) {
        self.specular_texture = tex;
    }

    /// Returns current specular texture.
    #[inline]
    pub fn specular_texture(&self) -> Option<Texture> {
        self.specular_texture.clone()
    }

    /// Sets new roughness texture.
    #[inline]
    pub fn set_roughness_texture(&mut self, tex: Option<Texture>) {
        self.roughness_texture = tex;
    }

    /// Returns current roughness texture.
    #[inline]
    pub fn roughness_texture(&self) -> Option<Texture> {
        self.roughness_texture.clone()
    }

    /// Sets new lightmap texture.
    #[inline]
    pub fn set_lightmap_texture(&mut self, tex: Option<Texture>) {
        self.lightmap_texture = tex;
    }

    /// Returns lightmap texture.
    #[inline]
    pub fn lightmap_texture(&self) -> Option<Texture> {
        self.lightmap_texture.clone()
    }

    /// Sets color of surface. Keep in mind that alpha component is **not** compatible
    /// with deferred render path. You have to use forward render path if you need
    /// transparent surfaces.
    #[inline]
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    /// Returns current color of surface.
    #[inline]
    pub fn color(&self) -> Color {
        self.color
    }

    /// Returns list of bones that affects the surface.
    #[inline]
    pub fn bones(&self) -> &[Handle<Node>] {
        &self.bones
    }
}

impl Visit for Surface {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.data.visit("Data", visitor)?;
        self.normal_texture.visit("NormalTexture", visitor)?;
        self.diffuse_texture.visit("DiffuseTexture", visitor)?;
        let _ = self.specular_texture.visit("SpecularTexture", visitor);
        let _ = self.roughness_texture.visit("RoughnessTexture", visitor);
        self.color.visit("Color", visitor)?;
        self.bones.visit("Bones", visitor)?;
        // self.vertex_weights intentionally not serialized!

        // Try to get lightmap texture but don't care if it is missing, it can
        // be missing on previous versions.
        let _ = self.lightmap_texture.visit("LightmapTexture", visitor);

        visitor.leave_region()
    }
}

/// Surface builder allows you to create surfaces in declarative manner.
pub struct SurfaceBuilder {
    data: Arc<RwLock<SurfaceSharedData>>,
    diffuse_texture: Option<Texture>,
    normal_texture: Option<Texture>,
    lightmap_texture: Option<Texture>,
    specular_texture: Option<Texture>,
    roughness_texture: Option<Texture>,
    bones: Vec<Handle<Node>>,
    color: Color,
}

impl SurfaceBuilder {
    /// Creates new builder instance with given data and no textures or bones.
    pub fn new(data: Arc<RwLock<SurfaceSharedData>>) -> Self {
        Self {
            data,
            diffuse_texture: None,
            normal_texture: None,
            lightmap_texture: None,
            specular_texture: None,
            roughness_texture: None,
            bones: Default::default(),
            color: Color::WHITE,
        }
    }

    /// Sets desired diffuse texture.
    pub fn with_diffuse_texture(mut self, tex: Texture) -> Self {
        self.diffuse_texture = Some(tex);
        self
    }

    /// Sets desired normal map texture.
    pub fn with_normal_texture(mut self, tex: Texture) -> Self {
        self.normal_texture = Some(tex);
        self
    }

    /// Sets desired lightmap texture.
    pub fn with_lightmap_texture(mut self, tex: Texture) -> Self {
        self.lightmap_texture = Some(tex);
        self
    }

    /// Sets desired specular texture.
    pub fn with_specular_texture(mut self, tex: Texture) -> Self {
        self.specular_texture = Some(tex);
        self
    }

    /// Sets desired roughness texture.
    pub fn with_roughness_texture(mut self, tex: Texture) -> Self {
        self.roughness_texture = Some(tex);
        self
    }

    /// Sets desired color of surface.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
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
            diffuse_texture: self.diffuse_texture,
            normal_texture: self.normal_texture,
            lightmap_texture: self.lightmap_texture,
            specular_texture: self.specular_texture,
            roughness_texture: self.roughness_texture,
            vertex_weights: Default::default(),
            bones: self.bones,
            color: self.color,
        }
    }
}
