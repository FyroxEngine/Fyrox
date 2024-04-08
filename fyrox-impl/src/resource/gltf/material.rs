use std::path::{Path, PathBuf};

use crate::{
    asset::{manager::ResourceManager, state::LoadError, untyped::ResourceKind, Resource},
    core::{algebra::Vector4, color::Color, log::Log, sstorage::ImmutableString},
    material::{
        shader::{SamplerFallback, Shader, ShaderResource},
        Material, MaterialResource, PropertyValue,
    },
    resource::{
        model::MaterialSearchOptions,
        texture::{Texture, TextureError, TextureImportOptions, TextureResource},
    },
};
use gltf::{buffer::View, image, Document};
use lazy_static::lazy_static;

use super::uri;

type Result<T> = std::result::Result<T, GltfMaterialError>;

use crate::resource::texture::TextureMagnificationFilter as FyroxMagFilter;
use crate::resource::texture::TextureMinificationFilter as FyroxMinFilter;
use gltf::texture::MagFilter as GltfMagFilter;
use gltf::texture::MinFilter as GltfMinFilter;

pub const SHADER_NAME: &str = "glTF Shader";
pub const SHADER_SRC: &str = include_str!("gltf_standard.shader");

lazy_static! {
    static ref GLTF_SHADER: ShaderResource =
        ShaderResource::new_ok(SHADER_NAME.into(), Shader::from_string(SHADER_SRC).unwrap());
}

fn convert_mini(filter: GltfMinFilter) -> FyroxMinFilter {
    match filter {
        GltfMinFilter::Linear => FyroxMinFilter::Linear,
        GltfMinFilter::Nearest => FyroxMinFilter::Nearest,
        GltfMinFilter::LinearMipmapLinear => FyroxMinFilter::LinearMipMapLinear,
        GltfMinFilter::NearestMipmapLinear => FyroxMinFilter::NearestMipMapLinear,
        GltfMinFilter::LinearMipmapNearest => FyroxMinFilter::LinearMipMapNearest,
        GltfMinFilter::NearestMipmapNearest => FyroxMinFilter::NearestMipMapNearest,
    }
}

fn convert_mag(filter: GltfMagFilter) -> FyroxMagFilter {
    match filter {
        GltfMagFilter::Linear => FyroxMagFilter::Linear,
        GltfMagFilter::Nearest => FyroxMagFilter::Nearest,
    }
}

use crate::resource::texture::TextureWrapMode as FyroxWrapMode;
use gltf::texture::WrappingMode as GltfWrapMode;

