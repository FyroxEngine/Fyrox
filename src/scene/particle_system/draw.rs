use crate::core::algebra::{Vector2, Vector3};
use crate::core::color::Color;
use crate::core::math::TriangleDefinition;

/// OpenGL expects this structure packed as in C.
#[repr(C)]
#[derive(Debug)]
pub struct Vertex {
    pub position: Vector3<f32>,
    pub tex_coord: Vector2<f32>,
    pub size: f32,
    pub rotation: f32,
    pub color: Color,
}

/// Particle system is "rendered" into special buffer, which contains vertices and faces.
pub struct DrawData {
    pub(super) vertices: Vec<Vertex>,
    pub(super) triangles: Vec<TriangleDefinition>,
}

impl Default for DrawData {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            triangles: Vec::new(),
        }
    }
}

impl DrawData {
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.triangles.clear();
    }

    /// Returns shared reference to array of vertices.
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    /// Returns shared reference to array of triangles.
    pub fn triangles(&self) -> &[TriangleDefinition] {
        &self.triangles
    }
}
