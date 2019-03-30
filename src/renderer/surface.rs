use crate::math::vec3::*;
use crate::math::vec2::*;
use crate::math::vec4::*;
use crate::renderer::renderer::*;
use crate::renderer::gl;
use crate::renderer::gl::types::*;
use std::ffi::{CStr, CString, c_void};
use std::rc::Rc;
use std::rc::Weak;
use std::cell::RefCell;

pub struct SurfaceSharedData {
    need_upload: bool,
    vbo: GLuint,
    vao: GLuint,
    ebo: GLuint,
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    tex_coords: Vec<Vec2>,
    tangents: Vec<Vec4>,
    indices: Vec<i32>,
}

impl SurfaceSharedData {
    fn new() -> Self {
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
            vbo,
            vao,
            ebo,
            positions: Vec::new(),
            normals: Vec::new(),
            tex_coords: Vec::new(),
            tangents: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn upload(&mut self) {
        let positions_bytes = self.positions.len() * std::mem::size_of::<Vec3>();
        let tex_coords_bytes = self.tex_coords.len() * std::mem::size_of::<Vec2>();
        let normals_bytes = self.normals.len() * std::mem::size_of::<Vec3>();
        let tangents_bytes = self.tangents.len() * std::mem::size_of::<Vec4>();


        let total_size_bytes = positions_bytes + normals_bytes + tex_coords_bytes + tangents_bytes;

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
                           std::ptr::null(),
                           gl::STATIC_DRAW);

            let pos_offset = 0;
            let mut size = positions_bytes;
            gl::BufferSubData(gl::ARRAY_BUFFER,
                              pos_offset as GLsizeiptr,
                              size as GLsizeiptr,
                              self.positions.as_ptr() as *const c_void);

            let tex_coord_offset = pos_offset + size;
            size = tex_coords_bytes;
            gl::BufferSubData(gl::ARRAY_BUFFER,
                              tex_coord_offset as GLsizeiptr,
                              size as GLsizeiptr,
                              self.tex_coords.as_ptr() as *const c_void);

            let normals_offset = tex_coord_offset + size;
            size = normals_bytes;
            gl::BufferSubData(gl::ARRAY_BUFFER,
                              normals_offset as GLsizeiptr,
                              size as GLsizeiptr,
                              self.normals.as_ptr() as *const c_void);

            let tangents_offset = normals_offset + size;
            size = tangents_bytes;
            gl::BufferSubData(gl::ARRAY_BUFFER,
                              tangents_offset as GLsizeiptr,
                              size as GLsizeiptr,
                              self.tangents.as_ptr() as *const c_void);

            // Setup attribute locations
            gl::VertexAttribPointer(0,
                                    3,
                                    gl::FLOAT,
                                    gl::FALSE,
                                    std::mem::size_of::<Vec3>() as GLint,
                                    pos_offset as *const c_void);
            gl::EnableVertexAttribArray(0);

            gl::VertexAttribPointer(1,
                                    2,
                                    gl::FLOAT,
                                    gl::FALSE,
                                    std::mem::size_of::<Vec2>() as GLint,
                                    tex_coord_offset as *const c_void);
            gl::EnableVertexAttribArray(1);

            gl::VertexAttribPointer(2,
                                    3,
                                    gl::FLOAT,
                                    gl::FALSE,
                                    std::mem::size_of::<Vec3>() as GLint,
                                    normals_offset as *const c_void);
            gl::EnableVertexAttribArray(2);

            gl::VertexAttribPointer(2,
                                    3,
                                    gl::FLOAT,
                                    gl::FALSE,
                                    std::mem::size_of::<Vec4>() as GLint,
                                    tangents_offset as *const c_void);
            gl::EnableVertexAttribArray(3);

            gl::BindVertexArray(0);

            check_gl_error();
        }

