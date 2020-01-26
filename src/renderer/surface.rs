use crate::{
    renderer::{
        geometry_buffer::{GeometryBuffer, GeometryBufferKind, AttributeDefinition, AttributeKind},
        TriangleDefinition,
    },
    scene::node::Node,
    resource::texture::Texture,
};

use crate::core::{
    math::{
        vec2::Vec2,
        vec3::Vec3,
        vec4::Vec4,
    },
    pool::{Handle, ErasedHandle},
};
use std::{
    sync::{Mutex, Arc},
    cell::Cell,
};
use crate::renderer::geometry_buffer::ElementKind;

#[derive(Copy, Clone, Debug)]
#[repr(C)] // OpenGL expects this structure packed as in C
pub struct Vertex {
    pub position: Vec3,
    pub tex_coord: Vec2,
    pub normal: Vec3,
    pub tangent: Vec4,
    pub bone_weights: [f32; 4],
    pub bone_indices: [u8; 4],
}

pub struct SurfaceSharedData {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    need_upload: Cell<bool>,
    geometry_buffer: GeometryBuffer<Vertex>,
}

impl Default for SurfaceSharedData {
    fn default() -> Self {
        Self::new()
    }
}

impl SurfaceSharedData {
    pub fn new() -> Self {
        let geometry_buffer = GeometryBuffer::new(GeometryBufferKind::StaticDraw, ElementKind::Triangle);

        geometry_buffer.describe_attributes(vec![
            AttributeDefinition { kind: AttributeKind::Float3, normalized: false },
            AttributeDefinition { kind: AttributeKind::Float2, normalized: false },
            AttributeDefinition { kind: AttributeKind::Float3, normalized: false },
            AttributeDefinition { kind: AttributeKind::Float4, normalized: false },
            AttributeDefinition { kind: AttributeKind::Float4, normalized: false },
            AttributeDefinition { kind: AttributeKind::UnsignedByte4, normalized: false },
        ]).unwrap();

        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            need_upload: Cell::new(true),
            geometry_buffer,
        }
    }

    #[inline]
    pub fn add_vertex(&mut self, vertex: Vertex) {
        self.vertices.push(vertex);
    }

    /// Draws surface, returns amount of triangles were rendered.
    pub fn draw(&self) -> usize {
        if self.need_upload.get() {
            self.geometry_buffer.set_vertices(self.vertices.as_slice());

            let mut triangles = Vec::with_capacity(self.indices.len() / 3);
            for i in (0..self.indices.len()).step_by(3) {
                triangles.push(TriangleDefinition { indices: [self.indices[i], self.indices[i + 1], self.indices[i + 2]] });
            }

            self.geometry_buffer.set_triangles(&triangles);
            self.need_upload.set(false);
        }

        self.geometry_buffer.draw()
    }

    /// Inserts vertex or its index. Performs optimizing insertion with checking if such
    /// vertex already exists.
    /// Returns true if inserted vertex was unique.
    #[inline]
    pub fn insert_vertex(&mut self, vertex: Vertex) -> bool {
        // Reverse search is much faster because it is most likely that we'll find identic
        // vertex at the end of the array.
        let mut is_unique = false;
        self.indices.push(match self.vertices.iter().rposition(|v| {
            v.position == vertex.position && v.normal == vertex.normal && v.tex_coord == vertex.tex_coord
        }) {
            Some(existing_index) => existing_index as u32, // Already have such vertex
            None => { // No such vertex, add it
                is_unique = true;
                let index = self.vertices.len() as u32;
                self.vertices.push(vertex);
                index
            }
        });
        self.need_upload.set(true);
        is_unique
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
    pub fn get_indices(&self) -> &[u32] {
        self.indices.as_slice()
    }

    pub fn calculate_tangents(&mut self) {
        let mut tan1 = vec![Vec3::ZERO; self.vertices.len()];
        let mut tan2 = vec![Vec3::ZERO; self.vertices.len()];

        for i in (0..self.indices.len()).step_by(3) {
            let i1 = self.indices[i] as usize;
            let i2 = self.indices[i + 1] as usize;
            let i3 = self.indices[i + 2] as usize;

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

        for i in 0..self.vertices.len() {
            let n = &self.vertices[i].normal;
            let t = tan1[i];

            // Gram-Schmidt orthogonalize
            let tangent = (t - n.scale(n.dot(&t))).normalized().unwrap_or_else(|| Vec3::new(0.0, 1.0, 0.0));

            self.vertices[i].tangent = Vec4 {
                x: tangent.x,
                y: tangent.y,
                z: tangent.z,
                // Handedness
                w: if n.cross(&t).dot(&tan2[i]) < 0.0 {
                    -1.0
                } else {
                    1.0
                },
            };
        }
    }

    pub fn make_unit_xy_quad() -> Self {
        let mut data = Self::new();

        data.vertices = vec![
            Vertex {
                position: Vec3 { x: 0.0, y: 0.0, z: 0.0 },
                normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 1.0, y: 0.0, z: 0.0 },
                normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 1.0, y: 1.0, z: 0.0 },
                normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.0, y: 1.0, z: 0.0 },
                normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            }
        ];

        data.indices = vec![
            0, 1, 2,
            0, 2, 3
        ];

        data
    }

    pub fn make_collapsed_xy_quad() -> Self {
        let mut data = Self::new();

        data.vertices = vec![
            Vertex {
                position: Vec3 { x: 0.0, y: 0.0, z: 0.0 },
                normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.0, y: 0.0, z: 0.0 },
                normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.0, y: 0.0, z: 0.0 },
                normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.0, y: 0.0, z: 0.0 },
                normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            }
        ];

        data.indices = vec![
            0, 1, 2,
            0, 2, 3
        ];

        data
    }

    pub fn insert_vertex_pos_tex(&mut self, pos: &Vec3, tex: Vec2) {
        self.insert_vertex(Vertex {
            position: *pos,
            tex_coord: tex,
            normal: Vec3::new(0.0, 1.0, 0.0),
            tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            bone_weights: [0.0, 0.0, 0.0, 0.0],
            bone_indices: Default::default(),
        });
    }

    pub fn calculate_normals(&mut self) {
        for m in (0..self.indices.len()).step_by(3) {
            let ia = self.indices[m] as usize;
            let ib = self.indices[m + 1] as usize;
            let ic = self.indices[m + 2] as usize;

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
        let mut data = Self::new();

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
                    data.insert_vertex_pos_tex(&Vec3::new(k0 * k1, k0 * k2, k3), Vec2::new(d_tc_x * j as f32, d_tc_y * i as f32));
                    data.insert_vertex_pos_tex(&Vec3::new(k4 * k1, k4 * k2, k7), Vec2::new(d_tc_x * j as f32, d_tc_y * ni as f32));
                    data.insert_vertex_pos_tex(&Vec3::new(k4 * k5, k4 * k6, k7), Vec2::new(d_tc_x * nj as f32, d_tc_y * ni as f32));
                }

                if i != 0 {
                    data.insert_vertex_pos_tex(&Vec3::new(k4 * k5, k4 * k6, k7), Vec2::new(d_tc_x * nj as f32, d_tc_y * ni as f32));
                    data.insert_vertex_pos_tex(&Vec3::new(k0 * k5, k0 * k6, k3), Vec2::new(d_tc_x * nj as f32, d_tc_y * i as f32));
                    data.insert_vertex_pos_tex(&Vec3::new(k0 * k1, k0 * k2, k3), Vec2::new(d_tc_x * j as f32, d_tc_y * i as f32));
                }
            }
        }

        data.calculate_normals();
        data.calculate_tangents();

        data
    }

    pub fn make_cube() -> Self {
        let mut data = Self::new();

        data.vertices = vec![
            // Front
            Vertex {
                position: Vec3 { x: -0.5, y: -0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: -0.5, y: 0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: 0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: -0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },

            // Back
            Vertex {
                position: Vec3 { x: -0.5, y: -0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: -1.0 },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: -0.5, y: 0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: -1.0 },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: 0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: -1.0 },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: -0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: -1.0 },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },

            // Left
            Vertex {
                position: Vec3 { x: -0.5, y: -0.5, z: -0.5 },
                normal: Vec3 { x: -1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: -0.5, y: 0.5, z: -0.5 },
                normal: Vec3 { x: -1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: -0.5, y: 0.5, z: 0.5 },
                normal: Vec3 { x: -1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: -0.5, y: -0.5, z: 0.5 },
                normal: Vec3 { x: -1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },

            // Right
            Vertex {
                position: Vec3 { x: 0.5, y: -0.5, z: -0.5 },
                normal: Vec3 { x: 1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: 0.5, z: -0.5 },
                normal: Vec3 { x: 1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: 0.5, z: 0.5 },
                normal: Vec3 { x: 1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: -0.5, z: 0.5 },
                normal: Vec3 { x: 1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },

            // Top
            Vertex {
                position: Vec3 { x: -0.5, y: 0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: 1.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: -0.5, y: 0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: 1.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: 0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: 1.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: 0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: 1.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },

            // Bottom
            Vertex {
                position: Vec3 { x: -0.5, y: -0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: -0.5, y: -0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: -0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: -0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: [0.0, 0.0, 0.0, 0.0],
                bone_indices: [0, 0, 0, 0],
            },
        ];

        data.indices = vec![
            2, 1, 0,
            3, 2, 0,
            4, 5, 6,
            4, 6, 7,
            10, 9, 8,
            11, 10, 8,
            12, 13, 14,
            12, 14, 15,
            18, 17, 16,
            19, 18, 16,
            20, 21, 22,
            20, 22, 23
        ];

        data.calculate_tangents();

        data
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
}

pub struct Surface {
    data: Arc<Mutex<SurfaceSharedData>>,
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
            data: Arc::clone(&self.data),
            diffuse_texture: self.diffuse_texture.clone(),
            normal_texture: self.normal_texture.clone(),
            bones: self.bones.clone(),
            vertex_weights: Vec::new(),
        }
    }
}

impl Surface {
    #[inline]
    pub fn new(data: Arc<Mutex<SurfaceSharedData>>) -> Self {
        Self {
            data,
            diffuse_texture: None,
            normal_texture: None,
            bones: Vec::new(),
            vertex_weights: Vec::new(),
        }
    }

    #[inline]
    pub fn get_data(&self) -> Arc<Mutex<SurfaceSharedData>> {
        self.data.clone()
    }

    #[inline]
    pub fn get_diffuse_texture(&self) -> Option<Arc<Mutex<Texture>>> {
        self.diffuse_texture.clone()
    }

    #[inline]
    pub fn get_normal_texture(&self) -> Option<Arc<Mutex<Texture>>> {
        self.normal_texture.clone()
    }

    #[inline]
    pub fn set_diffuse_texture(&mut self, tex: Arc<Mutex<Texture>>) {
        self.diffuse_texture = Some(tex);
    }

    #[inline]
    pub fn set_normal_texture(&mut self, tex: Arc<Mutex<Texture>>) {
        self.normal_texture = Some(tex);
    }
}