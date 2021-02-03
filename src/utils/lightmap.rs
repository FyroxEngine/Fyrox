//! Module to generate lightmaps for surfaces.
//!
//! # Performance
//!
//! This is CPU lightmapper, its performance is linear with core count of your CPU.
//!
//! WARNING: There is still work-in-progress, so it is not advised to use lightmapper
//! now!

#![forbid(unsafe_code)]

use crate::{
    core::{
        algebra::{Matrix3, Matrix4, Point3, Vector2, Vector3, Vector4},
        arrayvec::ArrayVec,
        math::{self, ray::Ray, Matrix4Ext, Rect, TriangleDefinition, Vector2Ext},
        octree::{Octree, OctreeNode},
        pool::Handle,
        visitor::{Visit, VisitResult, Visitor},
    },
    engine::resource_manager::{ResourceManager, TextureRegistrationError},
    renderer::surface::SurfaceSharedData,
    resource::texture::{Texture, TextureData, TextureKind, TexturePixelKind, TextureState},
    scene::{light::Light, node::Node, Scene},
    utils::{uvgen, uvgen::SurfaceDataPatch},
};
use rayon::prelude::*;
use std::{
    collections::HashMap,
    ops::Deref,
    path::Path,
    sync::{
        atomic::{self, AtomicBool, AtomicU32},
        Arc, RwLock,
    },
};

///
#[derive(Default, Clone, Debug)]
pub struct LightmapEntry {
    /// Lightmap texture.
    ///
    /// TODO: Is single texture enough? There may be surfaces with huge amount of faces
    ///  which may not fit into texture, because there is hardware limit on most GPUs
    ///  up to 8192x8192 pixels.
    pub texture: Option<Texture>,
    /// List of lights that were used to generate this lightmap. This list is used for
    /// masking when applying dynamic lights for surfaces with light, it prevents double
    /// lighting.
    pub lights: Vec<Handle<Node>>,
}

impl Visit for LightmapEntry {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.texture.visit("Texture", visitor)?;
        self.lights.visit("Lights", visitor)?;

        visitor.leave_region()
    }
}

/// Lightmap is a texture with precomputed lighting.
#[derive(Default, Clone, Debug)]
pub struct Lightmap {
    /// Node handle to lightmap mapping. It is used to quickly get information about
    /// lightmaps for any node in scene.
    pub map: HashMap<Handle<Node>, Vec<LightmapEntry>>,

    /// List of surface data patches. Each patch will be applied to corresponding
    /// surface data on resolve stage.
    pub patches: HashMap<u64, SurfaceDataPatch>,
}

impl Visit for Lightmap {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.map.visit("Map", visitor)?;
        self.patches.visit("Patches", visitor)?;

        visitor.leave_region()
    }
}

struct WorldVertex {
    world_normal: Vector3<f32>,
    world_position: Vector3<f32>,
    second_tex_coord: Vector2<f32>,
}

struct InstanceData {
    /// World-space vertices.
    vertices: Vec<WorldVertex>,
    triangles: Vec<TriangleDefinition>,
    octree: Octree,
}

struct Instance {
    owner: Handle<Node>,
    source_data: Arc<RwLock<SurfaceSharedData>>,
    data: Option<InstanceData>,
    transform: Matrix4<f32>,
}

impl Instance {
    pub fn data(&self) -> &InstanceData {
        self.data.as_ref().unwrap()
    }
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
        &*self.0
    }
}

/// An error that may occur during ligthmap generation.
#[derive(Debug)]
pub enum LightmapGenerationError {
    /// Generation was cancelled by user.
    Cancelled,
}

