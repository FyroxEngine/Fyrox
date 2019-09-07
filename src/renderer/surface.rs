use crate::{
    math::{
        vec2::Vec2,
        vec3::Vec3,
        vec4::Vec4,
    },
    renderer::{
        gl,
        gl::types::*,
    },
    resource::*,
    scene::node::Node,
    utils::pool::{Handle, ErasedHandle},
};
use std::{
    rc::Rc,
    cell::{
        RefCell,
        Cell
    },
    mem::size_of,
    ffi::c_void,
};

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
    need_upload: Cell<bool>,
    vbo: GLuint,
    vao: GLuint,
    ebo: GLuint,
    vertices: Vec<Vertex>,
    indices: Vec<i32>,
}

impl SurfaceSharedData {
    pub fn new() -> Self {
        let mut vbo: GLuint = 0;
        let mut ebo: GLuint = 0;
        let mut vao: GLuint = 0;

        unsafe {
            gl::GenBuffers(1, &mut vbo);
            gl::GenBuffers(1, &mut ebo);
            gl::GenVertexArrays(1, &mut vao);
        }

        Self {
            need_upload: Cell::new(true),
            vbo,
            vao,
            ebo,
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    #[inline]
    pub fn add_vertex(&mut self, vertex: Vertex) {
        self.vertices.push(vertex);
    }

    #[inline]
    pub fn get_vertex_array_object(&self) -> GLuint {
        self.vao
    }

    #[inline]
    pub fn get_vertex_buffer_object(&self) -> GLuint {
        self.vbo
    }

    #[inline]
    pub fn get_element_buffer_object(&self) -> GLuint {
        self.ebo
    }

    pub fn draw(&self) {
        unsafe {
            if self.need_upload.get() {
                let total_size_bytes = self.get_vertices().len() * std::mem::size_of::<Vertex>();

                gl::BindVertexArray(self.get_vertex_array_object());

                // Upload indices
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.get_element_buffer_object());
                gl::BufferData(gl::ELEMENT_ARRAY_BUFFER,
                               (self.get_indices().len() * size_of::<i32>()) as GLsizeiptr,
                               self.get_indices().as_ptr() as *const GLvoid,
                               gl::STATIC_DRAW);

                // Upload vertices
                gl::BindBuffer(gl::ARRAY_BUFFER, self.get_vertex_buffer_object());
                gl::BufferData(gl::ARRAY_BUFFER,
                               total_size_bytes as GLsizeiptr,
                               self.get_vertices().as_ptr() as *const GLvoid,
                               gl::STATIC_DRAW);

                let mut offset = 0;

                // Positions
                gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE,
                                        size_of::<Vertex>() as GLint, offset as *const c_void);
                gl::EnableVertexAttribArray(0);
                offset += size_of::<Vec3>();

                // Texture coordinates
                gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE,
                                        size_of::<Vertex>() as GLint, offset as *const c_void);
                gl::EnableVertexAttribArray(1);
                offset += size_of::<Vec2>();

                // Normals
                gl::VertexAttribPointer(2, 3, gl::FLOAT, gl::FALSE,
                                        size_of::<Vertex>() as GLint, offset as *const c_void);
                gl::EnableVertexAttribArray(2);
                offset += size_of::<Vec3>();

                // Tangents
                gl::VertexAttribPointer(3, 4, gl::FLOAT, gl::FALSE,
                                        size_of::<Vertex>() as GLint, offset as *const c_void);
                gl::EnableVertexAttribArray(3);
                offset += size_of::<Vec4>();

                // Bone weights
                gl::VertexAttribPointer(4, 4, gl::FLOAT, gl::FALSE,
                                        size_of::<Vertex>() as GLint, offset as *const c_void);
                gl::EnableVertexAttribArray(4);
                offset += size_of::<Vec4>();

                // Bone indices
                gl::VertexAttribPointer(5, 4, gl::UNSIGNED_BYTE, gl::FALSE,
                                        size_of::<Vertex>() as GLint, offset as *const c_void);
                gl::EnableVertexAttribArray(5);

                gl::BindVertexArray(0);

                self.need_upload.set(false);
            } else {}

            gl::BindVertexArray(self.get_vertex_array_object());
            gl::DrawElements(gl::TRIANGLES,
                             self.get_indices().len() as GLint,
                             gl::UNSIGNED_INT,
                             std::ptr::null());
        }
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
            Some(exisiting_index) => exisiting_index as i32, // Already have such vertex
            None => { // No such vertex, add it
                is_unique = true;
                let index = self.vertices.len() as i32;
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
    pub fn get_indices(&self) -> &[i32] {
        self.indices.as_slice()
    }

    pub fn calculate_tangents(&mut self) {
        let mut tan1 = vec![Vec3::new(); self.vertices.len()];
        let mut tan2 = vec![Vec3::new(); self.vertices.len()];

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

            let sdir = Vec3::make(
                (t2 * x1 - t1 * x2) * r,
                (t2 * y1 - t1 * y2) * r,
                (t2 * z1 - t1 * z2) * r,
            );

            tan1[i1] += sdir;
            tan1[i2] += sdir;
            tan1[i3] += sdir;

            let tdir = Vec3::make(
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
            let tangent = (t - n.scale(n.dot(&t))).normalized().unwrap_or(Vec3::make(0.0, 1.0, 0.0));

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

    pub fn insert_vertex_pos_tex(&mut self, pos: &Vec3, tex: Vec2) {
        self.insert_vertex(Vertex {
            position: *pos,
            tex_coord: tex,
            normal: Vec3::make(0.0, 1.0, 0.0),
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
                    data.insert_vertex_pos_tex(&Vec3::make(k0 * k1, k0 * k2, k3), Vec2::make(d_tc_x * j as f32, d_tc_y * i as f32));
                    data.insert_vertex_pos_tex(&Vec3::make(k4 * k1, k4 * k2, k7), Vec2::make(d_tc_x * j as f32, d_tc_y * ni as f32));
                    data.insert_vertex_pos_tex(&Vec3::make(k4 * k5, k4 * k6, k7), Vec2::make(d_tc_x * nj as f32, d_tc_y * ni as f32));
                }

                if i != 0 {
                    data.insert_vertex_pos_tex(&Vec3::make(k4 * k5, k4 * k6, k7), Vec2::make(d_tc_x * nj as f32, d_tc_y * ni as f32));
                    data.insert_vertex_pos_tex(&Vec3::make(k0 * k5, k0 * k6, k3), Vec2::make(d_tc_x * nj as f32, d_tc_y * i as f32));
                    data.insert_vertex_pos_tex(&Vec3::make(k0 * k1, k0 * k2, k3), Vec2::make(d_tc_x * j as f32, d_tc_y * i as f32));
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

impl Drop for SurfaceSharedData {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.vbo);
            gl::DeleteBuffers(1, &self.ebo);
            gl::DeleteVertexArrays(1, &self.vao);
        }
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

    pub fn iter(&self) -> std::slice::Iter<VertexWeight> {
        self.weights[0..self.count].iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<VertexWeight> {
        self.weights[0..self.count].iter_mut()
    }
}

pub struct Surface {
    data: Rc<RefCell<SurfaceSharedData>>,
    diffuse_texture: Option<Rc<RefCell<Resource>>>,
    normal_texture: Option<Rc<RefCell<Resource>>>,
    /// Temporal array for FBX conversion needs, it holds skinning data (weight + bone handle)
    /// and will be used to fill actual bone indices and weight in vertices that will be
    /// sent to GPU. The idea is very simple: GPU needs to know only indices of matrices of
    /// bones so we can use `bones` array as reference to get those indices. This could be done
    /// like so: iterate over all vertices and weight data and calculate index of node handle that
    /// associated with vertex in `bones` array and store it as bone index in vertex.
    pub vertex_weights: Vec<VertexWeightSet>,
    pub bones: Vec<Handle<Node>>,
}

impl Surface {
    #[inline]
    pub fn new(data: Rc<RefCell<SurfaceSharedData>>) -> Self {
        Self {
            data,
            diffuse_texture: None,
            normal_texture: None,
            bones: Vec::new(),
            vertex_weights: Vec::new(),
        }
    }

    #[inline]
    pub fn get_data(&self) -> Rc<RefCell<SurfaceSharedData>> {
        Rc::clone(&self.data)
    }

    #[inline]
    pub fn get_diffuse_texture(&self) -> Option<Rc<RefCell<Resource>>> {
        match &self.diffuse_texture {
            Some(resource) => Some(Rc::clone(resource)),
            None => None
        }
    }

    #[inline]
    pub fn get_normal_texture(&self) -> Option<Rc<RefCell<Resource>>> {
        match &self.normal_texture {
            Some(resource) => Some(Rc::clone(resource)),
            None => None
        }
    }

    #[inline]
    pub fn set_diffuse_texture(&mut self, tex: Rc<RefCell<Resource>>) {
        self.diffuse_texture = Some(tex);
    }

    #[inline]
    pub fn set_normal_texture(&mut self, tex: Rc<RefCell<Resource>>) {
        self.normal_texture = Some(tex);
    }

    #[inline]
    pub fn make_copy(&self) -> Surface {
        Surface {
            data: Rc::clone(&self.data),
            diffuse_texture: match &self.diffuse_texture {
                Some(resource) => Some(Rc::clone(resource)),
                None => None
            },
            normal_texture: match &self.normal_texture {
                Some(resource) => Some(Rc::clone(resource)),
                None => None
            },
            // Note: Handles will be remapped on Resolve stage.
            bones: self.bones.clone(),
            vertex_weights: Vec::new(),
        }
    }
}