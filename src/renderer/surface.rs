use crate::math::{vec2::*, vec3::*, vec4::*};
use crate::renderer::renderer::*;
use crate::renderer::gl;
use crate::renderer::gl::types::*;
use std::{ffi::c_void, rc::Rc, cell::RefCell, mem::size_of, fmt};
use crate::resource::*;

#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: Vec3,
    pub tex_coord: Vec2,
    pub normal: Vec3,
    pub tangent: Vec4,
}

pub struct SurfaceSharedData {
    need_upload: bool,
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
            need_upload: true,
            vbo, vao, ebo,
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn add_vertex(&mut self, vertex: Vertex) {
        self.vertices.push(vertex);
    }

    /// Inserts vertex or its index. Performs optimizing insertion with checking if such
    /// vertex already exists.
    pub fn insert_vertex(&mut self, vertex: Vertex) {
        // Reverse search is much faster because it is most likely that we'll find identic
        // vertex at the end of the array.
        self.indices.push(match self.vertices.iter().rposition(|v| {
            v.position == vertex.position && v.normal == vertex.normal && v.tex_coord == vertex.tex_coord
        }) {
            Some(exisiting_index) => exisiting_index as i32, // Already have such vertex
            None => { // No such vertex, add it
                let index = self.vertices.len() as i32;
                self.vertices.push(vertex);
                index
            }
        });
        self.need_upload = true;
    }

    pub fn upload(&mut self) {
        let total_size_bytes = self.vertices.len() * std::mem::size_of::<Vertex>();

        unsafe {
            gl::BindVertexArray(self.vao);

            // Upload indices
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ebo);
            gl::BufferData(gl::ELEMENT_ARRAY_BUFFER,
                           (self.indices.len() * std::mem::size_of::<i32>()) as GLsizeiptr,
                           self.indices.as_ptr() as *const GLvoid,
                           gl::STATIC_DRAW);

            // Upload vertices
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            gl::BufferData(gl::ARRAY_BUFFER,
                           total_size_bytes as GLsizeiptr,
                           self.vertices.as_ptr() as *const GLvoid,
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

            gl::BindVertexArray(0);

            check_gl_error();
        }

        self.need_upload = false;
    }

    pub fn calculate_tangents(&self) {}

    pub fn make_cube() -> Self {
        let mut data = Self::new();

        data.vertices = vec![
            // Front
            Vertex {
                position: Vec3 { x: -0.5, y: -0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },
            Vertex {
                position: Vec3 { x: -0.5, y: 0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },
            Vertex {
                position: Vec3 { x: 0.5, y: 0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },
            Vertex {
                position: Vec3 { x: 0.5, y: -0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },

            // Back
            Vertex {
                position: Vec3 { x: -0.5, y: -0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: -1.0 },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },
            Vertex {
                position: Vec3 { x: -0.5, y: 0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: -1.0 },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },
            Vertex {
                position: Vec3 { x: 0.5, y: 0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: -1.0 },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },
            Vertex {
                position: Vec3 { x: 0.5, y: -0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: -1.0 },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },

            // Left
            Vertex {
                position: Vec3 { x: -0.5, y: -0.5, z: -0.5 },
                normal: Vec3 { x: -1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },
            Vertex {
                position: Vec3 { x: -0.5, y: 0.5, z: -0.5 },
                normal: Vec3 { x: -1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },
            Vertex {
                position: Vec3 { x: -0.5, y: 0.5, z: 0.5 },
                normal: Vec3 { x: -1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },
            Vertex {
                position: Vec3 { x: -0.5, y: -0.5, z: 0.5 },
                normal: Vec3 { x: -1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },

            // Right
            Vertex {
                position: Vec3 { x: 0.5, y: -0.5, z: -0.5 },
                normal: Vec3 { x: 1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },
            Vertex {
                position: Vec3 { x: 0.5, y: 0.5, z: -0.5 },
                normal: Vec3 { x: 1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },
            Vertex {
                position: Vec3 { x: 0.5, y: 0.5, z: 0.5 },
                normal: Vec3 { x: 1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },
            Vertex {
                position: Vec3 { x: 0.5, y: -0.5, z: 0.5 },
                normal: Vec3 { x: 1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },

            // Top
            Vertex {
                position: Vec3 { x: -0.5, y: 0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: 1.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },
            Vertex {
                position: Vec3 { x: -0.5, y: 0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: 1.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },
            Vertex {
                position: Vec3 { x: 0.5, y: 0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: 1.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },
            Vertex {
                position: Vec3 { x: 0.5, y: 0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: 1.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },

            // Bottom
            Vertex {
                position: Vec3 { x: -0.5, y: -0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },
            Vertex {
                position: Vec3 { x: -0.5, y: -0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },
            Vertex {
                position: Vec3 { x: 0.5, y: -0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
            },
            Vertex {
                position: Vec3 { x: 0.5, y: -0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
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

        data
    }
}

impl Drop for SurfaceSharedData {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &mut self.vbo);
            gl::DeleteBuffers(1, &mut self.ebo);
            gl::DeleteVertexArrays(1, &mut self.vao);
        }
    }
}

pub struct Surface {
    pub(crate) data: Rc<RefCell<SurfaceSharedData>>,
    pub(crate) texture: Option<Rc<RefCell<Resource>>>,
}

impl Surface {
    pub fn new(data: &Rc<RefCell<SurfaceSharedData>>) -> Self {
        Self {
            data: data.clone(),
            texture: Option::None,
        }
    }

    pub fn draw(&self) {
        unsafe {
            let mut data = self.data.borrow_mut();
            if data.need_upload {
                data.upload();
            }
            if let Some(ref resource) = self.texture {
                if let ResourceKind::Texture(texture) = &resource.borrow_mut().borrow_kind() {
                    gl::BindTexture(gl::TEXTURE_2D, texture.gpu_tex);
                } else {
                    gl::BindTexture(gl::TEXTURE_2D, 0);
                }
            } else {
                gl::BindTexture(gl::TEXTURE_2D, 0);
            }
            gl::BindVertexArray(data.vao);
            gl::DrawElements(gl::TRIANGLES,
                             data.indices.len() as GLint,
                             gl::UNSIGNED_INT,
                             std::ptr::null());
        }
    }

    pub fn set_texture(&mut self, tex: Rc<RefCell<Resource>>) {
        if let ResourceKind::Texture(_) = tex.borrow_mut().borrow_kind() {
            self.texture = Some(tex.clone());
        } else {
            self.texture = None;
        }
    }
}