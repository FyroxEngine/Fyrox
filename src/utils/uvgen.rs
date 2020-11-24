//! UV Map generator. Used to generate second texture coordinates for lightmaps.
//!
//! Current implementation uses simple planar mapping.
use crate::{
    core::{
        algebra::Vector2,
        math::{self, PlaneClass, Vector2Ext},
        rectpack::RectPacker,
    },
    renderer::surface::SurfaceSharedData,
    scene::mesh::Mesh,
};
use rayon::prelude::*;

#[derive(Debug)]
struct UvMesh {
    // Array of indices of triangles.
    triangles: Vec<usize>,
    uv_max: Vector2<f32>,
    uv_min: Vector2<f32>,
}

impl UvMesh {
    fn new(first_triangle: usize) -> Self {
        Self {
            triangles: vec![first_triangle],
            uv_max: Vector2::new(-std::f32::MAX, -std::f32::MAX),
            uv_min: Vector2::new(std::f32::MAX, std::f32::MAX),
        }
    }

    pub fn width(&self) -> f32 {
        self.uv_max.x - self.uv_min.x
    }

    pub fn height(&self) -> f32 {
        self.uv_max.y - self.uv_min.y
    }

    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }
}

#[derive(Default, Debug)]
struct UvBox {
    px: Vec<usize>,
    nx: Vec<usize>,
    py: Vec<usize>,
    ny: Vec<usize>,
    pz: Vec<usize>,
    nz: Vec<usize>,
}

fn face_vs_face(
    data: &mut SurfaceSharedData,
    face_triangles: &[usize],
    other_face_triangles: &[usize],
) {
    for other_triangle_index in other_face_triangles.iter() {
        let other_triangle = data.triangles[*other_triangle_index].clone();
        for triangle_index in face_triangles.iter() {
            for vertex_index in data.triangles[*triangle_index].indices_mut() {
                for &other_vertex_index in other_triangle.indices() {
                    if *vertex_index == other_vertex_index {
                        // We have adjacency, add new vertex and fix current index.
                        let vertex = data.vertices[other_vertex_index as usize];
                        *vertex_index = data.vertices.len() as u32;
                        data.vertices.push(vertex);
                    }
                }
            }
        }
    }
}

