//! Module to generate lightmaps for surfaces.
//!
//! # Performance
//!
//! This is CPU lightmapper, its performance is linear with core count of your CPU.

#![forbid(unsafe_code)]

use crate::{
    asset::manager::{ResourceManager, ResourceRegistrationError},
    core::{
        algebra::{Matrix3, Matrix4, Point3, Vector2, Vector3},
        math::{Matrix4Ext, TriangleDefinition},
        pool::Handle,
        reflect::prelude::*,
        sstorage::ImmutableString,
        visitor::{prelude::*, BinaryBlob},
    },
    graph::SceneGraph,
    material::PropertyValue,
    resource::texture::{Texture, TextureKind, TexturePixelKind, TextureResource},
    scene::{
        light::{directional::DirectionalLight, point::PointLight, spot::SpotLight},
        mesh::{
            buffer::{
                VertexAttributeDataType, VertexAttributeDescriptor, VertexAttributeUsage,
                VertexFetchError, VertexReadTrait, VertexWriteTrait,
            },
            surface::{SurfaceData, SurfaceResource},
            Mesh,
        },
        node::Node,
        Scene,
    },
    utils::{uvgen, uvgen::SurfaceDataPatch},
};
use fxhash::FxHashMap;
use lightmap::light::{
    DirectionalLightDefinition, LightDefinition, PointLightDefinition, SpotLightDefinition,
};
use rayon::prelude::*;
use std::{
    fmt::{Display, Formatter},
    ops::Deref,
    path::Path,
    sync::{
        atomic::{self, AtomicBool, AtomicU32},
        Arc,
    },
};

/// Applies surface data patch to a surface data.
pub fn apply_surface_data_patch(data: &mut SurfaceData, patch: &SurfaceDataPatch) {
    if !data
        .vertex_buffer
        .has_attribute(VertexAttributeUsage::TexCoord1)
    {
        data.vertex_buffer
            .modify()
            .add_attribute(
                VertexAttributeDescriptor {
                    usage: VertexAttributeUsage::TexCoord1,
                    data_type: VertexAttributeDataType::F32,
                    size: 2,
                    divisor: 0,
                    shader_location: 6, // HACK: GBuffer renderer expects it to be at 6
                    normalized: false,
                },
                Vector2::<f32>::default(),
            )
            .unwrap();
    }

    data.geometry_buffer.set_triangles(
        patch
            .triangles
            .iter()
            .map(|t| TriangleDefinition(*t))
            .collect::<Vec<_>>(),
    );

    let mut vertex_buffer_mut = data.vertex_buffer.modify();
    for &v in patch.additional_vertices.iter() {
        vertex_buffer_mut.duplicate(v as usize);
    }

    assert_eq!(
        vertex_buffer_mut.vertex_count() as usize,
        patch.second_tex_coords.len()
    );
    for (mut view, &tex_coord) in vertex_buffer_mut
        .iter_mut()
        .zip(patch.second_tex_coords.iter())
    {
        view.write_2_f32(VertexAttributeUsage::TexCoord1, tex_coord)
            .unwrap();
    }
}

/// Lightmap entry.
#[derive(Default, Clone, Debug, Visit, Reflect)]
pub struct LightmapEntry {
    /// Lightmap texture.
    ///
    /// TODO: Is single texture enough? There may be surfaces with huge amount of faces
    ///  which may not fit into texture, because there is hardware limit on most GPUs
    ///  up to 8192x8192 pixels.
    pub texture: Option<TextureResource>,
    /// List of lights that were used to generate this lightmap. This list is used for
    /// masking when applying dynamic lights for surfaces with light, it prevents double
    /// lighting.
    pub lights: Vec<Handle<Node>>,
}

#[doc(hidden)]
#[derive(Default, Debug, Clone)]
pub struct SurfaceDataPatchWrapper(pub SurfaceDataPatch);

impl Visit for SurfaceDataPatchWrapper {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.0.data_id.visit("DataId", &mut region)?;
        BinaryBlob {
            vec: &mut self.0.triangles,
        }
        .visit("Triangles", &mut region)?;
        BinaryBlob {
            vec: &mut self.0.second_tex_coords,
        }
        .visit("SecondTexCoords", &mut region)?;
        BinaryBlob {
            vec: &mut self.0.additional_vertices,
        }
        .visit("AdditionalVertices", &mut region)?;

        Ok(())
    }
}

/// Lightmap is a texture with precomputed lighting.
#[derive(Default, Clone, Debug, Visit, Reflect)]
pub struct Lightmap {
    /// Node handle to lightmap mapping. It is used to quickly get information about
    /// lightmaps for any node in scene.
    pub map: FxHashMap<Handle<Node>, Vec<LightmapEntry>>,

