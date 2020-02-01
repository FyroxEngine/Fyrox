mod fbx_ascii;
mod fbx_binary;
mod texture;
mod attribute;
mod geometry;
pub mod error;

use std::{
    path::Path,
    fs::File,
    io::{Read, Cursor},
    collections::{HashMap, HashSet},
    time::Instant,
    sync::{Arc, Mutex}
};
use crate::{
    resource::{
        texture::TextureKind,
        fbx::{
            texture::FbxTexture,
            attribute::FbxAttribute,
            error::FbxError,
        },
        fbx::geometry::FbxGeometry
    },
    animation::{
        AnimationContainer,
        Track,
        KeyFrame,
        Animation,
    },
    scene::{
        graph::Graph,
        Scene,
        node::Node,
        mesh::Mesh,
        light::{
            Light,
            LightKind,
            PointLight,
            SpotLight,
        },
        base::{Base, AsBase},
    },
    engine::resource_manager::ResourceManager,
    renderer::{
        surface::{
            SurfaceSharedData, Surface,
            Vertex, VertexWeightSet,
        }
    },
    core::{
        color::Color,
        pool::{Handle, Pool},
        math::{
            vec4::Vec4,
            vec3::Vec3,
            vec2::Vec2,
            mat4::Mat4,
            quat::{Quat, RotationOrder},
            triangulator::triangulate,
        },
    },
    utils::log::Log
};

// https://help.autodesk.com/view/FBX/2016/ENU/?guid=__cpp_ref_class_fbx_anim_curve_html
const FBX_TIME_UNIT: f64 = 1.0 / 46_186_158_000.0;

struct FbxTimeValuePair {
    time: f32,
    value: f32,
}

struct FbxSubDeformer {
    model: Handle<FbxComponent>,
    weights: Vec<(i32, f32)>,
    transform: Mat4,
}

impl FbxSubDeformer {
    fn read(sub_deformer_handle: Handle<FbxNode>, nodes: &Pool<FbxNode>) -> Result<Self, String> {
        // For some reason FBX exported from Blender can have sub deformer without weights and
        // indices. This is still valid and we have to return dummy in this case, instead of
        // error.
        if let Ok(indices_handle) = find_node(nodes, sub_deformer_handle, "Indexes") {
            let indices = find_and_borrow_node(nodes, indices_handle, "a")?;

            let weights_handle = find_node(nodes, sub_deformer_handle, "Weights")?;
            let weights = find_and_borrow_node(nodes, weights_handle, "a")?;

            let transform_handle = find_node(nodes, sub_deformer_handle, "Transform")?;
            let transform_node = find_and_borrow_node(nodes, transform_handle, "a")?;

            if transform_node.attrib_count() != 16 {
                return Err(format!("FBX: Wrong transform size! Expect 16, got {}", transform_node.attrib_count()));
            }

            if indices.attrib_count() != weights.attrib_count() {
                return Err(String::from("invalid sub deformer, weights count does not match index count"));
            }

            let mut transform = Mat4::IDENTITY;
            for i in 0..16 {
                transform.f[i] = transform_node.get_attrib(i)?.as_f64()? as f32;
            }

            let mut sub_deformer = FbxSubDeformer {
                model: Handle::NONE,
                weights: Vec::with_capacity(weights.attrib_count()),
                transform,
            };

            for i in 0..weights.attrib_count() {
                sub_deformer.weights.push((indices.get_attrib(i)?.as_i32()?, weights.get_attrib(i)?.as_f64()? as f32));
            }

            Ok(sub_deformer)
        } else {
            Ok(FbxSubDeformer {
                model: Handle::NONE,
                weights: Default::default(),
                transform: Default::default(),
            })
        }
    }
}

struct FbxMaterial {
    diffuse_texture: Handle<FbxComponent>
}

impl FbxMaterial {
    fn read(_material_node_handle: Handle<FbxNode>) -> Result<FbxMaterial, String> {
        Ok(FbxMaterial {
            diffuse_texture: Handle::NONE
        })
    }
}

struct FbxDeformer {
    sub_deformers: Vec<Handle<FbxComponent>>
}

impl FbxDeformer {
    fn read(_sub_deformer_handle: Handle<FbxNode>, _nodes: &Pool<FbxNode>) -> Result<Self, String> {
        Ok(FbxDeformer {
            sub_deformers: Vec::new()
        })
    }
}

struct FbxAnimationCurve {
    keys: Vec<FbxTimeValuePair>
}

