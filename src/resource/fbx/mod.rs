use crate::utils::pool::*;
use std::path::*;
use std::fs::File;
use std::io::Read;
use std::cell::*;
use crate::math::vec3::*;
use crate::math::vec2::*;
use crate::math::mat4::*;

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
            FbxAttribute::Integer(val) => *val ,
            FbxAttribute::Long(val) => *val as i32,
            FbxAttribute::Bool(val) => *val as i32,
            FbxAttribute::String(val) => val.parse::<i32>().unwrap(),
        }
    }

    pub fn as_double(&self) -> f64 {
        match self {
            FbxAttribute::Double(val) => *val,
            FbxAttribute::Float(val) => *val as f64,
            FbxAttribute::Integer(val) => *val as f64 ,
            FbxAttribute::Long(val) => *val as f64,
            FbxAttribute::Bool(val) => (*val as i64) as f64,
            FbxAttribute::String(val) => val.parse::<f64>().unwrap(),
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
    sub_deformers: Vec<FbxSubDeformer>
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
    curves: Vec<FbxComponent>
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
    IndexToDirect
}

struct FbxNode {
    name: String,
    attributes: Vec<FbxAttribute>,
    parent: Handle<FbxNode>,
    children: Vec<Handle<FbxNode>>,
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

    // deformers
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
    a: u8
}

struct FbxLight {
    actual_type: FbxLightType,
    color: FbxColor,
    radius: f32,
    cone_angle: f32
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
}

enum FbxComponentKind {
    Deformer(FbxDeformer),
    SubDeformer(FbxSubDeformer),
    Texture(FbxTexture),
    Light(FbxLight),
    Model(FbxModel),
    Material(FbxMaterial),
    AnimationCurveNode(FbxAnimationCurveNode),
    AnimationCurve(FbxAnimationCurve),
    Geometry(FbxGeometry)
}

struct FbxComponent {
    index: i64,
    kind: FbxComponentKind
}

pub struct Fbx {
    /// Every FBX DOM node lives in this pool, other code uses handles to
    /// borrow references to actual nodes.
    nodes: Pool<FbxNode>,
    /// Pool for FBX components, filled after "prepare" method
    components: Pool<FbxComponent>,
    root: Handle<FbxNode>,
    /// RefCell here to be able to search using immutable reference
    traversal_stack: RefCell<Vec<Handle<FbxNode>>>,
}

impl Fbx {
    fn string_to_mapping(value: &String) -> FbxMapping {
        FbxMapping::Unknown // todo
    }

    pub fn read_ascii(path: &Path) -> Result<Fbx, &'static str> {
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

                //let temp: String = buffer.iter().collect();
                //println!("line is: {:?}", temp);

                /* Parse string */
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
                                node.attributes.push(FbxAttribute::String(value.iter().collect()));
                            } else {
                                return Err("1: failed to fetch node by handle");
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
                            node.attributes.push(FbxAttribute::String(value.iter().collect()));
                        } else {
                            return Err("2: failed to fetch node by handle");
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
            traversal_stack: RefCell::new(Vec::new()),
        })
    }

    fn find_node(&self, root: &Handle<FbxNode>, name: &str) -> Option<Handle<FbxNode>> {
        let mut stack = self.traversal_stack.borrow_mut();
        stack.clear();
        stack.push(root.clone());
        while let Some(handle) = stack.pop() {
            if let Some(node) = self.nodes.borrow(&handle) {
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

    /// Parses FBX DOM and filling internal lists to prepare
    /// for conversion to engine format
    pub fn prepare(&mut self) -> Result<(), &'static str> {
        // Check version
        if let Some(header_handle) = self.find_node(&self.root, "FBXHeaderExtension") {
            if let Some(header) = self.nodes.borrow(&header_handle) {
                if let Some(version_handle) = self.find_node(&header_handle, "FBXVersion") {
                    if let Some(version) = self.nodes.borrow(&version_handle) {
                        if version.attributes[0].as_int() < 7100 {
                            return Err("unsupported version");
                        }
                    }
                }
            }
        } else {
            return Err("unable to find header");
        }

        // Read objects
        if let Some(objects_handle) = self.find_node(&self.root, "Objects") {
            if let Some(objects) = self.nodes.borrow(&objects_handle) {
                for child_handle in objects.children.iter() {
                    if let Some(object) = self.nodes.borrow(&child_handle) {
                        match object.name.as_str() {
                            "Geometry" => {
                                println!("reading a Geometry");
                            },
                            "Model" => {
                                println!("reading a Model");
                            },
                            "Material" => {
                                println!("reading a Material");
                            },
                            "Texture" => {
                                println!("reading a Texture");
                            },
                            "NodeAttribute" => {
                                println!("reading a NodeAttribute");
                            },
                            "AnimationCurve" => {
                                println!("reading a AnimationCurve");
                            },
                            "AnimationCurveNode" => {
                                println!("reading a AnimationCurveNode");
                            },
                            "Deformer" => {
                                println!("reading a Deformer");
                            },
                            _ => ()
                        }
                    }
                }
            }
        } else {
            return Err("Objects missing");
        }

        return Ok(());
    }

    pub fn print(&mut self) {
        let mut stack = self.traversal_stack.borrow_mut();
        stack.clear();
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