impl Lightmap {
    /// Generates lightmap for given scene. This method **automatically** generates secondary
    /// texture coordinates! This method is blocking, however internally it uses massive parallelism
    /// to use all available CPU power efficiently.
    ///
    /// `texels_per_unit` defines resolution of lightmap, the higher value is, the more quality
    /// lightmap will be generated, but also it will be slow to generate.
    /// `progress_indicator` allows you to get info about current progress.
    /// `cancellation_token` allows you to stop generation in any time.
    pub fn new(
        scene: &mut Scene,
        texels_per_unit: u32,
        cancellation_token: CancellationToken,
        progress_indicator: ProgressIndicator,
    ) -> Result<Self, LightmapGenerationError> {
        scene.graph.update_hierarchical_data();

        // Extract info about lights first. We need it to be in separate array because
        // it won't be possible to store immutable references to light sources and at the
        // same time modify meshes. Also it precomputes a lot of things for faster calculations.
        let mut light_count = 0;
        for node in scene.graph.linear_iter() {
            if matches!(node, Node::Light(_)) {
                light_count += 1;
            }
        }

        progress_indicator.set_stage(ProgressStage::LightsCaching, light_count);

        let mut lights = Vec::with_capacity(light_count as usize);

        for (handle, light) in scene.graph.pair_iter().filter_map(|(h, n)| {
            if let Node::Light(light) = n {
                Some((h, light))
            } else {
                None
            }
        }) {
            if cancellation_token.is_cancelled() {
                return Err(LightmapGenerationError::Cancelled);
            }

            match light {
                Light::Directional(_) => {
                    lights.push(LightDefinition::Directional(DirectionalLightDefinition {
                        handle,
                        intensity: 1.0,
                        direction: light
                            .up_vector()
                            .try_normalize(std::f32::EPSILON)
                            .unwrap_or_else(Vector3::y),
                        color: light.color().as_frgb(),
                    }))
                }
                Light::Spot(spot) => lights.push(LightDefinition::Spot(SpotLightDefinition {
                    handle,
                    intensity: 1.0,
                    edge0: ((spot.hotspot_cone_angle() + spot.falloff_angle_delta()) * 0.5).cos(),
                    edge1: (spot.hotspot_cone_angle() * 0.5).cos(),
                    color: light.color().as_frgb(),
                    direction: light
                        .up_vector()
                        .try_normalize(std::f32::EPSILON)
                        .unwrap_or_else(Vector3::y),
                    position: light.global_position(),
                    distance: spot.distance(),
                    sqr_distance: spot.distance() * spot.distance(),
                })),
                Light::Point(point) => lights.push(LightDefinition::Point(PointLightDefinition {
                    handle,
                    intensity: 1.0,
                    position: light.global_position(),
                    color: light.color().as_frgb(),
                    radius: point.radius(),
                    sqr_radius: point.radius() * point.radius(),
                })),
            }

            progress_indicator.advance_progress()
        }

        let mut instances = Vec::new();
        let mut data_set = HashMap::new();

        for (handle, node) in scene.graph.pair_iter() {
            if let Node::Mesh(mesh) = node {
                if !mesh.global_visibility() {
                    continue;
                }
                let global_transform = mesh.global_transform();
                for surface in mesh.surfaces() {
                    // Gather unique "list" of surface data to generate UVs for.
                    let data = surface.data();
                    let key = &*data.read().unwrap() as *const _ as u64;
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

        progress_indicator.set_stage(ProgressStage::UvGeneration, data_set.len() as u32);

        let patches = data_set
            .into_par_iter()
            .map(|(_, data)| {
                if cancellation_token.is_cancelled() {
                    Err(LightmapGenerationError::Cancelled)
                } else {
                    let mut data = data.write().unwrap();
                    let patch = uvgen::generate_uvs(&mut data, 0.005);
                    progress_indicator.advance_progress();
                    Ok((patch.data_id, patch))
                }
            })
            .collect::<Result<HashMap<_, _>, LightmapGenerationError>>()?;

        progress_indicator.set_stage(ProgressStage::GeometryCaching, instances.len() as u32);

        instances
            .par_iter_mut()
            .map(|instance: &mut Instance| {
                if cancellation_token.is_cancelled() {
                    Err(LightmapGenerationError::Cancelled)
                } else {
                    let data = instance.source_data.read().unwrap();

                    let normal_matrix = instance
                        .transform
                        .basis()
                        .try_inverse()
                        .map(|m| m.transpose())
                        .unwrap_or_else(Matrix3::identity);

                    let world_vertices = data
                        .vertices
                        .iter()
                        .map(|v| {
                            let world_position = instance
                                .transform
                                .transform_point(&Point3::from(v.position))
                                .coords;
                            let world_normal = (normal_matrix * v.normal)
                                .try_normalize(f32::EPSILON)
                                .unwrap_or_default();
                            WorldVertex {
                                world_normal,
                                world_position,
                                second_tex_coord: v.second_tex_coord,
                            }
                        })
                        .collect::<Vec<_>>();

                    let world_triangles = data
                        .triangles()
                        .iter()
                        .map(|tri| {
                            [
                                world_vertices[tri[0] as usize].world_position,
                                world_vertices[tri[1] as usize].world_position,
                                world_vertices[tri[2] as usize].world_position,
                            ]
                        })
                        .collect::<Vec<_>>();

                    instance.data = Some(InstanceData {
                        vertices: world_vertices,
                        triangles: data.triangles.clone(),
                        octree: Octree::new(&world_triangles, 64),
                    });

                    progress_indicator.advance_progress();

                    Ok(())
                }
            })
            .collect::<Result<(), LightmapGenerationError>>()?;

        progress_indicator.set_stage(ProgressStage::CalculatingLight, instances.len() as u32);

        let mut map: HashMap<Handle<Node>, Vec<LightmapEntry>> = HashMap::new();
        for instance in instances.iter() {
            if cancellation_token.is_cancelled() {
                return Err(LightmapGenerationError::Cancelled);
            }

            let lightmap = generate_lightmap(&instance, &instances, &lights, texels_per_unit);
            map.entry(instance.owner).or_default().push(LightmapEntry {
                texture: Some(Texture::new(TextureState::Ok(lightmap))),
                lights: lights.iter().map(|light| light.handle()).collect(),
            });

            progress_indicator.advance_progress();
        }

        Ok(Self { map, patches })
    }

    /// Saves lightmap textures into specified folder.
    pub fn save<P: AsRef<Path>>(
        &self,
        base_path: P,
        resource_manager: ResourceManager,
    ) -> Result<(), TextureRegistrationError> {
        if !base_path.as_ref().exists() {
            std::fs::create_dir(base_path.as_ref()).unwrap();
        }

        for (handle, entries) in self.map.iter() {
            let handle_path = handle.index().to_string();
            for (i, entry) in entries.iter().enumerate() {
                let file_path = handle_path.clone() + "_" + i.to_string().as_str() + ".png";
                let texture = entry.texture.clone().unwrap();
                resource_manager.register_texture(texture, base_path.as_ref().join(file_path))?;
            }
        }
        Ok(())
    }
}

/// Directional light is a light source with parallel rays. Example: Sun.
pub struct DirectionalLightDefinition {
    /// A handle of light in the scene.
    pub handle: Handle<Node>,
    /// Intensity is how bright light is. Default is 1.0.
    pub intensity: f32,
    /// Direction of light rays.
    pub direction: Vector3<f32>,
    /// Color of light.
    pub color: Vector3<f32>,
}

/// Spot light is a cone light source. Example: flashlight.
pub struct SpotLightDefinition {
    /// A handle of light in the scene.
    pub handle: Handle<Node>,
    /// Intensity is how bright light is. Default is 1.0.
    pub intensity: f32,
    /// Color of light.
    pub color: Vector3<f32>,
    /// Direction vector of light.
    pub direction: Vector3<f32>,
    /// Position of light in world coordinates.
    pub position: Vector3<f32>,
    /// Distance at which light intensity decays to zero.
    pub distance: f32,
    /// Square of distance.
    pub sqr_distance: f32,
    /// Smoothstep left bound. It is ((hotspot_cone_angle + falloff_angle_delta) * 0.5).cos()
    pub edge0: f32,
    /// Smoothstep right bound. It is (hotspot_cone_angle * 0.5).cos()
    pub edge1: f32,
}

/// Point light is a spherical light source. Example: light bulb.
pub struct PointLightDefinition {
    /// A handle of light in the scene.
    pub handle: Handle<Node>,
    /// Intensity is how bright light is. Default is 1.0.
    pub intensity: f32,
    /// Position of light in world coordinates.
    pub position: Vector3<f32>,
    /// Color of light.
    pub color: Vector3<f32>,
    /// Radius of sphere at which light intensity decays to zero.
    pub radius: f32,
    /// Square of radius.
    pub sqr_radius: f32,
}

/// Light definition for lightmap rendering.
pub enum LightDefinition {
    /// See docs of [DirectionalLightDefinition](struct.PointLightDefinition.html)
    Directional(DirectionalLightDefinition),
    /// See docs of [SpotLightDefinition](struct.SpotLightDefinition.html)
    Spot(SpotLightDefinition),
    /// See docs of [PointLightDefinition](struct.PointLightDefinition.html)
    Point(PointLightDefinition),
}

impl LightDefinition {
    fn handle(&self) -> Handle<Node> {
        match self {
            LightDefinition::Directional(v) => v.handle,
            LightDefinition::Spot(v) => v.handle,
            LightDefinition::Point(v) => v.handle,
        }
    }
}

/// Computes total area of triangles in surface data and returns size of square
/// in which triangles can fit.
fn estimate_size(data: &InstanceData, texels_per_unit: u32) -> u32 {
    let mut area = 0.0;
    for triangle in data.triangles.iter() {
        let a = data.vertices[triangle[0] as usize].world_position;
        let b = data.vertices[triangle[1] as usize].world_position;
        let c = data.vertices[triangle[2] as usize].world_position;
        area += math::triangle_area(a, b, c);
    }
    area.sqrt().ceil() as u32 * texels_per_unit
}

/// Calculates distance attenuation for a point using given distance to the point and
/// radius of a light.
fn distance_attenuation(distance: f32, sqr_radius: f32) -> f32 {
    let attenuation = (1.0 - distance * distance / sqr_radius).max(0.0).min(1.0);
    attenuation * attenuation
}

/// Calculates properties of pixel (world position, normal) at given position.
fn pick(
    uv: Vector2<f32>,
    grid: &Grid,
    data: &InstanceData,
    scale: f32,
) -> Option<(Vector3<f32>, Vector3<f32>)> {
    if let Some(cell) = grid.pick(uv) {
        for triangle in cell.triangles.iter().map(|&ti| &data.triangles[ti]) {
            let ia = triangle[0] as usize;
            let ib = triangle[1] as usize;
            let ic = triangle[2] as usize;

            let uv_a = data.vertices[ia].second_tex_coord;
            let uv_b = data.vertices[ib].second_tex_coord;
            let uv_c = data.vertices[ic].second_tex_coord;

            let center = (uv_a + uv_b + uv_c).scale(1.0 / 3.0);
            let to_center = (center - uv)
                .try_normalize(std::f32::EPSILON)
                .unwrap_or_default()
                .scale(scale * 0.3333333);

            let mut current_uv = uv;
            for _ in 0..3 {
                let barycentric = math::get_barycentric_coords_2d(current_uv, uv_a, uv_b, uv_c);

                if math::barycentric_is_inside(barycentric) {
                    let a = data.vertices[ia].world_position;
                    let b = data.vertices[ib].world_position;
                    let c = data.vertices[ic].world_position;

                    let na = data.vertices[ia].world_normal;
                    let nb = data.vertices[ib].world_normal;
                    let nc = data.vertices[ic].world_normal;

                    return Some((
                        math::barycentric_to_world(barycentric, a, b, c),
                        math::barycentric_to_world(barycentric, na, nb, nc),
                    ));
                }

                // Offset uv to center for conservative rasterization.
                current_uv += to_center;
            }
        }
    }
    None
}

struct GridCell {
    // List of triangle indices.
    triangles: Vec<usize>,
}

struct Grid {
    cells: Vec<GridCell>,
    size: usize,
    fsize: f32,
}

impl Grid {
    /// Creates uniform grid where each cell contains list of triangles
    /// whose second texture coordinates intersects with it.
    fn new(data: &InstanceData, size: usize) -> Self {
        let mut cells = Vec::with_capacity(size);
        let fsize = size as f32;
        for y in 0..size {
            for x in 0..size {
                let bounds =
                    Rect::new(x as f32 / fsize, y as f32 / fsize, 1.0 / fsize, 1.0 / fsize);

                let mut triangles = Vec::new();

                for (triangle_index, triangle) in data.triangles.iter().enumerate() {
                    let uv_a = data.vertices[triangle[0] as usize].second_tex_coord;
                    let uv_b = data.vertices[triangle[1] as usize].second_tex_coord;
                    let uv_c = data.vertices[triangle[2] as usize].second_tex_coord;
                    let uv_min = uv_a.per_component_min(&uv_b).per_component_min(&uv_c);
                    let uv_max = uv_a.per_component_max(&uv_b).per_component_max(&uv_c);
                    let triangle_bounds =
                        Rect::new(uv_min.x, uv_min.y, uv_max.x - uv_min.x, uv_max.y - uv_min.y);
                    if triangle_bounds.intersects(bounds) {
                        triangles.push(triangle_index);
                    }
                }

                cells.push(GridCell { triangles })
            }
        }

        Self {
            cells,
            size,
            fsize: size as f32,
        }
    }

    fn pick(&self, v: Vector2<f32>) -> Option<&GridCell> {
        let ix = (v.x * self.fsize) as usize;
        let iy = (v.y * self.fsize) as usize;
        self.cells.get(iy * self.size + ix)
    }
}

/// https://en.wikipedia.org/wiki/Lambert%27s_cosine_law
fn lambertian(light_vec: Vector3<f32>, normal: Vector3<f32>) -> f32 {
    normal.dot(&light_vec).max(0.0)
}

/// https://en.wikipedia.org/wiki/Smoothstep
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let k = ((x - edge0) / (edge1 - edge0)).max(0.0).min(1.0);
    k * k * (3.0 - 2.0 * k)
}

/// Generates lightmap for given surface data with specified transform.
///
/// # Performance
///
/// This method is has linear complexity - the more complex mesh you pass, the more
/// time it will take. Required time increases drastically if you enable shadows and
/// global illumination (TODO), because in this case your data will be raytraced.
fn generate_lightmap(
    instance: &Instance,
    other_instances: &[Instance],
    lights: &[LightDefinition],
    texels_per_unit: u32,
) -> TextureData {
    // We have to re-generate new set of world-space vertices because UV generator
    // may add new vertices on seams.
    let atlas_size = estimate_size(&instance.data(), texels_per_unit);
    let scale = 1.0 / atlas_size as f32;
    let grid = Grid::new(instance.data(), (atlas_size / 32).max(4) as usize);

    let mut pixels: Vec<Vector4<u8>> =
        vec![Vector4::new(0, 0, 0, 0); (atlas_size * atlas_size) as usize];

    let half_pixel = scale * 0.5;
    pixels
        .par_iter_mut()
        .enumerate()
        .for_each(|(i, pixel): (usize, &mut Vector4<u8>)| {
            let x = i as u32 % atlas_size;
            let y = i as u32 / atlas_size;

            let uv = Vector2::new(x as f32 * scale + half_pixel, y as f32 * scale + half_pixel);

            if let Some((world_position, world_normal)) = pick(uv, &grid, instance.data(), scale) {
                let mut pixel_color = Vector3::default();
                for light in lights {
                    let (light_color, mut attenuation, light_position) = match light {
                        LightDefinition::Directional(directional) => {
                            let attenuation = directional.intensity
                                * lambertian(directional.direction, world_normal);
                            (directional.color, attenuation, Vector3::default())
                        }
                        LightDefinition::Spot(spot) => {
                            let d = spot.position - world_position;
                            let distance = d.norm();
                            let light_vec = d.scale(1.0 / distance);
                            let spot_angle_cos = light_vec.dot(&spot.direction);
                            let cone_factor = smoothstep(spot.edge0, spot.edge1, spot_angle_cos);
                            let attenuation = cone_factor
                                * spot.intensity
                                * lambertian(light_vec, world_normal)
                                * distance_attenuation(distance, spot.sqr_distance);
                            (spot.color, attenuation, spot.position)
                        }
                        LightDefinition::Point(point) => {
                            let d = point.position - world_position;
                            let distance = d.norm();
                            let light_vec = d.scale(1.0 / distance);
                            let attenuation = point.intensity
                                * lambertian(light_vec, world_normal)
                                * distance_attenuation(distance, point.sqr_radius);
                            (point.color, attenuation, point.position)
                        }
                    };
                    // Shadows
                    if attenuation >= 0.01 {
                        let mut query_buffer = ArrayVec::<[Handle<OctreeNode>; 64]>::new();
                        let shadow_bias = 0.01;
                        let ray = Ray::from_two_points(light_position, world_position);
                        'outer_loop: for other_instance in other_instances {
                            other_instance
                                .data()
                                .octree
                                .ray_query_static(&ray, &mut query_buffer);
                            for &node in query_buffer.iter() {
                                match other_instance.data().octree.node(node) {
                                    OctreeNode::Leaf { indices, .. } => {
                                        let other_data = other_instance.data();
                                        for &triangle_index in indices {
                                            let triangle =
                                                &other_data.triangles[triangle_index as usize];
                                            let va = other_data.vertices[triangle[0] as usize]
                                                .world_position;
                                            let vb = other_data.vertices[triangle[1] as usize]
                                                .world_position;
                                            let vc = other_data.vertices[triangle[2] as usize]
                                                .world_position;
                                            if let Some(pt) =
                                                ray.triangle_intersection(&[va, vb, vc])
                                            {
                                                if ray.origin.metric_distance(&pt) + shadow_bias
                                                    < ray.dir.norm()
                                                {
                                                    attenuation = 0.0;
                                                    break 'outer_loop;
                                                }
                                            }
                                        }
                                    }
                                    OctreeNode::Branch { .. } => unreachable!(),
                                }
                            }
                        }
                    }
                    pixel_color += light_color.scale(attenuation);
                }

                *pixel = Vector4::new(
                    (pixel_color.x.max(0.0).min(1.0) * 255.0) as u8,
                    (pixel_color.y.max(0.0).min(1.0) * 255.0) as u8,
                    (pixel_color.z.max(0.0).min(1.0) * 255.0) as u8,
                    255, // Indicates that this pixel was "filled"
                );
            }
        });

    // Prepare light map for bilinear filtration. This step is mandatory to prevent bleeding.
    let mut rgb_pixels: Vec<Vector3<u8>> = Vec::with_capacity((atlas_size * atlas_size) as usize);
    for y in 0..(atlas_size as i32) {
        for x in 0..(atlas_size as i32) {
            let fetch = |dx: i32, dy: i32| -> Option<Vector3<u8>> {
                pixels
                    .get(((y + dy) * (atlas_size as i32) + x + dx) as usize)
                    .and_then(|p| {
                        if p.w != 0 {
                            Some(Vector3::new(p.x, p.y, p.z))
                        } else {
                            None
                        }
                    })
            };

            let src_pixel = pixels[(y * (atlas_size as i32) + x) as usize];
            if src_pixel.w == 0 {
                // Check neighbour pixels marked as "filled" and use it as value.
                if let Some(west) = fetch(-1, 0) {
                    rgb_pixels.push(west);
                } else if let Some(east) = fetch(1, 0) {
                    rgb_pixels.push(east);
                } else if let Some(north) = fetch(0, -1) {
                    rgb_pixels.push(north);
                } else if let Some(south) = fetch(0, 1) {
                    rgb_pixels.push(south);
                } else if let Some(north_west) = fetch(-1, -1) {
                    rgb_pixels.push(north_west);
                } else if let Some(north_east) = fetch(1, -1) {
                    rgb_pixels.push(north_east);
                } else if let Some(south_east) = fetch(1, 1) {
                    rgb_pixels.push(south_east);
                } else if let Some(south_west) = fetch(-1, 1) {
                    rgb_pixels.push(south_west);
                } else {
                    rgb_pixels.push(Vector3::new(0, 0, 0));
                }
            } else {
                rgb_pixels.push(Vector3::new(src_pixel.x, src_pixel.y, src_pixel.z))
            }
        }
    }

    // Blur lightmap using simplest box filter.
    let mut bytes = Vec::with_capacity((atlas_size * atlas_size * 3) as usize);
    for y in 0..(atlas_size as i32) {
        for x in 0..(atlas_size as i32) {
            if x < 1 || y < 1 || x + 1 == atlas_size as i32 || y + 1 == atlas_size as i32 {
                bytes.extend_from_slice(
                    rgb_pixels[(y * (atlas_size as i32) + x) as usize].as_slice(),
                );
            } else {
                let fetch = |dx: i32, dy: i32| -> Vector3<i16> {
                    let u8_pixel = rgb_pixels[((y + dy) * (atlas_size as i32) + x + dx) as usize];
                    Vector3::new(u8_pixel.x as i16, u8_pixel.y as i16, u8_pixel.z as i16)
                };

                let north_west = fetch(-1, -1);
                let north = fetch(0, -1);
                let north_east = fetch(1, -1);
                let west = fetch(-1, 0);
                let center = fetch(0, 0);
                let east = fetch(1, 0);
                let south_west = fetch(-1, 1);
                let south = fetch(0, 1);
                let south_east = fetch(-1, 1);

                let sum = north_west
                    + north
                    + north_east
                    + west
                    + center
                    + east
                    + south_west
                    + south
                    + south_east;

                bytes.push((sum.x / 9).max(0).min(255) as u8);
                bytes.push((sum.y / 9).max(0).min(255) as u8);
                bytes.push((sum.z / 9).max(0).min(255) as u8);
            }
        }
    }

    TextureData::from_bytes(
        TextureKind::Rectangle {
            width: atlas_size,
            height: atlas_size,
        },
        TexturePixelKind::RGB8,
        bytes,
    )
    .unwrap()
}

#[cfg(test)]
mod test {
    use crate::renderer::surface::SurfaceBuilder;
    use crate::scene::base::BaseBuilder;
    use crate::scene::light::{BaseLightBuilder, PointLightBuilder};
    use crate::scene::mesh::MeshBuilder;
    use crate::scene::transform::TransformBuilder;
    use crate::scene::Scene;
    use crate::utils::lightmap::Lightmap;
    use crate::{
        core::algebra::{Matrix4, Vector3},
        renderer::surface::SurfaceSharedData,
    };
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_generate_lightmap() {
        let mut scene = Scene::new();

        let data = SurfaceSharedData::make_cone(
            16,
            1.0,
            1.0,
            Matrix4::new_nonuniform_scaling(&Vector3::new(1.0, 1.1, 1.0)),
        );

        MeshBuilder::new(BaseBuilder::new())
            .with_surfaces(vec![
                SurfaceBuilder::new(Arc::new(RwLock::new(data))).build()
            ])
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

        let lightmap =
            Lightmap::new(&mut scene, 64, Default::default(), Default::default()).unwrap();

        let mut counter = 0;
        for entry_set in lightmap.map.values() {
            for entry in entry_set {
                let mut data = entry.texture.as_ref().unwrap().data_ref();
                data.set_path(format!("{}.png", counter));
                data.save().unwrap();
                counter += 1;
            }
        }
    }
}