impl FbxAnimationCurve {
    pub fn read(curve_handle: Handle<FbxNode>, nodes: &Pool<FbxNode>) -> Result<Self, String> {
        let key_time_handle = find_node(nodes, curve_handle, "KeyTime")?;
        let key_time_array = find_and_borrow_node(nodes, key_time_handle, "a")?;

        let key_value_handle = find_node(nodes, curve_handle, "KeyValueFloat")?;
        let key_value_array = find_and_borrow_node(nodes, key_value_handle, "a")?;

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

    fn eval(&self, time: f32) -> f32 {
        if self.keys.is_empty() {
            Log::writeln("FBX: Trying to evaluate curve with no keys!".to_owned());

            return 0.0;
        }

        if time <= self.keys[0].time {
            return self.keys[0].value;
        }

        if time >= self.keys[self.keys.len() - 1].time {
            return self.keys[self.keys.len() - 1].value;
        }

        // Do linear search for span
        for i in 0..(self.keys.len() - 1) {
            let cur = &self.keys[i];
            if cur.time >= time {
                let next = &self.keys[i + 1];

                // calculate interpolation coefficient
                let time_span = next.time - cur.time;
                let k = (time - cur.time) / time_span;

                // TODO: for now assume that we have only linear transitions
                let val_span = next.value - cur.value;
                return cur.value + k * val_span;
            }
        }

        // Edge-case when we are at the end of curve.
        self.keys.last().unwrap().value
    }
}

#[derive(PartialEq)]
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
    pub fn read(node_handle: Handle<FbxNode>, nodes: &Pool<FbxNode>) -> Result<Self, String> {
        let node = nodes.borrow(node_handle);
        Ok(FbxAnimationCurveNode {
            actual_type: match node.get_attrib(1)?.as_string().as_str() {
                "T" | "AnimCurveNode::T" => { FbxAnimationCurveNodeType::Translation }
                "R" | "AnimCurveNode::R" => { FbxAnimationCurveNodeType::Rotation }
                "S" | "AnimCurveNode::S" => { FbxAnimationCurveNodeType::Scale }
                _ => { FbxAnimationCurveNodeType::Unknown }
            },
            curves: Vec::new(),
        })
    }

