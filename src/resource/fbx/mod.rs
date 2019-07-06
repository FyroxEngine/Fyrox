use std::{path::{PathBuf, Path}, fs::File, io::Read, collections::HashMap, time::Instant};
use crate::utils::pool::*;
use crate::math::{vec4::*, vec3::*, vec2::*, mat4::*, quat::*, triangulator::*};
use crate::scene::{*, node::*};
use crate::renderer::surface::{SurfaceSharedData, Surface, Vertex};
use crate::engine::State;

pub enum FbxAttribute {
    Double(f64),
    Float(f32),
    Integer(i32),
    Long(i64),
    Bool(bool),
    String(String), // ASCII Fbx always have every attribute in string form
}

const FBX_TIME_UNIT: f64 = 1.0 / 46186158000.0;

impl FbxAttribute {
    pub fn as_i32(&self) -> Result<i32, String> {
        match self {
            FbxAttribute::Double(val) => Ok(*val as i32),
            FbxAttribute::Float(val) => Ok(*val as i32),
            FbxAttribute::Integer(val) => Ok(*val),
            FbxAttribute::Long(val) => Ok(*val as i32),
            FbxAttribute::Bool(val) => Ok(*val as i32),
            FbxAttribute::String(val) => {
                match lexical::try_parse::<i32, _>(val.as_str()) {
                    Ok(i) => Ok(i),
                    Err(_) => Err(format!("Unable to convert string {} to i32", val))
                }
            }
        }
    }

    pub fn as_i64(&self) -> Result<i64, String> {
        match self {
            FbxAttribute::Double(val) => Ok(*val as i64),
            FbxAttribute::Float(val) => Ok(*val as i64),
            FbxAttribute::Integer(val) => Ok(*val as i64),
            FbxAttribute::Long(val) => Ok(*val as i64),
            FbxAttribute::Bool(val) => Ok(*val as i64),
            FbxAttribute::String(val) => {
                match lexical::try_parse::<i64, _>(val.as_str()) {
                    Ok(i) => Ok(i),
                    Err(_) => Err(format!("Unable to convert string {} to i64", val))
                }
            }
        }
    }

    pub fn as_f64(&self) -> Result<f64, String> {
        match self {
            FbxAttribute::Double(val) => Ok(*val),
            FbxAttribute::Float(val) => Ok(*val as f64),
            FbxAttribute::Integer(val) => Ok(*val as f64),
            FbxAttribute::Long(val) => Ok(*val as f64),
            FbxAttribute::Bool(val) => Ok((*val as i64) as f64),
            FbxAttribute::String(val) => {
                match lexical::try_parse_lossy::<f64, _>(val.as_str()) {
                    Ok(i) => Ok(i),
                    Err(_) => Err(format!("Unable to convert string {} to f64", val))
                }
            }
        }
    }

    pub fn as_f32(&self) -> Result<f32, String> {
        match self {
            FbxAttribute::Double(val) => Ok(*val as f32),
            FbxAttribute::Float(val) => Ok(*val),
            FbxAttribute::Integer(val) => Ok(*val as f32),
            FbxAttribute::Long(val) => Ok(*val as f32),
            FbxAttribute::Bool(val) => Ok((*val as i32) as f32),
            FbxAttribute::String(val) => {
                match lexical::try_parse_lossy::<f32, _>(val.as_str()) {
                    Ok(i) => Ok(i),
                    Err(_) => Err(format!("Unable to convert string {} to f32", val))
                }
            }
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            FbxAttribute::Double(val) => val.to_string(),
            FbxAttribute::Float(val) => val.to_string(),
            FbxAttribute::Integer(val) => val.to_string(),
            FbxAttribute::Long(val) => val.to_string(),
            FbxAttribute::Bool(val) => val.to_string(),
            FbxAttribute::String(val) => val.clone(),
        }
    }
}

struct FbxKeyframe {
    time: f32,
    position: Vec3,
    scale: Vec3,
    rotation: Vec3,
}

struct FbxTimeValuePair {
    time: f32,
    value: f32,
}

struct FbxSubDeformer {
    model: Handle<FbxComponent>,
    indices: Vec<i32>,
    weights: Vec<f32>,
    transform: Mat4,
}

impl FbxSubDeformer {
    fn read(sub_deformer_handle: &Handle<FbxNode>, nodes: &Pool<FbxNode>,
            stack: &mut Vec<Handle<FbxNode>>) -> Result<Self, String> {
        let indices_handle = find_node(nodes, stack, sub_deformer_handle, "Indexes")?;
        let indices = find_and_borrow_node(nodes, stack, &indices_handle, "a")?;

        let weights_handle = find_node(nodes, stack, sub_deformer_handle, "Weights")?;
        let weights = find_and_borrow_node(nodes, stack, &weights_handle, "a")?;

        let transform_handle = find_node(nodes, stack, sub_deformer_handle, "Transform")?;
        let transform_node = find_and_borrow_node(nodes, stack, &transform_handle, "a")?;

        if transform_node.attrib_count() != 16 {
            return Err(format!("FBX: Wrong transform size! Expect 16, got {}", transform_node.attrib_count()));
        }

        let mut transform = Mat4::identity();
        for i in 0..16 {
            transform.f[i] = transform_node.get_attrib(i)?.as_f64()? as f32;
        }

        let mut sub_deformer = FbxSubDeformer {
            model: Handle::none(),
            indices: Vec::with_capacity(indices.attrib_count()),
            weights: Vec::with_capacity(weights.attrib_count()),
            transform,
        };

        for i in 0..indices.attrib_count() {
            sub_deformer.indices.push(indices.get_attrib(i)?.as_i32()?);
        }

        for i in 0..weights.attrib_count() {
            sub_deformer.weights.push(weights.get_attrib(i)?.as_f64()? as f32);
        }

        Ok(sub_deformer)
    }
}

struct FbxTexture {
    filename: PathBuf,
}