    /// List of surface data patches. Each patch will be applied to corresponding
    /// surface data on resolve stage.
    // We don't need to inspect patches, because they contain no useful data.
    #[reflect(hidden)]
    pub patches: FxHashMap<u64, SurfaceDataPatchWrapper>,
}

struct Instance {
    owner: Handle<Node>,
    source_data: SurfaceResource,
    data: Option<lightmap::input::Mesh>,
    transform: Matrix4<f32>,
}

/// Small helper that allows you stop lightmap generation in any time.
#[derive(Clone, Default)]
pub struct CancellationToken(pub Arc<AtomicBool>);

impl CancellationToken {
    /// Creates new cancellation token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks if generation was cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(atomic::Ordering::SeqCst)
    }

    /// Raises cancellation flag, actual cancellation is not immediate!
    pub fn cancel(&self) {
        self.0.store(true, atomic::Ordering::SeqCst)
    }
}

/// Lightmap generation stage.
#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq)]
#[repr(u32)]
pub enum ProgressStage {
    /// Gathering info about lights, doing precalculations.
    LightsCaching = 0,
    /// Generating secondary texture coordinates.
    UvGeneration = 1,
    /// Caching geometry, building octrees.
    GeometryCaching = 2,
    /// Actual lightmap generation.
    CalculatingLight = 3,
}

impl Display for ProgressStage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ProgressStage::LightsCaching => {
                write!(f, "Caching Lights")
            }
            ProgressStage::UvGeneration => {
                write!(f, "Generating UVs")
            }
            ProgressStage::GeometryCaching => {
                write!(f, "Caching Geometry")
            }
            ProgressStage::CalculatingLight => {
                write!(f, "Calculating Light")
            }
        }
    }
}

/// Progress internals.
#[derive(Default)]
pub struct ProgressData {
    stage: AtomicU32,
    // Range is [0; max_iterations]
    progress: AtomicU32,
    max_iterations: AtomicU32,
}

impl ProgressData {
    /// Returns progress percentage in [0; 100] range.
    pub fn progress_percent(&self) -> u32 {
        let iterations = self.max_iterations.load(atomic::Ordering::SeqCst);
        if iterations > 0 {
            self.progress.load(atomic::Ordering::SeqCst) * 100 / iterations
        } else {
            0
        }
    }

    /// Returns current stage.
    pub fn stage(&self) -> ProgressStage {
        match self.stage.load(atomic::Ordering::SeqCst) {
            0 => ProgressStage::LightsCaching,
            1 => ProgressStage::UvGeneration,
            2 => ProgressStage::GeometryCaching,
            3 => ProgressStage::CalculatingLight,
            _ => unreachable!(),
        }
    }

    /// Sets new stage with max iterations per stage.
    fn set_stage(&self, stage: ProgressStage, max_iterations: u32) {
        self.max_iterations
            .store(max_iterations, atomic::Ordering::SeqCst);
        self.progress.store(0, atomic::Ordering::SeqCst);
        self.stage.store(stage as u32, atomic::Ordering::SeqCst);
    }

    /// Advances progress.
    fn advance_progress(&self) {
        self.progress.fetch_add(1, atomic::Ordering::SeqCst);
    }
}

/// Small helper that allows you to track progress of lightmap generation.
#[derive(Clone, Default)]
pub struct ProgressIndicator(pub Arc<ProgressData>);

impl ProgressIndicator {
    /// Creates new progress indicator.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Deref for ProgressIndicator {
    type Target = ProgressData;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// An error that may occur during ligthmap generation.
#[derive(Debug)]
pub enum LightmapGenerationError {
    /// Generation was cancelled by user.
    Cancelled,
    /// An index of a vertex in a triangle is out of bounds.
    InvalidIndex,
    /// Vertex buffer of a mesh lacks required data.
    InvalidData(VertexFetchError),
}

impl Display for LightmapGenerationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LightmapGenerationError::Cancelled => {
                write!(f, "Lightmap generation was cancelled by the user.")
            }
            LightmapGenerationError::InvalidIndex => {
                write!(f, "An index of a vertex in a triangle is out of bounds.")
            }
            LightmapGenerationError::InvalidData(v) => {
                write!(f, "Vertex buffer of a mesh lacks required data {v}.")
            }
        }
    }
}

impl From<VertexFetchError> for LightmapGenerationError {
    fn from(e: VertexFetchError) -> Self {
        Self::InvalidData(e)
    }
}