fn make_seam(data: &mut SurfaceSharedData, face_triangles: &[usize], other_faces: &[&[usize]]) {
    for &other_face_triangles in other_faces.iter() {
        face_vs_face(data, face_triangles, other_face_triangles);
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
    let mut uv_box = UvBox::default();
    let mut projections = Vec::new();
    for (i, triangle) in data.triangles.iter().enumerate() {
        let a = data.vertices[triangle[0] as usize].position;
        let b = data.vertices[triangle[1] as usize].position;
        let c = data.vertices[triangle[2] as usize].position;
        let normal = (b - a).cross(&(c - a));
        let class = math::classify_plane(normal);
        match class {
            PlaneClass::XY => {
                if normal.z < 0.0 {
                    uv_box.nz.push(i);
                    projections.push([a.yx(), b.yx(), c.yx()])
                } else {
                    uv_box.pz.push(i);
                    projections.push([a.xy(), b.xy(), c.xy()]);
                }
            }
            PlaneClass::XZ => {
                if normal.y < 0.0 {
                    uv_box.ny.push(i);
                    projections.push([a.xz(), b.xz(), c.xz()])
                } else {
                    uv_box.py.push(i);
                    projections.push([a.zx(), b.zx(), c.zx()])
                }
            }
            PlaneClass::YZ => {
                if normal.x < 0.0 {
                    uv_box.nx.push(i);
                    projections.push([a.zy(), b.zy(), c.zy()])
                } else {
                    uv_box.px.push(i);
                    projections.push([a.yz(), b.yz(), c.yz()])
                }
            }
        }
    }

    // Step 2. Split vertices at boundary between each face. This step multiplies the
    // number of vertices at boundary so we'll get separate texture coordinates at
    // seams.
    make_seam(
        data,
        &uv_box.px,
        &[&uv_box.nx, &uv_box.py, &uv_box.ny, &uv_box.pz, &uv_box.nz],
    );
    make_seam(
        data,
        &uv_box.nx,
        &[&uv_box.px, &uv_box.py, &uv_box.ny, &uv_box.pz, &uv_box.nz],
    );

    make_seam(
        data,
        &uv_box.py,
        &[&uv_box.px, &uv_box.nx, &uv_box.ny, &uv_box.pz, &uv_box.nz],
    );
    make_seam(
        data,
        &uv_box.ny,
        &[&uv_box.py, &uv_box.nx, &uv_box.px, &uv_box.pz, &uv_box.nz],
    );

    make_seam(
        data,
        &uv_box.pz,
        &[&uv_box.nz, &uv_box.px, &uv_box.nx, &uv_box.py, &uv_box.ny],
    );
    make_seam(
        data,
        &uv_box.nz,
        &[&uv_box.pz, &uv_box.px, &uv_box.nx, &uv_box.py, &uv_box.ny],
    );

    // Step 3. Find separate "meshes" on uv map. After box mapping we will most likely
    // end up with set of faces, some of them may form meshes and each such mesh must
    // be moved with all faces it has.
    let mut meshes = Vec::new();
    let mut removed_triangles = vec![false; data.triangles.len()];
    for triangle_index in 0..data.triangles.len() {
        if !removed_triangles[triangle_index] {
            // Start off random triangle and continue gather adjacent triangles one by one.
            let mut mesh = UvMesh::new(triangle_index);
            removed_triangles[triangle_index] = true;

            let mut last_triangle = 1;
            let mut i = 0;
            while i < last_triangle {
                let triangle = &data.triangles[mesh.triangles[i]];
                // Push all adjacent triangles into mesh. This is brute force implementation.
                for (other_triangle_index, other_triangle) in data.triangles.iter().enumerate() {
                    if !removed_triangles[other_triangle_index] {
                        'vertex_loop: for &vertex_index in triangle.indices() {
                            for &other_vertex_index in other_triangle.indices() {
                                if vertex_index == other_vertex_index {
                                    mesh.triangles.push(other_triangle_index);
                                    removed_triangles[other_triangle_index] = true;
                                    // Push border further to continue iterating from added
                                    // triangle. This is needed because we checking one triangle
                                    // after another and we must continue if new triangles have
                                    // some adjacent ones.
                                    last_triangle += 1;
                                    break 'vertex_loop;
                                }
                            }
                        }
                    }
                }
                i += 1;
            }

            // Calculate bounds.
            for &triangle_index in mesh.triangles.iter() {
                let [a, b, c] = projections[triangle_index];
                mesh.uv_min = a
                    .per_component_min(&b)
                    .per_component_min(&c)
                    .per_component_min(&mesh.uv_min);
                mesh.uv_max = a
                    .per_component_max(&b)
                    .per_component_max(&c)
                    .per_component_max(&mesh.uv_max);
            }
            // Apply spacing to bounds.
            mesh.uv_min -= Vector2::new(spacing, spacing);
            mesh.uv_max += Vector2::new(spacing * 2.0, spacing * 2.0);
            meshes.push(mesh);
        }
    }

    // Step 4. Arrange and scale all meshes on uv map so it fits into [0;1] range.
    let area = meshes.iter().fold(0.0, |area, mesh| area + mesh.area());
    let square_side = area.sqrt();

    meshes.sort_unstable_by(|a, b| b.area().partial_cmp(&a.area()).unwrap());

    let mut rects = Vec::new();

    // Some empiric coefficient that large enough to make size big enough for all meshes.
    // This should be large enough to fit all meshes, but small to prevent losing of space.
    // We'll use iterative approach to pack everything as tight as possible: at each iteration
    // scale will be increased until packer is able to pack everything.
    let mut empiric_scale = 1.1;
    let mut scale = 1.0;
    'try_loop: for _ in 0..100 {
        rects.clear();

        // Calculate size of atlas for packer, we'll scale it later on.
        scale = 1.0 / (square_side * empiric_scale);

        // We'll pack into 1.0 square, our UVs must be in [0;1] range, no wrapping is allowed.
        let mut packer = RectPacker::new(1.0, 1.0);
        for mesh in meshes.iter() {
            if let Some(rect) = packer.find_free(mesh.width() * scale, mesh.height() * scale) {
                rects.push(rect);
            } else {
                // I don't know how to pass this by without iterative approach :(
                empiric_scale *= 1.33;
                continue 'try_loop;
            }
        }
    }

    for (i, rect) in rects.into_iter().enumerate() {
        let mesh = &meshes[i];

        for &triangle_index in mesh.triangles.iter() {
            for (&vertex_index, &projection) in data.triangles[triangle_index]
                .indices()
                .iter()
                .zip(&projections[triangle_index])
            {
                data.vertices[vertex_index as usize].second_tex_coord =
                    (projection - mesh.uv_min).scale(scale) + rect.position;
            }
        }
    }
}

/// Generates UVs for a specified mesh.
pub fn generate_uvs_mesh(mesh: &Mesh, spacing: f32) {
    let last = std::time::Instant::now();

    let data_set = mesh
        .surfaces()
        .iter()
        .map(|s| s.data().clone())
        .collect::<Vec<_>>();

    data_set.par_iter().for_each(|data| {
        generate_uvs(&mut data.lock().unwrap(), spacing);
    });

    println!("Generate UVs: {:?}", std::time::Instant::now() - last);
}

#[cfg(test)]
mod test {
    use crate::core::algebra::{Matrix4, Vector3};
    use crate::{renderer::surface::SurfaceSharedData, utils::uvgen::generate_uvs};
    use image::{Rgb, RgbImage};
    use imageproc::drawing::draw_line_segment_mut;

    #[test]
    fn test_generate_uvs() {
        //let mut data = SurfaceSharedData::make_sphere(100, 100, 1.0);
        //let mut data = SurfaceSharedData::make_cylinder(80, 1.0, 1.0, true, Matrix4::identity());
        //let mut data = SurfaceSharedData::make_cube(Matrix4::identity());
        let mut data = SurfaceSharedData::make_cone(
            16,
            1.0,
            1.0,
            Matrix4::new_nonuniform_scaling(&Vector3::new(1.0, 1.1, 1.0)),
        );
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

            draw_line_segment_mut(&mut image, (a.x, a.y), (b.x, b.y), white);
            draw_line_segment_mut(&mut image, (b.x, b.y), (c.x, c.y), white);
            draw_line_segment_mut(&mut image, (c.x, c.y), (a.x, a.y), white);
        }
        image.save("uvgen.png").unwrap();
    }
}
