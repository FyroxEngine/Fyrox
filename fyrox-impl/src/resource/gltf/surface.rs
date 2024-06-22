use crate::asset::core::math::TriangleDefinition;
use crate::core::algebra::{Vector2, Vector3, Vector4};
use crate::core::math::Vector3Ext;
use crate::fxhash::FxHashMap;
use crate::scene::mesh;
use crate::scene::mesh::buffer::{
    self, TriangleBuffer, ValidationError, VertexAttributeUsage, VertexBuffer, VertexReadTrait,
    VertexTrait,
};
use crate::scene::mesh::surface::{InputBlendShapeData, SurfaceData};
use crate::scene::mesh::vertex::{AnimatedVertex, SimpleVertex, StaticVertex};
use gltf::buffer::Buffer;
use gltf::mesh::util::ReadJoints;
use gltf::mesh::Mode;
use gltf::mesh::Semantic;
use gltf::Primitive;
use half::f16;

use std::num::TryFromIntError;

/// This type represents any error that may occur while importing mesh data from glTF.
#[derive(Debug)]
pub enum SurfaceDataError {
    /// The mesh data had more vertex positions than some other vertex attributes.
    /// For example, there might be positions for 10 vertices but normals for only 9 vertices.
    CountMismatch,
    /// The mesh vertex data in the glTF files does not include position vectors.
    MissingPosition,
    /// The mesh vertex data in the glTF files does not include normal vectors.
    MissingNormal,
    /// The mesh vertex data in the glTF files does not include UV coordinates.
    MissingTexCoords,
    /// The mesh vertex data in the glTF files does not include bone weight values.
    MissingBoneWeight,
    /// The mesh vertex data in the glTF files does not include bone index values.
    MissingBoneIndex,
    /// Bone indices in glTF format can be stored as u8 or u16, but Fyrox only supports
    /// bone indices in u8. This error is produced if an index is found which does not fit
    /// into u8.
    InvalidBoneIndex,
    /// The glTF format includes options for drawing points and lines, but this module
    /// only supports drawing triangles.
    InvalidMode,
    /// An internal error in a glTF file. The glTF format uses indices to allow one
    /// resource to reference another within the same file. This error indicates that
    /// one of those indices was out-of-bounds. This should never happen.
    InvalidIndex,
    /// An error in converting u32 to usize, or from usize to u32.
    Int(TryFromIntError),
    /// Depending on the geometry type, certain numbers of vertices are errors.
    /// For example, if the geometry is a list of triangles, then the number of vertices
    /// needs to be a multiple of three. This error indicates a glTF file had the wrong
    /// number of vertices in some mesh.
    InvalidVertexCount(GeometryType, u32),
    /// An error may occur while constructing a mesh buffer.
    Validation(ValidationError),
    /// An error may occur while writing mesh data.
    Fetch(buffer::VertexFetchError),
}

impl From<ValidationError> for SurfaceDataError {
    fn from(error: ValidationError) -> Self {
        SurfaceDataError::Validation(error)
    }
}

impl From<TryFromIntError> for SurfaceDataError {
    fn from(error: TryFromIntError) -> Self {
        SurfaceDataError::Int(error)
    }
}

impl From<buffer::VertexFetchError> for SurfaceDataError {
    fn from(error: buffer::VertexFetchError) -> Self {
        SurfaceDataError::Fetch(error)
    }
}

#[derive(Debug)]
pub enum GeometryType {
    Triangles,
    TriangleStrip,
    TriangleFan,
}

#[derive(Debug, Default, Clone)]
pub struct BlendShapeInfo {
    pub name: String,
    pub default_weight: f32,
}

#[derive(Debug, Default, Clone)]
pub struct BlendShapeInfoContainer {
    names: Vec<String>,
    weights: Vec<f32>,
}

