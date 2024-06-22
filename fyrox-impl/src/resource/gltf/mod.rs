//! [GltfLoader] enables the importing of *.gltf and *.glb files in the glTF format.
//! This requires the "gltf" feature.
use gltf::json;
use gltf::Document;
use gltf::Gltf;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::asset::io::ResourceIo;
use crate::asset::loader;
use crate::asset::manager::ResourceManager;
use crate::asset::options;
use crate::asset::state::LoadError;
use crate::core::algebra::{Matrix4, Unit};
use crate::core::log::Log;
use crate::core::pool::Handle;
use crate::core::TypeUuidProvider;
use crate::graph::BaseSceneGraph;
use crate::graph::NodeMapping;
use crate::gui::core::io::FileLoadError;
use crate::material::MaterialResource;
use crate::resource::model::{MaterialSearchOptions, Model, ModelImportOptions};
use crate::resource::texture::{TextureError, TextureResource};
use crate::scene::animation::{AnimationContainer, AnimationPlayerBuilder};
use crate::scene::base::BaseBuilder;
use crate::scene::graph::Graph;
use crate::scene::mesh::surface::{BlendShape, Surface, SurfaceResource};
use crate::scene::mesh::{Mesh, MeshBuilder};
use crate::scene::node::Node;
use crate::scene::pivot::PivotBuilder;
use crate::scene::transform::TransformBuilder;
use crate::scene::Scene;

mod animation;
mod iter;
mod material;
mod node_names;
mod simplify;
mod surface;
mod uri;

use animation::import_animations;
use fyrox_resource::untyped::ResourceKind;
use material::*;
pub use surface::SurfaceDataError;
use surface::{build_surface_data, BlendShapeInfoContainer, GeometryStatistics};
pub use uri::{parse_uri, Scheme, Uri};

type Result<T> = std::result::Result<T, GltfLoadError>;

#[cfg(feature = "gltf_blend_shapes")]
const TARGET_NAMES_KEY: &str = "targetNames";

#[derive(Debug)]
#[allow(dead_code)]
enum GltfLoadError {
    InvalidIndex,
    InvalidPath,
    UnsupportedURI(Box<str>),
    MissingEmbeddedBin,
    Gltf(gltf::Error),
    Texture(TextureError),
    File(FileLoadError),
    Base64(base64::DecodeError),
    Load(LoadError),
    Material(GltfMaterialError),
    Surface(SurfaceDataError),
    JSON(json::Error),
}

impl From<json::Error> for GltfLoadError {
    fn from(error: json::Error) -> Self {
        GltfLoadError::JSON(error)
    }
}

impl From<gltf::Error> for GltfLoadError {
    fn from(error: gltf::Error) -> Self {
        GltfLoadError::Gltf(error)
    }
}

impl From<TextureError> for GltfLoadError {
    fn from(error: TextureError) -> Self {
        GltfLoadError::Texture(error)
    }
}

impl From<FileLoadError> for GltfLoadError {
    fn from(error: FileLoadError) -> Self {
        GltfLoadError::File(error)
    }
}

impl From<LoadError> for GltfLoadError {
    fn from(error: LoadError) -> Self {
        GltfLoadError::Load(error)
    }
}

impl From<base64::DecodeError> for GltfLoadError {
    fn from(error: base64::DecodeError) -> Self {
        GltfLoadError::Base64(error)
    }
}

impl From<GltfMaterialError> for GltfLoadError {
    fn from(error: GltfMaterialError) -> Self {
        GltfLoadError::Material(error)
    }
}

impl From<SurfaceDataError> for GltfLoadError {
    fn from(error: SurfaceDataError) -> Self {
        GltfLoadError::Surface(error)
    }
}

fn decode_base64(source: &str) -> Result<Vec<u8>> {
    Ok(uri::decode_base64(source)?)
}

struct MeshData {
    surfaces: Vec<Surface>,
    blend_shapes: Vec<BlendShape>,
}

struct NodeFamily {
    main_node: Handle<Node>,
    bone_children: Vec<SkinNodePair>,
}

struct SkinNodePair {
    skin_index: usize,
    node: Handle<Node>,
}