/// Data set required to generate a lightmap. It could be produced from a scene using [`LightmapInputData::from_scene`] method.
/// It is used to split preparation step from the actual lightmap generation; to be able to put heavy generation in a separate
/// thread.
pub struct LightmapInputData {
    data_set: FxHashMap<u64, SurfaceResource>,
    instances: Vec<Instance>,
    lights: FxHashMap<Handle<Node>, LightDefinition>,
}

impl LightmapInputData {
    /// Creates a new input data that can be later used to generate a lightmap.
    pub fn from_scene<F>(
        scene: &Scene,
        mut filter: F,
        cancellation_token: CancellationToken,
        progress_indicator: ProgressIndicator,
    ) -> Result<Self, LightmapGenerationError>
    where
        F: FnMut(Handle<Node>, &Node) -> bool,
    {
        // Extract info about lights first. We need it to be in separate array because
        // it won't be possible to store immutable references to light sources and at the
        // same time modify meshes. Also it precomputes a lot of things for faster calculations.
        let mut light_count = 0;
        for (handle, node) in scene.graph.pair_iter() {
            if filter(handle, node)
                && (node.cast::<PointLight>().is_some()
                    || node.cast::<SpotLight>().is_some()
                    || node.cast::<DirectionalLight>().is_some())
            {
                light_count += 1;
            }
        }

        progress_indicator.set_stage(ProgressStage::LightsCaching, light_count);

        let mut lights = FxHashMap::default();

        for (handle, node) in scene.graph.pair_iter() {
            if !filter(handle, node) {
                continue;
            }

            if cancellation_token.is_cancelled() {
                return Err(LightmapGenerationError::Cancelled);
            }

            if !node.is_globally_enabled() {
                continue;
            }

            if let Some(point) = node.cast::<PointLight>() {
                lights.insert(
                    handle,
                    LightDefinition::Point(PointLightDefinition {
                        intensity: point.base_light_ref().intensity(),
                        position: node.global_position(),
                        color: point.base_light_ref().color().srgb_to_linear().as_frgb(),
                        radius: point.radius(),
                        sqr_radius: point.radius() * point.radius(),
                    }),
                )
            } else if let Some(spot) = node.cast::<SpotLight>() {
                lights.insert(
                    handle,
                    LightDefinition::Spot(SpotLightDefinition {
                        intensity: spot.base_light_ref().intensity(),
                        edge0: ((spot.hotspot_cone_angle() + spot.falloff_angle_delta()) * 0.5)
                            .cos(),
                        edge1: (spot.hotspot_cone_angle() * 0.5).cos(),
                        color: spot.base_light_ref().color().srgb_to_linear().as_frgb(),
                        direction: node
                            .up_vector()
                            .try_normalize(f32::EPSILON)
                            .unwrap_or_else(Vector3::y),
                        position: node.global_position(),
                        distance: spot.distance(),
                        sqr_distance: spot.distance() * spot.distance(),
                    }),
                )
            } else if let Some(directional) = node.cast::<DirectionalLight>() {
                lights.insert(
                    handle,
                    LightDefinition::Directional(DirectionalLightDefinition {
                        intensity: directional.base_light_ref().intensity(),
                        direction: node
                            .up_vector()
                            .try_normalize(f32::EPSILON)
                            .unwrap_or_else(Vector3::y),
                        color: directional
                            .base_light_ref()
                            .color()
                            .srgb_to_linear()
                            .as_frgb(),
                    }),
                )
            } else {
                continue;
            };

            progress_indicator.advance_progress()
        }

        let mut instances = Vec::new();
        let mut data_set = FxHashMap::default();

        'node_loop: for (handle, node) in scene.graph.pair_iter() {
            if !filter(handle, node) {
                continue 'node_loop;
            }

            if let Some(mesh) = node.cast::<Mesh>() {
                if !mesh.global_visibility() || !mesh.is_globally_enabled() {
                    continue;
                }
                let global_transform = mesh.global_transform();
                'surface_loop: for surface in mesh.surfaces() {
                    // Check material for compatibility.

                    let mut material_state = surface.material().state();
                    if let Some(material) = material_state.data() {
                        if !material
                            .properties()
                            .get(&ImmutableString::new("lightmapTexture"))
                            .map(|v| matches!(v, PropertyValue::Sampler { .. }))
                            .unwrap_or_default()
                        {
                            continue 'surface_loop;
                        }
                    }

                    // Gather unique "list" of surface data to generate UVs for.
                    let data = surface.data();
                    let key = &*data.data_ref() as *const _ as u64;
                    data_set.entry(key).or_insert_with(|| surface.data());

                    instances.push(Instance {
                        owner: handle,
                        source_data: data.clone(),
                        transform: global_transform,
                        // Calculated down below.
                        data: None,
                    });
                }
            }
        }

        Ok(Self {
            data_set,
            instances,
            lights,
        })
    }
}

