use crate::core::algebra::{Matrix4, Vector3};
use crate::{
    core::pool::{Handle, Pool, PoolPairIterator},
    resource::fbx::{
        document::{attribute::FbxAttribute, FbxDocument, FbxNode, FbxNodeContainer},
        error::FbxError,
        scene::{
            animation::{FbxAnimationCurve, FbxAnimationCurveNode},
            geometry::FbxGeometry,
            light::FbxLight,
            model::FbxModel,
            texture::FbxTexture,
        },
    },
};
use std::collections::HashMap;

pub mod animation;
pub mod geometry;
pub mod light;
pub mod model;
pub mod texture;

pub struct FbxScene {
    components: Pool<FbxComponent>,
}

impl FbxScene {
    /// Parses FBX DOM and filling internal lists to prepare
    /// for conversion to engine format
    pub fn new(document: &FbxDocument) -> Result<Self, FbxError> {
        let mut components = Pool::new();
        let mut index_to_component = HashMap::new();

        let nodes = document.nodes();

        // Check version
        let header_handle = nodes.find(document.root(), "FBXHeaderExtension")?;
        let version = nodes.get_by_name(header_handle, "FBXVersion")?;
        let version = version.get_attrib(0)?.as_i32()?;
        if version < 7100 && version > 7400 {
            return Err(FbxError::UnsupportedVersion(version));
        }

        // Read objects
        let objects_node = nodes.get_by_name(document.root(), "Objects")?;
        for object_handle in objects_node.children() {
            let object = nodes.get(*object_handle);
            let index = object.get_attrib(0)?.as_i64()?;
            let mut component_handle: Handle<FbxComponent> = Handle::NONE;
            match object.name() {
                "Geometry" => {
                    component_handle = components.spawn(FbxComponent::Geometry(Box::new(
                        FbxGeometry::read(*object_handle, nodes)?,
                    )));
                }
                "Model" => {
                    component_handle = components.spawn(FbxComponent::Model(Box::new(
                        FbxModel::read(*object_handle, nodes)?,
                    )));
                }
                "Material" => {
                    component_handle =
                        components.spawn(FbxComponent::Material(FbxMaterial::read(*object_handle)));
                }
                "Texture" => {
                    component_handle = components.spawn(FbxComponent::Texture(FbxTexture::read(
                        *object_handle,
                        nodes,
                    )?));
                }
                "NodeAttribute" => {
                    if object.attrib_count() > 2 && object.get_attrib(2)?.as_string() == "Light" {
                        component_handle = components
                            .spawn(FbxComponent::Light(FbxLight::read(*object_handle, nodes)?));
                    }
                }
                "AnimationCurve" => {
                    component_handle = components.spawn(FbxComponent::AnimationCurve(
                        FbxAnimationCurve::read(*object_handle, nodes)?,
                    ));
                }
                "AnimationCurveNode" => {
                    component_handle = components.spawn(FbxComponent::AnimationCurveNode(
                        FbxAnimationCurveNode::read(*object_handle, nodes)?,
                    ));
                }
                "Deformer" => match object.get_attrib(2)?.as_string().as_str() {
                    "Cluster" => {
                        component_handle = components.spawn(FbxComponent::SubDeformer(
                            FbxSubDeformer::read(*object_handle, nodes)?,
                        ));
                    }
                    "Skin" => {
                        component_handle = components.spawn(FbxComponent::Deformer(
                            FbxDeformer::read(*object_handle, nodes),
                        ));
                    }
                    _ => (),
                },
                _ => (),
            }
            if !component_handle.is_none() {
                index_to_component.insert(index, component_handle);
            }
        }

        // Read connections
        let connections_node = nodes.get_by_name(document.root(), "Connections")?;
        for connection_handle in connections_node.children() {
            let connection = nodes.get(*connection_handle);
            let child_index = connection.get_attrib(1)?.as_i64()?;
            let parent_index = connection.get_attrib(2)?.as_i64()?;
            let property = match connection.get_attrib(3) {
                Ok(attrib) => attrib.as_string(),
                Err(_) => String::from(""),
            };
            if let Some(parent_handle) = index_to_component.get(&parent_index) {
                if let Some(child_handle) = index_to_component.get(&child_index) {
                    let (child, parent) =
                        components.borrow_two_mut((*child_handle, *parent_handle));
                    link_child_with_parent_component(parent, child, *child_handle, property);
                }
            }
        }

        Ok(Self { components })
    }

    pub fn pair_iter(&self) -> PoolPairIterator<FbxComponent> {
        self.components.pair_iter()
    }

    pub fn get(&self, handle: Handle<FbxComponent>) -> &FbxComponent {
        self.components.borrow(handle)
    }
}

