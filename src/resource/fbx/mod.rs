use crate::utils::pool::*;
use std::path::*;
use std::fs::File;
use std::io::Read;
use crate::math::vec3::*;
use crate::math::vec2::*;
use crate::math::mat4::*;
use std::collections::HashMap;
use crate::scene::*;

pub enum FbxAttribute {
    Double(f64),
    Float(f32),
    Integer(i32),
    Long(i64),
    Bool(bool),
    String(String), // ASCII Fbx always have every attribute in string form
}

impl FbxAttribute {
    pub fn as_int(&self) -> i32 {
        match self {
            FbxAttribute::Double(val) => *val as i32,
            FbxAttribute::Float(val) => *val as i32,
            FbxAttribute::Integer(val) => *val,
            FbxAttribute::Long(val) => *val as i32,
            FbxAttribute::Bool(val) => *val as i32,
            FbxAttribute::String(val) => val.parse::<i32>().unwrap(),
        }
    }

    pub fn as_int64(&self) -> i64 {
        match self {
            FbxAttribute::Double(val) => *val as i64,
            FbxAttribute::Float(val) => *val as i64,
            FbxAttribute::Integer(val) => *val as i64,
            FbxAttribute::Long(val) => *val as i64,
            FbxAttribute::Bool(val) => *val as i64,
            FbxAttribute::String(val) => val.parse::<i64>().unwrap(),
        }
    }

    pub fn as_double(&self) -> f64 {
        match self {
            FbxAttribute::Double(val) => *val,
            FbxAttribute::Float(val) => *val as f64,
            FbxAttribute::Integer(val) => *val as f64,
            FbxAttribute::Long(val) => *val as f64,
            FbxAttribute::Bool(val) => (*val as i64) as f64,
            FbxAttribute::String(val) => val.parse::<f64>().unwrap(),
        }
    }