impl BlendShapeInfoContainer {
    pub fn new(names: Vec<String>, weights: Vec<f32>) -> Self {
        BlendShapeInfoContainer { names, weights }
    }
    pub fn get(&self, index: usize) -> BlendShapeInfo {
        BlendShapeInfo {
            name: self
                .names
                .get(index)
                .cloned()
                .unwrap_or_else(|| index.to_string()),
            default_weight: self.weights.get(index).cloned().unwrap_or(0.0),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct GeometryStatistics {
    pub min_edge_length_squared: f32,
    pub repeated_index_count: usize,
}

impl GeometryStatistics {
    pub fn update_length(&mut self, len: f32) {
        if len < self.min_edge_length_squared {
            self.min_edge_length_squared = len;
        }
    }
    pub fn min_edge_length(&self) -> f32 {
        self.min_edge_length_squared.sqrt()
    }
}

impl Default for GeometryStatistics {
    fn default() -> Self {
        GeometryStatistics {
            min_edge_length_squared: f32::INFINITY,
            repeated_index_count: 0,
        }
    }
}

type Result<T> = std::result::Result<T, SurfaceDataError>;
const DEFAULT_TANGENT: Vector4<f32> = Vector4::new(0.0, 0.0, 0.0, 0.0);

#[derive(Debug)]
enum IndexData {
    Buffer(Vec<u32>),
    Direct(u32),
}

impl IndexData {
    fn get_tri(&self, a: u32, b: u32, c: u32) -> Result<[u32; 3]> {
        Ok([self.get(a)?, self.get(b)?, self.get(c)?])
    }
    fn get(&self, source_index: u32) -> Result<u32> {
        match self {
            IndexData::Buffer(data) => Ok(*data
                .get(usize::try_from(source_index)?)
                .ok_or(SurfaceDataError::InvalidIndex)?),
            IndexData::Direct(size) if source_index < *size => Ok(source_index),
            _ => Err(SurfaceDataError::InvalidIndex),
        }
    }
    fn len(&self) -> Result<u32> {
        match self {
            IndexData::Buffer(data) => Ok(u32::try_from(data.len())?),
            IndexData::Direct(size) => Ok(*size),
        }
    }
}

pub fn build_surface_data(
    primitive: &Primitive,
    morph_info: &BlendShapeInfoContainer,
    buffers: &[Vec<u8>],
    stats: &mut GeometryStatistics,
) -> Result<Option<SurfaceData>> {
    match primitive.mode() {
        Mode::Points => {
            return Ok(None);
        }
        Mode::Lines => {
            return Ok(None);
        }
        Mode::LineLoop => {
            return Ok(None);
        }
        Mode::LineStrip => {
            return Ok(None);
        }
        Mode::Triangles => (),
        Mode::TriangleStrip => (),
        Mode::TriangleFan => (),
    }
    let vs: VertexBuffer = build_vertex_data(primitive, buffers)?;
    let tris: TriangleBuffer = build_triangle_data(primitive, vs.vertex_count(), buffers)?;
    #[cfg(feature = "mesh_analysis")]
    update_statistics(&vs, &tris, stats)?;
    let morphs: Vec<InputBlendShapeData> = build_morph_data(primitive, morph_info, buffers)?;
    let mut surf = if !morphs.is_empty() {
        let shapes = mesh::surface::BlendShapesContainer::from_lists(&vs, morphs.as_slice());
        let mut surf = SurfaceData::new(vs, tris);
        surf.blend_shapes_container = Some(shapes);
        surf
    } else {
        SurfaceData::new(vs, tris)
    };
    let has_tex = primitive.get(&Semantic::TexCoords(0)).is_some();
    let has_norm = primitive.get(&Semantic::Normals).is_some();
    let has_tang = primitive.get(&Semantic::Tangents).is_some();
    if has_tex && !has_norm {
        surf.calculate_normals()?;
        surf.calculate_tangents()?;
    } else if has_tex && has_norm && !has_tang {
        surf.calculate_tangents()?;
    }
    Ok(Some(surf))
}

#[cfg(feature = "mesh_analysis")]
fn update_statistics(
    vs: &VertexBuffer,
    tris: &TriangleBuffer,
    stats: &mut GeometryStatistics,
) -> Result<()> {
    for tri in tris.iter() {
        if tri[0] == tri[1] || tri[1] == tri[2] || tri[0] == tri[2] {
            stats.repeated_index_count += 1;
        }
        stats.update_length(edge_length_squared(tri[0], tri[1], vs)?);
        stats.update_length(edge_length_squared(tri[1], tri[2], vs)?);
        stats.update_length(edge_length_squared(tri[0], tri[2], vs)?);
    }
    Ok(())
}

fn edge_length_squared(a: u32, b: u32, vs: &VertexBuffer) -> Result<f32> {
    let a = usize::try_from(a)?;
    let b = usize::try_from(b)?;
    let a = vs.get(a).ok_or(SurfaceDataError::InvalidIndex)?;
    let b = vs.get(b).ok_or(SurfaceDataError::InvalidIndex)?;
    let a = a.read_3_f32(VertexAttributeUsage::Position)?;
    let b = b.read_3_f32(VertexAttributeUsage::Position)?;
    Ok(a.sqr_distance(&b))
}

fn build_morph_data(
    primitive: &Primitive,
    morph_info: &BlendShapeInfoContainer,
    buffers: &[Vec<u8>],
) -> Result<Vec<InputBlendShapeData>> {
    #[cfg(feature = "gltf_blend_shapes")]
    return inner_build_morph_data(primitive, morph_info, buffers);
    #[cfg(not(feature = "gltf_blend_shapes"))]
    return Ok(Vec::new());
}
#[cfg(feature = "gltf_blend_shapes")]
fn inner_build_morph_data(
    primitive: &Primitive,
    morph_info: &BlendShapeInfoContainer,
    buffers: &[Vec<u8>],
) -> Result<Vec<InputBlendShapeData>> {
    let reader = primitive.reader(|buf: Buffer| buffers.get(buf.index()).map(Vec::as_slice));
    let reader = reader.read_morph_targets();
    let mut result: Vec<InputBlendShapeData> = Vec::new();
    for (i, (pos, norm, tang)) in reader.enumerate() {
        let info = morph_info.get(i);
        let positions = if let Some(iter) = pos {
            iter_to_map(iter)
        } else {
            FxHashMap::default()
        };
        let normals = if let Some(iter) = norm {
            iter_to_map(iter)
        } else {
            FxHashMap::default()
        };
        let tangents = if let Some(iter) = tang {
            iter_to_map(iter)
        } else {
            FxHashMap::default()
        };
        result.push(InputBlendShapeData {
            default_weight: info.default_weight,
            name: info.name,
            positions,
            normals,
            tangents,
        });
    }
    Ok(result)
}

fn iter_to_map<I>(iter: I) -> FxHashMap<u32, Vector3<f16>>
where
    I: Iterator<Item = [f32; 3]>,
{
    let mut map: FxHashMap<u32, Vector3<f16>> = FxHashMap::default();
    for (i, v) in iter.enumerate() {
        if v == [0.0; 3] {
            continue;
        }
        if let Ok(index) = u32::try_from(i) {
            let v: [f16; 3] = v.map(f16::from_f32);
            map.insert(index, Vector3::<f16>::from(v));
        }
    }
    map
}

fn build_vertex_data(primitive: &Primitive, buffers: &[Vec<u8>]) -> Result<VertexBuffer> {
    build_vertex_data_with(primitive, |buf: Buffer| {
        buffers.get(buf.index()).map(Vec::as_slice)
    })
}

fn build_triangle_data(
    primitive: &Primitive,
    vertex_count: u32,
    buffers: &[Vec<u8>],
) -> Result<TriangleBuffer> {
    let reader = primitive.reader(|buf: Buffer| buffers.get(buf.index()).map(Vec::as_slice));
    let index_data = match reader.read_indices() {
        Some(index_reader) => IndexData::Buffer(index_reader.into_u32().collect()),
        None => IndexData::Direct(vertex_count),
    };
    let tris: Vec<TriangleDefinition> = (match primitive.mode() {
        Mode::Points => Err(SurfaceDataError::InvalidMode),
        Mode::Lines => Err(SurfaceDataError::InvalidMode),
        Mode::LineLoop => Err(SurfaceDataError::InvalidMode),
        Mode::LineStrip => Err(SurfaceDataError::InvalidMode),
        Mode::Triangles => build_triangles(&index_data),
        Mode::TriangleStrip => build_triangle_strip(&index_data),
        Mode::TriangleFan => build_triangle_fan(&index_data),
    })?;
    Ok(TriangleBuffer::new(tris))
}

fn build_triangles(data: &IndexData) -> Result<Vec<TriangleDefinition>> {
    let vertex_count = data.len()?;
    if vertex_count == 0 || vertex_count % 3 != 0 {
        return Err(SurfaceDataError::InvalidVertexCount(
            GeometryType::Triangles,
            vertex_count,
        ));
    }
    let tri_count: u32 = vertex_count / 3;
    let mut tris: Vec<TriangleDefinition> = Vec::with_capacity(tri_count as usize);
    for i in 0..tri_count {
        let v: u32 = i * 3;
        tris.push(TriangleDefinition(data.get_tri(v, v + 1, v + 2)?));
    }
    Ok(tris)
}
fn build_triangle_strip(data: &IndexData) -> Result<Vec<TriangleDefinition>> {
    let vertex_count = data.len()?;
    if vertex_count < 3 {
        return Err(SurfaceDataError::InvalidVertexCount(
            GeometryType::TriangleStrip,
            vertex_count,
        ));
    }
    let tri_count: u32 = vertex_count - 2;
    let mut tris: Vec<TriangleDefinition> = Vec::with_capacity(tri_count as usize);
    for i in 0..tri_count {
        let odd = i % 2;
        tris.push(TriangleDefinition(data.get_tri(
            i,
            i + 1 + odd,
            i + 2 - odd,
        )?));
    }
    Ok(tris)
}
fn build_triangle_fan(data: &IndexData) -> Result<Vec<TriangleDefinition>> {
    let vertex_count = data.len()?;
    if vertex_count < 3 {
        return Err(SurfaceDataError::InvalidVertexCount(
            GeometryType::TriangleFan,
            vertex_count,
        ));
    }
    let tri_count: u32 = vertex_count - 2;
    let mut tris: Vec<TriangleDefinition> = Vec::with_capacity(tri_count as usize);
    for i in 0..tri_count {
        tris.push(TriangleDefinition(data.get_tri(i + 1, i + 2, 0)?));
    }
    Ok(tris)
}

fn build_vertex_data_with<'a, 's, F>(
    primitive: &'a Primitive,
    get_buffer_data: F,
) -> Result<VertexBuffer>
where
    F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
{
    let reader = primitive.reader(get_buffer_data.clone());
    if reader.read_weights(0).is_some() {
        let vs: Vec<AnimatedVertex> = AnimatedVertex::convert(primitive, get_buffer_data)?;
        Ok(VertexBuffer::new(vs.len(), vs)?)
    } else if reader.read_normals().is_some() {
        let vs: Vec<StaticVertex> = StaticVertex::convert(primitive, get_buffer_data)?;
        Ok(VertexBuffer::new(vs.len(), vs)?)
    } else {
        let vs: Vec<SimpleVertex> = SimpleVertex::convert(primitive, get_buffer_data)?;
        Ok(VertexBuffer::new(vs.len(), vs)?)
    }
}

trait GltfVertexConvert: VertexTrait {
    fn convert<'a, 's, F>(primitive: &'a Primitive, get_buffer_data: F) -> Result<Vec<Self>>
    where
        F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>;
}

impl GltfVertexConvert for SimpleVertex {
    fn convert<'a, 's, F>(primitive: &'a Primitive, get_buffer_data: F) -> Result<Vec<Self>>
    where
        F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
    {
        let reader = primitive.reader(get_buffer_data);
        if let Some(iter) = reader.read_positions() {
            Ok(iter
                .map(Vector3::from)
                .map(|v| SimpleVertex { position: v })
                .collect())
        } else {
            Err(SurfaceDataError::MissingPosition)
        }
    }
}

impl GltfVertexConvert for StaticVertex {
    fn convert<'a, 's, F>(primitive: &'a Primitive, get_buffer_data: F) -> Result<Vec<Self>>
    where
        F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
    {
        let reader = primitive.reader(get_buffer_data);
        let pos_iter = reader
            .read_positions()
            .ok_or(SurfaceDataError::MissingPosition)?;
        let mut norm_iter = reader
            .read_normals()
            .ok_or(SurfaceDataError::MissingNormal)?;
        let mut tang_iter = reader.read_tangents();
        let mut uv_iter = reader
            .read_tex_coords(0)
            .ok_or(SurfaceDataError::MissingTexCoords)?
            .into_f32();
        let mut result: Vec<StaticVertex> = Vec::with_capacity(pos_iter.len());
        for pos in pos_iter {
            let pos: Vector3<f32> = Vector3::from(pos);
            let norm: Option<Vector3<f32>> = norm_iter.next().map(Vector3::from);
            let uv: Option<Vector2<f32>> = uv_iter.next().map(Vector2::from);
            let tang: Option<Vector4<f32>> = if let Some(iter) = tang_iter.as_mut() {
                iter.next().map(Vector4::from)
            } else {
                Some(DEFAULT_TANGENT)
            };
            if let (Some(normal), Some(tex_coord), Some(tangent)) = (norm, uv, tang) {
                result.push(StaticVertex {
                    position: pos,
                    normal,
                    tex_coord,
                    tangent,
                });
            } else {
                return Err(SurfaceDataError::CountMismatch);
            }
        }
        Ok(result)
    }
}

impl GltfVertexConvert for AnimatedVertex {
    fn convert<'a, 's, F>(primitive: &'a Primitive, get_buffer_data: F) -> Result<Vec<Self>>
    where
        F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
    {
        let reader = primitive.reader(get_buffer_data);
        let pos_iter = reader
            .read_positions()
            .ok_or(SurfaceDataError::MissingPosition)?;
        let mut norm_iter = reader
            .read_normals()
            .ok_or(SurfaceDataError::MissingNormal)?;
        let mut tang_iter = reader.read_tangents();
        let mut uv_iter = reader
            .read_tex_coords(0)
            .ok_or(SurfaceDataError::MissingTexCoords)?
            .into_f32();
        let mut wgt_iter = reader
            .read_weights(0)
            .ok_or(SurfaceDataError::MissingBoneWeight)?
            .into_f32();
        let mut jnt_iter = reader
            .read_joints(0)
            .ok_or(SurfaceDataError::MissingBoneIndex)?;
        let mut result: Vec<AnimatedVertex> = Vec::with_capacity(pos_iter.len());
        for pos in pos_iter {
            let pos: Vector3<f32> = Vector3::from(pos);
            let norm: Option<Vector3<f32>> = norm_iter.next().map(Vector3::from);
            let uv: Option<Vector2<f32>> = uv_iter.next().map(Vector2::from);
            let bone_weights: Option<[f32; 4]> = wgt_iter.next();
            let bone_indices: Option<[u8; 4]> = read_valid_index(&mut jnt_iter)?;
            let tang: Option<Vector4<f32>> = if let Some(iter) = tang_iter.as_mut() {
                iter.next().map(Vector4::from)
            } else {
                Some(DEFAULT_TANGENT)
            };
            if let (
                Some(normal),
                Some(tex_coord),
                Some(tangent),
                Some(bone_weights),
                Some(bone_indices),
            ) = (norm, uv, tang, bone_weights, bone_indices)
            {
                result.push(AnimatedVertex {
                    position: pos,
                    normal,
                    tex_coord,
                    tangent,
                    bone_weights,
                    bone_indices,
                });
            } else {
                return Err(SurfaceDataError::CountMismatch);
            }
        }
        Ok(result)
    }
}

fn read_valid_index(reader: &mut ReadJoints) -> Result<Option<[u8; 4]>> {
    match reader {
        ReadJoints::U8(iter) => Ok(iter.next()),
        ReadJoints::U16(iter) => {
            if let Some(value) = iter.next() {
                let mut result: [u8; 4] = [0; 4];
                for (i, v) in value.iter().enumerate() {
                    result[i] = u8::try_from(*v).or(Err(SurfaceDataError::InvalidBoneIndex))?;
                }
                Ok(Some(result))
            } else {
                Ok(None)
            }
        }
    }
}