type SkinData = Vec<SkinBone>;

#[derive(PartialEq, Debug, Clone)]
struct SkinBone {
    pub node_index: usize,
    pub inv_bind_pose: Matrix4<f32>,
}

impl From<(usize, Matrix4<f32>)> for SkinBone {
    fn from(pair: (usize, Matrix4<f32>)) -> Self {
        let (node_index, inv_bind_pose) = pair;
        SkinBone {
            node_index,
            inv_bind_pose,
        }
    }
}

#[derive(Clone, Copy)]
struct SkinBonePair<'a> {
    skin_index: usize,
    bone: &'a SkinBone,
}

struct SkinBoneIter<'a> {
    skin_index: usize,
    bone_index: usize,
    skin_list: &'a [SkinData],
}

impl<'a> SkinBoneIter<'a> {
    fn new(skin_list: &'a [SkinData]) -> SkinBoneIter<'a> {
        SkinBoneIter {
            skin_index: 0,
            bone_index: 0,
            skin_list,
        }
    }
}

impl<'a> Iterator for SkinBoneIter<'a> {
    type Item = SkinBonePair<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.skin_index >= self.skin_list.len() {
                return None;
            }
            let skin: &SkinData = &self.skin_list[self.skin_index];
            if self.bone_index >= skin.len() {
                self.bone_index = 0;
                self.skin_index += 1;
            } else {
                let bone = &skin[self.bone_index];
                self.bone_index += 1;
                return Some(SkinBonePair {
                    skin_index: self.skin_index,
                    bone,
                });
            }
        }
    }
}

struct ImportContext {
    io: Arc<dyn ResourceIo>,
    resource_manager: ResourceManager,
    model_path: PathBuf,
    search_options: MaterialSearchOptions,
}

impl ImportContext {
    fn as_texture_context(&self) -> TextureContext {
        TextureContext {
            resource_manager: &self.resource_manager,
            model_path: &self.model_path,
            search_options: &self.search_options,
        }
    }
}

#[derive(Default)]
struct ImportResults {
    buffers: Option<Vec<Vec<u8>>>,
    textures: Option<Vec<TextureResource>>,
    materials: Option<Vec<MaterialResource>>,
    skins: Option<Vec<SkinData>>,
    meshes: Option<Vec<MeshData>>,
    families: Option<Vec<NodeFamily>>,
}

impl ImportResults {
    fn get_buffer_data_access<'s>(
        &'s self,
    ) -> impl Clone + Fn(gltf::Buffer<'_>) -> Option<&'s [u8]> {
        |b| {
            self.buffers
                .as_ref()
                .unwrap()
                .get(b.index())
                .map(Vec::as_slice)
        }
    }
}

/// This object performs the loading of files in glTF format with extension "gltf" or "glb".
pub struct GltfLoader {
    /// ResourceManager is needed so that textures and mesh data can be loaded from additional resources.
    /// The glTF format allows for other assets to be referenced by file path.
    pub resource_manager: ResourceManager,
    /// Import options control where this loader should search for additional resources.
    pub default_import_options: ModelImportOptions,
}

impl loader::ResourceLoader for GltfLoader {
    fn extensions(&self) -> &[&str] {
        &["gltf", "glb"]
    }

