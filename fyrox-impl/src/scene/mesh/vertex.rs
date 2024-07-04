//! Contains most common vertex formats and their layouts.

use crate::{
    core::{
        algebra::{Vector2, Vector3, Vector4},
        visitor::{Visit, VisitResult, Visitor},
    },
    scene::mesh::buffer::{
        VertexAttributeDataType, VertexAttributeDescriptor, VertexAttributeUsage, VertexTrait,
    },
};
use bytemuck::{Pod, Zeroable};
use std::hash::{Hash, Hasher};

/// A vertex for static meshes.
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
#[repr(C)] // OpenGL expects this structure packed as in C
pub struct StaticVertex {
    /// Position of vertex in local coordinates.
    pub position: Vector3<f32>,
    /// Texture coordinates.
    pub tex_coord: Vector2<f32>,
    /// Normal in local coordinates.
    pub normal: Vector3<f32>,
    /// Tangent vector in local coordinates.
    pub tangent: Vector4<f32>,
}

impl StaticVertex {
    /// Creates new vertex from given position and texture coordinates.
    pub fn from_pos_uv(position: Vector3<f32>, tex_coord: Vector2<f32>) -> Self {
        Self {
            position,
            tex_coord,
            normal: Vector3::new(0.0, 1.0, 0.0),
            tangent: Vector4::default(),
        }
    }

    /// Creates new vertex from given position and texture coordinates.
    pub fn from_pos_uv_normal(
        position: Vector3<f32>,
        tex_coord: Vector2<f32>,
        normal: Vector3<f32>,
    ) -> Self {
        Self {
            position,
            tex_coord,
            normal,
            tangent: Vector4::default(),
        }
    }
}

impl VertexTrait for StaticVertex {
    fn layout() -> &'static [VertexAttributeDescriptor] {
        static LAYOUT: [VertexAttributeDescriptor; 4] = [
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Position,
                data_type: VertexAttributeDataType::F32,
                size: 3,
                divisor: 0,
                shader_location: 0,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::TexCoord0,
                data_type: VertexAttributeDataType::F32,
                size: 2,
                divisor: 0,
                shader_location: 1,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Normal,
                data_type: VertexAttributeDataType::F32,
                size: 3,
                divisor: 0,
                shader_location: 2,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Tangent,
                data_type: VertexAttributeDataType::F32,
                size: 4,
                divisor: 0,
                shader_location: 3,
                normalized: false,
            },
        ];
        &LAYOUT
    }
}

impl PartialEq for StaticVertex {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
            && self.tex_coord == other.tex_coord
            && self.normal == other.normal
            && self.tangent == other.tangent
    }
}

// This is safe because Vertex is tightly packed struct with C representation
// there is no padding bytes which may contain garbage data. This is strictly
// required because vertices will be directly passed on GPU.
impl Hash for StaticVertex {
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

/// A vertex for animated (via skinning) mesh.
#[derive(Copy, Clone, Debug, Default)]
#[repr(C)] // OpenGL expects this structure packed as in C
pub struct AnimatedVertex {
    /// Position of vertex in local coordinates.
    pub position: Vector3<f32>,
    /// Texture coordinates.
    pub tex_coord: Vector2<f32>,
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

impl VertexTrait for AnimatedVertex {
    fn layout() -> &'static [VertexAttributeDescriptor] {
        &[
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Position,
                data_type: VertexAttributeDataType::F32,
                size: 3,
                divisor: 0,
                shader_location: 0,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::TexCoord0,
                data_type: VertexAttributeDataType::F32,
                size: 2,
                divisor: 0,
                shader_location: 1,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Normal,
                data_type: VertexAttributeDataType::F32,
                size: 3,
                divisor: 0,
                shader_location: 2,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Tangent,
                data_type: VertexAttributeDataType::F32,
                size: 4,
                divisor: 0,
                shader_location: 3,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::BoneWeight,
                data_type: VertexAttributeDataType::F32,
                size: 4,
                divisor: 0,
                shader_location: 4,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::BoneIndices,
                data_type: VertexAttributeDataType::U8,
                size: 4,
                divisor: 0,
                shader_location: 5,
                normalized: false,
            },
        ]
    }
}