fn link_child_with_parent_component(
    parent: &mut FbxComponent,
    child: &mut FbxComponent,
    child_handle: Handle<FbxComponent>,
    property: String,
) {
    match parent {
        // Link model with other components
        FbxComponent::Model(model) => match child {
            FbxComponent::Geometry(_) => model.geoms.push(child_handle),
            FbxComponent::Material(_) => model.materials.push(child_handle),
            FbxComponent::AnimationCurveNode(_) => model.animation_curve_nodes.push(child_handle),
            FbxComponent::Light(_) => model.light = child_handle,
            FbxComponent::Model(_) => model.children.push(child_handle),
            _ => (),
        },
        // Link material with textures
        FbxComponent::Material(material) => {
            if let FbxComponent::Texture(_) = child {
                material.textures.push((property, child_handle));
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
        _ => (),
    }
}

pub enum FbxComponent {
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
        pub fn $name(&$self) -> Result<&$ty, FbxError> {
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

// https://help.autodesk.com/view/FBX/2016/ENU/?guid=__cpp_ref_class_fbx_anim_curve_html
const FBX_TIME_UNIT: f64 = 1.0 / 46_186_158_000.0;

pub struct FbxSubDeformer {
    model: Handle<FbxComponent>,
    weights: Vec<(i32, f32)>,
    transform: Matrix4<f32>,
}

impl FbxSubDeformer {
    fn read(
        sub_deformer_handle: Handle<FbxNode>,
        nodes: &FbxNodeContainer,
    ) -> Result<Self, String> {
        // For some reason FBX exported from Blender can have sub deformer without weights and
        // indices. This is still valid and we have to return dummy in this case, instead of
        // error.
        if let Ok(indices_handle) = nodes.find(sub_deformer_handle, "Indexes") {
            let indices = nodes.get_by_name(indices_handle, "a")?;

            let weights_handle = nodes.find(sub_deformer_handle, "Weights")?;
            let weights = nodes.get_by_name(weights_handle, "a")?;

            let transform_handle = nodes.find(sub_deformer_handle, "Transform")?;
            let transform_node = nodes.get_by_name(transform_handle, "a")?;

            if transform_node.attrib_count() != 16 {
                return Err(format!(
                    "FBX: Wrong transform size! Expect 16, got {}",
                    transform_node.attrib_count()
                ));
            }

            if indices.attrib_count() != weights.attrib_count() {
                return Err(String::from(
                    "invalid sub deformer, weights count does not match index count",
                ));
            }

            let mut transform = Matrix4::identity();
            for i in 0..16 {
                transform[i] = transform_node.get_attrib(i)?.as_f64()? as f32;
            }

            let mut sub_deformer = FbxSubDeformer {
                model: Handle::NONE,
                weights: Vec::with_capacity(weights.attrib_count()),
                transform,
            };

            for i in 0..weights.attrib_count() {
                sub_deformer.weights.push((
                    indices.get_attrib(i)?.as_i32()?,
                    weights.get_attrib(i)?.as_f64()? as f32,
                ));
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

pub struct FbxMaterial {
    pub textures: Vec<(String, Handle<FbxComponent>)>,
}

impl FbxMaterial {
    fn read(_material_node_handle: Handle<FbxNode>) -> FbxMaterial {
        FbxMaterial {
            textures: Default::default(),
        }
    }
}

pub struct FbxDeformer {
    pub sub_deformers: Vec<Handle<FbxComponent>>,
}

impl FbxDeformer {
    fn read(_sub_deformer_handle: Handle<FbxNode>, _nodes: &FbxNodeContainer) -> Self {
        FbxDeformer {
            sub_deformers: Vec::new(),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum FbxMapping {
    ByPolygon,
    ByPolygonVertex,
    ByVertex,
    ByEdge,
    AllSame,
}

impl FbxMapping {
    pub fn from_string<P: AsRef<str>>(value: P) -> Result<Self, FbxError> {
        match value.as_ref() {
            "ByPolygon" => Ok(FbxMapping::ByPolygon),
            "ByPolygonVertex" => Ok(FbxMapping::ByPolygonVertex),
            "ByVertex" | "ByVertice" => Ok(FbxMapping::ByVertex),
            "ByEdge" => Ok(FbxMapping::ByEdge),
            "AllSame" => Ok(FbxMapping::AllSame),
            _ => Err(FbxError::InvalidMapping),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum FbxReference {
    Direct,
    IndexToDirect,
}

impl FbxReference {
    pub fn from_string<P: AsRef<str>>(value: P) -> Result<Self, FbxError> {
        match value.as_ref() {
            "Direct" => Ok(FbxReference::Direct),
            "IndexToDirect" => Ok(FbxReference::IndexToDirect),
            "Index" => Ok(FbxReference::IndexToDirect),
            _ => Err(FbxError::InvalidReference),
        }
    }
}

pub struct FbxContainer<T> {
    pub elements: Vec<T>,
    pub index: Vec<i32>,
    pub mapping: FbxMapping,
    pub reference: FbxReference,
}

impl<T> FbxContainer<T> {
    pub fn new<M, P>(
        nodes: &FbxNodeContainer,
        container_node: Handle<FbxNode>,
        data_name: P,
        mapper: M,
    ) -> Result<Self, FbxError>
    where
        M: FnOnce(&[FbxAttribute]) -> Result<Vec<T>, FbxError>,
        P: AsRef<str>,
    {
        let map_type_node = nodes.get_by_name(container_node, "MappingInformationType")?;
        let mapping = FbxMapping::from_string(map_type_node.get_attrib(0)?.as_string())?;

        let ref_type_node = nodes.get_by_name(container_node, "ReferenceInformationType")?;
        let mut reference = FbxReference::from_string(ref_type_node.get_attrib(0)?.as_string())?;

        let array_node_handle = nodes.find(container_node, data_name.as_ref())?;
        let array_node = nodes.get_by_name(array_node_handle, "a")?;

        let mut index = Vec::new();

        // This check is needed because FBX expects materials to be always IndexToDirect
        // See: https://developer.blender.org/D402
        if data_name.as_ref() != "Materials" {
            if reference == FbxReference::IndexToDirect {
                let index_node = nodes.find(
                    container_node,
                    format!("{}Index", data_name.as_ref()).as_str(),
                )?;
                let index_array_node = nodes.get_by_name(index_node, "a")?;
                for attribute in index_array_node.attributes() {
                    index.push(attribute.as_i32()?);
                }
            }
        } else {
            // As said earlier, this is actually direct mapping in case of Materials, so fix this.
            // Nice specification, Autodesk, very consistent, good job.
            reference = FbxReference::Direct;
        }

        Ok(Self {
            elements: mapper(array_node.attributes())?,
            index,
            mapping,
            reference,
        })
    }

    fn map_index(&self, index: usize) -> Result<usize, FbxError> {
        match self.reference {
            FbxReference::Direct => Ok(index),
            FbxReference::IndexToDirect => {
                Ok(*self.index.get(index).ok_or(FbxError::IndexOutOfBounds)? as usize)
            }
        }
    }

    /// Returns reference to element at given index. There are two kind of indices:
    /// 1) `index` - direct index of element
    /// 2) `index_in_polygon` - global index but relative to polygon. Such index is used
    /// mostly for normals because any mesh can contain few normals per vertex and such
    /// normals are unpacked into plain array. For example we have two polygons,
    ///
    /// A____B_____C
    /// |    |    |
    /// |    |    |
    /// |____|____|
    /// D    E    F
    ///
    /// As you can see these polygons share BE edge, vertices B and E can have two normal
    /// vectors if these faces are not coplanar. So to handle this case FBX stores unpacked
    /// array of normals - in this case 4 normals *per-face* like this (A,B,E,D,B*,C,F,E*).
    /// Situation may become worse if we have arbitrary polygon - it must be triangulated:
    ///
    /// A____B_____C
    /// |\   |\   |
    /// |  \ |  \ |
    /// |___\|___\|
    /// D    E    F
    ///
    /// So here we have AED, ABE, BFE, BCF triangles which has same set of normals as before.
    /// To correctly fetch normals from array after triangulation we have to do it like so:
    ///
    /// counter = 0
    /// Iterate over each initial *non-triangulated* polygon
    ///   triangles = triangulate polygon
    ///   for each triangle in triangles
    ///     for each index in triangle
    ///       fetch normal at (counter + index)
    ///   counter += polygon vertex count
    ///
    /// # Notes
    ///
    /// FBX uses a lot of optimizations to store data as compact as possible, so there are
    /// separate arrays of vertex positions and normals instead of array of vertices that has
    /// position *and* normal together. This  fact introduces a lot of head ache when you need
    /// to "inflate" such "packed" data in form that suitable for GPU.
    ///
    /// # Useful links
    ///
    /// Check this article:
    /// https://banexdevblog.wordpress.com/2014/06/23/a-quick-tutorial-about-the-fbx-ascii-format/
    /// it has some nice pictures which will clarify this mess.
    ///
    pub fn get(&self, index: usize, index_in_polygon: usize) -> Result<&T, FbxError> {
        Ok(match self.mapping {
            FbxMapping::ByPolygon | FbxMapping::ByVertex | FbxMapping::ByEdge => self
                .elements
                .get(self.map_index(index)?)
                .ok_or(FbxError::IndexOutOfBounds)
                .unwrap(),
            FbxMapping::ByPolygonVertex => self
                .elements
                .get(self.map_index(index_in_polygon).unwrap())
                .ok_or(FbxError::IndexOutOfBounds)
                .unwrap(),
            FbxMapping::AllSame => self
                .elements
                .first()
                .ok_or(FbxError::IndexOutOfBounds)
                .unwrap(),
        })
    }
}

pub fn make_vec3_container<P: AsRef<str>>(
    nodes: &FbxNodeContainer,
    container_node: Handle<FbxNode>,
    data_name: P,
) -> Result<FbxContainer<Vector3<f32>>, FbxError> {
    FbxContainer::new(nodes, container_node, data_name, |attributes| {
        let mut normals = Vec::with_capacity(attributes.len() / 3);
        for normal in attributes.chunks_exact(3) {
            normals.push(Vector3::new(
                normal[0].as_f32()?,
                normal[1].as_f32()?,
                normal[2].as_f32()?,
            ));
        }
        Ok(normals)
    })
}