impl Lightmap {
    /// Loads a light map from the given path.
    pub async fn load<P: AsRef<Path>>(
        path: P,
        resource_manager: ResourceManager,
    ) -> Result<Lightmap, VisitError> {
        let mut visitor = Visitor::load_binary(path).await?;
        visitor.blackboard.register(Arc::new(resource_manager));
        let mut lightmap = Lightmap::default();
        lightmap.visit("Lightmap", &mut visitor)?;
        Ok(lightmap)
    }

    /// Saves a light map to the given file. Keep in mind, that the textures should be saved separately first, via
    /// [`Self::save_textures`] method.
    pub fn save<P: AsRef<Path>>(&mut self, path: P) -> VisitResult {
        let mut visitor = Visitor::new();
        self.visit("Lightmap", &mut visitor)?;
        visitor.save_binary(path)?;
        Ok(())
    }

    /// Generates lightmap for given scene. This method **automatically** generates secondary
    /// texture coordinates! This method is blocking, however internally it uses massive parallelism
    /// to use all available CPU power efficiently.
    ///
    /// `texels_per_unit` defines resolution of lightmap, the higher value is, the more quality
    /// lightmap will be generated, but also it will be slow to generate.
    /// `progress_indicator` allows you to get info about current progress.
    /// `cancellation_token` allows you to stop generation in any time.
    pub fn new(
        data: LightmapInputData,
        texels_per_unit: u32,
        uv_spacing: f32,
        cancellation_token: CancellationToken,
        progress_indicator: ProgressIndicator,
    ) -> Result<Self, LightmapGenerationError> {
        let LightmapInputData {
            data_set,
            mut instances,
            lights,
        } = data;

        progress_indicator.set_stage(ProgressStage::UvGeneration, data_set.len() as u32);

        let patches = data_set
            .into_par_iter()
            .map(|(_, data)| {
                if cancellation_token.is_cancelled() {
                    Err(LightmapGenerationError::Cancelled)
                } else {
                    let mut data = data.data_ref();
                    let data = &mut *data;

                    let mut patch = uvgen::generate_uvs(
                        data.vertex_buffer
                            .iter()
                            .map(|v| v.read_3_f32(VertexAttributeUsage::Position).unwrap()),
                        data.geometry_buffer.iter().map(|t| t.0),
                        uv_spacing,
                    )
                    .ok_or_else(|| LightmapGenerationError::InvalidIndex)?;
                    patch.data_id = data.content_hash();

                    apply_surface_data_patch(data, &patch);

                    progress_indicator.advance_progress();
                    Ok((patch.data_id, SurfaceDataPatchWrapper(patch)))
                }
            })
            .collect::<Result<FxHashMap<_, _>, LightmapGenerationError>>()?;

        progress_indicator.set_stage(ProgressStage::GeometryCaching, instances.len() as u32);

        instances
            .par_iter_mut()
            .map(|instance: &mut Instance| {
                if cancellation_token.is_cancelled() {
                    Err(LightmapGenerationError::Cancelled)
                } else {
                    let data = instance.source_data.data_ref();

                    let normal_matrix = instance
                        .transform
                        .basis()
                        .try_inverse()
                        .map(|m| m.transpose())
                        .unwrap_or_else(Matrix3::identity);

                    let world_vertices = data
                        .vertex_buffer
                        .iter()
                        .map(|view| {
                            let world_position = instance
                                .transform
                                .transform_point(&Point3::from(
                                    view.read_3_f32(VertexAttributeUsage::Position).unwrap(),
                                ))
                                .coords;
                            let world_normal = (normal_matrix
                                * view.read_3_f32(VertexAttributeUsage::Normal).unwrap())
                            .try_normalize(f32::EPSILON)
                            .unwrap_or_default();
                            lightmap::input::WorldVertex {
                                world_normal,
                                world_position,
                                second_tex_coord: view
                                    .read_2_f32(VertexAttributeUsage::TexCoord1)
                                    .unwrap(),
                            }
                        })
                        .collect::<Vec<_>>();

                    instance.data = Some(
                        lightmap::input::Mesh::new(
                            world_vertices,
                            data.geometry_buffer
                                .triangles_ref()
                                .iter()
                                .map(|t| t.0)
                                .collect(),
                        )
                        .unwrap(),
                    );

                    progress_indicator.advance_progress();

                    Ok(())
                }
            })
            .collect::<Result<(), LightmapGenerationError>>()?;

        progress_indicator.set_stage(ProgressStage::CalculatingLight, instances.len() as u32);

        let mut map: FxHashMap<Handle<Node>, Vec<LightmapEntry>> = FxHashMap::default();
        let meshes = instances
            .iter_mut()
            .filter_map(|i| i.data.take())
            .collect::<Vec<_>>();
        let light_definitions = lights.values().cloned().collect::<Vec<_>>();
        for (mesh, instance) in meshes.iter().zip(instances.iter()) {
            if cancellation_token.is_cancelled() {
                return Err(LightmapGenerationError::Cancelled);
            }

            let lightmap = generate_lightmap(mesh, &meshes, &light_definitions, texels_per_unit);
            map.entry(instance.owner).or_default().push(LightmapEntry {
                texture: Some(TextureResource::new_ok(Default::default(), lightmap)),
                lights: lights.keys().cloned().collect(),
            });

            progress_indicator.advance_progress();
        }

        Ok(Self { map, patches })
    }

