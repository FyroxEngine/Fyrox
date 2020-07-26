//! UV Map generator. Used to generate second texture coordinates for lightmaps.
//!
//! Current implementation uses simple planar mapping.
use crate::{
    core::{
        math::{self, vec2::Vec2, vec3::Vec3, PlaneClass},
        rectpack::RectPacker,
    },
    renderer::surface::SurfaceSharedData,
};
use std::collections::HashSet;

#[derive(Clone, Copy)]
#[repr(usize)]
enum BoxFace {
    PositiveX = 0,
    NegativeX = 1,
    PositiveY = 2,
    NegativeY = 3,
    PositiveZ = 4,
    NegativeZ = 5,
}

fn box_map(a: Vec3, b: Vec3, c: Vec3) -> (BoxFace, [Vec2; 3]) {
    let normal = (b - a).cross(&(c - a));
    let class = math::classify_plane(normal);
    match class {
        PlaneClass::XY => {
            if normal.z < 0.0 {
                (BoxFace::NegativeZ, [a.yx(), b.yx(), c.yx()])
            } else {
                (BoxFace::PositiveZ, [a.xy(), b.xy(), c.xy()])
            }
        }
        PlaneClass::XZ => {
            if normal.y < 0.0 {
                (BoxFace::NegativeY, [a.xz(), b.xz(), c.xz()])
            } else {
                (BoxFace::PositiveY, [a.zx(), b.zx(), c.zx()])
            }
        }
        PlaneClass::YZ => {
            if normal.x < 0.0 {
                (BoxFace::NegativeX, [a.zy(), b.zy(), c.zy()])
            } else {
                (BoxFace::PositiveX, [a.yz(), b.yz(), c.yz()])
            }
        }
    }
}

#[derive(Debug)]
struct UvMesh {
    // Array of indices of triangles.
    triangles: Vec<usize>,
    uv_max: Vec2,
    uv_min: Vec2,
}

impl UvMesh {
    fn new(first_triangle: usize) -> Self {
        Self {
            triangles: vec![first_triangle],
            uv_max: Vec2::new(-std::f32::MAX, -std::f32::MAX),
            uv_min: Vec2::new(std::f32::MAX, std::f32::MAX),
        }
    }

    pub fn width(&self) -> f32 {
        self.uv_max.x - self.uv_min.x
    }

    pub fn height(&self) -> f32 {
        self.uv_max.y - self.uv_min.y
    }
}

