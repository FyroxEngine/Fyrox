use crate::{
    core::{
        color::Color,
        math::{mat4::Mat4, vec2::Vec2, vec3::Vec3, vec4::Vec4, TriangleDefinition},
        pool::{ErasedHandle, Handle},
        visitor::{Visit, VisitResult, Visitor},
    },
    resource::texture::Texture,
    scene::node::Node,
    utils::raw_mesh::{RawMesh, RawMeshBuilder},
};
use std::{
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)] // OpenGL expects this structure packed as in C
pub struct Vertex {
    pub position: Vec3,
    pub tex_coord: Vec2,
    pub normal: Vec3,
    pub tangent: Vec4,
    pub bone_weights: [f32; 4],
    pub bone_indices: [u8; 4],
}

impl Visit for Vertex {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.position.visit("Position", visitor)?;
        self.tex_coord.visit("TexCoord", visitor)?;
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
    pub fn from_pos_uv(position: Vec3, tex_coord: Vec2) -> Self {
        Self {
            position,
            tex_coord,
            normal: Vec3::new(0.0, 1.0, 0.0),
            tangent: Vec4 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 0.0,
            },
            bone_weights: [0.0, 0.0, 0.0, 0.0],
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

    pub fn from_raw_mesh(raw: RawMesh<Vertex>, is_procedural: bool) -> Self {
        Self {
            vertices: raw.vertices,
            triangles: raw.triangles,
            is_procedural,
        }
    }

    #[inline]
    pub fn get_vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    #[inline]
    pub fn get_vertices_mut(&mut self) -> &mut [Vertex] {
        &mut self.vertices
    }

    #[inline]
    pub fn triangles(&self) -> &[TriangleDefinition] {
        self.triangles.as_slice()
    }

    pub fn calculate_tangents(&mut self) {
        let mut tan1 = vec![Vec3::ZERO; self.vertices.len()];
        let mut tan2 = vec![Vec3::ZERO; self.vertices.len()];

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

            let sdir = Vec3::new(
                (t2 * x1 - t1 * x2) * r,
                (t2 * y1 - t1 * y2) * r,
                (t2 * z1 - t1 * z2) * r,
            );

            tan1[i1] += sdir;
            tan1[i2] += sdir;
            tan1[i3] += sdir;

            let tdir = Vec3::new(
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
                .normalized()
                .unwrap_or_else(|| Vec3::new(0.0, 1.0, 0.0));
            let handedness = v.normal.cross(&t1).dot(&t2).signum();
            v.tangent = Vec4::from_vec3(tangent, handedness);
        }
    }

    pub fn make_unit_xy_quad() -> Self {
        let vertices = vec![
            Vertex {
                position: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: 1.0,
                    y: 1.0,
                    z: 0.0,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
        ];

        let indices = vec![TriangleDefinition([0, 1, 2]), TriangleDefinition([0, 2, 3])];

        Self::new(vertices, indices, true)
    }

    pub fn make_collapsed_xy_quad() -> Self {
        let vertices = vec![
            Vertex {
                position: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
        ];

        let indices = vec![TriangleDefinition([0, 1, 2]), TriangleDefinition([0, 2, 3])];

        Self::new(vertices, indices, true)
    }

    pub fn make_quad(transform: Mat4) -> Self {
        let mut vertices = vec![
            Vertex {
                position: Vec3 {
                    x: -0.5,
                    y: 0.0,
                    z: 0.5,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: 0.5,
                    y: 0.0,
                    z: 0.5,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: 0.5,
                    y: 0.0,
                    z: -0.5,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: -0.5,
                    y: 0.0,
                    z: -0.5,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
        ];

        for v in vertices.iter_mut() {
            v.position = transform.transform_vector(v.position);
        }

        let indices = vec![TriangleDefinition([0, 1, 2]), TriangleDefinition([0, 2, 3])];

        let mut data = Self::new(vertices, indices, true);

        data.calculate_normals();
        data.calculate_tangents();

        data
    }

    pub fn calculate_normals(&mut self) {
        for triangle in self.triangles.iter() {
            let ia = triangle[0] as usize;
            let ib = triangle[1] as usize;
            let ic = triangle[2] as usize;

            let a = self.vertices[ia].position;
            let b = self.vertices[ib].position;
            let c = self.vertices[ic].position;

            let normal = (b - a).cross(&(c - a)).normalized().unwrap();

            self.vertices[ia].normal = normal;
            self.vertices[ib].normal = normal;
            self.vertices[ic].normal = normal;
        }
    }

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
                        Vec3::new(k0 * k1, k0 * k2, k3),
                        Vec2::new(d_tc_x * j as f32, d_tc_y * i as f32),
                    ));
                    builder.insert(Vertex::from_pos_uv(
                        Vec3::new(k4 * k1, k4 * k2, k7),
                        Vec2::new(d_tc_x * j as f32, d_tc_y * ni as f32),
                    ));
                    builder.insert(Vertex::from_pos_uv(
                        Vec3::new(k4 * k5, k4 * k6, k7),
                        Vec2::new(d_tc_x * nj as f32, d_tc_y * ni as f32),
                    ));
                }

                if i != 0 {
                    builder.insert(Vertex::from_pos_uv(
                        Vec3::new(k4 * k5, k4 * k6, k7),
                        Vec2::new(d_tc_x * nj as f32, d_tc_y * ni as f32),
                    ));
                    builder.insert(Vertex::from_pos_uv(
                        Vec3::new(k0 * k5, k0 * k6, k3),
                        Vec2::new(d_tc_x * nj as f32, d_tc_y * i as f32),
                    ));
                    builder.insert(Vertex::from_pos_uv(
                        Vec3::new(k0 * k1, k0 * k2, k3),
                        Vec2::new(d_tc_x * j as f32, d_tc_y * i as f32),
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
    pub fn make_cone(sides: usize, r: f32, h: f32, transform: Mat4) -> Self {
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
                transform.transform_vector(Vec3::new(0.0, 0.0, 0.0)),
                Vec2::new(0.0, 0.0),
            ));
            builder.insert(Vertex::from_pos_uv(
                transform.transform_vector(Vec3::new(x0, 0.0, z0)),
                Vec2::new(tx0, 1.0),
            ));
            builder.insert(Vertex::from_pos_uv(
                transform.transform_vector(Vec3::new(x1, 0.0, z1)),
                Vec2::new(tx1, 0.0),
            ));

            // sides
            builder.insert(Vertex::from_pos_uv(
                transform.transform_vector(Vec3::new(0.0, h, 0.0)),
                Vec2::new(tx1, 0.0),
            ));
            builder.insert(Vertex::from_pos_uv(
                transform.transform_vector(Vec3::new(x1, 0.0, z1)),
                Vec2::new(tx0, 1.0),
            ));
            builder.insert(Vertex::from_pos_uv(
                transform.transform_vector(Vec3::new(x0, 0.0, z0)),
                Vec2::new(tx0, 0.0),
            ));
        }

        let mut data = Self::from_raw_mesh(builder.build(), true);
        data.calculate_normals();
        data.calculate_tangents();
        data
    }

    /// Creates vertical cylinder.
    pub fn make_cylinder(sides: usize, r: f32, h: f32, caps: bool, transform: Mat4) -> Self {
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
                    transform.transform_vector(Vec3::new(x1, h, z1)),
                    Vec2::new(tx1, 1.0),
                ));
                builder.insert(Vertex::from_pos_uv(
                    transform.transform_vector(Vec3::new(x0, h, z0)),
                    Vec2::new(tx0, 1.0),
                ));
                builder.insert(Vertex::from_pos_uv(
                    transform.transform_vector(Vec3::new(0.0, h, 0.0)),
                    Vec2::new(0.0, 0.0),
                ));

                // back cap
                builder.insert(Vertex::from_pos_uv(
                    transform.transform_vector(Vec3::new(x0, 0.0, z0)),
                    Vec2::new(tx1, 1.0),
                ));
                builder.insert(Vertex::from_pos_uv(
                    transform.transform_vector(Vec3::new(x1, 0.0, z1)),
                    Vec2::new(tx0, 1.0),
                ));
                builder.insert(Vertex::from_pos_uv(
                    transform.transform_vector(Vec3::new(0.0, 0.0, 0.0)),
                    Vec2::new(0.0, 0.0),
                ));
            }