    fn data_type_uuid(&self) -> crate::core::type_traits::prelude::Uuid {
        Model::type_uuid()
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> loader::BoxedLoaderFuture {
        let resource_manager = self.resource_manager.clone();
        let default_import_options = self.default_import_options.clone();

        Box::pin(async move {
            let import_options = options::try_get_import_settings(&path, io.as_ref())
                .await
                .unwrap_or(default_import_options);

            let model = load(path, io, resource_manager, import_options)
                .await
                .map_err(LoadError::new)?;

            Ok(loader::LoaderPayload::new(model))
        })
    }

    fn try_load_import_settings(
        &self,
        resource_path: PathBuf,
        io: Arc<dyn ResourceIo>,
    ) -> loader::BoxedImportOptionsLoaderFuture {
        Box::pin(async move {
            options::try_get_import_settings_opaque::<ModelImportOptions>(&resource_path, &*io)
                .await
        })
    }

    fn default_import_options(&self) -> Option<Box<dyn options::BaseImportOptions>> {
        Some(Box::<ModelImportOptions>::default())
    }
}

async fn load(
    path: PathBuf,
    io: Arc<dyn ResourceIo>,
    resource_manager: ResourceManager,
    options: ModelImportOptions,
) -> Result<Model> {
    let mut scene = Scene::new();
    let context = ImportContext {
        io,
        resource_manager,
        model_path: path.clone(),
        search_options: options.material_search_options,
    };
    let root_name = path
        .file_name()
        .ok_or(GltfLoadError::InvalidPath)?
        .to_string_lossy();
    let root = scene.graph.get_root();
    scene.graph[root].set_name(root_name.clone());
    import_from_path(&mut scene.graph, &context).await?;
    node_names::resolve_name_conflicts(context.model_path.as_path(), &mut scene.graph);
    Ok(Model::new(NodeMapping::UseNames, scene))
}

async fn import_from_path(graph: &mut Graph, context: &ImportContext) -> Result<()> {
    let file: Vec<u8> = context.io.load_file(context.model_path.as_path()).await?;
    import_from_slice(file.as_slice(), graph, context).await
}

async fn import_from_slice(slice: &[u8], graph: &mut Graph, context: &ImportContext) -> Result<()> {
    let gltf: Gltf = Gltf::from_slice(slice)?;
    let doc = gltf.document;
    let data = gltf.blob;
    let mut imports: ImportResults = ImportResults {
        buffers: Some(import_buffers(&doc, data, context).await?),
        ..Default::default()
    };
    let buffers: &[Vec<u8>] = imports.buffers.as_ref().unwrap().as_slice();
    let images: Vec<SourceImage> = import_images(&doc, buffers)?;
    imports.textures =
        Some(import_textures(&doc, images.as_slice(), context.as_texture_context()).await?);
    let textures = imports.textures.as_ref().unwrap().as_slice();
    imports.materials = Some(import_materials(&doc, textures, &context.resource_manager).await?);
    let materials = imports.materials.as_ref().unwrap().as_slice();
    imports.skins = Some(import_skins(&doc, &imports)?);
    imports.meshes = Some(import_meshes(
        &doc,
        &context.model_path,
        materials,
        buffers,
    )?);
    imports.families = Some(import_nodes(&doc, graph, &imports)?);
    link_child_nodes(&doc, graph, &imports)?;
    let node_handles: Vec<Handle<Node>> = imports
        .families
        .as_ref()
        .unwrap()
        .iter()
        .map(|f| f.main_node)
        .collect();
    let animations = import_animations(&doc, &node_handles, graph, buffers);
    if !animations.is_empty() {
        let mut anim_con = AnimationContainer::new();
        for animation in animations {
            anim_con.add(animation);
        }
        AnimationPlayerBuilder::new(BaseBuilder::new().with_name("AnimationPlayer"))
            .with_animations(anim_con)
            .build(graph);
    }
    Ok(())
}

async fn import_buffers(
    gltf: &Document,
    mut data_chunk: Option<Vec<u8>>,
    context: &ImportContext,
) -> Result<Vec<Vec<u8>>> {
    let mut result: Vec<Vec<u8>> = Vec::with_capacity(gltf.buffers().len());
    for buf in gltf.buffers() {
        match buf.source() {
            gltf::buffer::Source::Bin => match data_chunk.take() {
                Some(data) => result.push(data),
                None => {
                    return Err(GltfLoadError::MissingEmbeddedBin);
                }
            },
            gltf::buffer::Source::Uri(uri) => result.push(load_bin_from_uri(uri, context).await?),
        }
    }
    Ok(result)
}

async fn load_bin_from_uri(uri: &str, context: &ImportContext) -> Result<Vec<u8>> {
    let parsed_uri = uri::parse_uri(uri);
    match parsed_uri.scheme {
        uri::Scheme::Data if parsed_uri.data.is_some() => {
            Ok(decode_base64(parsed_uri.data.unwrap())?)
        }
        uri::Scheme::None => load_external_bin(uri, context).await,
        _ => Err(GltfLoadError::UnsupportedURI(uri.into())),
    }
}

async fn load_external_bin(path: &str, context: &ImportContext) -> Result<Vec<u8>> {
    let parent = context
        .model_path
        .parent()
        .ok_or(GltfLoadError::InvalidPath)?
        .to_owned();
    let path = parent.join(path);
    Ok(context.io.load_file(&path).await?)
}

fn import_meshes(
    gltf: &Document,
    path: &Path,
    mats: &[MaterialResource],
    bufs: &[Vec<u8>],
) -> Result<Vec<MeshData>> {
    let mut result: Vec<MeshData> = Vec::with_capacity(gltf.nodes().len());
    let mut stats = GeometryStatistics::default();
    for node in gltf.nodes() {
        if let Some(mesh) = node.mesh() {
            result.push(import_mesh(mesh, mats, bufs, path, &mut stats)?);
        }
    }
    if cfg!(feature = "mesh_analysis") {
        if stats.repeated_index_count > 0 {
            Log::err(format!(
                "{}: Model has triangles with repeated vertices: {}",
                path.to_string_lossy(),
                stats.repeated_index_count
            ));
        }
        let min_length = stats.min_edge_length();
        if min_length == 0.0 {
            Log::err(format!(
                "{}: Mesh has a triangle with a zero-length edge!",
                path.to_string_lossy()
            ));
        } else if min_length <= f32::EPSILON {
            Log::err(format!(
                "{}: Mesh has a triangle with edge length: {}",
                path.to_string_lossy(),
                min_length
            ));
        } else {
            Log::info(format!(
                "{}: Smallest triangle edge: {}",
                path.to_string_lossy(),
                min_length
            ));
        }
    }
    Ok(result)
}

fn import_mesh(
    mesh: gltf::Mesh,
    mats: &[MaterialResource],
    bufs: &[Vec<u8>],
    path: &Path,
    stats: &mut GeometryStatistics,
) -> Result<MeshData> {
    #[cfg(feature = "gltf_blend_shapes")]
    let morph_info = import_morph_info(&mesh)?;
    #[cfg(not(feature = "gltf_blend_shapes"))]
    let morph_info = BlendShapeInfoContainer::default();
    let mut surfs: Vec<Surface> = Vec::with_capacity(mesh.primitives().len());
    let mut blend_shapes: Option<Vec<BlendShape>> = None;
    for prim in mesh.primitives() {
        if let Some((surf, shapes)) = import_surface(prim, &morph_info, mats, bufs, path, stats)? {
            surfs.push(surf);
            blend_shapes.get_or_insert(shapes);
        }
    }
    Ok(MeshData {
        surfaces: surfs,
        blend_shapes: blend_shapes.unwrap_or_default(),
    })
}

#[cfg(feature = "gltf_blend_shapes")]
fn import_morph_info(mesh: &gltf::Mesh) -> Result<BlendShapeInfoContainer> {
    let weights: &[f32] = mesh.weights().unwrap_or_default();
    let weights: Vec<f32> = weights.iter().map(|w| w * 100.0).collect();
    let extras = mesh.extras();
    let names = if let Some(extras) = extras {
        let extras: json::Value = json::deserialize::from_str(extras.get())?;
        match extras {
            json::Value::Object(map) => {
                if let Some(names) = map.get(TARGET_NAMES_KEY) {
                    match names {
                        json::Value::Array(names) => {
                            values_to_strings(names.as_slice()).unwrap_or_default()
                        }
                        _ => Vec::default(),
                    }
                } else {
                    Vec::default()
                }
            }
            _ => Vec::default(),
        }
    } else {
        Vec::default()
    };
    if extras.is_some() && names.is_empty() {
        Log::warn(format!(
            "glTF: Unable to extract blend shape names from JSON: {}",
            extras.as_ref().unwrap().get()
        ));
    }
    Ok(BlendShapeInfoContainer::new(names, weights))
}

#[cfg(feature = "gltf_blend_shapes")]
fn values_to_strings(values: &[json::Value]) -> Option<Vec<String>> {
    let mut result: Vec<String> = Vec::with_capacity(values.len());
    for v in values {
        if let json::Value::String(str) = v {
            result.push(str.clone());
        } else {
            return None;
        }
    }
    Some(result)
}

fn import_surface(
    prim: gltf::Primitive,
    morph_info: &BlendShapeInfoContainer,
    mats: &[MaterialResource],
    bufs: &[Vec<u8>],
    path: &Path,
    stats: &mut GeometryStatistics,
) -> Result<Option<(Surface, Vec<BlendShape>)>> {
    if let Some(data) = build_surface_data(&prim, morph_info, bufs, stats)? {
        let mut blend_shapes = Vec::new();
        if let Some(shape_con) = data.blend_shapes_container.as_ref() {
            blend_shapes.clone_from(&shape_con.blend_shapes)
        }
        let mut surf = Surface::new(SurfaceResource::new_ok(
            ResourceKind::External(path.to_path_buf()),
            data,
        ));
        if let Some(mat_index) = prim.material().index() {
            surf.set_material(
                mats.get(mat_index)
                    .ok_or(GltfLoadError::InvalidIndex)?
                    .clone(),
            );
            Ok(Some((surf, blend_shapes)))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

fn import_nodes(
    doc: &gltf::Document,
    graph: &mut Graph,
    imports: &ImportResults,
) -> Result<Vec<NodeFamily>> {
    let skins: &[SkinData] = imports.skins.as_ref().unwrap().as_slice();
    let mut result: Vec<NodeFamily> = Vec::with_capacity(doc.nodes().len());
    for node in doc.nodes() {
        result.push(build_node_family(&node, skins, graph, imports)?);
    }
    for node in doc.nodes() {
        let family: &NodeFamily = result
            .get(node.index())
            .ok_or(GltfLoadError::InvalidIndex)?;
        if let Some(mesh) = graph[family.main_node].cast_mut::<Mesh>() {
            assign_bones_to_surfaces(node, mesh, result.as_slice())?;
        }
    }
    Ok(result)
}

fn build_node_family(
    node: &gltf::Node,
    skins: &[SkinData],
    graph: &mut Graph,
    imports: &ImportResults,
) -> Result<NodeFamily> {
    let node_index = node.index();
    let skin_iter = SkinBoneIter::new(skins).filter(move |sb| sb.bone.node_index == node_index);
    let mut bones: Vec<SkinBonePair> = Vec::new();
    let mut new_handle: Option<Handle<Node>> = None;
    let mut bone_children: Vec<SkinNodePair> = Vec::new();
    let name = node.name().unwrap_or("");
    for pair in skin_iter {
        if bones.is_empty() {
            // Our first bone. Set the inv_bind_pose of the main node.
            // We only create children if we later find an inv_bind_pose that does not match this.
            let new_node = import_node(node, pair.bone.inv_bind_pose, imports)?;
            new_handle = Some(graph.add_node(new_node));
            // Record that we have seen this inv_bind_pose.
            bones.push(pair);
        } else if let Some(p) = bones.iter().find(|p| p.bone == pair.bone) {
            // We have a previously existing bone with the exact same inv_bind_pose.
            // Do not record having seen this inv_bind_pose.
            // Let the already recorded inv_bind_pose stand in for all bones with this inv_bind_pose.
            let prev_skin_index = p.skin_index;
            if let Some(prev_pair) = bone_children
                .iter()
                .find(|b| b.skin_index == prev_skin_index)
            {
                // We have a child for the skin of the previously existing bone. Re-use that child for this skin.
                bone_children.push(SkinNodePair {
                    skin_index: pair.skin_index,
                    node: prev_pair.node,
                });
            }
            // Otherwise, the previously existing bone must be using the main node, so do nothing.
        } else {
            // We have a never-seen-before inv_bind_pose, so create a child for that inv_bind_pose.
            let skin_index = pair.skin_index;
            let base_builder = BaseBuilder::new()
                .with_name(format!("{}:{}", name, skin_index))
                .with_inv_bind_pose_transform(pair.bone.inv_bind_pose);
            let handle: Handle<Node> = graph.add_node(PivotBuilder::new(base_builder).build_node());
            bone_children.push(SkinNodePair {
                skin_index,
                node: handle,
            });
            graph.link_nodes(handle, new_handle.unwrap());
            // Record that we have seen this inv_bind_pose.
            bones.push(pair);
        }
    }
    if let Some(handle) = new_handle {
        Ok(NodeFamily {
            main_node: handle,
            bone_children,
        })
    } else {
        Ok(NodeFamily {
            main_node: graph.add_node(import_node(node, Matrix4::identity(), imports)?),
            bone_children: Vec::new(),
        })
    }
}

fn assign_bones_to_surfaces(
    node: gltf::Node,
    mesh: &mut Mesh,
    families: &[NodeFamily],
) -> Result<()> {
    if let Some(skin) = node.skin() {
        let skin_index = skin.index();
        let mut bones: Vec<Handle<Node>> = Vec::with_capacity(skin.joints().len());
        for joint in skin.joints() {
            let joint_family = families
                .get(joint.index())
                .ok_or(GltfLoadError::InvalidIndex)?;
            let bone_children = &joint_family.bone_children;
            let handle =
                if let Some(pair) = bone_children.iter().find(|p| p.skin_index == skin_index) {
                    pair.node
                } else {
                    joint_family.main_node
                };
            bones.push(handle);
        }
        for surf in mesh.surfaces_mut() {
            surf.bones.set_value_and_mark_modified(bones.clone());
        }
    }
    Ok(())
}

fn import_node(
    node: &gltf::Node,
    inv_bind_pose: Matrix4<f32>,
    imports: &ImportResults,
) -> Result<Node> {
    let meshes: &[MeshData] = imports.meshes.as_ref().unwrap().as_slice();
    let trans = node.transform().decomposed();
    let trans_builder: TransformBuilder = TransformBuilder::new()
        .with_local_position(trans.0.into())
        .with_local_rotation(Unit::new_normalize(trans.1.into()))
        .with_local_scale(trans.2.into());
    let name = node.name().unwrap_or("");
    let base_builder = BaseBuilder::new()
        .with_name(name)
        .with_local_transform(trans_builder.build())
        .with_inv_bind_pose_transform(inv_bind_pose);
    if let Some(mesh) = node.mesh() {
        let mut mesh_builder = MeshBuilder::new(base_builder);
        let mesh = meshes
            .get(mesh.index())
            .ok_or(GltfLoadError::InvalidIndex)?;
        mesh_builder = mesh_builder.with_blend_shapes(mesh.blend_shapes.clone());
        mesh_builder = mesh_builder.with_surfaces(mesh.surfaces.clone());
        Ok(mesh_builder.build_node())
    } else {
        Ok(PivotBuilder::new(base_builder).build_node())
    }
}

fn link_child_nodes(doc: &Document, graph: &mut Graph, imports: &ImportResults) -> Result<()> {
    let families: &[NodeFamily] = imports.families.as_ref().unwrap().as_slice();
    for node in doc.nodes() {
        let parent_family = families
            .get(node.index())
            .ok_or(GltfLoadError::InvalidIndex)?;
        for child in node.children() {
            let child_family = families
                .get(child.index())
                .ok_or(GltfLoadError::InvalidIndex)?;
            graph.link_nodes(child_family.main_node, parent_family.main_node);
        }
    }
    Ok(())
}

fn import_skins(doc: &gltf::Document, imports: &ImportResults) -> Result<Vec<SkinData>> {
    let mut result: Vec<SkinData> = Vec::with_capacity(doc.skins().len());
    for skin in doc.skins() {
        let bone_node_indices = skin.joints().map(|n| n.index());
        let bone_pairs = {
            let reader = skin.reader(imports.get_buffer_data_access());
            if let Some(iter) = reader.read_inverse_bind_matrices() {
                bone_node_indices
                    .zip(iter.map(Matrix4::from))
                    .map(SkinBone::from)
                    .collect()
            } else {
                bone_node_indices
                    .zip(std::iter::repeat(Matrix4::identity()))
                    .map(SkinBone::from)
                    .collect()
            }
        };
        result.push(bone_pairs);
    }
    Ok(result)
}