/// Generates UV map for given surface data.
///
/// # Performance
///
/// This method utilizes lots of "brute force" algorithms, so it is not fast as it
/// could be in ideal case. It also allocates some memory for internal needs.
pub fn generate_uvs(data: &mut SurfaceSharedData, spacing: f32) {
    // Step 1. Map each triangle from surface to appropriate side of box.
    let mut face_triangles = vec![Vec::default(); 6];
    let mut projections = Vec::new();
    for (i, triangle) in data.triangles.iter().enumerate() {
        let a = data.vertices[triangle[0] as usize].position;
        let b = data.vertices[triangle[1] as usize].position;
        let c = data.vertices[triangle[2] as usize].position;
        let (face, proj) = box_map(a, b, c);
        face_triangles[face as usize].push(i);
        projections.push(proj);
    }

    // Step 2. Split vertices at boundary between each face. This step multiplies the
    // number of vertices at boundary so we'll get separate texture coordinates at
    // seams.
    //
    // TODO: This more or less brute force approach, needs profiling to check if it
    //  is slow and needs optimization.

    // It is safe to take second immutable reference to face triangles, we'll enforce
    // borrowing rules at runtime so there won't be two mutable references to same
    // memory. Borrow checker cannot understand this and we helping it by taking
    // responsibility to ourselves.
    let face_triangles2 = unsafe { &*(&face_triangles as *const Vec<Vec<usize>>) };

    for (i, part) in face_triangles.iter_mut().enumerate() {
        for (j, other_face_triangles) in face_triangles2.iter().enumerate() {
            // Enforce borrowing rules at runtime.
            if i == j {
                continue;
            }

            for triangle_idx in part.iter_mut() {
                // Check if other part has adjacent triangle to current one. And if so
                // add new vertices at boundary to form seam.
                for other_triangle_idx in other_face_triangles.iter() {
                    // This assert must always be true - triangle cannot belong to two
                    // faces of box at the same time, if assert fails - then first step
                    // is bugged.
                    assert_ne!(*triangle_idx, *other_triangle_idx);
                    let other_triangle = data.triangles[*other_triangle_idx].clone();
                    let triangle = &mut data.triangles[*triangle_idx];
                    for vi in triangle.indices_mut() {
                        for &ovi in other_triangle.indices() {
                            if *vi == ovi {
                                // We have adjacency, add new vertex and fix current index.
                                let vertex = data.vertices[ovi as usize].clone();
                                *vi = data.vertices.len() as u32;
                                data.vertices.push(vertex);
                            }
                        }
                    }
                }
            }
        }
    }

    // Step 3. Find separate "meshes" on uv map. After box mapping we will most likely
    // end up with set of faces, some of them may form meshes and each such mesh must
    // be moved with all faces it has.
    let mut meshes = Vec::new();
    let mut removed_triangles = HashSet::new();
    let mut new_triangles = Vec::new();
    for i in 0..data.triangles.len() {
        if !removed_triangles.contains(&i) {
            // Start off random triangle and continue gather adjacent triangles one by one.
            let mut mesh = UvMesh::new(i);
            removed_triangles.insert(i);
            'search: loop {
                new_triangles.clear();
                let mut adjacent_count = 0;
                for &k in mesh.triangles.iter() {
                    let triangle = &data.triangles[k];
                    // Push all adjacent triangles into mesh. This is brute force implementation.
                    for (j, other_triangle) in data.triangles.iter().enumerate() {
                        if !removed_triangles.contains(&j) {
                            'vloop: for &vi in triangle.indices() {
                                for &ovi in other_triangle.indices() {
                                    if vi == ovi {
                                        new_triangles.push(j);
                                        removed_triangles.insert(j);
                                        adjacent_count += 1;
                                        break 'vloop;
                                    }
                                }
                            }
                        }
                    }
                }
                mesh.triangles.extend_from_slice(&new_triangles);
                if adjacent_count == 0 {
                    break 'search;
                }
            }
            // Calculate bounds.
            for &triangle_index in mesh.triangles.iter() {
                let [a, b, c] = projections[triangle_index];
                mesh.uv_min = a.min(b).min(c).min(mesh.uv_min);
                mesh.uv_max = a.max(b).max(c).max(mesh.uv_max);
            }
            // Apply spacing to bounds.
            mesh.uv_min -= Vec2::new(spacing, spacing);
            mesh.uv_max += Vec2::new(spacing * 2.0, spacing * 2.0);
            meshes.push(mesh);
        }
    }

    // Step 4. Arrange and scale all meshes on uv map so it fits into [0;1] range.

    // Some empiric coefficient that large enough to make size big enough for all meshes.
    // This should be large enough to fit all meshes, but small to prevent losing of space.
    let empiric_scale = 1.25;

    // Calculate size of atlas for packer, we'll scale it later on.
    let size = meshes
        .iter()
        .fold(0.0, |area, mesh| area + mesh.width() * mesh.height())
        .sqrt()
        * empiric_scale;

    // We'll pack into 1.0 square, our UVs must be in [0;1] range, no wrapping is allowed.
    let mut packer = RectPacker::new(1.0, 1.0);
    for mesh in meshes {
        let scale = 1.0 / size;
        let w = mesh.width() * scale;
        let h = mesh.height() * scale;
        let rect = packer.find_free(w, h).unwrap();
        dbg!(rect);
        for &triangle_index in mesh.triangles.iter() {
            let [mut a, mut b, mut c] = projections[triangle_index];
            // Move to origin.
            a -= mesh.uv_min;
            b -= mesh.uv_min;
            c -= mesh.uv_min;
            // Scale.
            a = a.scale(scale);
            b = b.scale(scale);
            c = c.scale(scale);
            // Move back.
            a += Vec2::from(rect.position());
            b += Vec2::from(rect.position());
            c += Vec2::from(rect.position());
            let triangle = &data.triangles[triangle_index];
            data.vertices[triangle[0] as usize].second_tex_coord = a;
            data.vertices[triangle[1] as usize].second_tex_coord = b;
            data.vertices[triangle[2] as usize].second_tex_coord = c;
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{renderer::surface::SurfaceSharedData, utils::uvgen::generate_uvs};
    use image::{Rgb, RgbImage};
    use imageproc::drawing::draw_line_segment_mut;

    #[test]
    fn test_generate_uvs() {
        //let mut data = SurfaceSharedData::make_sphere(10, 10, 1.0);
        let mut data = SurfaceSharedData::make_cylinder(20, 1.0, 1.0, true, Default::default());
        // let mut data = SurfaceSharedData::make_cube(Default::default());
        generate_uvs(&mut data, 0.01);

        let white = Rgb([255u8, 255u8, 255u8]);
        let mut image = RgbImage::new(1024, 1024);
        for triangle in data.triangles.iter() {
            let a = data.vertices[triangle[0] as usize]
                .second_tex_coord
                .scale(1024.0);
            let b = data.vertices[triangle[1] as usize]
                .second_tex_coord
                .scale(1024.0);
            let c = data.vertices[triangle[2] as usize]
                .second_tex_coord
                .scale(1024.0);

            draw_line_segment_mut(&mut image, a.into(), b.into(), white);
            draw_line_segment_mut(&mut image, b.into(), c.into(), white);
            draw_line_segment_mut(&mut image, c.into(), a.into(), white);
        }
        image.save("uvgen.png").unwrap();
    }
}