impl FbxTexture {
    fn read(texture_node_hanle: &Handle<FbxNode>, nodes: &Pool<FbxNode>,
            stack: &mut Vec<Handle<FbxNode>>) -> Result<Self, String> {
        let mut texture = FbxTexture {
            filename: PathBuf::new()
        };
        if let Ok(relative_file_name_node) = find_and_borrow_node(nodes, stack, texture_node_hanle, "RelativeFilename") {
            let relative_filename = relative_file_name_node.get_attrib(0)?.as_string();
            let path = Path::new(relative_filename.as_str());
            if let Some(filename) = path.file_name() {
                texture.filename = PathBuf::from(filename);
            }
        }
        Ok(texture)
    }
}

struct FbxMaterial {
    diffuse_texture: Handle<FbxComponent>
}

impl FbxMaterial {
    fn read(_material_node_handle: &Handle<FbxNode>) -> Result<FbxMaterial, String> {
        Ok(FbxMaterial {
            diffuse_texture: Handle::none()
        })
    }
}

struct FbxDeformer {
    sub_deformers: Vec<Handle<FbxComponent>>
}

impl FbxDeformer {
    fn read(_sub_deformer_handle: &Handle<FbxNode>, _nodes: &Pool<FbxNode>,
            _stack: &mut Vec<Handle<FbxNode>>) -> Result<Self, String> {
        Ok(FbxDeformer {
            sub_deformers: Vec::new()
        })
    }
}

struct FbxAnimationCurve {
    keys: Vec<FbxTimeValuePair>
}

impl FbxAnimationCurve {
    pub fn read(curve_handle: &Handle<FbxNode>,
                nodes: &Pool<FbxNode>,
                stack: &mut Vec<Handle<FbxNode>>) -> Result<Self, String> {
        let key_time_handle = find_node(nodes, stack, curve_handle, "KeyTime")?;
        let key_time_array = find_and_borrow_node(nodes, stack, &key_time_handle, "a")?;

        let key_value_handle = find_node(nodes, stack, curve_handle, "KeyValueFloat")?;
        let key_value_array = find_and_borrow_node(nodes, stack, &key_value_handle, "a")?;

        if key_time_array.attrib_count() != key_value_array.attrib_count() {
            return Err(String::from("FBX: Animation curve contains wrong key data!"));
        }

        let mut curve = FbxAnimationCurve {
            keys: Vec::new()
        };

        for i in 0..key_value_array.attrib_count() {
            curve.keys.push(FbxTimeValuePair {
                time: ((key_time_array.get_attrib(i)?.as_i64()? as f64) * FBX_TIME_UNIT) as f32,
                value: key_value_array.get_attrib(i)?.as_f32()?,
            });
        }

        Ok(curve)
    }
}

enum FbxAnimationCurveNodeType {
    Unknown,
    Translation,
    Rotation,
    Scale,
}

struct FbxAnimationCurveNode {
    actual_type: FbxAnimationCurveNodeType,
    curves: Vec<Handle<FbxComponent>>,
}