    /// Saves lightmap textures into specified folder.
    pub fn save_textures<P: AsRef<Path>>(
        &self,
        base_path: P,
        resource_manager: ResourceManager,
    ) -> Result<(), ResourceRegistrationError> {
        if !base_path.as_ref().exists() {
            std::fs::create_dir_all(base_path.as_ref())
                .map_err(|_| ResourceRegistrationError::UnableToRegister)?;
        }

        for (handle, entries) in self.map.iter() {
            let handle_path = handle.index().to_string();
            for (i, entry) in entries.iter().enumerate() {
                let file_path = handle_path.clone() + "_" + i.to_string().as_str() + ".png";
                let texture = entry.texture.clone().unwrap();
                resource_manager.register(
                    texture.into_untyped(),
                    base_path.as_ref().join(file_path),
                    |texture, path| texture.save(path).is_ok(),
                )?;
            }
        }
        Ok(())
    }
}

/// Generates lightmap for given surface data with specified transform.
///
/// # Performance
///
/// This method is has linear complexity - the more complex mesh you pass, the more
/// time it will take. Required time increases drastically if you enable shadows and
/// global illumination (TODO), because in this case your data will be raytraced.
fn generate_lightmap(
    mesh: &lightmap::input::Mesh,
    other_meshes: &[lightmap::input::Mesh],
    lights: &[LightDefinition],
    texels_per_unit: u32,
) -> Texture {
    let map = lightmap::LightMap::new(mesh, other_meshes, lights, texels_per_unit as usize);

    Texture::from_bytes(
        TextureKind::Rectangle {
            width: map.width as u32,
            height: map.height as u32,
        },
        TexturePixelKind::RGB8,
        map.pixels,
    )
    .unwrap()
}

#[cfg(test)]
mod test {
    use crate::{
        asset::ResourceData,
        core::algebra::{Matrix4, Vector3},
        scene::{
            base::BaseBuilder,
            light::{point::PointLightBuilder, BaseLightBuilder},
            mesh::{
                surface::SurfaceResource,
                surface::{SurfaceBuilder, SurfaceData},
                MeshBuilder,
            },
            transform::TransformBuilder,
            Scene,
        },
        utils::lightmap::{Lightmap, LightmapInputData},
    };
    use fyrox_resource::untyped::ResourceKind;
    use std::path::Path;

    #[test]
    fn test_generate_lightmap() {
        let mut scene = Scene::new();

        let data = SurfaceData::make_cone(
            16,
            1.0,
            1.0,
            &Matrix4::new_nonuniform_scaling(&Vector3::new(1.0, 1.1, 1.0)),
        );

        MeshBuilder::new(BaseBuilder::new())
            .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_ok(
                ResourceKind::Embedded,
                data,
            ))
            .build()])
            .build(&mut scene.graph);

        PointLightBuilder::new(BaseLightBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(Vector3::new(0.0, 2.0, 0.0))
                    .build(),
            ),
        ))
        .with_radius(4.0)
        .build(&mut scene.graph);

        let data = LightmapInputData::from_scene(
            &scene,
            |_, _| true,
            Default::default(),
            Default::default(),
        )
        .unwrap();

        let lightmap =
            Lightmap::new(data, 64, 0.005, Default::default(), Default::default()).unwrap();

        let mut counter = 0;
        for entry_set in lightmap.map.values() {
            for entry in entry_set {
                let mut data = entry.texture.as_ref().unwrap().data_ref();
                data.save(Path::new(&format!("{}.png", counter))).unwrap();
                counter += 1;
            }
        }
    }
}