fn convert_wrap(mode: GltfWrapMode) -> FyroxWrapMode {
    match mode {
        GltfWrapMode::Repeat => FyroxWrapMode::Repeat,
        GltfWrapMode::ClampToEdge => FyroxWrapMode::ClampToEdge,
        GltfWrapMode::MirroredRepeat => FyroxWrapMode::MirroredRepeat,
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum GltfMaterialError {
    ShaderLoadFailed,
    InvalidIndex,
    UnsupportedURI(Box<str>),
    TextureNotFound(Box<str>),
    Load(LoadError),
    Base64(base64::DecodeError),
    Texture(TextureError),
}

impl From<LoadError> for GltfMaterialError {
    fn from(error: LoadError) -> Self {
        GltfMaterialError::Load(error)
    }
}

impl From<base64::DecodeError> for GltfMaterialError {
    fn from(error: base64::DecodeError) -> Self {
        GltfMaterialError::Base64(error)
    }
}

impl From<TextureError> for GltfMaterialError {
    fn from(error: TextureError) -> Self {
        GltfMaterialError::Texture(error)
    }
}

pub enum SourceImage<'a> {
    External(&'a str),
    View(&'a [u8]),
    Embedded(Vec<u8>),
}

pub fn decode_base64(source: &str) -> Result<Vec<u8>> {
    Ok(uri::decode_base64(source)?)
}

/// Extract a list of [MaterialResource] from the give glTF document, if that document contains any.
/// The resulting list of materials is guaranteed to be the same length as the list of materials
/// in the document so that an index into the document's list of materials will be the same as the index
/// of the matching MaterialResource in the returned list. This is important since the glTF document
/// refers to materials by index.
///
/// * `doc`: The document in which to find the materials.
///
/// * `textures`: A slice containing a [TextureResource] for every texture defined in the document, in that order, so that
/// a texture can be looked up using the index of a texture within the document. Materials in glTF specify their target
/// textures by their index within the node list of the document, and these indices need to be translated into handles.
///
/// * `buffers`: A slice containing a list of byte-vectors, one for each buffer in the glTF document.
/// Animations in glTF make reference to data stored in the document's list of buffers by index.
/// This slcie allows an index into the document's list of buffers to be translated into actual bytes of data.
///
/// * `resource_manager`: A [ResourceManager] makes it possible to access shaders and create materials.
pub async fn import_materials(
    gltf: &Document,
    textures: &[TextureResource],
    resource_manager: &ResourceManager,
) -> Result<Vec<MaterialResource>> {
    let mut result: Vec<MaterialResource> = Vec::with_capacity(gltf.materials().len());
    for mat in gltf.materials() {
        match import_material(mat, textures, resource_manager).await {
            Ok(res) => result.push(res),
            Err(err) => {
                Log::err(format!("glTF material failed to import. Reason: {:?}", err));
                result.push(MaterialResource::new_ok(
                    ResourceKind::Embedded,
                    Material::default(),
                ));
            }
        }
    }
    Ok(result)
}

async fn import_material(
    mat: gltf::Material<'_>,
    textures: &[TextureResource],
    resource_manager: &ResourceManager,
) -> Result<MaterialResource> {
    let shader: ShaderResource = GLTF_SHADER.clone(); //resource_manager.request(SHADER_PATH).await?;
    if !shader.is_ok() {
        return Err(GltfMaterialError::ShaderLoadFailed);
    }
    let mut result: Material = Material::from_shader(shader, Some(resource_manager.clone()));
    let pbr = mat.pbr_metallic_roughness();
    if let Some(tex) = pbr.base_color_texture() {
        set_texture(
            &mut result,
            "diffuseTexture",
            textures,
            tex.texture().index(),
            SamplerFallback::White,
        )?;
    }
    if let Some(tex) = mat.normal_texture() {
        set_texture(
            &mut result,
            "normalTexture",
            textures,
            tex.texture().index(),
            SamplerFallback::Normal,
        )?;
    }
    if let Some(tex) = pbr.metallic_roughness_texture() {
        set_texture(
            &mut result,
            "metallicRoughnessTexture",
            textures,
            tex.texture().index(),
            SamplerFallback::White,
        )?;
    }
    if let Some(tex) = mat.emissive_texture() {
        set_texture(
            &mut result,
            "emissionTexture",
            textures,
            tex.texture().index(),
            SamplerFallback::Black,
        )?;
    }
    if let Some(tex) = mat.occlusion_texture() {
        set_texture(
            &mut result,
            "aoTexture",
            textures,
            tex.texture().index(),
            SamplerFallback::White,
        )?;
    }
    set_material_color(
        &mut result,
        "diffuseColor",
        Vector4::<f32>::from(pbr.base_color_factor()).into(),
    )?;
    set_material_vector3(&mut result, "emissionStrength", mat.emissive_factor())?;
    set_material_scalar(&mut result, "metallicFactor", pbr.metallic_factor())?;
    set_material_scalar(&mut result, "roughnessFactor", pbr.roughness_factor())?;
    Ok(Resource::new_ok(ResourceKind::Embedded, result))
}

fn set_material_scalar(material: &mut Material, name: &'static str, value: f32) -> Result<()> {
    let value: PropertyValue = PropertyValue::Float(value);
    match material.set_property(&ImmutableString::new(name), value) {
        Ok(()) => Ok(()),
        Err(err) => {
            Log::err(format!(
                "Unable to set material property {} for glTF material! Reason: {:?}",
                name, err
            ));
            Ok(())
        }
    }
}

fn set_material_color(material: &mut Material, name: &'static str, color: Color) -> Result<()> {
    let value: PropertyValue = PropertyValue::Color(color);
    match material.set_property(&ImmutableString::new(name), value) {
        Ok(()) => Ok(()),
        Err(err) => {
            Log::err(format!(
                "Unable to set material property {} for GLTF material! Reason: {:?}",
                name, err
            ));
            Ok(())
        }
    }
}

fn set_material_vector3(
    material: &mut Material,
    name: &'static str,
    vector: [f32; 3],
) -> Result<()> {
    let value: PropertyValue = PropertyValue::Vector3(vector.into());
    match material.set_property(&ImmutableString::new(name), value) {
        Ok(()) => Ok(()),
        Err(err) => {
            Log::err(format!(
                "Unable to set material property {} for GLTF material! Reason: {:?}",
                name, err
            ));
            Ok(())
        }
    }
}

#[allow(dead_code)]
fn set_material_vector4(
    material: &mut Material,
    name: &'static str,
    vector: [f32; 4],
) -> Result<()> {
    let value: PropertyValue = PropertyValue::Vector4(vector.into());
    match material.set_property(&ImmutableString::new(name), value) {
        Ok(()) => Ok(()),
        Err(err) => {
            Log::err(format!(
                "Unable to set material property {} for GLTF material! Reason: {:?}",
                name, err
            ));
            Ok(())
        }
    }
}

fn set_texture(
    material: &mut Material,
    name: &'static str,
    textures: &[TextureResource],
    index: usize,
    fallback: SamplerFallback,
) -> Result<()> {
    let tex: TextureResource = textures
        .get(index)
        .ok_or(GltfMaterialError::InvalidIndex)?
        .clone();
    match material.set_property(
        &ImmutableString::new(name),
        PropertyValue::Sampler {
            value: Some(tex),
            fallback,
        },
    ) {
        Ok(()) => Ok(()),
        Err(err) => {
            Log::err(format!(
                "Unable to set material property {} for GLTF material! Reason: {:?}",
                name, err
            ));
            Ok(())
        }
    }
}

pub fn import_images<'a, 'b>(
    gltf: &'a Document,
    buffers: &'b [Vec<u8>],
) -> Result<Vec<SourceImage<'b>>>
where
    'a: 'b,
{
    let mut result: Vec<SourceImage> = Vec::new();
    for image in gltf.images() {
        match image.source() {
            image::Source::Uri { uri, mime_type: _ } => result.push(import_image_from_uri(uri)?),
            image::Source::View { view, mime_type: _ } => {
                result.push(import_image_from_view(view, buffers)?)
            }
        }
    }
    Ok(result)
}

fn import_image_from_uri(uri: &str) -> Result<SourceImage> {
    let parsed_uri = uri::parse_uri(uri);
    match parsed_uri.scheme {
        uri::Scheme::Data if parsed_uri.data.is_some() => Ok(SourceImage::Embedded(decode_base64(
            parsed_uri.data.unwrap(),
        )?)),
        uri::Scheme::None => Ok(SourceImage::External(uri)),
        _ => Err(GltfMaterialError::UnsupportedURI(uri.into())),
    }
}

fn import_image_from_view<'a>(view: View, buffers: &'a [Vec<u8>]) -> Result<SourceImage<'a>> {
    let offset: usize = view.offset();
    let length: usize = view.length();
    let buf: &Vec<u8> = buffers
        .get(view.buffer().index())
        .ok_or(GltfMaterialError::InvalidIndex)?;
    Ok(SourceImage::View(&buf[offset..offset + length]))
}

pub struct TextureContext<'a> {
    pub resource_manager: &'a ResourceManager,
    pub model_path: &'a Path,
    pub search_options: &'a MaterialSearchOptions,
}