impl FbxAnimationCurveNode {
    pub fn read(node_handle: &Handle<FbxNode>,
                nodes: &Pool<FbxNode>,
                _stack: &mut Vec<Handle<FbxNode>>) -> Result<Self, String> {
        match nodes.borrow(node_handle) {
            Some(node) =>
                Ok(FbxAnimationCurveNode {
                    actual_type: match node.get_attrib(0)?.as_string().as_str() {
                        "T" | "AnimCurveNode::T" => FbxAnimationCurveNodeType::Translation,
                        "R" | "AnimCurveNode::R" => FbxAnimationCurveNodeType::Rotation,
                        "S" | "AnimCurveNode::S" => FbxAnimationCurveNodeType::Scale,
                        _ => FbxAnimationCurveNodeType::Unknown
                    },
                    curves: Vec::new(),
                }),
            None => Err(String::from("Invalid FBX node handle!"))
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
enum FbxMapping {
    Unknown,
    ByPolygon,
    ByPolygonVertex,
    ByVertex,
    ByEdge,
    AllSame,
}

#[derive(Copy, Clone, PartialEq)]
enum FbxReference {
    Unknown,
    Direct,
    IndexToDirect,
}

struct FbxNode {
    name: String,
    attribs: Vec<FbxAttribute>,
    parent: Handle<FbxNode>,
    children: Vec<Handle<FbxNode>>,
}

impl FbxNode {
    fn get_vec3_at(&self, n: usize) -> Result<Vec3, String> {
        return Ok(Vec3 {
            x: self.get_attrib(n)?.as_f32()?,
            y: self.get_attrib(n + 1)?.as_f32()?,
            z: self.get_attrib(n + 2)?.as_f32()?,
        });
    }

    fn get_vec2_at(&self, n: usize) -> Result<Vec2, String> {
        return Ok(Vec2 {
            x: self.get_attrib(n)?.as_f32()?,
            y: self.get_attrib(n + 1)?.as_f32()?,
        });
    }

    fn get_attrib(&self, n: usize) -> Result<&FbxAttribute, String> {
        match self.attribs.get(n) {
            Some(attrib) => Ok(attrib),
            None => Err(format!("Unable to get {} attribute because index out of bounds.", n))
        }
    }

    fn attrib_count(&self) -> usize {
        self.attribs.len()
    }
}

struct FbxGeometry {
    vertices: Vec<Vec3>,
    indices: Vec<i32>,

    normals: Vec<Vec3>,
    normal_mapping: FbxMapping,
    normal_reference: FbxReference,

    tangents: Vec<Vec3>,
    tangent_mapping: FbxMapping,
    tangent_reference: FbxReference,

    binormals: Vec<Vec3>,
    binormal_mapping: FbxMapping,
    binormal_reference: FbxReference,

    uvs: Vec<Vec2>,
    uv_index: Vec<i32>,
    uv_mapping: FbxMapping,
    uv_reference: FbxReference,

    materials: Vec<i32>,
    material_mapping: FbxMapping,
    material_reference: FbxReference,

    deformers: Vec<Handle<FbxComponent>>,
}

impl FbxGeometry {
    pub fn read(geom_node_handle: &Handle<FbxNode>,
                nodes: &Pool<FbxNode>,
                stack: &mut Vec<Handle<FbxNode>>) -> Result<FbxGeometry, String> {
        let mut geom = FbxGeometry {
            vertices: Vec::new(),
            indices: Vec::new(),
            normals: Vec::new(),
            normal_mapping: FbxMapping::Unknown,
            normal_reference: FbxReference::Unknown,
            tangents: Vec::new(),
            tangent_mapping: FbxMapping::Unknown,
            tangent_reference: FbxReference::Unknown,
            binormals: Vec::new(),
            binormal_mapping: FbxMapping::Unknown,
            binormal_reference: FbxReference::Unknown,
            uvs: Vec::new(),
            uv_index: Vec::new(),
            uv_mapping: FbxMapping::Unknown,
            uv_reference: FbxReference::Unknown,
            materials: Vec::new(),
            material_mapping: FbxMapping::Unknown,
            material_reference: FbxReference::Unknown,
            deformers: Vec::new(),
        };

        // Read vertices
        let vertices_node_handle = find_node(nodes, stack, geom_node_handle, "Vertices")?;
        let vertices_array_node = find_and_borrow_node(nodes, stack, &vertices_node_handle, "a")?;
        let vertex_count = vertices_array_node.attrib_count() / 3;
        geom.vertices = Vec::with_capacity(vertex_count);
        for i in 0..vertex_count {
            geom.vertices.push(vertices_array_node.get_vec3_at(i * 3)?);
        }

        // Read faces
        let indices_node_handle = find_node(nodes, stack, geom_node_handle, "PolygonVertexIndex")?;
        let indices_array_node = find_and_borrow_node(nodes, stack, &indices_node_handle, "a")?;
        let index_count = indices_array_node.attrib_count();
        geom.indices = Vec::with_capacity(index_count);
        for i in 0..index_count {
            let index = indices_array_node.get_attrib(i)?.as_i32()?;
            geom.indices.push(index);
        }

        // Read normals (normals can not exist)
        if let Ok(layer_element_normal_node_handle) = find_node(nodes, stack, geom_node_handle, "LayerElementNormal") {
            let map_type_node = find_and_borrow_node(nodes, stack, &layer_element_normal_node_handle, "MappingInformationType")?;
            geom.normal_mapping = string_to_mapping(&map_type_node.get_attrib(0)?.as_string());

            let ref_type_node = find_and_borrow_node(nodes, stack, &layer_element_normal_node_handle, "ReferenceInformationType")?;
            geom.normal_reference = string_to_reference(&ref_type_node.get_attrib(0)?.as_string());

            let normals_node_handle = find_node(nodes, stack, &layer_element_normal_node_handle, "Normals")?;
            let normals_array_node = find_and_borrow_node(nodes, stack, &normals_node_handle, "a")?;
            let count = normals_array_node.attrib_count() / 3;
            for i in 0..count {
                geom.normals.push(normals_array_node.get_vec3_at(i * 3)?);
            }
        }

        // todo: read tangents

        // Read UVs
        if let Ok(layer_element_uv_node_handle) = find_node(nodes, stack, geom_node_handle, "LayerElementUV") {
            let map_type_node = find_and_borrow_node(nodes, stack, &layer_element_uv_node_handle, "MappingInformationType")?;
            geom.uv_mapping = string_to_mapping(&map_type_node.get_attrib(0)?.as_string());

            let ref_type_node = find_and_borrow_node(nodes, stack, &layer_element_uv_node_handle, "ReferenceInformationType")?;
            geom.uv_reference = string_to_reference(&ref_type_node.get_attrib(0)?.as_string());

            let uvs_node_handle = find_node(nodes, stack, &layer_element_uv_node_handle, "UV")?;
            let uvs_array_node = find_and_borrow_node(nodes, stack, &uvs_node_handle, "a")?;
            let count = uvs_array_node.attrib_count() / 2;
            for i in 0..count {
                let uv = uvs_array_node.get_vec2_at(i * 2)?;
                geom.uvs.push(Vec2 { x: uv.x, y: -uv.y }); // Hack
            }

            if geom.uv_reference == FbxReference::IndexToDirect {
                let uv_index_node = find_node(nodes, stack, &layer_element_uv_node_handle, "UVIndex")?;
                let uv_index_array_node = find_and_borrow_node(nodes, stack, &uv_index_node, "a")?;
                for i in 0..uv_index_array_node.attrib_count() {
                    geom.uv_index.push(uv_index_array_node.get_attrib(i)?.as_i32()?);
                }
            }
        }

        // Read materials
        if let Ok(layer_element_material_node_handle) = find_node(nodes, stack, geom_node_handle, "LayerElementMaterial") {
            let map_type_node = find_and_borrow_node(nodes, stack, &layer_element_material_node_handle, "MappingInformationType")?;
            geom.material_mapping = string_to_mapping(&map_type_node.get_attrib(0)?.as_string());

            let ref_type_node = find_and_borrow_node(nodes, stack, &layer_element_material_node_handle, "ReferenceInformationType")?;
            geom.material_reference = string_to_reference(&ref_type_node.get_attrib(0)?.as_string());

            let materials_node_handle = find_node(nodes, stack, &layer_element_material_node_handle, "Materials")?;
            let materials_array_node = find_and_borrow_node(nodes, stack, &materials_node_handle, "a")?;
            for i in 0..materials_array_node.attrib_count() {
                geom.materials.push(materials_array_node.get_attrib(i)?.as_i32()?);
            }
        }

        return Ok(geom);
    }
}

enum FbxLightType {
    Point,
    Directional,
    Spot,
    Area,
    Volume,
}

struct FbxColor {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

struct FbxLight {
    actual_type: FbxLightType,
    color: FbxColor,
    radius: f32,
    cone_angle: f32,
}

struct FbxModel {
    name: String,
    pre_rotation: Vec3,
    post_rotation: Vec3,
    rotation_offset: Vec3,
    rotation_pivot: Vec3,
    scaling_offset: Vec3,
    scaling_pivot: Vec3,
    rotation: Vec3,
    scale: Vec3,
    translation: Vec3,
    geometric_translation: Vec3,
    geometric_rotation: Vec3,
    geometric_scale: Vec3,
    inv_bind_transform: Mat4,
    geoms: Vec<Handle<FbxComponent>>,
    /// List of handles of materials
    materials: Vec<Handle<FbxComponent>>,
    /// List of handles of animation curve nodes
    animation_curve_nodes: Vec<Handle<FbxComponent>>,
    /// List of handles of children models
    children: Vec<Handle<FbxComponent>>,
    /// Handle to light component
    light: Handle<FbxComponent>,
}

impl FbxModel {
    pub fn read(model_node_handle: &Handle<FbxNode>,
                nodes: &Pool<FbxNode>,
                stack: &mut Vec<Handle<FbxNode>>) -> Result<FbxModel, String> {
        let mut name = String::from("Unnamed");

        let model_node = nodes.borrow(&model_node_handle).unwrap();
        if let Ok(name_attrib) = model_node.get_attrib(1) {
            name = name_attrib.as_string();
        }

        // Remove prefix
        if name.starts_with("Model::") {
            name = name.chars().skip(7).collect();
        }

        let mut model = FbxModel {
            name,
            pre_rotation: Vec3::zero(),
            post_rotation: Vec3::zero(),
            rotation_offset: Vec3::zero(),
            rotation_pivot: Vec3::zero(),
            scaling_offset: Vec3::zero(),
            scaling_pivot: Vec3::zero(),
            rotation: Vec3::zero(),
            scale: Vec3::unit(),
            translation: Vec3::zero(),
            geometric_translation: Vec3::zero(),
            geometric_rotation: Vec3::zero(),
            geometric_scale: Vec3::unit(),
            inv_bind_transform: Mat4::identity(),
            geoms: Vec::new(),
            materials: Vec::new(),
            animation_curve_nodes: Vec::new(),
            children: Vec::new(),
            light: Handle::none(),
        };

        let properties70_node_handle = find_node(nodes, stack, model_node_handle, "Properties70")?;
        let properties70_node = nodes.borrow(&properties70_node_handle).unwrap();
        for property_handle in properties70_node.children.iter() {
            let property_node = nodes.borrow(&property_handle).unwrap();
            let name_attrib = property_node.get_attrib(0)?;
            match name_attrib.as_string().as_str() {
                "Lcl Translation" => model.translation = property_node.get_vec3_at(4)?,
                "Lcl Rotation" => model.rotation = property_node.get_vec3_at(4)?,
                "Lcl Scaling" => model.scale = property_node.get_vec3_at(4)?,
                "PreRotation" => model.pre_rotation = property_node.get_vec3_at(4)?,
                "PostRotation" => model.post_rotation = property_node.get_vec3_at(4)?,
                "RotationOffset" => model.rotation_offset = property_node.get_vec3_at(4)?,
                "RotationPivot" => model.rotation_pivot = property_node.get_vec3_at(4)?,
                "ScalingOffset" => model.scaling_offset = property_node.get_vec3_at(4)?,
                "ScalingPivot" => model.scaling_pivot = property_node.get_vec3_at(4)?,
                "GeometricTranslation" => model.geometric_translation = property_node.get_vec3_at(4)?,
                "GeometricScaling" => model.geometric_scale = property_node.get_vec3_at(4)?,
                "GeometricRotation" => model.geometric_rotation = property_node.get_vec3_at(4)?,
                _ => () // Unused properties
            }
        }
        return Ok(model);
    }
}

#[derive(Copy, Clone, PartialEq)]
enum FbxComponentTypeId {
    Unknown,
    Deformer,
    SubDeformer,
    Texture,
    Light,
    Model,
    Material,
    AnimationCurveNode,
    AnimationCurve,
    Geometry,
}

enum FbxComponent {
    Deformer(FbxDeformer),
    SubDeformer(FbxSubDeformer),
    Texture(FbxTexture),
    Light(FbxLight),
    Model(FbxModel),
    Material(FbxMaterial),
    AnimationCurveNode(FbxAnimationCurveNode),
    AnimationCurve(FbxAnimationCurve),
    Geometry(FbxGeometry),
}

impl FbxComponent {
    fn type_id(&self) -> FbxComponentTypeId {
        match self {
            FbxComponent::Deformer(_) => FbxComponentTypeId::Deformer,
            FbxComponent::SubDeformer(_) => FbxComponentTypeId::SubDeformer,
            FbxComponent::Texture(_) => FbxComponentTypeId::Texture,
            FbxComponent::Light(_) => FbxComponentTypeId::Light,
            FbxComponent::Model(_) => FbxComponentTypeId::Model,
            FbxComponent::Material(_) => FbxComponentTypeId::Material,
            FbxComponent::AnimationCurveNode(_) => FbxComponentTypeId::AnimationCurveNode,
            FbxComponent::AnimationCurve(_) => FbxComponentTypeId::AnimationCurve,
            FbxComponent::Geometry(_) => FbxComponentTypeId::Geometry
        }
    }
}

pub struct Fbx {
    /// Every FBX DOM node lives in this pool, other code uses handles to
    /// borrow references to actual nodes.
    nodes: Pool<FbxNode>,
    /// Pool for FBX components, filled in "prepare" method
    component_pool: Pool<FbxComponent>,
    root: Handle<FbxNode>,
    /// Map used for fast look up of components by their fbx-indices
    index_to_component: HashMap<i64, Handle<FbxComponent>>,
    /// Actual list of created components
    components: Vec<Handle<FbxComponent>>,
}

/// Searches node by specified name and returns its handle if found
/// Uses provided stack to do depth search
fn find_node(pool: &Pool<FbxNode>, stack: &mut Vec<Handle<FbxNode>>, root: &Handle<FbxNode>, name: &str) -> Result<Handle<FbxNode>, String> {
    stack.clear();
    stack.push(root.clone());
    while let Some(handle) = stack.pop() {
        if let Some(node) = pool.borrow(&handle) {
            if node.name == name {
                return Ok(handle);
            }
            for child_handle in node.children.iter() {
                stack.push(child_handle.clone());
            }
        }
    }
    Err(format!("Unable to find {} node", name))
}

/// Searches node by specified name and borrows a reference to it
/// Uses provided stack to do depth search
fn find_and_borrow_node<'a>(pool: &'a Pool<FbxNode>, stack: &mut Vec<Handle<FbxNode>>, root: &Handle<FbxNode>, name: &str) -> Result<&'a FbxNode, String> {
    stack.clear();
    stack.push(root.clone());
    while let Some(handle) = stack.pop() {
        if let Some(node) = pool.borrow(&handle) {
            if node.name == name {
                return Ok(node);
            }
            for child_handle in node.children.iter() {
                stack.push(child_handle.clone());
            }
        }
    }
    Err(format!("Unable to find {} node", name))
}

/// Links child component with parent component so parent will know about child
fn link_child_with_parent_component(parent: &mut FbxComponent, child_handle: &Handle<FbxComponent>, child_type_id: FbxComponentTypeId) {
    match parent {
        // Link model with other components
        FbxComponent::Model(model) => {
            match child_type_id {
                FbxComponentTypeId::Geometry => model.geoms.push(child_handle.clone()),
                FbxComponentTypeId::Material => model.materials.push(child_handle.clone()),
                FbxComponentTypeId::AnimationCurveNode => model.animation_curve_nodes.push(child_handle.clone()),
                FbxComponentTypeId::Light => model.light = child_handle.clone(),
                FbxComponentTypeId::Model => model.children.push(child_handle.clone()),
                _ => ()
            }
        }
        // Link material with textures
        FbxComponent::Material(material) => {
            if child_type_id == FbxComponentTypeId::Texture {
                material.diffuse_texture = child_handle.clone();
            }
        }
        // Link animation curve node with animation curve
        FbxComponent::AnimationCurveNode(anim_curve_node) => {
            if child_type_id == FbxComponentTypeId::AnimationCurve {
                anim_curve_node.curves.push(child_handle.clone());
            }
        }
        // Link deformer with sub-deformers
        FbxComponent::Deformer(deformer) => {
            if child_type_id == FbxComponentTypeId::SubDeformer {
                deformer.sub_deformers.push(child_handle.clone());
            }
        }
        // Link geometry with deformers
        FbxComponent::Geometry(geometry) => {
            if child_type_id == FbxComponentTypeId::Deformer {
                geometry.deformers.push(child_handle.clone());
            }
        }
        // Link sub-deformer with model
        FbxComponent::SubDeformer(sub_deformer) => {
            if child_type_id == FbxComponentTypeId::Model {
                sub_deformer.model = child_handle.clone();
            }
        }
        // Ignore rest
        _ => ()
    }
}

fn read_ascii(path: &Path) -> Result<Fbx, String> {
    let mut nodes: Pool<FbxNode> = Pool::new();
    let root_handle = nodes.spawn(FbxNode {
        name: String::from("__ROOT__"),
        children: Vec::new(),
        parent: Handle::none(),
        attribs: Vec::new(),
    });
    let mut parent_handle: Handle<FbxNode> = root_handle.clone();
    let mut node_handle: Handle<FbxNode> = Handle::none();
    let mut buffer: Vec<u8> = Vec::new();
    let mut name: Vec<u8> = Vec::new();
    let mut value: Vec<u8> = Vec::new();
    if let Ok(ref mut file) = File::open(path) {
        let mut read_ptr: usize = 0;
        let mut file_content: Vec<u8> = Vec::with_capacity(file.metadata().unwrap().len() as usize);
        file.read_to_end(&mut file_content).unwrap();

        // Read line by line
        while read_ptr < file_content.len() {
            // Read line, trim spaces (but leave spaces in quotes)
            buffer.clear();

            let mut read_all = false;
            while read_ptr < file_content.len() {
                let symbol = unsafe { *file_content.get_unchecked(read_ptr) };
                read_ptr += 1;
                if symbol == b'\n' {
                    break;
                } else if symbol == b'"' {
                    read_all = !read_all;
                } else if read_all || !symbol.is_ascii_whitespace() {
                    buffer.push(symbol);
                }
            }

            // Ignore comments and empty lines
            if buffer.len() == 0 || buffer[0] == b';' {
                continue;
            }

            // Parse string
            let mut read_value = false;
            name.clear();
            for i in 0..buffer.len() {
                let symbol = unsafe { *buffer.get_unchecked(i as usize) };
                if i == 0 && (symbol == b'-' || symbol.is_ascii_digit()) {
                    read_value = true;
                }
                if symbol == b':' && !read_value {
                    read_value = true;
                    if let Ok(name_copy) = String::from_utf8(name.clone()) {
                        let node = FbxNode {
                            name: name_copy,
                            attribs: Vec::new(),
                            parent: parent_handle.clone(),
                            children: Vec::new(),
                        };
                        node_handle = nodes.spawn(node);
                        name.clear();
                        if let Some(parent) = nodes.borrow_mut(&parent_handle) {
                            parent.children.push(node_handle.clone());
                        }
                    } else {
                        return Err(String::from("FBX: Node name is not valid utf8 string!"));
                    }
                } else if symbol == b'{' {
                    // Enter child scope
                    parent_handle = node_handle.clone();
                    // Commit attribute if we have one
                    if value.len() > 0 {
                        if let Some(node) = nodes.borrow_mut(&node_handle) {
                            if let Ok(string_value) = String::from_utf8(value.clone()) {
                                let attrib = FbxAttribute::String(string_value);
                                node.attribs.push(attrib);
                            } else {
                                return Err(String::from("FBX: Attribute is not valid utf8 string!"));
                            }
                        } else {
                            return Err(String::from("FBX: Failed to fetch node by handle when entering child scope"));
                        }
                        value.clear();
                    }
                } else if symbol == b'}' {
                    // Exit child scope
                    if let Some(parent) = nodes.borrow_mut(&parent_handle) {
                        parent_handle = parent.parent.clone();
                    }
                } else if symbol == b',' || (i == buffer.len() - 1) {
                    // Commit attribute
                    if symbol != b',' {
                        value.push(symbol);
                    }
                    if let Some(node) = nodes.borrow_mut(&node_handle) {
                        if let Ok(string_value) = String::from_utf8(value.clone()) {
                            let attrib = FbxAttribute::String(string_value);
                            node.attribs.push(attrib);
                        } else {
                            return Err(String::from("FBX: Attribute is not valid utf8 string!"));
                        }
                    } else {
                        return Err(String::from("FBX: Failed to fetch node by handle when committing attribute"));
                    }
                    value.clear();
                } else {
                    if !read_value {
                        name.push(symbol);
                    } else {
                        value.push(symbol);
                    }
                }
            }
        }
    }

    Ok(Fbx {
        nodes,
        root: root_handle,
        component_pool: Pool::new(),
        components: Vec::new(),
        index_to_component: HashMap::new(),
    })
}

fn string_to_mapping(value: &String) -> FbxMapping {
    match value.as_str() {
        "ByPolygon" => FbxMapping::ByPolygon,
        "ByPolygonVertex" => FbxMapping::ByPolygonVertex,
        "ByVertex" => FbxMapping::ByVertex,
        "ByVertice" => FbxMapping::ByVertex,
        "ByEdge" => FbxMapping::ByEdge,
        "AllSame" => FbxMapping::AllSame,
        _ => FbxMapping::Unknown
    }
}

fn string_to_reference(value: &String) -> FbxReference {
    match value.as_str() {
        "Direct" => FbxReference::Direct,
        "IndexToDirect" => FbxReference::IndexToDirect,
        "Index" => FbxReference::IndexToDirect,
        _ => FbxReference::Unknown
    }
}

/// Input angles in degrees
fn quat_from_euler(euler: Vec3) -> Quat {
    Quat::from_euler(
        Vec3::make(euler.x.to_radians(), euler.y.to_radians(), euler.z.to_radians()),
        RotationOrder::XYZ)
}

/// Fixes index that is used as indicator of end of a polygon
/// FBX stores array of indices like so 0,1,-3,... where -3
/// is actually index 2 but it xor'ed using -1.
fn fix_index(index: i32) -> usize {
    if index < 0 {
        (-index - 1) as usize
    } else {
        index as usize
    }
}

/// Triangulates polygon face if needed.
/// Returns number of processed indices.
fn prepare_next_face(geom: &FbxGeometry,
                     start: usize,
                     temp_vertices: &mut Vec<Vec3>,
                     out_triangles: &mut Vec<(usize, usize, usize)>,
                     out_relative_triangles: &mut Vec<(usize, usize, usize)>) -> usize {
    out_triangles.clear();
    out_relative_triangles.clear();

    // Find out how much vertices do we have per face.
    let mut vertex_per_face = 0;
    for i in start..geom.indices.len() {
        vertex_per_face += 1;
        if geom.indices[i] < 0 {
            break;
        }
    }

    if vertex_per_face == 3 {
        let a = fix_index(geom.indices[start]);
        let b = fix_index(geom.indices[start + 1]);
        let c = fix_index(geom.indices[start + 2]);

        // Ensure that we have valid indices here. Some exporters may fuck up indices
        // and they'll blow up loader.
        if a < geom.vertices.len() && b < geom.vertices.len() && c < geom.vertices.len() {
            // We have a triangle
            out_triangles.push((a, b, c));
            out_relative_triangles.push((0, 1, 2));
        }
    } else if vertex_per_face > 3 {
        // Triangulate a polygon. Triangulate it!
        temp_vertices.clear();
        for i in 0..vertex_per_face {
            temp_vertices.push(geom.vertices[fix_index(geom.indices[start + i])]);
        }
        triangulate(&temp_vertices.as_slice(), out_relative_triangles);
        for triangle in out_relative_triangles.iter() {
            out_triangles.push((
                fix_index(geom.indices[start + triangle.0]),
                fix_index(geom.indices[start + triangle.1]),
                fix_index(geom.indices[start + triangle.2])));
        }
    }

    return vertex_per_face;
}

fn convert_vertex(geom: &FbxGeometry,
                  mesh: &mut Mesh,
                  state: &mut State,
                  geometric_transform: &Mat4,
                  material_index: usize,
                  origin: usize,
                  index: usize,
                  relative_index: usize) {
    let position = geometric_transform.transform_vector(geom.vertices[index]);

    let normal = geometric_transform.transform_vector_normal(match geom.normal_mapping {
        FbxMapping::ByPolygonVertex => geom.normals[origin + relative_index],
        FbxMapping::ByVertex => geom.normals[index],
        _ => Vec3 { x: 0.0, y: 1.0, z: 0.0 }
    });

    let tangent = geometric_transform.transform_vector_normal(match geom.tangent_mapping {
        FbxMapping::ByPolygonVertex => geom.tangents[origin + relative_index],
        FbxMapping::ByVertex => geom.tangents[index],
        _ => Vec3 { x: 0.0, y: 1.0, z: 0.0 }
    });

    let uv = match geom.uv_mapping {
        FbxMapping::ByPolygonVertex => {
            match geom.uv_reference {
                FbxReference::Direct => geom.uvs[origin + relative_index],
                FbxReference::IndexToDirect => geom.uvs[geom.uv_index[origin + relative_index] as usize],
                _ => Vec2 { x: 0.0, y: 0.0 }
            }
        }
        _ => Vec2 { x: 0.0, y: 0.0 }
    };

    let material = match geom.material_mapping {
        FbxMapping::AllSame => geom.materials[0] as usize,
        FbxMapping::ByPolygon => geom.materials[material_index] as usize,
        _ => 0
    };

    let surface = mesh.get_surfaces_mut().get_mut(material).unwrap();
    let data_storage = state.get_surface_data_storage_mut();
    match data_storage.borrow_mut(surface.get_data_handle()) {
        Some(surface_data) => {
            surface_data.insert_vertex(Vertex {
                position,
                normal,
                tex_coord: uv,
                tangent: Vec4 { x: tangent.x, y: tangent.y, z: tangent.z, w: 1.0 },
                // FIXME
                bone_weights: Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 },
                bone_indices: [0, 0, 0, 0], // Correct indices will be calculated later
            });
        }
        None => println!("Unable to get surface data!")
    }
}

impl Fbx {
    /// Parses FBX DOM and filling internal lists to prepare
    /// for conversion to engine format
    fn prepare(&mut self) -> Result<(), String> {
        // Search-stack for internal routines
        let mut traversal_stack: Vec<Handle<FbxNode>> = Vec::new();

        // Check version
        let header_handle = find_node(&self.nodes, &mut traversal_stack, &self.root, "FBXHeaderExtension")?;
        let version = find_and_borrow_node(&self.nodes, &mut traversal_stack, &header_handle, "FBXVersion")?;
        let version = version.get_attrib(0)?.as_i32()?;
        if version < 7100 {
            return Err(format!("FBX: Unsupported {} version. Version must be >= 7100", version));
        }

        // Read objects
        let objects_node = find_and_borrow_node(&self.nodes, &mut traversal_stack, &self.root, "Objects")?;
        for object_handle in objects_node.children.iter() {
            let object = self.nodes.borrow(&object_handle).unwrap();
            let index = object.get_attrib(0)?.as_i64()?;
            let mut component_handle: Handle<FbxComponent> = Handle::none();
            match object.name.as_str() {
                "Geometry" => {
                    component_handle = self.component_pool.spawn(FbxComponent::Geometry(
                        FbxGeometry::read(object_handle, &self.nodes, &mut traversal_stack)?));
                }
                "Model" => {
                    component_handle = self.component_pool.spawn(FbxComponent::Model(
                        FbxModel::read(object_handle, &self.nodes, &mut traversal_stack)?));
                }
                "Material" => {
                    component_handle = self.component_pool.spawn(FbxComponent::Material(
                        FbxMaterial::read(object_handle)?));
                }
                "Texture" => {
                    component_handle = self.component_pool.spawn(FbxComponent::Texture(
                        FbxTexture::read(object_handle, &self.nodes, &mut traversal_stack)?));
                }
                "NodeAttribute" => {
                    //println!("reading a NodeAttribute");
                }
                "AnimationCurve" => {
                    component_handle = self.component_pool.spawn(FbxComponent::AnimationCurve(
                        FbxAnimationCurve::read(object_handle, &self.nodes, &mut traversal_stack)?));
                }
                "AnimationCurveNode" => {
                    component_handle = self.component_pool.spawn(FbxComponent::AnimationCurveNode(
                        FbxAnimationCurveNode::read(object_handle, &self.nodes, &mut traversal_stack)?));
                }
                "Deformer" => {
                    match object.get_attrib(2)?.as_string().as_str() {
                        "Cluster" => {
                            component_handle = self.component_pool.spawn(FbxComponent::SubDeformer(
                                FbxSubDeformer::read(object_handle, &self.nodes, &mut traversal_stack)?));
                        }
                        "Skin" => {
                            component_handle = self.component_pool.spawn(FbxComponent::Deformer(
                                FbxDeformer::read(object_handle, &self.nodes, &mut traversal_stack)?));
                        }
                        _ => ()
                    }
                }
                _ => ()
            }
            if !component_handle.is_none() {
                self.index_to_component.insert(index, component_handle.clone());
                self.components.push(component_handle.clone());
            }
        }

        // Read connections
        let connections_node = find_and_borrow_node(&self.nodes, &mut traversal_stack, &self.root, "Connections")?;
        for connection_handle in connections_node.children.iter() {
            let connection = self.nodes.borrow(&connection_handle).unwrap();
            let child_index = connection.get_attrib(1)?.as_i64()?;
            let parent_index = connection.get_attrib(2)?.as_i64()?;
            if let Some(parent_handle) = self.index_to_component.get(&parent_index) {
                if let Some(child_handle) = self.index_to_component.get(&child_index) {
                    let child_type_id = self.component_pool.borrow(child_handle).unwrap().type_id();
                    let parent = self.component_pool.borrow_mut(parent_handle).unwrap();
                    link_child_with_parent_component(parent, child_handle, child_type_id);
                }
            }
        }

        return Ok(());
    }