        self.need_upload = false;
    }

    pub fn calculate_tangents(&self) {

    }

    pub fn make_cube() -> Self {
        let mut data = Self::new();

        data.positions = vec![
            // Front
            Vec3 { x: -0.5, y: -0.5, z: 0.5 },
            Vec3 { x: -0.5, y: 0.5, z: 0.5 },
            Vec3 { x: 0.5, y: 0.5, z: 0.5 },
            Vec3 { x: 0.5, y: -0.5, z: 0.5 },

            // Back
            Vec3 { x: -0.5, y: -0.5, z: -0.5 },
            Vec3 { x: -0.5, y: 0.5, z: -0.5 },
            Vec3 { x: 0.5, y: 0.5, z: -0.5 },
            Vec3 { x: 0.5, y: -0.5, z: -0.5 },

            // Left
            Vec3 { x: -0.5, y: -0.5, z: -0.5 },
            Vec3 { x: -0.5, y: 0.5, z: -0.5 },
            Vec3 { x: -0.5, y: 0.5, z: 0.5 },
            Vec3 { x: -0.5, y: -0.5, z: 0.5 },

            // Right
            Vec3 { x: 0.5, y: -0.5, z: -0.5 },
            Vec3 { x: 0.5, y: 0.5, z: -0.5 },
            Vec3 { x: 0.5, y: 0.5, z: 0.5 },
            Vec3 { x: 0.5, y: -0.5, z: 0.5 },

            // Top
            Vec3 { x: -0.5, y: 0.5, z: 0.5 },
            Vec3 { x: -0.5, y: 0.5, z: -0.5 },
            Vec3 { x: 0.5, y: 0.5, z: -0.5 },
            Vec3 { x: 0.5, y: 0.5, z: 0.5 },

            // Bottom
            Vec3 { x: -0.5, y: -0.5, z: 0.5 },
            Vec3 { x: -0.5, y: -0.5, z: -0.5 },
            Vec3 { x: 0.5, y: -0.5, z: -0.5 },
            Vec3 { x: 0.5, y: -0.5, z: 0.5 },
        ];

        data.normals = vec![
            // Front
            Vec3 { x: 0.0, y: 0.0, z: 1.0 },
            Vec3 { x: 0.0, y: 0.0, z: 1.0 },
            Vec3 { x: 0.0, y: 0.0, z: 1.0 },
            Vec3 { x: 0.0, y: 0.0, z: 1.0 },

            // Back
            Vec3 { x: 0.0, y: 0.0, z: -1.0 },
            Vec3 { x: 0.0, y: 0.0, z: -1.0 },
            Vec3 { x: 0.0, y: 0.0, z: -1.0 },
            Vec3 { x: 0.0, y: 0.0, z: -1.0 },

            // Left
            Vec3 { x: -1.0, y: 0.0, z: 0.0 },
            Vec3 { x: -1.0, y: 0.0, z: 0.0 },
            Vec3 { x: -1.0, y: 0.0, z: 0.0 },
            Vec3 { x: -1.0, y: 0.0, z: 0.0 },

            // Right
            Vec3 { x: 1.0, y: 0.0, z: 0.0 },
            Vec3 { x: 1.0, y: 0.0, z: 0.0 },
            Vec3 { x: 1.0, y: 0.0, z: 0.0 },
            Vec3 { x: 1.0, y: 0.0, z: 0.0 },

            // Top
            Vec3 { x: 0.0, y: 1.0, z: 0.0 },
            Vec3 { x: 0.0, y: 1.0, z: 0.0 },
            Vec3 { x: 0.0, y: 1.0, z: 0.0 },
            Vec3 { x: 0.0, y: 1.0, z: 0.0 },

            // Bottom
            Vec3 { x: 0.0, y: -1.0, z: 0.0 },
            Vec3 { x: 0.0, y: -1.0, z: 0.0 },
            Vec3 { x: 0.0, y: -1.0, z: 0.0 },
            Vec3 { x: 0.0, y: -1.0, z: 0.0 },
        ];

        data.tex_coords = vec![
            // Front
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 0.0, y: 1.0 },
            Vec2 { x: 1.0, y: 1.0 },
            Vec2 { x: 1.0, y: 0.0 },

            // Back
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 0.0, y: 1.0 },
            Vec2 { x: 1.0, y: 1.0 },
            Vec2 { x: 1.0, y: 0.0 },

            // Left
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 0.0, y: 1.0 },
            Vec2 { x: 1.0, y: 1.0 },
            Vec2 { x: 1.0, y: 0.0 },

            // Right
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 0.0, y: 1.0 },
            Vec2 { x: 1.0, y: 1.0 },
            Vec2 { x: 1.0, y: 0.0 },

            // Top
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 0.0, y: 1.0 },
            Vec2 { x: 1.0, y: 1.0 },
            Vec2 { x: 1.0, y: 0.0 },

            // Bottom
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 0.0, y: 1.0 },
            Vec2 { x: 1.0, y: 1.0 },
            Vec2 { x: 1.0, y: 0.0 },
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

type SurfaceSharedDataRef = Rc<RefCell<SurfaceSharedData>>;

pub struct Surface {
    pub(crate) data: SurfaceSharedDataRef
}

impl Surface {
    pub fn new(data: &SurfaceSharedDataRef) -> Self {
        Self {
            data: data.clone()
        }
    }

    pub fn draw(&self) {
        unsafe {
            let mut data = self.data.borrow_mut();
            if data.need_upload {
                data.upload();
            }
            gl::BindVertexArray(data.vao);
            gl::DrawElements(gl::TRIANGLES,
                             data.indices.len() as GLint,
                             gl::UNSIGNED_INT,
                             std::ptr::null());
        }
    }
}