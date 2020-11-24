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
        color::Color,
        math::{self, Matrix4Ext, Rect, TriangleDefinition, Vector2Ext},
        pool::Handle,
        visitor::{Visit, VisitResult, Visitor},
    },
    renderer::{surface::SurfaceSharedData, surface::Vertex},
    resource::texture::{
        Texture, TextureData, TextureError, TextureKind, TexturePixelKind, TextureState,
    },
    scene::{light::Light, node::Node, Scene},
};
use rayon::prelude::*;
use std::{collections::HashMap, path::Path, time};

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
                            direction: light
                                .up_vector()
                                .try_normalize(std::f32::EPSILON)
                                .unwrap_or_else(Vector3::y),
                            color: light.color().as_frgb(),
                        }),
                    )),
                    Light::Spot(spot) => lights.push((
                        handle,
                        LightDefinition::Spot(SpotLightDefinition {
                            intensity: 1.0,
                            edge0: ((spot.hotspot_cone_angle() + spot.falloff_angle_delta()) * 0.5)
                                .cos(),
                            edge1: (spot.hotspot_cone_angle() * 0.5).cos(),
                            color: light.color().as_frgb(),
                            direction: light
                                .up_vector()
                                .try_normalize(std::f32::EPSILON)
                                .unwrap_or_else(Vector3::y),
                            position: light.global_position(),
                            distance: spot.distance(),
                        }),
                    )),
                    Light::Point(point) => lights.push((
                        handle,
                        LightDefinition::Point(PointLightDefinition {
                            intensity: 1.0,
                            position: light.global_position(),
                            color: light.color().as_frgb(),
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
                let global_transform = mesh.global_transform();
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
                        texture: Some(Texture::new(TextureState::Ok(lightmap))),
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
    pub fn save<P: AsRef<Path>>(&self, base_path: P) -> Result<(), TextureError> {
        for (handle, entries) in self.map.iter() {
            let handle_path = handle.index().to_string();
            for (i, entry) in entries.iter().enumerate() {
                let file_path = handle_path.clone() + "_" + i.to_string().as_str() + ".png";
                let texture = entry.texture.clone().unwrap();
                let mut texture = texture.state();
                if let TextureState::Ok(texture) = &mut *texture {
                    texture.set_path(&base_path.as_ref().join(file_path));
                    texture.save()?;
                }
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
    pub direction: Vector3<f32>,
    /// Color of light.
    pub color: Vector3<f32>,
}

/// Spot light is a cone light source. Example: flashlight.
pub struct SpotLightDefinition {
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
    color: Color,
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
            for _ in 0..2 {
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
/// time it will take. Required time increases drastically if you enable shadows (TODO) and
/// global illumination (TODO), because in this case your data will be raytraced.
fn generate_lightmap<'a, I: IntoIterator<Item = &'a LightDefinition>>(
    data: &SurfaceSharedData,
    transform: &Matrix4<f32>,
    lights: I,
    texels_per_unit: u32,
) -> TextureData {
    let last_time = time::Instant::now();

    let world_positions = transform_vertices(data, transform);
    let size = estimate_size(&world_positions, &data.triangles, texels_per_unit);

    let scale = 1.0 / size as f32;

    let grid = Grid::new(data, (size / 16).max(4) as usize);

    let normal_matrix = transform
        .basis()
        .try_inverse()
        .map(|m| m.transpose())
        .unwrap_or_else(Matrix3::identity);

    let mut pixels = Vec::with_capacity((size * size) as usize);
    for y in 0..(size as usize) {
        for x in 0..(size as usize) {
            pixels.push(Pixel {
                coords: Vector2::new(x as u16, y as u16),
                color: Color::TRANSPARENT,
            });
        }
    }

    let lights: Vec<&LightDefinition> = lights.into_iter().collect();

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
            for light in &lights {
                let (light_color, attenuation) = match light {
                    LightDefinition::Directional(directional) => {
                        let attenuation =
                            directional.intensity * lambertian(directional.direction, world_normal);
                        (directional.color, attenuation)
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
                        (spot.color, attenuation)
                    }
                    LightDefinition::Point(point) => {
                        let d = point.position - world_position;
                        let distance = d.norm();
                        let light_vec = d.scale(1.0 / distance);
                        let attenuation = point.intensity
                            * lambertian(light_vec, world_normal)
                            * distance_attenuation(distance, point.radius);
                        (point.color, attenuation)
                    }
                };
                pixel_color += light_color.scale(attenuation);
            }

            pixel.color = Color::from(pixel_color);
        }
    });

    let mut bytes = Vec::with_capacity((size * size * 4) as usize);
    for pixel in pixels {
        bytes.push(pixel.color.r);
        bytes.push(pixel.color.g);
        bytes.push(pixel.color.b);
        bytes.push(pixel.color.a);
    }
    let data = TextureData::from_bytes(
        TextureKind::Rectangle {
            width: size,
            height: size,
        },
        TexturePixelKind::RGBA8,
        bytes,
    )
    .unwrap();

    println!(
        "Lightmap generated in: {:?}",
        time::Instant::now() - last_time
    );

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

        generate_uvs(&mut data, 0.01);

        let lights = [LightDefinition::Point(PointLightDefinition {
            intensity: 3.0,
            position: Vector3::new(0.0, 2.0, 0.0),
            color: Color::WHITE.as_frgb(),
            radius: 4.0,
        })];
        let lightmap = generate_lightmap(&data, &Matrix4::identity(), &lights, 128);

        let (w, h) = if let TextureKind::Rectangle { width, height } = lightmap.kind {
            (width, height)
        } else {
            unreachable!();
        };

        let image = RgbaImage::from_raw(w, h, lightmap.bytes).unwrap();
        image.save("lightmap.png").unwrap();
    }
}