    pub fn as_float(&self) -> f32 {
        match self {
            FbxAttribute::Double(val) => *val as f32,
            FbxAttribute::Float(val) => *val,
            FbxAttribute::Integer(val) => *val as f32,
            FbxAttribute::Long(val) => *val as f32,
            FbxAttribute::Bool(val) => (*val as i32) as f32,
            FbxAttribute::String(val) => val.parse::<f32>().unwrap(),
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
    transform_link: Mat4,
}

struct FbxTexture {
    filename: String,
}

struct FbxMaterial {
    diffuse_texture: Handle<FbxComponent>
}

struct FbxDeformer {
    sub_deformers: Vec<Handle<FbxComponent>>
}

struct FbxAnimationCurve {
    keys: Vec<FbxTimeValuePair>
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

enum FbxMapping {
    Unknown,
    ByPolygon,
    ByPolygonVertex,
    ByVertex,
    ByEdge,
    AllSame,
}

enum FbxReference {
    Unknown,
    Direct,
    IndexToDirect,
}

struct FbxNode {
    name: String,
    attributes: Vec<FbxAttribute>,
    parent: Handle<FbxNode>,
    children: Vec<Handle<FbxNode>>,
}

impl FbxNode {
    fn get_vec3_at(&self, n: usize) -> Result<Vec3, &'static str> {
        if n + 3 <= self.attributes.len() {
            return Ok(Vec3 {
                x: self.attributes[n].as_float(),
                y: self.attributes[n + 1].as_float(),
                z: self.attributes[n + 2].as_float(),
            });
        }
        return Err("FBX: Attribute index out of bounds");
    }

    fn get_vec3_at_unchecked(&self, n: usize) -> Vec3 {
        Vec3 {
            x: self.attributes[n].as_float(),
            y: self.attributes[n + 1].as_float(),
            z: self.attributes[n + 2].as_float(),
        }
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
                stack: &mut Vec<Handle<FbxNode>>) -> Result<FbxGeometry, &'static str> {
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
        if let Some(vertices_node_handle) = find_node(nodes, stack, geom_node_handle, "Vertices") {
            if let Some(vertices_array_node) = find_and_borrow_node(nodes, stack, &vertices_node_handle, "a") {
                let vertex_count = vertices_array_node.attributes.len() / 3;
                geom.vertices = Vec::with_capacity(vertex_count);
                for i in 0..vertex_count {
                    geom.vertices.push(vertices_array_node.get_vec3_at_unchecked(i * 3));
                }
            } else {
                return Err("FBX: Unable to find array node of vertices");
            }
        } else {
            return Err("FBX: Unable to find vertices node");
        }

        // Read faces
        if let Some(indices_node_handle) = find_node(nodes, stack, geom_node_handle, "PolygonVertexIndex") {
            if let Some(indices_array_node) = find_and_borrow_node(nodes, stack, &indices_node_handle, "a") {
                let index_count = indices_array_node.attributes.len();
                geom.indices = Vec::with_capacity(index_count);
                for i in 0..index_count {
                    geom.indices.push(indices_array_node.attributes[i].as_int());
                }
            } else {
                return Err("FBX: Unable to find array node of indices");
            }
        } else {
            return Err("FBX: Unable to find indices node");
        }

        // read normals
        if let Some(layer_element_normal_node_handle) = find_node(nodes, stack, geom_node_handle, "LayerElementNormal") {
            if let Some(map_type_node) = find_and_borrow_node(nodes, stack, &layer_element_normal_node_handle, "MappingInformationType") {
                geom.normal_mapping = string_to_mapping(&map_type_node.attributes[0].as_string());
            }
            if let Some(ref_type_node) = find_and_borrow_node(nodes, stack, &layer_element_normal_node_handle, "ReferenceInformationType") {
                geom.normal_reference = string_to_reference(&ref_type_node.attributes[0].as_string());
            }
            // finally read normals
            if let Some(normals_node_handle) = find_node(nodes, stack, &layer_element_normal_node_handle, "Normals") {
                if let Some(normals_array_node) = find_and_borrow_node(nodes, stack, &normals_node_handle, "a") {
                    let count = normals_array_node.attributes.len() / 3;
                    for i in 0..count {
                        geom.normals.push(normals_array_node.get_vec3_at_unchecked(i * 3));
                    }
                }
            }
        }

        // todo: read tangents

        // todo: read uvs

        // todo: read materials

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
                stack: &mut Vec<Handle<FbxNode>>) -> Result<FbxModel, &'static str> {
        let mut name = String::from("Unnamed");
        if let Some(model_node) = nodes.borrow(&model_node_handle) {
            if let Some(name_attrib) = model_node.attributes.get(1) {
                name = name_attrib.as_string();
            }
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

        if let Some(properties70_node_handle) = find_node(nodes, stack, model_node_handle, "Properties70") {
            if let Some(properties70_node) = nodes.borrow(&properties70_node_handle) {
                for property_handle in properties70_node.children.iter() {
                    if let Some(property_node) = nodes.borrow(&property_handle) {
                        if let Some(name_attrib) = property_node.attributes.get(0) {
                            if let FbxAttribute::String(name) = &name_attrib {
                                match name.as_str() {
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
                        }
                    } else {
                        return Err("FBX: Invalid property handle");
                    }
                }
            } else {
                return Err("FBX: Invalid fbx node handle");
            }
        } else {
            return Err("FBX: Properties70 not found!");
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
    components: Pool<FbxComponent>,
    root: Handle<FbxNode>,
    /// Map used for fast look up of components by their fbx-indices
    index_to_component: HashMap<i64, Handle<FbxComponent>>,
}

/// Searches node by specified name and returns its handle if found
/// Uses provided stack to do depth search
fn find_node(pool: &Pool<FbxNode>, stack: &mut Vec<Handle<FbxNode>>, root: &Handle<FbxNode>, name: &str) -> Option<Handle<FbxNode>> {
    stack.clear();
    stack.push(root.clone());
    while let Some(handle) = stack.pop() {
        if let Some(node) = pool.borrow(&handle) {
            if node.name == name {
                return Some(handle);
            }
            for child_handle in node.children.iter() {
                stack.push(child_handle.clone());
            }
        }
    }
    None
}

/// Searches node by specified name and borrows a reference to it
/// Uses provided stack to do depth search
fn find_and_borrow_node<'a>(pool: &'a Pool<FbxNode>, stack: &mut Vec<Handle<FbxNode>>, root: &Handle<FbxNode>, name: &str) -> Option<&'a FbxNode> {
    stack.clear();
    stack.push(root.clone());
    while let Some(handle) = stack.pop() {
        if let Some(node) = pool.borrow(&handle) {
            if node.name == name {
                return Some(node);
            }
            for child_handle in node.children.iter() {
                stack.push(child_handle.clone());
            }
        }
    }
    None
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

fn read_ascii(path: &Path) -> Result<Fbx, &'static str> {
    let mut nodes: Pool<FbxNode> = Pool::new();
    let root_handle = nodes.spawn(FbxNode {
        name: String::from("__ROOT__"),
        children: Vec::new(),
        parent: Handle::none(),
        attributes: Vec::new(),
    });
    let mut parent_handle: Handle<FbxNode> = root_handle.clone();
    let mut node_handle: Handle<FbxNode> = Handle::none();
    let mut read_all = false;
    let mut read_value = false;
    let mut buffer: Vec<char> = Vec::new();
    let mut name: Vec<char> = Vec::new();
    let mut value: Vec<char> = Vec::new();
    if let Ok(ref mut file) = File::open(path) {
        // Read line by line
        loop {
            // Read line, trim spaces (but leave spaces in quotes)
            buffer.clear();
            let mut end = false;
            loop {
                let mut temp: [u8; 1] = [0];
                if let Ok(n) = file.read(&mut temp) {
                    if n != 1 {
                        end = true;
                        break;
                    }
                    let symbol = temp[0] as char;
                    if symbol == '\n' {
                        break;
                    } else if symbol == '"' {
                        read_all = !read_all;
                    } else {
                        if read_all || !symbol.is_ascii_whitespace() {
                            buffer.push(symbol);
                        }
                    }
                } else {
                    return Err("unexpected end of file");
                }
            }
            if end {
                break;
            }
            if buffer.len() == 0 {
                continue;
            }
            // Ignore comments
            if buffer[0] == ';' {
                continue;
            }

            // Parse string
            read_value = false;
            name.clear();
            for i in 0..buffer.len() {
                let symbol = buffer[i as usize] as char;
                if i == 0 {
                    if symbol == '-' || symbol.is_ascii_digit() {
                        read_value = true;
                    }
                }
                if symbol == ':' && !read_value {
                    read_value = true;
                    let mut node = FbxNode {
                        name: name.iter().collect(),
                        attributes: Vec::new(),
                        parent: parent_handle.clone(),
                        children: Vec::new(),
                    };
                    node_handle = nodes.spawn(node);
                    name.clear();
                    if let Some(parent) = nodes.borrow_mut(&parent_handle) {
                        parent.children.push(node_handle.clone());
                    }
                } else if symbol == '{' {
                    // Enter child scope
                    parent_handle = node_handle.clone();
                    // Commit attribute if we have one
                    if value.len() > 0 {
                        if let Some(node) = nodes.borrow_mut(&node_handle) {
                            let attrib = FbxAttribute::String(value.iter().collect());
                            node.attributes.push(attrib);
                        } else {
                            return Err("FBX: Failed to fetch node by handle when entering child scope");
                        }
                        value.clear();
                    }
                } else if symbol == '}' {
                    // Exit child scope
                    if let Some(parent) = nodes.borrow_mut(&parent_handle) {
                        parent_handle = parent.parent.clone();
                    }
                } else if symbol == ',' || (i == buffer.len() - 1) {
                    // Commit attribute
                    if symbol != ',' {
                        value.push(symbol);
                    }

                    if let Some(node) = nodes.borrow_mut(&node_handle) {
                        let attrib = FbxAttribute::String(value.iter().collect());
                        node.attributes.push(attrib);
                    } else {
                        return Err("FBX: Failed to fetch node by handle when committing attribute");
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
        components: Pool::new(),
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

impl Fbx {
    /// Parses FBX DOM and filling internal lists to prepare
    /// for conversion to engine format
    fn prepare(&mut self) -> Result<(), &'static str> {
        // Search-stack for internal routines
        let mut traversal_stack: Vec<Handle<FbxNode>> = Vec::new();

        // Check version
        if let Some(header_handle) = find_node(&self.nodes, &mut traversal_stack, &self.root, "FBXHeaderExtension") {
            if let Some(version) = find_and_borrow_node(&self.nodes, &mut traversal_stack, &header_handle, "FBXVersion") {
                if let Some(version_attrib) = version.attributes.get(0) {
                    if version_attrib.as_int() < 7100 {
                        return Err("FBX: Unsupported version. Version must be >= 7100");
                    }
                } else {
                    return Err("FBX: Version attribute not found");
                }
            }
        } else {
            return Err("FBX: Unable to find header");
        }

        // Read objects
        if let Some(objects_node) = find_and_borrow_node(&self.nodes, &mut traversal_stack, &self.root, "Objects") {
            for object_handle in objects_node.children.iter() {
                if let Some(object) = self.nodes.borrow(&object_handle) {
                    let index: i64 = object.attributes[0].as_int64();
                    let mut component_handle: Handle<FbxComponent> = Handle::none();
                    match object.name.as_str() {
                        "Geometry" => {
                            component_handle = self.components.spawn(FbxComponent::Geometry(
                                FbxGeometry::read(object_handle, &self.nodes, &mut traversal_stack)?))
                        }
                        "Model" => {
                            component_handle = self.components.spawn(FbxComponent::Model(
                                FbxModel::read(object_handle, &self.nodes, &mut traversal_stack)?))
                        }
                        "Material" => {
                            println!("reading a Material");
                        }
                        "Texture" => {
                            println!("reading a Texture");
                        }
                        "NodeAttribute" => {
                            println!("reading a NodeAttribute");
                        }
                        "AnimationCurve" => {
                            println!("reading a AnimationCurve");
                        }
                        "AnimationCurveNode" => {
                            println!("reading a AnimationCurveNode");
                        }
                        "Deformer" => {
                            println!("reading a Deformer");
                        }
                        _ => ()
                    }
                    if !component_handle.is_none() {
                        self.index_to_component.insert(index, component_handle);
                    }
                }
            }
        } else {
            return Err("FBX: Objects missing");
        }

        // Read connections
        if let Some(connections_node) = find_and_borrow_node(&self.nodes, &mut traversal_stack, &self.root, "Connections") {
            for connection_handle in connections_node.children.iter() {
                if let Some(connection) = self.nodes.borrow(&connection_handle) {
                    if connection.attributes.len() < 3 {
                        continue;
                    }

                    let child_index = connection.attributes[1].as_int64();
                    let parent_index = connection.attributes[2].as_int64();

                    if let Some(parent_handle) = self.index_to_component.get(&parent_index) {
                        if let Some(child_handle) = self.index_to_component.get(&child_index) {
                            let mut child_type_id = FbxComponentTypeId::Unknown;
                            if let Some(child) = self.components.borrow(child_handle) {
                                child_type_id = child.type_id();
                            }
                            if let Some(parent) = self.components.borrow_mut(parent_handle) {
                                link_child_with_parent_component(parent, child_handle, child_type_id);
                            }
                        }
                    }
                }
            }
        }

        return Ok(());
    }

    pub fn print(&mut self) {


        let mut stack: Vec<Handle<FbxNode>> = Vec::new();
        stack.push(self.root.clone());
        while let Some(handle) = stack.pop() {
            if let Some(node) = self.nodes.borrow(&handle) {
                println!("{}", node.name);

                // Continue printing children
                for child_handle in node.children.iter() {
                    stack.push(child_handle.clone());
                }
            }
        }
    }
}

pub fn load_to_scene(scene: &mut Scene, path: &Path) {
    if let Ok(ref mut fbx) = read_ascii(path) {
        if let Ok(_) = fbx.prepare() {

        }
    }
}