    fn convert_light(&self, light: &mut Light, fbx_light: &FbxLight) {
        light.set_color(Vec3::make(fbx_light.color.r as f32 / 255.0,
                                   fbx_light.color.g as f32 / 255.0,
                                   fbx_light.color.b as f32 / 255.0));
        light.set_radius(fbx_light.radius);
    }

    fn create_surfaces(&self,
                       mesh: &mut Mesh,
                       state: &mut State,
                       model: &FbxModel) {
        // Create surfaces per material
        if model.materials.is_empty() {
            mesh.add_surface(Surface::new(state.get_surface_data_storage_mut().spawn(SurfaceSharedData::new())));
        } else {
            for material_handle in model.materials.iter() {
                let mut surface = Surface::new(state.get_surface_data_storage_mut().spawn(SurfaceSharedData::new()));
                if let FbxComponent::Material(material) = self.component_pool.borrow(&material_handle).unwrap() {
                    if let Some(texture_handle) = self.component_pool.borrow(&material.diffuse_texture) {
                        if let FbxComponent::Texture(texture) = texture_handle {
                            if texture.filename.is_relative() {
                                let fullpath = state.get_resource_manager_mut().get_textures_path().join(&texture.filename);
                                surface.set_texture(state.request_resource(fullpath.as_path()));
                            }
                        }
                    }
                }
                mesh.add_surface(surface);
            }
        }
    }