            // sides
            builder.insert(Vertex::from_pos_uv(
                transform.transform_vector(Vec3::new(x0, 0.0, z0)),
                Vec2::new(tx0, 0.0),
            ));
            builder.insert(Vertex::from_pos_uv(
                transform.transform_vector(Vec3::new(x0, h, z0)),
                Vec2::new(tx0, 1.0),
            ));
            builder.insert(Vertex::from_pos_uv(
                transform.transform_vector(Vec3::new(x1, 0.0, z1)),
                Vec2::new(tx1, 0.0),
            ));

            builder.insert(Vertex::from_pos_uv(
                transform.transform_vector(Vec3::new(x1, 0.0, z1)),
                Vec2::new(tx1, 0.0),
            ));
            builder.insert(Vertex::from_pos_uv(
                transform.transform_vector(Vec3::new(x0, h, z0)),
                Vec2::new(tx0, 1.0),
            ));
            builder.insert(Vertex::from_pos_uv(
                transform.transform_vector(Vec3::new(x1, h, z1)),
                Vec2::new(tx1, 1.0),
            ));
        }

        let mut data = Self::from_raw_mesh(builder.build(), true);
        data.calculate_normals();
        data.calculate_tangents();
        data
    }

    pub fn make_cube(transform: Mat4) -> Self {
        let mut vertices = vec![
            // Front
            Vertex {
                position: Vec3 {
                    x: -0.5,
                    y: -0.5,
                    z: 0.5,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: -0.5,
                    y: 0.5,
                    z: 0.5,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: 0.5,
                    y: 0.5,
                    z: 0.5,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: 0.5,
                    y: -0.5,
                    z: 0.5,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            // Back
            Vertex {
                position: Vec3 {
                    x: -0.5,
                    y: -0.5,
                    z: -0.5,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: -1.0,
                },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: -0.5,
                    y: 0.5,
                    z: -0.5,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: -1.0,
                },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: 0.5,
                    y: 0.5,
                    z: -0.5,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: -1.0,
                },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: 0.5,
                    y: -0.5,
                    z: -0.5,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: -1.0,
                },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            // Left
            Vertex {
                position: Vec3 {
                    x: -0.5,
                    y: -0.5,
                    z: -0.5,
                },
                normal: Vec3 {
                    x: -1.0,
                    y: 0.0,
                    z: 0.0,
                },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: -0.5,
                    y: 0.5,
                    z: -0.5,
                },
                normal: Vec3 {
                    x: -1.0,
                    y: 0.0,
                    z: 0.0,
                },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: -0.5,
                    y: 0.5,
                    z: 0.5,
                },
                normal: Vec3 {
                    x: -1.0,
                    y: 0.0,
                    z: 0.0,
                },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: -0.5,
                    y: -0.5,
                    z: 0.5,
                },
                normal: Vec3 {
                    x: -1.0,
                    y: 0.0,
                    z: 0.0,
                },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            // Right
            Vertex {
                position: Vec3 {
                    x: 0.5,
                    y: -0.5,
                    z: -0.5,
                },
                normal: Vec3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: 0.5,
                    y: 0.5,
                    z: -0.5,
                },
                normal: Vec3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: 0.5,
                    y: 0.5,
                    z: 0.5,
                },
                normal: Vec3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: 0.5,
                    y: -0.5,
                    z: 0.5,
                },
                normal: Vec3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            // Top
            Vertex {
                position: Vec3 {
                    x: -0.5,
                    y: 0.5,
                    z: 0.5,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: -0.5,
                    y: 0.5,
                    z: -0.5,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: 0.5,
                    y: 0.5,
                    z: -0.5,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: 0.5,
                    y: 0.5,
                    z: 0.5,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            // Bottom
            Vertex {
                position: Vec3 {
                    x: -0.5,
                    y: -0.5,
                    z: 0.5,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: -1.0,
                    z: 0.0,
                },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: -0.5,
                    y: -0.5,
                    z: -0.5,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: -1.0,
                    z: 0.0,
                },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: 0.5,
                    y: -0.5,
                    z: -0.5,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: -1.0,
                    z: 0.0,
                },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 {
                    x: 0.5,
                    y: -0.5,
                    z: 0.5,
                },
                normal: Vec3 {
                    x: 0.0,
                    y: -1.0,
                    z: 0.0,
                },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
        ];

        for v in vertices.iter_mut() {
            v.position = transform.transform_vector(v.position);
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

#[derive(Copy, Clone, Debug)]
pub struct VertexWeight {
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
    pub fn push(&mut self, weight: VertexWeight) -> bool {
        if self.count < self.weights.len() {
            self.weights[self.count] = weight;
            self.count += 1;
            true
        } else {
            false
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn iter(&self) -> std::slice::Iter<VertexWeight> {
        self.weights[0..self.count].iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<VertexWeight> {
        self.weights[0..self.count].iter_mut()
    }

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

#[derive(Debug, Default)]
pub struct Surface {
    // Wrapped into option to be able to implement Default for serialization.
    // In normal conditions it must never be None!
    data: Option<Arc<Mutex<SurfaceSharedData>>>,
    diffuse_texture: Option<Arc<Mutex<Texture>>>,
    normal_texture: Option<Arc<Mutex<Texture>>>,
    /// Temporal array for FBX conversion needs, it holds skinning data (weight + bone handle)
    /// and will be used to fill actual bone indices and weight in vertices that will be
    /// sent to GPU. The idea is very simple: GPU needs to know only indices of matrices of
    /// bones so we can use `bones` array as reference to get those indices. This could be done
    /// like so: iterate over all vertices and weight data and calculate index of node handle that
    /// associated with vertex in `bones` array and store it as bone index in vertex.
    pub vertex_weights: Vec<VertexWeightSet>,
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
        Surface {
            data: Some(Arc::clone(&self.data.as_ref().unwrap())),
            diffuse_texture: self.diffuse_texture.clone(),
            normal_texture: self.normal_texture.clone(),
            bones: self.bones.clone(),
            vertex_weights: Vec::new(),
            color: self.color,
        }
    }
}

impl Surface {
    #[inline]
    pub fn new(data: Arc<Mutex<SurfaceSharedData>>) -> Self {
        Self {
            data: Some(data),
            diffuse_texture: None,
            normal_texture: None,
            bones: Vec::new(),
            vertex_weights: Vec::new(),
            color: Color::WHITE,
        }
    }

    #[inline]
    pub fn data(&self) -> Arc<Mutex<SurfaceSharedData>> {
        self.data.as_ref().unwrap().clone()
    }

    #[inline]
    pub fn set_diffuse_texture(&mut self, tex: Arc<Mutex<Texture>>) {
        self.diffuse_texture = Some(tex);
    }

    #[inline]
    pub fn diffuse_texture(&self) -> Option<Arc<Mutex<Texture>>> {
        self.diffuse_texture.clone()
    }

    #[inline]
    pub fn set_normal_texture(&mut self, tex: Arc<Mutex<Texture>>) {
        self.normal_texture = Some(tex);
    }

    #[inline]
    pub fn normal_texture(&self) -> Option<Arc<Mutex<Texture>>> {
        self.normal_texture.clone()
    }

    #[inline]
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    #[inline]
    pub fn color(&self) -> Color {
        self.color
    }
}

impl Visit for Surface {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.data.visit("Data", visitor)?;
        self.normal_texture.visit("NormalTexture", visitor)?;
        self.diffuse_texture.visit("DiffuseTexture", visitor)?;
        self.color.visit("Color", visitor)?;
        self.bones.visit("Bones", visitor)?;
        // self.vertex_weights intentionally not serialized!

        visitor.leave_region()
    }
}

pub struct SurfaceBuilder {
    data: Arc<Mutex<SurfaceSharedData>>,
    diffuse_texture: Option<Arc<Mutex<Texture>>>,
    normal_texture: Option<Arc<Mutex<Texture>>>,
    bones: Vec<Handle<Node>>,
    color: Color,
}

impl SurfaceBuilder {
    pub fn new(data: Arc<Mutex<SurfaceSharedData>>) -> Self {
        Self {
            data,
            diffuse_texture: None,
            normal_texture: None,
            bones: Default::default(),
            color: Color::WHITE,
        }
    }

    pub fn with_diffuse_texture(mut self, tex: Arc<Mutex<Texture>>) -> Self {
        self.diffuse_texture = Some(tex);
        self
    }

    pub fn with_normal_texture(mut self, tex: Arc<Mutex<Texture>>) -> Self {
        self.normal_texture = Some(tex);
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_bones(mut self, bones: Vec<Handle<Node>>) -> Self {
        self.bones = bones;
        self
    }

    pub fn build(self) -> Surface {
        Surface {
            data: Some(self.data),
            diffuse_texture: self.diffuse_texture,
            normal_texture: self.normal_texture,
            vertex_weights: Default::default(),
            bones: self.bones,
            color: self.color,
        }
    }
}