    pub fn eval_vec3(&self, components: &Pool<FbxComponent>, time: f32) -> Vec3 {
        let x =
            if let FbxComponent::AnimationCurve(curve) = components.borrow(self.curves[0]) {
                curve.eval(time)
            } else {
                0.0
            };

        let y =
            if let FbxComponent::AnimationCurve(curve) = components.borrow(self.curves[1]) {
                curve.eval(time)
            } else {
                0.0
            };

        let z =
            if let FbxComponent::AnimationCurve(curve) = components.borrow(self.curves[2]) {
                curve.eval(time)
            } else {
                0.0
            };

        Vec3::new(x, y, z)
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

impl Default for FbxNode {
    fn default() -> Self {
        Self {
            name: "".to_string(),
            attribs: Vec::new(),
            parent: Default::default(),
            children: Vec::new(),
        }
    }
}

impl FbxNode {
    fn get_vec3_at(&self, n: usize) -> Result<Vec3, String> {
        Ok(Vec3 {
            x: self.get_attrib(n)?.as_f32()?,
            y: self.get_attrib(n + 1)?.as_f32()?,
            z: self.get_attrib(n + 2)?.as_f32()?,
        })
    }

    fn get_vec2_at(&self, n: usize) -> Result<Vec2, String> {
        Ok(Vec2 {
            x: self.get_attrib(n)?.as_f32()?,
            y: self.get_attrib(n + 1)?.as_f32()?,
        })
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

pub struct FbxContainer<T> {
    elements: Vec<T>,
    index: Vec<i32>,
    mapping: FbxMapping,
    reference: FbxReference,
}

impl<T> Default for FbxContainer<T> {
    fn default() -> Self {
        Self {
            index: Vec::new(),
            elements: Vec::new(),
            mapping: FbxMapping::Unknown,
            reference: FbxReference::Unknown,
        }
    }
}

enum FbxLightType {
    Point = 0,
    Directional = 1,
    Spot = 2,
    Area = 3,
    Volume = 4,
}

struct FbxLight {
    actual_type: FbxLightType,
    color: Color,
    radius: f32,
    cone_angle: f32,
}

impl FbxLight {
    pub fn read(light_node_handle: Handle<FbxNode>, nodes: &Pool<FbxNode>) -> Result<Self, String> {
        let mut light = Self {
            actual_type: FbxLightType::Point,
            color: Color::WHITE,
            radius: 10.0,
            cone_angle: std::f32::consts::PI,
        };

        let props = find_and_borrow_node(nodes, light_node_handle, "Properties70")?;
        for prop_handle in props.children.iter() {
            let prop = nodes.borrow(*prop_handle);
            match prop.get_attrib(0)?.as_string().as_str() {
                "DecayStart" => light.radius = prop.get_attrib(4)?.as_f64()? as f32,
                "Color" => {
                    let r = (prop.get_attrib(4)?.as_f64()? * 255.0) as u8;
                    let g = (prop.get_attrib(5)?.as_f64()? * 255.0) as u8;
                    let b = (prop.get_attrib(6)?.as_f64()? * 255.0) as u8;
                    light.color = Color::from_rgba(r, g, b, 255);
                }
                "HotSpot" => light.cone_angle = (prop.get_attrib(4)?.as_f64()? as f32).to_degrees(),
                "LightType" => {
                    let type_code = prop.get_attrib(4)?.as_i32()?;
                    light.actual_type = match type_code {
                        0 => FbxLightType::Point,
                        1 => FbxLightType::Directional,
                        2 => FbxLightType::Spot,
                        3 => FbxLightType::Area,
                        4 => FbxLightType::Volume,
                        _ => {
                            Log::writeln(format!("FBX: Unknown light type {}, fallback to Point!", type_code));
                            FbxLightType::Point
                        }
                    };
                }
                _ => ()
            }
        }

        Ok(light)
    }
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
    pub fn read(model_node_handle: Handle<FbxNode>, nodes: &Pool<FbxNode>) -> Result<FbxModel, String> {
        let mut name = String::from("Unnamed");

        let model_node = nodes.borrow(model_node_handle);
        if let Ok(name_attrib) = model_node.get_attrib(1) {
            name = name_attrib.as_string();
        }

        // Remove prefix
        if name.starts_with("Model::") {
            name = name.chars().skip(7).collect();
        }

        let mut model = FbxModel {
            name,
            pre_rotation: Vec3::ZERO,
            post_rotation: Vec3::ZERO,
            rotation_offset: Vec3::ZERO,
            rotation_pivot: Vec3::ZERO,
            scaling_offset: Vec3::ZERO,
            scaling_pivot: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: Vec3::UNIT,
            translation: Vec3::ZERO,
            geometric_translation: Vec3::ZERO,
            geometric_rotation: Vec3::ZERO,
            geometric_scale: Vec3::UNIT,
            inv_bind_transform: Mat4::IDENTITY,
            geoms: Vec::new(),
            materials: Vec::new(),
            animation_curve_nodes: Vec::new(),
            children: Vec::new(),
            light: Handle::NONE,
        };

        let properties70_node_handle = find_node(nodes, model_node_handle, "Properties70")?;
        let properties70_node = nodes.borrow(properties70_node_handle);
        for property_handle in properties70_node.children.iter() {
            let property_node = nodes.borrow(*property_handle);
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
        Ok(model)
    }
}

enum FbxComponent {
    Deformer(FbxDeformer),
    SubDeformer(FbxSubDeformer),
    Texture(FbxTexture),
    Light(FbxLight),
    Model(Box<FbxModel>),
    Material(FbxMaterial),
    AnimationCurveNode(FbxAnimationCurveNode),
    AnimationCurve(FbxAnimationCurve),
    Geometry(Box<FbxGeometry>),
}

macro_rules! define_as {
    ($self:ident, $name:ident, $ty:ty, $kind:ident) => {
        fn $name(&$self) -> Result<&$ty, FbxError> {
            if let FbxComponent::$kind(component) = $self {
                Ok(component)
            } else {
                Err(FbxError::UnexpectedType)
            }
        }
    }
}

impl FbxComponent {
    define_as!(self, as_deformer, FbxDeformer, Deformer);
    define_as!(self, as_sub_deformer, FbxSubDeformer, SubDeformer);
    define_as!(self, as_texture, FbxTexture, Texture);
    define_as!(self, as_light, FbxLight, Light);
    define_as!(self, as_material, FbxMaterial, Material);
    define_as!(self, as_geometry, FbxGeometry, Geometry);
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
fn find_node(pool: &Pool<FbxNode>, root: Handle<FbxNode>, name: &str) -> Result<Handle<FbxNode>, String> {
    let node = pool.borrow(root);

    if node.name == name {
        return Ok(root);
    }

    for child_handle in node.children.iter() {
        if let Ok(result) = find_node(pool, *child_handle, name) {
            return Ok(result);
        }
    }

    Err(format!("FBX DOM: Unable to find {} node", name))
}

/// Searches node by specified name and borrows a reference to it
fn find_and_borrow_node<'a>(pool: &'a Pool<FbxNode>, root: Handle<FbxNode>, name: &str) -> Result<&'a FbxNode, String> {
    let node = pool.borrow(root);

    if node.name == name {
        return Ok(node);
    }

    for child_handle in node.children.iter() {
        if let Ok(result) = find_and_borrow_node(pool, *child_handle, name) {
            return Ok(result);
        }
    }

    Err(format!("FBX DOM: Unable to find {} node", name))
}

/// Links child component with parent component so parent will know about child
fn link_child_with_parent_component(parent: &mut FbxComponent, child: &mut FbxComponent, child_handle: Handle<FbxComponent>) {
    match parent {
        // Link model with other components
        FbxComponent::Model(model) => {
            match child {
                FbxComponent::Geometry(_) => model.geoms.push(child_handle),
                FbxComponent::Material(_) => model.materials.push(child_handle),
                FbxComponent::AnimationCurveNode(_) => model.animation_curve_nodes.push(child_handle),
                FbxComponent::Light(_) => model.light = child_handle,
                FbxComponent::Model(_) => model.children.push(child_handle),
                _ => ()
            }
        }
        // Link material with textures
        FbxComponent::Material(material) => {
            if let FbxComponent::Texture(_) = child {
                material.diffuse_texture = child_handle;
            }
        }
        // Link animation curve node with animation curve
        FbxComponent::AnimationCurveNode(anim_curve_node) => {
            if let FbxComponent::AnimationCurve(_) = child {
                anim_curve_node.curves.push(child_handle);
            }
        }
        // Link deformer with sub-deformers
        FbxComponent::Deformer(deformer) => {
            if let FbxComponent::SubDeformer(_) = child {
                deformer.sub_deformers.push(child_handle);
            }
        }
        // Link geometry with deformers
        FbxComponent::Geometry(geometry) => {
            if let FbxComponent::Deformer(_) = child {
                geometry.deformers.push(child_handle);
            }
        }
        // Link sub-deformer with model
        FbxComponent::SubDeformer(sub_deformer) => {
            if let FbxComponent::Model(model) = child {
                sub_deformer.model = child_handle;
                model.inv_bind_transform = sub_deformer.transform;
            }
        }
        // Ignore rest
        _ => ()
    }
}


fn string_to_mapping(value: &str) -> FbxMapping {
    match value {
        "ByPolygon" => FbxMapping::ByPolygon,
        "ByPolygonVertex" => FbxMapping::ByPolygonVertex,
        "ByVertex" | "ByVertice" => FbxMapping::ByVertex,
        "ByEdge" => FbxMapping::ByEdge,
        "AllSame" => FbxMapping::AllSame,
        _ => FbxMapping::Unknown
    }
}

fn string_to_reference(value: &str) -> FbxReference {
    match value {
        "Direct" => FbxReference::Direct,
        "IndexToDirect" => FbxReference::IndexToDirect,
        "Index" => FbxReference::IndexToDirect,
        _ => FbxReference::Unknown
    }
}

/// Input angles in degrees
fn quat_from_euler(euler: Vec3) -> Quat {
    Quat::from_euler(
        Vec3::new(euler.x.to_radians(), euler.y.to_radians(), euler.z.to_radians()),
        RotationOrder::XYZ)
}

/// Fixes index that is used as indicator of end of a polygon
/// FBX stores array of indices like so 0,1,-3,... where -3
/// is actually index 2 but it xor'ed using -1.
fn fix_index(index: i32) -> usize {
    if index < 0 {
        (index ^ -1) as usize
    } else {
        index as usize
    }
}

/// Triangulates polygon face if needed.
/// Returns number of processed indices.
fn prepare_next_face(geom: &FbxGeometry,
                     start: usize,
                     temp_vertices: &mut Vec<Vec3>,
                     out_triangles: &mut Vec<[usize; 3]>,
                     out_relative_triangles: &mut Vec<[usize; 3]>) -> usize {
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
            out_triangles.push([a, b, c]);
            out_relative_triangles.push([0, 1, 2]);
        }
    } else if vertex_per_face > 3 {
        // Found arbitrary polygon, triangulate it.
        temp_vertices.clear();
        for i in 0..vertex_per_face {
            temp_vertices.push(geom.vertices[fix_index(geom.indices[start + i])]);
        }
        triangulate(&temp_vertices.as_slice(), out_relative_triangles);
        for triangle in out_relative_triangles.iter() {
            out_triangles.push([
                fix_index(geom.indices[start + triangle[0]]),
                fix_index(geom.indices[start + triangle[1]]),
                fix_index(geom.indices[start + triangle[2]])]);
        }
    }

    vertex_per_face
}

fn convert_vertex(geom: &FbxGeometry,
                  mesh: &mut Mesh,
                  geometric_transform: &Mat4,
                  material_index: usize,
                  index: usize,
                  relative_index: usize,
                  skin_data: &[VertexWeightSet]) -> Result<(), FbxError> {
    let position = geometric_transform.transform_vector(*geom.vertices.get(index)
        .ok_or(FbxError::IndexOutOfBounds)?);

    let normal = geometric_transform.transform_vector_normal(match geom.normals.mapping {
        FbxMapping::ByPolygonVertex => *geom.normals
            .elements
            .get(relative_index)
            .ok_or(FbxError::IndexOutOfBounds)?,
        FbxMapping::ByVertex => *geom.normals
            .elements
            .get(index)
            .ok_or(FbxError::IndexOutOfBounds)?,
        _ => Vec3 { x: 0.0, y: 1.0, z: 0.0 }
    });

    let tangent = geometric_transform.transform_vector_normal(match geom.tangents.mapping {
        FbxMapping::ByPolygonVertex => *geom.tangents
            .elements
            .get(relative_index)
            .ok_or(FbxError::IndexOutOfBounds)?,
        FbxMapping::ByVertex => *geom.tangents.elements.get(index).ok_or(FbxError::IndexOutOfBounds)?,
        _ => Vec3 { x: 0.0, y: 1.0, z: 0.0 }
    });

    let uv = match geom.uvs.mapping {
        FbxMapping::ByPolygonVertex => {
            match geom.uvs.reference {
                FbxReference::Direct => *geom.uvs
                    .elements
                    .get(relative_index)
                    .ok_or(FbxError::IndexOutOfBounds)?,
                FbxReference::IndexToDirect => {
                    let uv_index = *geom.uvs
                        .index
                        .get(relative_index)
                        .ok_or(FbxError::IndexOutOfBounds)? as usize;
                    *geom.uvs
                        .elements
                        .get(uv_index)
                        .ok_or(FbxError::IndexOutOfBounds)?
                }
                _ => Vec2 { x: 0.0, y: 0.0 }
            }
        }
        _ => Vec2 { x: 0.0, y: 0.0 }
    };

    let material = match geom.materials.mapping {
        FbxMapping::AllSame => *geom.materials
            .elements
            .first()
            .ok_or(FbxError::IndexOutOfBounds)? as usize,
        FbxMapping::ByPolygon => *geom.materials
            .elements
            .get(material_index)
            .ok_or(FbxError::IndexOutOfBounds)? as usize,
        _ => 0
    };

    let surface = mesh.get_surfaces_mut()
        .get_mut(material)
        .unwrap();

    let is_unique_vertex = surface.get_data().lock().unwrap().insert_vertex(Vertex {
        position,
        normal,
        tex_coord: uv,
        tangent: Vec4 { x: tangent.x, y: tangent.y, z: tangent.z, w: 1.0 },
        // We can't get correct values for bone weights and indices because
        // not all nodes are converted yet at this stage. Actual calculation
        // will be performed later on after converting all nodes.
        bone_weights: [0.0, 0.0, 0.0, 0.0],
        bone_indices: [0, 0, 0, 0],
    });

    if is_unique_vertex && !skin_data.is_empty() {
        surface.vertex_weights.push(*skin_data.get(index).ok_or(FbxError::IndexOutOfBounds)?);
    }

    Ok(())
}

impl Fbx {
    /// Parses FBX DOM and filling internal lists to prepare
    /// for conversion to engine format
    fn prepare(&mut self) -> Result<(), FbxError> {
        // Check version
        let header_handle = find_node(&self.nodes, self.root, "FBXHeaderExtension")?;
        let version = find_and_borrow_node(&self.nodes, header_handle, "FBXVersion")?;
        let version = version.get_attrib(0)?.as_i32()?;
        if version < 7100 && version > 7400 {
            return Err(FbxError::UnsupportedVersion(version));
        }

        // Read objects
        let objects_node = find_and_borrow_node(&self.nodes, self.root, "Objects")?;
        for object_handle in objects_node.children.iter() {
            let object = self.nodes.borrow(*object_handle);
            let index = object.get_attrib(0)?.as_i64()?;
            let mut component_handle: Handle<FbxComponent> = Handle::NONE;
            match object.name.as_str() {
                "Geometry" => {
                    component_handle = self.component_pool.spawn(FbxComponent::Geometry(
                        Box::new(FbxGeometry::read(*object_handle, &self.nodes)?)));
                }
                "Model" => {
                    component_handle = self.component_pool.spawn(FbxComponent::Model(
                        Box::new(FbxModel::read(*object_handle, &self.nodes)?)));
                }
                "Material" => {
                    component_handle = self.component_pool.spawn(FbxComponent::Material(
                        FbxMaterial::read(*object_handle)?));
                }
                "Texture" => {
                    component_handle = self.component_pool.spawn(FbxComponent::Texture(
                        FbxTexture::read(*object_handle, &self.nodes)?));
                }
                "NodeAttribute" => {
                    if object.attrib_count() > 2 && object.get_attrib(2)?.as_string() == "Light" {
                        component_handle = self.component_pool.spawn(FbxComponent::Light(
                            FbxLight::read(*object_handle, &self.nodes)?));
                    }
                }
                "AnimationCurve" => {
                    component_handle = self.component_pool.spawn(FbxComponent::AnimationCurve(
                        FbxAnimationCurve::read(*object_handle, &self.nodes)?));
                }
                "AnimationCurveNode" => {
                    component_handle = self.component_pool.spawn(FbxComponent::AnimationCurveNode(
                        FbxAnimationCurveNode::read(*object_handle, &self.nodes)?));
                }
                "Deformer" => {
                    match object.get_attrib(2)?.as_string().as_str() {
                        "Cluster" => {
                            component_handle = self.component_pool.spawn(FbxComponent::SubDeformer(
                                FbxSubDeformer::read(*object_handle, &self.nodes)?));
                        }
                        "Skin" => {
                            component_handle = self.component_pool.spawn(FbxComponent::Deformer(
                                FbxDeformer::read(*object_handle, &self.nodes)?));
                        }
                        _ => ()
                    }
                }
                _ => ()
            }
            if !component_handle.is_none() {
                self.index_to_component.insert(index, component_handle);
                self.components.push(component_handle);
            }
        }

        // Read connections
        let connections_node = find_and_borrow_node(&self.nodes, self.root, "Connections")?;
        for connection_handle in connections_node.children.iter() {
            let connection = self.nodes.borrow(*connection_handle);
            let child_index = connection.get_attrib(1)?.as_i64()?;
            let parent_index = connection.get_attrib(2)?.as_i64()?;
            if let Some(parent_handle) = self.index_to_component.get(&parent_index) {
                if let Some(child_handle) = self.index_to_component.get(&child_index) {
                    let pair = self.component_pool.borrow_two_mut((*child_handle, *parent_handle)).unwrap();
                    let child = pair.0;
                    let parent = pair.1;
                    link_child_with_parent_component(parent, child, *child_handle);
                }
            }
        }

        Ok(())
    }

    fn convert_light(&self, fbx_light: &FbxLight) -> Light {
        let light_kind = match fbx_light.actual_type {
            FbxLightType::Point | FbxLightType::Directional | FbxLightType::Area | FbxLightType::Volume => {
                LightKind::Point(PointLight::new(fbx_light.radius))
            }
            FbxLightType::Spot => {
                LightKind::Spot(SpotLight::new(fbx_light.radius, fbx_light.cone_angle))
            }
        };

        let mut light = Light::new(light_kind);

        light.set_color(Color::opaque(fbx_light.color.r, fbx_light.color.g, fbx_light.color.b));

        light
    }

    fn create_surfaces(&self,
                       mesh: &mut Mesh,
                       resource_manager: &mut ResourceManager,
                       model: &FbxModel) -> Result<(), FbxError> {
        // Create surfaces per material
        if model.materials.is_empty() {
            mesh.add_surface(Surface::new(Arc::new(Mutex::new(SurfaceSharedData::new()))));
        } else {
            for material_handle in model.materials.iter() {
                let mut surface = Surface::new(Arc::new(Mutex::new(SurfaceSharedData::new())));
                let material = self.component_pool.borrow(*material_handle).as_material()?;
                if material.diffuse_texture.is_some() {
                    let texture = self.component_pool.borrow(material.diffuse_texture).as_texture()?;
                    let path = texture.get_file_path();
                    if let Some(filename) = path.file_name() {
                        let file_stem = path.file_stem().ok_or(FbxError::InvalidPath)?;
                        let extension = path.extension().ok_or(FbxError::InvalidPath)?;

                        let diffuse_path = resource_manager.get_textures_path().join(&filename);
                        // Here we will load *every* texture as RGBA8, this probably is overkill,
                        // that will lead to higher memory consumption, but this will remove
                        // problems with transparent textures (like mesh texture, etc.)
                        surface.set_diffuse_texture(resource_manager.request_texture_async(diffuse_path.as_path(), TextureKind::RGBA8));

                        let mut normal_map_name = file_stem.to_os_string();
                        normal_map_name.push("_normal.");
                        normal_map_name.push(extension);
                        let normal_path = resource_manager.get_textures_path().join(normal_map_name);
                        if normal_path.exists() {
                            // Not sure if alpha channel is useful on normal maps, so will use RGB8 here.
                            // Potentially it can be used to store some per-pixel material data like
                            // roughness, shininess, etc. For now this is a TODO.
                            surface.set_normal_texture(resource_manager.request_texture_async(normal_path.as_path(), TextureKind::RGB8));
                        }
                    }
                }
                mesh.add_surface(surface);
            }
        }

        Ok(())
    }

    fn convert_mesh(&self,
                    resource_manager: &mut ResourceManager,
                    model: &FbxModel) -> Result<Mesh, FbxError> {
        let mut mesh = Mesh::default();

        let geometric_transform = Mat4::translate(model.geometric_translation) *
            Mat4::from_quat(quat_from_euler(model.geometric_rotation)) *
            Mat4::scale(model.geometric_scale);

        let mut temp_vertices: Vec<Vec3> = Vec::new();
        let mut triangles = Vec::new();
        let mut relative_triangles = Vec::new();

        for geom_handle in &model.geoms {
            let geom = self.component_pool.borrow(*geom_handle).as_geometry()?;
            self.create_surfaces(&mut mesh, resource_manager, model)?;

            let skin_data = geom.get_skin_data(&self.component_pool)?;

            let mut material_index = 0;
            let mut n = 0;
            while n < geom.indices.len() {
                let origin = n;
                n += prepare_next_face(geom, n, &mut temp_vertices, &mut triangles, &mut relative_triangles);
                for i in 0..triangles.len() {
                    let triangle = &triangles[i];
                    let relative_triangle = &relative_triangles[i];
                    for (index, relative_index) in triangle.iter().zip(relative_triangle.iter()) {
                        let relative_index = origin + *relative_index;
                        convert_vertex(geom, &mut mesh, &geometric_transform, material_index, *index, relative_index, &skin_data)?;
                    }
                }
                if geom.materials.mapping == FbxMapping::ByPolygon {
                    material_index += 1;
                }
            }

            if geom.tangents.mapping == FbxMapping::Unknown {
                for surface in mesh.get_surfaces_mut() {
                    surface.get_data().lock().unwrap().calculate_tangents();
                }
            }
        }

        Ok(mesh)
    }

    fn convert_model(&self,
                     model: &FbxModel,
                     resource_manager: &mut ResourceManager,
                     graph: &mut Graph,
                     animations: &mut AnimationContainer,
                     animation_handle: Handle<Animation>)
                     -> Result<Handle<Node>, FbxError> {
        // Create node with correct kind.
        let mut node =
            if !model.geoms.is_empty() {
                Node::Mesh(self.convert_mesh(resource_manager, model)?)
            } else if model.light.is_some() {
                let fbx_light_component = self.component_pool.borrow(model.light);
                Node::Light(self.convert_light(fbx_light_component.as_light()?))
            } else {
                Node::Base(Base::default())
            };

        node.base_mut().set_name(model.name.as_str());
        let node_local_rotation = quat_from_euler(model.rotation);
        let transform = node.base_mut().get_local_transform_mut();
        transform.set_rotation(node_local_rotation);
        transform.set_scale(model.scale);
        transform.set_position(model.translation);
        transform.set_post_rotation(quat_from_euler(model.post_rotation));
        transform.set_pre_rotation(quat_from_euler(model.pre_rotation));
        transform.set_rotation_offset(model.rotation_offset);
        transform.set_rotation_pivot(model.rotation_pivot);
        transform.set_scaling_offset(model.scaling_offset);
        transform.set_scaling_pivot(model.scaling_pivot);
        node.base_mut().inv_bind_pose_transform = model.inv_bind_transform;

        let node_handle = graph.add_node(node);

        // Convert animations
        if !model.animation_curve_nodes.is_empty() {
            // Find supported curve nodes (translation, rotation, scale)
            let mut lcl_translation = None;
            let mut lcl_rotation = None;
            let mut lcl_scale = None;
            for anim_curve_node_handle in model.animation_curve_nodes.iter() {
                let component = self.component_pool.borrow(*anim_curve_node_handle);
                if let FbxComponent::AnimationCurveNode(curve_node) = component {
                    if curve_node.actual_type == FbxAnimationCurveNodeType::Rotation {
                        lcl_rotation = Some(curve_node);
                    } else if curve_node.actual_type == FbxAnimationCurveNodeType::Translation {
                        lcl_translation = Some(curve_node);
                    } else if curve_node.actual_type == FbxAnimationCurveNodeType::Scale {
                        lcl_scale = Some(curve_node);
                    }
                }
            }

            // Convert to engine format
            let mut track = Track::new();
            track.set_node(node_handle);

            let mut time = 0.0;
            loop {
                let translation =
                    if let Some(curve) = lcl_translation {
                        curve.eval_vec3(&self.component_pool, time)
                    } else {
                        model.translation
                    };

                let rotation =
                    if let Some(curve) = lcl_rotation {
                        quat_from_euler(curve.eval_vec3(&self.component_pool, time))
                    } else {
                        node_local_rotation
                    };

                let scale = if let Some(curve) = lcl_scale {
                    curve.eval_vec3(&self.component_pool, time)
                } else {
                    model.scale
                };

                track.add_key_frame(KeyFrame::new(time, translation, scale, rotation));

                let mut next_time = std::f32::MAX;
                for node in &[lcl_translation, lcl_rotation, lcl_scale] {
                    if let Some(node) = node {
                        for curve_handle in node.curves.iter() {
                            let curve_component = self.component_pool.borrow(*curve_handle);
                            if let FbxComponent::AnimationCurve(curve) = curve_component {
                                for key in curve.keys.iter() {
                                    if key.time > time {
                                        let distance = key.time - time;
                                        if distance < next_time - key.time {
                                            next_time = key.time;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if next_time >= std::f32::MAX {
                    break;
                }

                time = next_time;
            }

            animations.get_mut(animation_handle).add_track(track);
        }

        Ok(node_handle)
    }

    ///
    /// Converts FBX DOM to native engine representation.
    ///
    pub fn convert(&self, resource_manager: &mut ResourceManager, scene: &mut Scene) -> Result<Handle<Node>, FbxError> {
        let mut instantiated_nodes = Vec::new();
        let root = scene.graph.add_node(Node::Base(Base::default()));
        let animation_handle = scene.animations.add(Animation::default());
        let mut fbx_model_to_node_map = HashMap::new();
        for component_handle in self.components.iter() {
            let component = self.component_pool.borrow(*component_handle);
            if let FbxComponent::Model(model) = component {
                let node = self.convert_model(model, resource_manager, &mut scene.graph, &mut scene.animations, animation_handle)?;
                instantiated_nodes.push(node);
                scene.graph.link_nodes(node, root);
                fbx_model_to_node_map.insert(*component_handle, node);
            }
        }
        // Link according to hierarchy
        for (fbx_model_handle, node_handle) in fbx_model_to_node_map.iter() {
            if let FbxComponent::Model(fbx_model) = self.component_pool.borrow(*fbx_model_handle) {
                for fbx_child_handle in fbx_model.children.iter() {
                    if let Some(child_handle) = fbx_model_to_node_map.get(fbx_child_handle) {
                        scene.graph.link_nodes(*child_handle, *node_handle);
                    }
                }
            }
        }
        scene.graph.update_transforms();

        // Remap handles from fbx model to handles of instantiated nodes
        // on each surface of each mesh.
        for handle in instantiated_nodes.iter() {
            let node = scene.graph.get_mut(*handle);
            if let Node::Mesh(mesh) = node {
                let mut surface_bones = HashSet::new();
                for surface in mesh.get_surfaces_mut() {
                    for weight_set in surface.vertex_weights.iter_mut() {
                        for weight in weight_set.iter_mut() {
                            let fbx_model: Handle<FbxComponent> = weight.effector.into();
                            let bone_handle = fbx_model_to_node_map.get(&fbx_model)
                                .ok_or(FbxError::UnableToRemapModelToNode)?;
                            surface_bones.insert(*bone_handle);
                            weight.effector = (*bone_handle).into();
                        }
                    }
                    surface.bones = surface_bones.iter().copied().collect();

                    // TODO: Add sanity check about unique owner of surface data.
                    // At this point owner of surface data *must* be only one.
                    // But who knows.
                    let data_rc = surface.get_data();
                    let mut data = data_rc.lock().unwrap();
                    if data.get_vertices().len() == surface.vertex_weights.len() {
                        for (i, vertex) in data.get_vertices_mut().iter_mut().enumerate() {
                            let weight_set = surface.vertex_weights.get_mut(i)
                                .ok_or(FbxError::IndexOutOfBounds)?;
                            for (k, weight) in weight_set.iter().enumerate() {
                                vertex.bone_indices[k] = {
                                    let mut index = None;
                                    for (n, bone_handle) in surface.bones.iter().enumerate() {
                                        if *bone_handle == weight.effector.into() {
                                            index = Some(n);
                                            break;
                                        }
                                    }
                                    index.ok_or(FbxError::UnableToFindBone)? as u8
                                };
                                vertex.bone_weights[k] = weight.value;
                            }
                        }
                    }
                }
            }
        }

        Ok(root)
    }

    pub fn pretty_print(&mut self) {
        let mut stack: Vec<Handle<FbxNode>> = Vec::new();
        stack.push(self.root);
        while let Some(handle) = stack.pop() {
            let node = self.nodes.borrow(handle);
            println!("{}", node.name);

            // Continue printing children
            for child_handle in node.children.iter() {
                stack.push(child_handle.clone());
            }
        }
    }
}

pub fn load_to_scene<P: AsRef<Path>>(scene: &mut Scene, resource_manager: &mut ResourceManager, path: P) -> Result<Handle<Node>, FbxError> {
    let start_time = Instant::now();

    let mut file = File::open(path.as_ref())?;

    Log::writeln(format!("Trying to load {:?}", path.as_ref()));

    let now = Instant::now();
    let is_bin = fbx_binary::is_binary(path.as_ref())?;

    let buf_len = file.metadata()?.len() as usize;
    let mut file_content = Vec::with_capacity(buf_len);
    file.read_to_end(&mut file_content)?;
    let mut reader = Cursor::new(file_content);

    let mut fbx = if is_bin {
        fbx_binary::read_binary(&mut reader)?
    } else {
        fbx_ascii::read_ascii(&mut reader, buf_len as u64)?
    };
    Log::writeln(format!("\t- Parsing - {} ms", now.elapsed().as_millis()));

    let now = Instant::now();
    fbx.prepare()?;
    Log::writeln(format!("\t- DOM Prepare - {} ms", now.elapsed().as_millis()));

    let now = Instant::now();
    let result = fbx.convert(resource_manager, scene);
    Log::writeln(format!("\t- Conversion - {} ms", now.elapsed().as_millis()));

    Log::writeln(format!("\t- {:?} loaded in {} ms", path.as_ref(), start_time.elapsed().as_millis()));

    result
}