    fn convert_mesh(&self,
                    mesh: &mut Mesh,
                    state: &mut State,
                    model: &FbxModel) {
        let geometric_transform = Mat4::translate(model.geometric_translation) *
            Mat4::from_quat(quat_from_euler(model.geometric_rotation)) *
            Mat4::scale(model.geometric_scale);

        let mut temp_vertices: Vec<Vec3> = Vec::new();
        let mut triangles: Vec<(usize, usize, usize)> = Vec::new();
        let mut relative_triangles: Vec<(usize, usize, usize)> = Vec::new();

        for geom_handle in &model.geoms {
            let geom_component = self.component_pool.borrow(&geom_handle).unwrap();
            if let FbxComponent::Geometry(geom) = geom_component {
                self.create_surfaces(mesh, state, model);

                let mut material_index = 0;
                let mut n = 0;
                while n < geom.indices.len() {
                    let origin = n;
                    n += prepare_next_face(geom, n, &mut temp_vertices, &mut triangles, &mut relative_triangles);
                    for i in 0..triangles.len() {
                        let triangle = &triangles[i];
                        let relative_triangle = &relative_triangles[i];

                        convert_vertex(geom, mesh, state, &geometric_transform, material_index, origin, triangle.0, relative_triangle.0);
                        convert_vertex(geom, mesh, state, &geometric_transform, material_index, origin, triangle.1, relative_triangle.1);
                        convert_vertex(geom, mesh, state, &geometric_transform, material_index, origin, triangle.2, relative_triangle.2);
                    }
                    if geom.material_mapping == FbxMapping::ByPolygon {
                        material_index += 1;
                    }
                }
            }
        }
    }

