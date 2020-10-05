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
        color::Color,
        math::{self, mat3::Mat3, mat4::Mat4, vec2::Vec2, vec3::Vec3, Rect, TriangleDefinition},
        pool::Handle,
        visitor::{Visit, VisitResult, Visitor},
    },
    renderer::{surface::SurfaceSharedData, surface::Vertex},
    resource::texture::{Texture, TextureKind},
    scene::{light::Light, node::Node, Scene},
};
use image::ImageError;
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
    time,
};

///
#[derive(Default, Clone, Debug)]
pub struct LightmapEntry {
    /// Lightmap texture.
    ///
    /// TODO: Is single texture enough? There may be surfaces with huge amount of faces
    ///  which may not fit into texture, because there is hardware limit on most GPUs
    ///  up to 8192x8192 pixels.
    pub texture: Option<Arc<Mutex<Texture>>>,
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
}

impl Visit for Lightmap {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.map.visit(name, visitor)?;

        visitor.leave_region()
    }
}

impl Lightmap {
    /// Generates lightmap for given scene.
    /// Each mesh *must* have generated UVs for lightmap, otherwise result will be incorrect!    
    pub fn new(scene: &Scene, texels_per_unit: u32) -> Self {
        // Extract info about lights first. We need it to be in separate array because
        // it won't be possible to store immutable references to light sources and at the
        // same time modify meshes.
        let mut lights = Vec::new();
        for (handle, node) in scene.graph.pair_iter() {
            if let Node::Light(light) = node {
                match light {
                    Light::Directional(_) => lights.push((
                        handle,
                        LightDefinition::Directional(DirectionalLightDefinition {
                            intensity: 1.0,
                            direction: light.up_vector().normalized().unwrap_or(Vec3::UP),
                            color: light.color(),
                        }),
                    )),
                    Light::Spot(spot) => lights.push((
                        handle,
                        LightDefinition::Spot(SpotLightDefinition {
                            intensity: 1.0,
                            hotspot_cone_angle: spot.hotspot_cone_angle(),
                            falloff_angle_delta: spot.falloff_angle_delta(),
                            color: light.color(),
                            direction: light.up_vector().normalized().unwrap_or(Vec3::UP),
                            position: light.global_position(),
                            distance: spot.distance(),
                        }),
                    )),
                    Light::Point(point) => lights.push((
                        handle,
                        LightDefinition::Point(PointLightDefinition {
                            intensity: 1.0,
                            position: light.global_position(),
                            color: light.color(),
                            radius: point.radius(),
                        }),
                    )),
                }
            }
        }
        let mut map = HashMap::new();
        for (handle, node) in scene.graph.pair_iter() {
            if let Node::Mesh(mesh) = node {
                if !mesh.global_visibility() {
                    continue;
                }
                let global_transform = mesh.global_transform;
                let mut surface_lightmaps = Vec::new();
                for surface in mesh.surfaces() {
                    let data = surface.data();
                    let data = data.lock().unwrap();
                    let lightmap = generate_lightmap(
                        &data,
                        &global_transform,
                        lights.iter().map(|(_, definition)| definition),
                        texels_per_unit,
                    );
                    surface_lightmaps.push(LightmapEntry {
                        texture: Some(Arc::new(Mutex::new(lightmap))),
                        lights: lights
                            .iter()
                            .map(|(light_handle, _)| *light_handle)
                            .collect(),
                    })
                }
                map.insert(handle, surface_lightmaps);
            }
        }
        Self { map }
    }

    /// Saves lightmap textures into specified folder.
    pub fn save<P: AsRef<Path>>(&self, base_path: P) -> Result<(), ImageError> {
        for (handle, entries) in self.map.iter() {
            let handle_path = handle.index().to_string();
            for (i, entry) in entries.iter().enumerate() {
                let file_path = handle_path.clone() + "_" + i.to_string().as_str() + ".png";
                let texture = entry.texture.clone().unwrap();
                let mut texture = texture.lock().unwrap();
                texture.set_path(&base_path.as_ref().join(file_path));
                texture.save()?;
            }
        }
        Ok(())
    }
}

/// Directional light is a light source with parallel rays. Example: Sun.
pub struct DirectionalLightDefinition {
    /// Intensity is how bright light is. Default is 1.0.
    pub intensity: f32,
    /// Direction of light rays.
    pub direction: Vec3,
    /// Color of light.
    pub color: Color,
}

/// Spot light is a cone light source. Example: flashlight.
pub struct SpotLightDefinition {
    /// Intensity is how bright light is. Default is 1.0.
    pub intensity: f32,
    /// Angle (in radians) at cone top which defines area with uniform light.  
    pub hotspot_cone_angle: f32,
    /// Angle delta (in radians) outside of cone top which sets area of smooth
    /// transition of intensity from max to min.
    pub falloff_angle_delta: f32,
    /// Color of light.
    pub color: Color,
    /// Direction vector of light.
    pub direction: Vec3,
    /// Position of light in world coordinates.
    pub position: Vec3,
    /// Distance at which light intensity decays to zero.
    pub distance: f32,
}