pub async fn import_textures<'a>(
    gltf: &'a Document,
    images: &[SourceImage<'a>],
    context: TextureContext<'a>,
) -> Result<Vec<TextureResource>> {
    let mut result: Vec<TextureResource> = Vec::with_capacity(gltf.textures().len());
    for tex in gltf.textures() {
        let sampler = tex.sampler();
        let source = tex.source();
        let image = images
            .get(source.index())
            .ok_or(GltfMaterialError::InvalidIndex)?;
        match image {
            SourceImage::Embedded(data) => result.push(import_embedded_texture(sampler, data)?),
            SourceImage::View(data) => result.push(import_embedded_texture(sampler, data)?),
            SourceImage::External(filename) => {
                import_external_texture(filename, &context).await?;
            } // result.push(import_external_texture(filename, &context).await?),
        }
    }
    Ok(result)
}

fn import_embedded_texture(
    sampler: gltf::texture::Sampler,
    data: &[u8],
) -> Result<TextureResource> {
    let mut options = TextureImportOptions::default();
    if let Some(filter) = sampler.min_filter() {
        options.set_minification_filter(convert_mini(filter));
    }
    if let Some(filter) = sampler.mag_filter() {
        options.set_magnification_filter(convert_mag(filter));
    }
    options.set_s_wrap_mode(convert_wrap(sampler.wrap_s()));
    options.set_t_wrap_mode(convert_wrap(sampler.wrap_t()));
    let tex = Texture::load_from_memory(data, options)?;
    Ok(Resource::new_ok(ResourceKind::Embedded, tex))
}

async fn import_external_texture(
    filename: &str,
    context: &TextureContext<'_>,
) -> Result<TextureResource> {
    let path = search_for_path(filename, context)
        .await
        .ok_or_else(|| GltfMaterialError::TextureNotFound(filename.into()))?;
    Ok(context.resource_manager.request(path))
}

async fn search_for_path(filename: &str, context: &TextureContext<'_>) -> Option<PathBuf> {
    match context.search_options {
        MaterialSearchOptions::MaterialsDirectory(ref directory) => Some(directory.join(filename)),
        MaterialSearchOptions::RecursiveUp => {
            let io = context.resource_manager.resource_io();
            let mut texture_path = None;
            let mut path: PathBuf = context.model_path.to_owned();
            while let Some(parent) = path.parent() {
                let candidate = parent.join(filename);
                if io.exists(&candidate).await {
                    texture_path = Some(candidate);
                    break;
                }
                path.pop();
            }
            texture_path
        }
        MaterialSearchOptions::WorkingDirectory => {
            let io = context.resource_manager.resource_io();
            let mut texture_path = None;
            let path = Path::new(".");
            if let Ok(iter) = io.walk_directory(path).await {
                for dir in iter {
                    if io.is_dir(&dir).await {
                        let candidate = dir.join(filename);
                        if candidate.exists() {
                            texture_path = Some(candidate);
                            break;
                        }
                    }
                }
            }
            texture_path
        }
        MaterialSearchOptions::UsePathDirectly => Some(filename.into()),
    }
}