    fn convert_model(&self,
                     model: &FbxModel,
                     state: &mut State,
                     scene: &mut Scene) -> Result<Handle<Node>, String> {
        // Create node with of correct kind.
        let mut node =
            if model.geoms.len() != 0 {
                Node::new(NodeKind::Mesh(Mesh::default()))
            } else if !model.light.is_none() {
                Node::new(NodeKind::Light(Light::default()))
            } else {
                Node::new(NodeKind::Base)
            };

        node.set_name(model.name.clone());
        node.set_local_rotation(quat_from_euler(model.rotation));
        node.set_local_scale(model.scale);
        node.set_local_position(model.translation);
        node.set_post_rotation(quat_from_euler(model.post_rotation));
        node.set_pre_rotation(quat_from_euler(model.pre_rotation));
        node.set_rotation_offset(model.rotation_offset);
        node.set_rotation_pivot(model.rotation_pivot);
        node.set_scaling_offset(model.scaling_offset);
        node.set_scaling_pivot(model.scaling_pivot);

        match node.borrow_kind_mut() {
            NodeKind::Light(light) => {
                let fbx_light_component = self.component_pool.borrow(&model.light).unwrap();
                if let FbxComponent::Light(fbx_light) = fbx_light_component {
                    self.convert_light(light, fbx_light);
                }
            }
            NodeKind::Mesh(mesh) => {
                self.convert_mesh(mesh, state, model);
            }
            _ => ()
        }

        Ok(scene.add_node(node))
    }