/// Point light is a spherical light source. Example: light bulb.
pub struct PointLightDefinition {
    /// Intensity is how bright light is. Default is 1.0.
    pub intensity: f32,
    /// Position of light in world coordinates.
    pub position: Vec3,
    /// Color of light.
    pub color: Color,
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

/// Computes total area of triangles in surface data and returns size of square
/// in which triangles can fit.
fn estimate_size(vertices: &[Vec3], triangles: &[TriangleDefinition], texels_per_unit: u32) -> u32 {
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
fn transform_vertices(data: &SurfaceSharedData, transform: &Mat4) -> Vec<Vec3> {
    data.vertices
        .iter()
        .map(|v| transform.transform_vector(v.position))
        .collect()
}

enum Pixel {
    Transparent,
    Color {
        color: Color,
        position: Vec3,
        normal: Vec3,
    },
}

/// Calculates properties of pixel (world position, normal) at given position.
fn pick(
    uv: Vec2,
    grid: &Grid,
    triangles: &[TriangleDefinition],
    vertices: &[Vertex],
    world_positions: &[Vec3],
    normal_matrix: &Mat3,
    scale: f32,
) -> Option<(Vec3, Vec3)> {
    if let Some(cell) = grid.pick(uv) {
        for triangle in cell.triangles.iter().map(|&ti| &triangles[ti]) {
            let uv_a = vertices[triangle[0] as usize].second_tex_coord;
            let uv_b = vertices[triangle[1] as usize].second_tex_coord;
            let uv_c = vertices[triangle[2] as usize].second_tex_coord;

            let center = (uv_a + uv_b + uv_c).scale(1.0 / 3.0);
            let to_center = (center - uv).normalized().unwrap_or_default().scale(scale);

            let mut current_uv = uv;
            for _ in 0..2 {
                let barycentric = math::get_barycentric_coords_2d(current_uv, uv_a, uv_b, uv_c);

                if math::barycentric_is_inside(barycentric) {
                    let a = world_positions[triangle[0] as usize];
                    let b = world_positions[triangle[1] as usize];
                    let c = world_positions[triangle[2] as usize];
                    return Some((
                        math::barycentric_to_world(barycentric, a, b, c),
                        normal_matrix
                            .transform_vector(math::barycentric_to_world(
                                barycentric,
                                vertices[triangle[0] as usize].normal,
                                vertices[triangle[1] as usize].normal,
                                vertices[triangle[2] as usize].normal,
                            ))
                            .normalized()
                            .unwrap_or(Vec3::UP),
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
                let bounds = Rect {
                    x: x as f32 / fsize,
                    y: y as f32 / fsize,
                    w: 1.0 / fsize,
                    h: 1.0 / fsize,
                };

                let mut triangles = Vec::new();

                for (triangle_index, triangle) in data.triangles.iter().enumerate() {
                    let uv_a = data.vertices[triangle[0] as usize].second_tex_coord;
                    let uv_b = data.vertices[triangle[1] as usize].second_tex_coord;
                    let uv_c = data.vertices[triangle[2] as usize].second_tex_coord;
                    let uv_min = uv_a.min(uv_b).min(uv_c);
                    let uv_max = uv_a.max(uv_b).max(uv_c);
                    let triangle_bounds = Rect {
                        x: uv_min.x,
                        y: uv_min.y,
                        w: uv_max.x - uv_min.x,
                        h: uv_max.y - uv_min.y,
                    };
                    if triangle_bounds.intersects(bounds) {
                        triangles.push(triangle_index);
                    }
                }

                cells.push(GridCell { triangles })
            }
        }

        Self { cells, size }
    }

    fn pick(&self, v: Vec2) -> Option<&GridCell> {
        let ix = (v.x as f32 * self.size as f32) as usize;
        let iy = (v.y as f32 * self.size as f32) as usize;
        self.cells.get(iy * self.size + ix)
    }
}

/// https://en.wikipedia.org/wiki/Lambert%27s_cosine_law
fn lambertian(light_vec: Vec3, normal: Vec3) -> f32 {
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
/// time it will take. Required time increases drastically if you enable shadows (TODO) and
/// global illumination (TODO), because in this case your data will be raytraced.
fn generate_lightmap<'a, I: IntoIterator<Item = &'a LightDefinition>>(
    data: &SurfaceSharedData,
    transform: &Mat4,
    lights: I,
    texels_per_unit: u32,
) -> Texture {
    let world_positions = transform_vertices(data, transform);
    let size = estimate_size(&world_positions, &data.triangles, texels_per_unit);
    let mut pixels = Vec::<Pixel>::with_capacity((size * size) as usize);

    let scale = 1.0 / size as f32;

    let last_time = time::Instant::now();

    let grid = Grid::new(data, (size / 16).max(4) as usize);

    println!("Step 0: {:?}", time::Instant::now() - last_time);

    // TODO: Must be inverse transposed to eliminate scale/shear.
    let normal_matrix = transform.basis();

    let last_time = time::Instant::now();

    let half_pixel = scale * 0.5;
    for y in 0..(size as usize) {
        for x in 0..(size as usize) {
            // Get uv in center of pixel.
            let uv = Vec2::new(x as f32 * scale + half_pixel, y as f32 * scale + half_pixel);

            if let Some((world_position, normal)) = pick(
                uv,
                &grid,
                &data.triangles,
                &data.vertices,
                &world_positions,
                &normal_matrix,
                scale,
            ) {
                pixels.push(Pixel::Color {
                    color: Color::opaque(0, 0, 0),
                    position: world_position,
                    normal,
                })
            } else {
                pixels.push(Pixel::Transparent)
            }
        }
    }

    println!("Step 1: {:?}", time::Instant::now() - last_time);

    let last_time = time::Instant::now();

    let lights: Vec<&LightDefinition> = lights.into_iter().collect();

    for pixel in pixels.iter_mut() {
        if let Pixel::Color {
            color,
            position,
            normal,
        } = pixel
        {
            for light in &lights {
                let (light_color, attenuation) = match light {
                    LightDefinition::Directional(directional) => {
                        let attenuation =
                            directional.intensity * lambertian(directional.direction, *normal);
                        (directional.color, attenuation)
                    }
                    LightDefinition::Spot(spot) => {
                        let d = spot.position - *position;
                        let distance = d.len();
                        let light_vec = d.scale(1.0 / distance);
                        let spot_angle_cos = light_vec.dot(&spot.direction);
                        let cone_factor = smoothstep(
                            ((spot.hotspot_cone_angle + spot.falloff_angle_delta) * 0.5).cos(),
                            (spot.hotspot_cone_angle * 0.5).cos(),
                            spot_angle_cos,
                        );
                        let attenuation = cone_factor
                            * spot.intensity
                            * lambertian(light_vec, *normal)
                            * distance_attenuation(distance, spot.distance);
                        (spot.color, attenuation)
                    }
                    LightDefinition::Point(point) => {
                        let d = point.position - *position;
                        let distance = d.len();
                        let light_vec = d.scale(1.0 / distance);
                        let attenuation = point.intensity
                            * lambertian(light_vec, *normal)
                            * distance_attenuation(distance, point.radius);
                        (point.color, attenuation)
                    }
                };
                color.r =
                    (color.r as f32 + ((light_color.r as f32) * attenuation)).min(255.0) as u8;
                color.g =
                    (color.g as f32 + ((light_color.g as f32) * attenuation)).min(255.0) as u8;
                color.b =
                    (color.b as f32 + ((light_color.b as f32) * attenuation)).min(255.0) as u8;
            }
        }
    }

    println!("Step 2: {:?}", time::Instant::now() - last_time);

    let mut bytes = Vec::with_capacity((size * size * 4) as usize);
    for pixel in pixels {
        let color = match pixel {
            Pixel::Transparent => Color::TRANSPARENT,
            Pixel::Color { color, .. } => color,
        };
        bytes.push(color.r);
        bytes.push(color.g);
        bytes.push(color.b);
        bytes.push(color.a);
    }
    Texture::from_bytes(size, size, TextureKind::RGBA8, bytes).unwrap()
}

#[cfg(test)]
mod test {
    use crate::{
        core::{color::Color, math::vec3::Vec3},
        renderer::surface::SurfaceSharedData,
        utils::{
            lightmap::{generate_lightmap, LightDefinition, PointLightDefinition},
            uvgen::generate_uvs,
        },
    };
    use image::RgbaImage;

    #[test]
    fn test_generate_lightmap() {
        let mut data = SurfaceSharedData::make_sphere(20, 20, 1.0);

        generate_uvs(&mut data, 0.01);

        let lights = [LightDefinition::Point(PointLightDefinition {
            intensity: 3.0,
            position: Vec3::new(0.0, 2.0, 0.0),
            color: Color::WHITE,
            radius: 4.0,
        })];
        let lightmap = generate_lightmap(&data, &Default::default(), &lights, 128);

        let image = RgbaImage::from_raw(lightmap.width, lightmap.height, lightmap.bytes).unwrap();
        image.save("lightmap.png").unwrap();
    }
}
