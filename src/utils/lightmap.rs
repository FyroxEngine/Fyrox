//! Module to generate lightmaps for surfaces.
//!
//! # Performance
//!
//! This is CPU lightmapper, its performance is linear with core count of your CPU.
//!
//! WARNING: There is still work-in-progress, so it is not advised to use lightmapper
//! now!

use crate::{
    core::{
        algebra::{Matrix3, Matrix4, Point3, Vector2, Vector3},
        arrayvec::ArrayVec,
        math::{self, ray::Ray, Matrix4Ext, Rect, TriangleDefinition, Vector2Ext},
        octree::{Octree, OctreeNode},
        pool::{ErasedHandle, Handle},
        visitor::{Visit, VisitResult, Visitor},
    },
    engine::resource_manager::{ResourceManager, TextureRegistrationError},
    renderer::{surface::SurfaceSharedData, surface::Vertex},
    resource::texture::{Texture, TextureData, TextureKind, TexturePixelKind, TextureState},
    scene::{light::Light, node::Node, Scene},
    utils::{uvgen, uvgen::SurfaceDataPatch},
};
use rayon::prelude::*;
use std::ops::Deref;
use std::{
    collections::HashMap,
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

struct Instance {
    owner: ErasedHandle,
    data: Arc<RwLock<SurfaceSharedData>>,
    transform: Matrix4<f32>,
    octree: Octree,
    world_vertices: Vec<Vector3<f32>>,
}

/// Small helper that allows you stop lightmap generation in any time.
#[derive(Clone)]
pub struct CancellationToken(pub Arc<AtomicBool>);

impl CancellationToken {
    /// Creates new cancellation token.
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
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
pub struct ProgressData {
    stage: AtomicU32,
    // Range is [0; max_iterations]
    progress: AtomicU32,
    max_iterations: AtomicU32,
}

impl ProgressData {
    fn new() -> Self {
        Self {
            stage: Default::default(),
            progress: Default::default(),
            max_iterations: Default::default(),
        }
    }

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
#[derive(Clone)]
pub struct ProgressIndicator(pub Arc<ProgressData>);

impl ProgressIndicator {
    /// Creates new progress indicator.
    pub fn new() -> Self {
        Self {
            0: Arc::new(ProgressData::new()),
        }
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
                        handle: handle.into(),
                        intensity: 1.0,
                        direction: light
                            .up_vector()
                            .try_normalize(std::f32::EPSILON)
                            .unwrap_or_else(Vector3::y),
                        color: light.color().as_frgb(),
                    }))
                }
                Light::Spot(spot) => lights.push(LightDefinition::Spot(SpotLightDefinition {
                    handle: handle.into(),
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
                })),
                Light::Point(point) => lights.push(LightDefinition::Point(PointLightDefinition {
                    handle: handle.into(),
                    intensity: 1.0,
                    position: light.global_position(),
                    color: light.color().as_frgb(),
                    radius: point.radius(),
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
                    if !data_set.contains_key(&key) {
                        data_set.insert(key, surface.data());
                    }

                    instances.push(Instance {
                        owner: handle.into(),
                        data: surface.data(),
                        transform: global_transform,
                        // Rest will be calculated below in parallel.
                        world_vertices: Default::default(),
                        octree: Default::default(),
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
                    let data = instance.data.read().unwrap();

                    let world_vertices = transform_vertices(&data, &instance.transform);
                    let world_triangles = data
                        .triangles()
                        .iter()
                        .map(|tri| {
                            [
                                world_vertices[tri[0] as usize],
                                world_vertices[tri[1] as usize],
                                world_vertices[tri[2] as usize],
                            ]
                        })
                        .collect::<Vec<_>>();
                    instance.octree = Octree::new(&world_triangles, 64);
                    instance.world_vertices = world_vertices;
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
            map.entry(instance.owner.into())
                .or_default()
                .push(LightmapEntry {
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
    pub handle: ErasedHandle,
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
    pub handle: ErasedHandle,
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
    /// Smoothstep left bound. It is ((hotspot_cone_angle + falloff_angle_delta) * 0.5).cos()
    pub edge0: f32,
    /// Smoothstep right bound. It is (hotspot_cone_angle * 0.5).cos()
    pub edge1: f32,
}

/// Point light is a spherical light source. Example: light bulb.
pub struct PointLightDefinition {
    /// A handle of light in the scene.
    pub handle: ErasedHandle,
    /// Intensity is how bright light is. Default is 1.0.
    pub intensity: f32,
    /// Position of light in world coordinates.
    pub position: Vector3<f32>,
    /// Color of light.
    pub color: Vector3<f32>,
    /// Radius of sphere at which light intensity decays to zero.
    pub radius: f32,
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
            LightDefinition::Directional(v) => v.handle.into(),
            LightDefinition::Spot(v) => v.handle.into(),
            LightDefinition::Point(v) => v.handle.into(),
        }
    }
}

/// Computes total area of triangles in surface data and returns size of square
/// in which triangles can fit.
fn estimate_size(
    vertices: &[Vector3<f32>],
    triangles: &[TriangleDefinition],
    texels_per_unit: u32,
) -> u32 {
    let mut area = 0.0;
    for triangle in triangles.iter() {
        let a = vertices[triangle[0] as usize];
        let b = vertices[triangle[1] as usize];
        let c = vertices[triangle[2] as usize];
        area += math::triangle_area(a, b, c);
    }
    area.sqrt().ceil() as u32 * texels_per_unit
}

/// Calculates distance attenuation for a point using given distance to the point and
/// radius of a light.
fn distance_attenuation(distance: f32, radius: f32) -> f32 {
    let attenuation = (1.0 - distance * distance / (radius * radius))
        .max(0.0)
        .min(1.0);
    attenuation * attenuation
}

/// Transforms vertices of surface data into set of world space positions.
fn transform_vertices(data: &SurfaceSharedData, transform: &Matrix4<f32>) -> Vec<Vector3<f32>> {
    data.vertices
        .iter()
        .map(|v| transform.transform_point(&Point3::from(v.position)).coords)
        .collect()
}

struct Pixel {
    coords: Vector2<u16>,
    color: Vector3<u8>,
}

/// Calculates properties of pixel (world position, normal) at given position.
fn pick(
    uv: Vector2<f32>,
    grid: &Grid,
    triangles: &[TriangleDefinition],
    vertices: &[Vertex],
    world_positions: &[Vector3<f32>],
    normal_matrix: &Matrix3<f32>,
    scale: f32,
) -> Option<(Vector3<f32>, Vector3<f32>)> {
    if let Some(cell) = grid.pick(uv) {
        for triangle in cell.triangles.iter().map(|&ti| &triangles[ti]) {
            let uv_a = vertices[triangle[0] as usize].second_tex_coord;
            let uv_b = vertices[triangle[1] as usize].second_tex_coord;
            let uv_c = vertices[triangle[2] as usize].second_tex_coord;

            let center = (uv_a + uv_b + uv_c).scale(1.0 / 3.0);
            let to_center = (center - uv)
                .try_normalize(std::f32::EPSILON)
                .unwrap_or_default()
                .scale(scale);

            let mut current_uv = uv;
            for _ in 0..3 {
                let barycentric = math::get_barycentric_coords_2d(current_uv, uv_a, uv_b, uv_c);

                if math::barycentric_is_inside(barycentric) {
                    let a = world_positions[triangle[0] as usize];
                    let b = world_positions[triangle[1] as usize];
                    let c = world_positions[triangle[2] as usize];
                    return Some((
                        math::barycentric_to_world(barycentric, a, b, c),
                        (normal_matrix
                            * math::barycentric_to_world(
                                barycentric,
                                vertices[triangle[0] as usize].normal,
                                vertices[triangle[1] as usize].normal,
                                vertices[triangle[2] as usize].normal,
                            ))
                        .try_normalize(std::f32::EPSILON)
                        .unwrap_or_else(Vector3::y),
                    ));
                }

                // Offset uv to center to remove seams.
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
}

impl Grid {
    /// Creates uniform grid where each cell contains list of triangles
    /// whose second texture coordinates intersects with it.
    fn new(data: &SurfaceSharedData, size: usize) -> Self {
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

        Self { cells, size }
    }

    fn pick(&self, v: Vector2<f32>) -> Option<&GridCell> {
        let ix = (v.x as f32 * self.size as f32) as usize;
        let iy = (v.y as f32 * self.size as f32) as usize;
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
    let data = instance.data.read().unwrap();

    let world_positions = transform_vertices(&data, &instance.transform);
    let atlas_size = estimate_size(&world_positions, &data.triangles, texels_per_unit);

    let scale = 1.0 / atlas_size as f32;

    // We have to re-generate new set of world-space vertices because UV generator
    // may add new vertices on seams.
    let world_positions = transform_vertices(&data, &instance.transform);
    let grid = Grid::new(&data, (atlas_size / 16).max(4) as usize);

    let normal_matrix = instance
        .transform
        .basis()
        .try_inverse()
        .map(|m| m.transpose())
        .unwrap_or_else(Matrix3::identity);

    let mut pixels = Vec::with_capacity((atlas_size * atlas_size) as usize);
    for y in 0..(atlas_size as usize) {
        for x in 0..(atlas_size as usize) {
            pixels.push(Pixel {
                coords: Vector2::new(x as u16, y as u16),
                color: Vector3::new(0, 0, 0),
            });
        }
    }

    let half_pixel = scale * 0.5;
    pixels.par_iter_mut().for_each(|pixel: &mut Pixel| {
        // Get uv in center of pixel.
        let uv = Vector2::new(
            pixel.coords.x as f32 * scale + half_pixel,
            pixel.coords.y as f32 * scale + half_pixel,
        );

        if let Some((world_position, world_normal)) = pick(
            uv,
            &grid,
            &data.triangles,
            &data.vertices,
            &world_positions,
            &normal_matrix,
            scale,
        ) {
            let mut pixel_color = Vector3::default();
            for light in lights {
                let (light_color, mut attenuation, light_position) = match light {
                    LightDefinition::Directional(directional) => {
                        let attenuation =
                            directional.intensity * lambertian(directional.direction, world_normal);
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
                            * distance_attenuation(distance, spot.distance);
                        (spot.color, attenuation, spot.position)
                    }
                    LightDefinition::Point(point) => {
                        let d = point.position - world_position;
                        let distance = d.norm();
                        let light_vec = d.scale(1.0 / distance);
                        let attenuation = point.intensity
                            * lambertian(light_vec, world_normal)
                            * distance_attenuation(distance, point.radius);
                        (point.color, attenuation, point.position)
                    }
                };
                // Shadows
                if attenuation >= 0.01 {
                    let mut query_buffer = ArrayVec::<[Handle<OctreeNode>; 64]>::new();
                    let shadow_bias = 0.01;
                    if let Some(ray) = Ray::from_two_points(&light_position, &world_position) {
                        'outer_loop: for other_instance in other_instances {
                            other_instance
                                .octree
                                .ray_query_static(&ray, &mut query_buffer);
                            for &node in query_buffer.iter() {
                                match other_instance.octree.node(node) {
                                    OctreeNode::Leaf { indices, .. } => {
                                        let other_data = other_instance.data.read().unwrap();
                                        for &triangle_index in indices {
                                            let triangle =
                                                &other_data.triangles[triangle_index as usize];
                                            let a =
                                                other_instance.world_vertices[triangle[0] as usize];
                                            let b =
                                                other_instance.world_vertices[triangle[1] as usize];
                                            let c =
                                                other_instance.world_vertices[triangle[2] as usize];
                                            if let Some(pt) = ray.triangle_intersection(&[a, b, c])
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
                }
                pixel_color += light_color.scale(attenuation);
            }

            pixel.color = Vector3::new(
                (pixel_color.x.max(0.0).min(1.0) * 255.0) as u8,
                (pixel_color.y.max(0.0).min(1.0) * 255.0) as u8,
                (pixel_color.z.max(0.0).min(1.0) * 255.0) as u8,
            );
        }
    });

    let mut bytes = Vec::with_capacity((atlas_size * atlas_size * 3) as usize);
    for pixel in pixels {
        bytes.push(pixel.color.x);
        bytes.push(pixel.color.y);
        bytes.push(pixel.color.z);
    }
    let data = TextureData::from_bytes(
        TextureKind::Rectangle {
            width: atlas_size,
            height: atlas_size,
        },
        TexturePixelKind::RGB8,
        bytes,
    )
    .unwrap();

    data
}

#[cfg(test)]
mod test {
    use crate::{
        core::{
            algebra::{Matrix4, Vector3},
            color::Color,
        },
        renderer::surface::SurfaceSharedData,
        resource::texture::TextureKind,
        utils::{
            lightmap::{generate_lightmap, LightDefinition, PointLightDefinition},
            uvgen::generate_uvs,
        },
    };
    use image::RgbaImage;

    #[test]
    fn test_generate_lightmap() {
        //let mut data = SurfaceSharedData::make_sphere(20, 20, 1.0);
        let mut data = SurfaceSharedData::make_cone(
            16,
            1.0,
            1.0,
            Matrix4::new_nonuniform_scaling(&Vector3::new(1.0, 1.1, 1.0)),
        );
        let lights = [LightDefinition::Point(PointLightDefinition {
            intensity: 3.0,
            position: Vector3::new(0.0, 2.0, 0.0),
            color: Color::WHITE.as_frgb(),
            radius: 4.0,
        })];
        let lightmap = generate_lightmap(&mut data, &Matrix4::identity(), &lights, 128, true);

        let (w, h) = if let TextureKind::Rectangle { width, height } = lightmap.kind {
            (width, height)
        } else {
            unreachable!();
        };

        let image = RgbaImage::from_raw(w, h, lightmap.bytes).unwrap();
        image.save("lightmap.png").unwrap();
    }
}