    ///
    /// Converts FBX DOM to native engine representation.
    ///
    pub fn convert(&self,
                   state: &mut State,
                   scene: &mut Scene)
                   -> Result<Handle<Node>, String> {
        let root = scene.add_node(Node::new(NodeKind::Base));
        let mut fbx_model_to_node_map: HashMap<Handle<FbxComponent>, Handle<Node>> = HashMap::new();
        for component_handle in self.components.iter() {
            if let Some(component) = self.component_pool.borrow(&component_handle) {
                if let FbxComponent::Model(model) = component {
                    if let Ok(node) = self.convert_model(model, state, scene) {
                        scene.link_nodes(&node, &root);
                        fbx_model_to_node_map.insert(component_handle.clone(), node.clone());
                    }
                }
            }
        }
        // Link according to hierarchy
        for (fbx_model_handle, node_handle) in fbx_model_to_node_map.iter() {
            if let FbxComponent::Model(fbx_model) = self.component_pool.borrow(&fbx_model_handle).unwrap() {
                for fbx_child_handle in fbx_model.children.iter() {
                    if let Some(child_handle) = fbx_model_to_node_map.get(fbx_child_handle) {
                        scene.link_nodes(&child_handle, &node_handle);
                    }
                }
            }
        }
        scene.calculate_transforms();
        Ok(root)
    }

    /// TODO: format trait maybe?
    pub fn print(&mut self) {
        let mut stack: Vec<Handle<FbxNode>> = Vec::new();
        stack.push(self.root.clone());
        while let Some(handle) = stack.pop() {
            let node = self.nodes.borrow(&handle).unwrap();
            println!("{}", node.name);

            // Continue printing children
            for child_handle in node.children.iter() {
                stack.push(child_handle.clone());
            }
        }
    }
}

pub fn load_to_scene(scene: &mut Scene, state: &mut State, path: &Path)
                     -> Result<Handle<Node>, String> {
    let start_time = Instant::now();

    println!("FBX;Trying to load {:?}", path);

    let now = Instant::now();
    let ref mut fbx = read_ascii(path)?;
    println!("\tFBX: Parsing - {} ms", now.elapsed().as_millis());

    let now = Instant::now();
    fbx.prepare()?;
    println!("\tFBX: DOM Prepare - {} ms", now.elapsed().as_millis());

    let now = Instant::now();
    let result = fbx.convert(state, scene);
    println!("\tFBX: Conversion - {} ms", now.elapsed().as_millis());

    println!("FBX: {:?} loaded in {} ms", path, start_time.elapsed().as_millis());

    result
}