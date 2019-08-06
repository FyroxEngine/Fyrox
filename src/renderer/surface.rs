use crate::{
    math::{vec2::*, vec3::*, vec4::*},
    renderer::{
        gl,
        gl::types::*
    },
    resource::*,
    utils::rcpool::{RcHandle},
    engine::State
};
use serde::{Serialize, Deserialize};

#[derive(Copy, Clone, Debug)]
#[repr(C)] // OpenGL expects this structure packed as in C
pub struct Vertex {
    pub position: Vec3,
    pub tex_coord: Vec2,
    pub normal: Vec3,
    pub tangent: Vec4,
    pub bone_weights: Vec4,
    pub bone_indices: [u8; 4],
}

#[derive(Serialize, Deserialize)]
pub struct SurfaceSharedData {
    pub need_upload: bool,
    #[serde(skip)]
    vbo: GLuint,
    #[serde(skip)]
    vao: GLuint,
    #[serde(skip)]
    ebo: GLuint,

    // Skip vertices and indices for now, later it can be useful for dynamic surfaces.
    #[serde(skip)]
    vertices: Vec<Vertex>,
    #[serde(skip)]
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

    /// Inserts vertex or its index. Performs optimizing insertion with checking if such
    /// vertex already exists.
    #[inline]
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

    #[inline]
    pub fn get_vertices(&self) -> &[Vertex] {
        self.vertices.as_slice()
    }

    #[inline]
    pub fn get_indices(&self) -> &[i32] {
        self.indices.as_slice()
    }

    pub fn calculate_tangents(&self) {

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
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: -0.5, y: 0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: 0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: -0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },

            // Back
            Vertex {
                position: Vec3 { x: -0.5, y: -0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: -1.0 },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: -0.5, y: 0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: -1.0 },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: 0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: -1.0 },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: -0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: 0.0, z: -1.0 },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },

            // Left
            Vertex {
                position: Vec3 { x: -0.5, y: -0.5, z: -0.5 },
                normal: Vec3 { x: -1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: -0.5, y: 0.5, z: -0.5 },
                normal: Vec3 { x: -1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: -0.5, y: 0.5, z: 0.5 },
                normal: Vec3 { x: -1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: -0.5, y: -0.5, z: 0.5 },
                normal: Vec3 { x: -1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },

            // Right
            Vertex {
                position: Vec3 { x: 0.5, y: -0.5, z: -0.5 },
                normal: Vec3 { x: 1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: 0.5, z: -0.5 },
                normal: Vec3 { x: 1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: 0.5, z: 0.5 },
                normal: Vec3 { x: 1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: -0.5, z: 0.5 },
                normal: Vec3 { x: 1.0, y: 0.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },

            // Top
            Vertex {
                position: Vec3 { x: -0.5, y: 0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: 1.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: -0.5, y: 0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: 1.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: 0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: 1.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: 0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: 1.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },

            // Bottom
            Vertex {
                position: Vec3 { x: -0.5, y: -0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: -0.5, y: -0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
                tex_coord: Vec2 { x: 0.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: -0.5, z: -0.5 },
                normal: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 1.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0],
            },
            Vertex {
                position: Vec3 { x: 0.5, y: -0.5, z: 0.5 },
                normal: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
                tex_coord: Vec2 { x: 1.0, y: 0.0 },
                tangent: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
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

#[derive(Serialize, Deserialize)]
pub struct Surface {
    data: RcHandle<SurfaceSharedData>,
    texture: RcHandle<Resource>,
}

impl Surface {
    #[inline]
    pub fn new(data: RcHandle<SurfaceSharedData>) -> Self {
        Self {
            data,
            texture: RcHandle::none(),
        }
    }

    #[inline]
    pub fn get_data_handle(&self) -> &RcHandle<SurfaceSharedData> {
        &self.data
    }

    #[inline]
    pub fn get_texture_resource_handle(&self) -> &RcHandle<Resource> {
        &self.texture
    }

    #[inline]
    pub fn set_texture(&mut self, tex: RcHandle<Resource>) {
        self.texture = tex;
    }

    #[inline]
    pub fn make_copy(&self, state: &State) -> Surface {
        Surface {
            data: state.get_surface_data_storage().share_handle(&self.data),
            texture: state.get_resource_manager().share_resource_handle(&self.texture)
        }
    }
}