impl PartialEq for AnimatedVertex {
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
impl Hash for AnimatedVertex {
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

/// Simple vertex with position.
#[derive(Copy, Clone, Debug, Default)]
#[repr(C)] // OpenGL expects this structure packed as in C
pub struct SimpleVertex {
    /// Position of vertex in local coordinates.
    pub position: Vector3<f32>,
}

impl SimpleVertex {
    /// Creates a new simple vertex using given coordinates.
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: Vector3::new(x, y, z),
        }
    }
}

impl VertexTrait for SimpleVertex {
    fn layout() -> &'static [VertexAttributeDescriptor] {
        &[VertexAttributeDescriptor {
            usage: VertexAttributeUsage::Position,
            data_type: VertexAttributeDataType::F32,
            size: 3,
            divisor: 0,
            shader_location: 0,
            normalized: false,
        }]
    }
}

impl PartialEq for SimpleVertex {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
    }
}

// This is safe because Vertex is tightly packed struct with C representation
// there is no padding bytes which may contain garbage data. This is strictly
// required because vertices will be directly passed on GPU.
impl Hash for SimpleVertex {
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

/// Fat vertex used before customizable VertexBuffer was made.
/// It is used only to be able to load scenes in old formats to
/// keep backward compatibility.
#[derive(Copy, Clone, Debug, Default)]
#[repr(C)] // OpenGL expects this structure packed as in C
pub struct OldVertex {
    /// Position of vertex in local coordinates.
    pub position: Vector3<f32>,
    /// Texture coordinates.
    pub tex_coord: Vector2<f32>,
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
    /// Second texture coordinates.
    pub second_tex_coord: Vector2<f32>,
}

impl Visit for OldVertex {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.position.visit("Position", &mut region)?;
        self.tex_coord.visit("TexCoord", &mut region)?;
        self.second_tex_coord.visit("SecondTexCoord", &mut region)?;
        self.normal.visit("Normal", &mut region)?;
        self.tangent.visit("Tangent", &mut region)?;

        self.bone_weights[0].visit("Weight0", &mut region)?;
        self.bone_weights[1].visit("Weight1", &mut region)?;
        self.bone_weights[2].visit("Weight2", &mut region)?;
        self.bone_weights[3].visit("Weight3", &mut region)?;

        self.bone_indices[0].visit("BoneIndex0", &mut region)?;
        self.bone_indices[1].visit("BoneIndex1", &mut region)?;
        self.bone_indices[2].visit("BoneIndex2", &mut region)?;
        self.bone_indices[3].visit("BoneIndex3", &mut region)?;

        Ok(())
    }
}

impl VertexTrait for OldVertex {
    fn layout() -> &'static [VertexAttributeDescriptor] {
        &[
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Position,
                data_type: VertexAttributeDataType::F32,
                size: 3,
                divisor: 0,
                shader_location: 0,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::TexCoord0,
                data_type: VertexAttributeDataType::F32,
                size: 2,
                divisor: 0,
                shader_location: 1,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Normal,
                data_type: VertexAttributeDataType::F32,
                size: 3,
                divisor: 0,
                shader_location: 2,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Tangent,
                data_type: VertexAttributeDataType::F32,
                size: 4,
                divisor: 0,
                shader_location: 3,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::BoneWeight,
                data_type: VertexAttributeDataType::F32,
                size: 4,
                divisor: 0,
                shader_location: 4,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::BoneIndices,
                data_type: VertexAttributeDataType::U8,
                size: 4,
                divisor: 0,
                shader_location: 5,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::TexCoord1,
                data_type: VertexAttributeDataType::F32,
                size: 2,
                divisor: 0,
                shader_location: 6,
                normalized: false,
            },
        ]
    }
}
