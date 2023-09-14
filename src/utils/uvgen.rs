//! UV Map generator. Used to generate second texture coordinates for lightmaps.
//!
//! Current implementation uses simple planar mapping.
use crate::{
    core::{
        algebra::Vector2,
        instant,
        math::{self, PlaneClass, TriangleDefinition, Vector2Ext},
        rectpack::RectPacker,
        reflect::prelude::*,
        visitor::prelude::*,
    },
    scene::mesh::{
        buffer::{
            TriangleBufferRefMut, VertexAttributeDataType, VertexAttributeDescriptor,
            VertexAttributeUsage, VertexBufferRefMut, VertexFetchError, VertexReadTrait,
            VertexWriteTrait,
        },
        surface::SurfaceData,
        Mesh,
    },
};
use fyrox_core::visitor::BinaryBlob;
use rayon::prelude::*;

/// A part of uv map.
#[derive(Debug)]
pub struct UvMesh {
    // Array of indices of triangles.
    triangles: Vec<usize>,
    uv_max: Vector2<f32>,
    uv_min: Vector2<f32>,
}

impl UvMesh {
    fn new(first_triangle: usize) -> Self {
        Self {
            triangles: vec![first_triangle],
            uv_max: Vector2::new(-f32::MAX, -f32::MAX),
            uv_min: Vector2::new(f32::MAX, f32::MAX),
        }
    }

    /// Returns total width of the mesh.
    pub fn width(&self) -> f32 {
        self.uv_max.x - self.uv_min.x
    }

    /// Returns total height of the mesh.
    pub fn height(&self) -> f32 {
        self.uv_max.y - self.uv_min.y
    }

    /// Returns total area of the mesh.
    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }
}

/// A set of faces with triangles belonging to faces.
#[derive(Default, Debug)]
pub struct UvBox {
    px: Vec<usize>,
    nx: Vec<usize>,
    py: Vec<usize>,
    ny: Vec<usize>,
    pz: Vec<usize>,
    nz: Vec<usize>,
    projections: Vec<[Vector2<f32>; 3]>,
}

fn face_vs_face(
    vertex_buffer: &mut VertexBufferRefMut,
    geometry_buffer_mut: &mut TriangleBufferRefMut,
    face_triangles: &[usize],
    other_face_triangles: &[usize],
    patch: &mut SurfaceDataPatch,
) {
    for other_triangle_index in other_face_triangles.iter() {
        let other_triangle = geometry_buffer_mut[*other_triangle_index].clone();
        for triangle_index in face_triangles.iter() {
            'outer_loop: for vertex_index in geometry_buffer_mut[*triangle_index].indices_mut() {
                for &other_vertex_index in other_triangle.indices() {
                    if *vertex_index == other_vertex_index {
                        // We have adjacency, add new vertex and fix current index.
                        patch.additional_vertices.push(other_vertex_index);
                        *vertex_index = vertex_buffer.vertex_count();
                        vertex_buffer.duplicate(other_vertex_index as usize);
                        continue 'outer_loop;
                    }
                }
            }
        }
    }
}

fn make_seam(
    vertex_buffer: &mut VertexBufferRefMut,
    geometry_buffer_mut: &mut TriangleBufferRefMut,
    face_triangles: &[usize],
    other_faces: &[&[usize]],
    patch: &mut SurfaceDataPatch,
) {
    for &other_face_triangles in other_faces.iter() {
        face_vs_face(
            vertex_buffer,
            geometry_buffer_mut,
            face_triangles,
            other_face_triangles,
            patch,
        );
    }
}

/// A patch for surface data that contains secondary texture coordinates and
/// new topology for data. It is needed for serialization: during the UV generation,
/// generator could multiply vertices to make seams, it adds new data to existing
/// vertices. The problem is that we do not serialize surface data - we store only a
/// "link" to resource from which we'll load surface data on deserialization. But
/// freshly loaded resource is not suitable for generated lightmap - in most cases
/// it just does not have secondary texture coordinates. So we have to patch data after
/// loading somehow with required data, this is where `SurfaceDataPatch` comes into
/// play.
#[derive(Clone, Debug, Default, Reflect)]
pub struct SurfaceDataPatch {
    /// A surface data id. Usually it is just a hash of surface data.
    pub data_id: u64,
    /// New topology for surface data. Old topology must be replaced with new,
    /// because UV generator splits vertices at uv map.
    pub triangles: Vec<TriangleDefinition>,
    /// List of second texture coordinates used for light maps.
    pub second_tex_coords: Vec<Vector2<f32>>,
    /// List of indices of vertices that must be cloned and pushed into vertices
    /// array of surface data.
    pub additional_vertices: Vec<u32>,
}

impl Visit for SurfaceDataPatch {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.data_id.visit("DataId", &mut region)?;
        BinaryBlob {
            vec: &mut self.triangles,
        }
        .visit("Triangles", &mut region)?;
        BinaryBlob {
            vec: &mut self.second_tex_coords,
        }
        .visit("SecondTexCoords", &mut region)?;
        BinaryBlob {
            vec: &mut self.additional_vertices,
        }
        .visit("AdditionalVertices", &mut region)?;

        Ok(())
    }
}

/// Maps each triangle from surface to appropriate side of box. This is so called
/// box mapping.
fn generate_uv_box(data: &SurfaceData) -> UvBox {
    let mut uv_box = UvBox::default();
    for (i, triangle) in data.geometry_buffer.iter().enumerate() {
        let a = data
            .vertex_buffer
            .get(triangle[0] as usize)
            .unwrap()
            .read_3_f32(VertexAttributeUsage::Position)
            .unwrap();
        let b = data
            .vertex_buffer
            .get(triangle[1] as usize)
            .unwrap()
            .read_3_f32(VertexAttributeUsage::Position)
            .unwrap();
        let c = data
            .vertex_buffer
            .get(triangle[2] as usize)
            .unwrap()
            .read_3_f32(VertexAttributeUsage::Position)
            .unwrap();
        let normal = (b - a).cross(&(c - a));
        let class = math::classify_plane(normal);
        match class {
            PlaneClass::XY => {
                if normal.z < 0.0 {
                    uv_box.nz.push(i);
                    uv_box.projections.push([a.yx(), b.yx(), c.yx()])
                } else {
                    uv_box.pz.push(i);
                    uv_box.projections.push([a.xy(), b.xy(), c.xy()]);
                }
            }
            PlaneClass::XZ => {
                if normal.y < 0.0 {
                    uv_box.ny.push(i);
                    uv_box.projections.push([a.xz(), b.xz(), c.xz()])
                } else {
                    uv_box.py.push(i);
                    uv_box.projections.push([a.zx(), b.zx(), c.zx()])
                }
            }
            PlaneClass::YZ => {
                if normal.x < 0.0 {
                    uv_box.nx.push(i);
                    uv_box.projections.push([a.zy(), b.zy(), c.zy()])
                } else {
                    uv_box.px.push(i);
                    uv_box.projections.push([a.yz(), b.yz(), c.yz()])
                }
            }
        }
    }
    uv_box
}

/// Generates a set of UV meshes.
pub fn generate_uv_meshes(
    uv_box: &UvBox,
    data_id: u64,
    vertex_buffer_mut: &mut VertexBufferRefMut,
    geometry_buffer_mut: &mut TriangleBufferRefMut,
) -> (Vec<UvMesh>, SurfaceDataPatch) {
    let mut mesh_patch = SurfaceDataPatch {
        data_id,
        ..Default::default()
    };

    if !vertex_buffer_mut.has_attribute(VertexAttributeUsage::TexCoord1) {
        vertex_buffer_mut
            .add_attribute(
                VertexAttributeDescriptor {
                    usage: VertexAttributeUsage::TexCoord1,
                    data_type: VertexAttributeDataType::F32,
                    size: 2,
                    divisor: 0,
                    shader_location: 6, // HACK: GBuffer renderer expects it to be at 6
                },
                Vector2::<f32>::default(),
            )
            .unwrap();
    }

    // Step 1. Split vertices at boundary between each face. This step multiplies the
    // number of vertices at boundary so we'll get separate texture coordinates at
    // seams.
    make_seam(
        vertex_buffer_mut,
        geometry_buffer_mut,
        &uv_box.px,
        &[&uv_box.nx, &uv_box.py, &uv_box.ny, &uv_box.pz, &uv_box.nz],
        &mut mesh_patch,
    );
    make_seam(
        vertex_buffer_mut,
        geometry_buffer_mut,
        &uv_box.nx,
        &[&uv_box.px, &uv_box.py, &uv_box.ny, &uv_box.pz, &uv_box.nz],
        &mut mesh_patch,
    );

    make_seam(
        vertex_buffer_mut,
        geometry_buffer_mut,
        &uv_box.py,
        &[&uv_box.px, &uv_box.nx, &uv_box.ny, &uv_box.pz, &uv_box.nz],
        &mut mesh_patch,
    );
    make_seam(
        vertex_buffer_mut,
        geometry_buffer_mut,
        &uv_box.ny,
        &[&uv_box.py, &uv_box.nx, &uv_box.px, &uv_box.pz, &uv_box.nz],
        &mut mesh_patch,
    );

    make_seam(
        vertex_buffer_mut,
        geometry_buffer_mut,
        &uv_box.pz,
        &[&uv_box.nz, &uv_box.px, &uv_box.nx, &uv_box.py, &uv_box.ny],
        &mut mesh_patch,
    );
    make_seam(
        vertex_buffer_mut,
        geometry_buffer_mut,
        &uv_box.nz,
        &[&uv_box.pz, &uv_box.px, &uv_box.nx, &uv_box.py, &uv_box.ny],
        &mut mesh_patch,
    );

    // Step 2. Find separate "meshes" on uv map. After box mapping we will most likely
    // end up with set of faces, some of them may form meshes and each such mesh must
    // be moved with all faces it has.
    let mut meshes = vec![];
    let mut removed_triangles = vec![false; geometry_buffer_mut.len()];
    for triangle_index in 0..geometry_buffer_mut.len() {
        if !removed_triangles[triangle_index] {
            // Start off random triangle and continue gather adjacent triangles one by one.
            let mut mesh = UvMesh::new(triangle_index);
            removed_triangles[triangle_index] = true;

            let mut last_triangle = 1;
            let mut i = 0;
            while i < last_triangle {
                let triangle = &geometry_buffer_mut[mesh.triangles[i]];
                // Push all adjacent triangles into mesh. This is brute force implementation.
                for (other_triangle_index, other_triangle) in geometry_buffer_mut.iter().enumerate()
                {
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
                let [a, b, c] = uv_box.projections[triangle_index];
                mesh.uv_min = a
                    .per_component_min(&b)
                    .per_component_min(&c)
                    .per_component_min(&mesh.uv_min);
                mesh.uv_max = a
                    .per_component_max(&b)
                    .per_component_max(&c)
                    .per_component_max(&mesh.uv_max);
            }
            meshes.push(mesh);
        }
    }

    (meshes, mesh_patch)
}

/// Generates UV map for given surface data.
///
/// # Performance
///
/// This method utilizes lots of "brute force" algorithms, so it is not fast as it
/// could be in ideal case. It also allocates some memory for internal needs.
pub fn generate_uvs(
    data: &mut SurfaceData,
    spacing: f32,
) -> Result<SurfaceDataPatch, VertexFetchError> {
    let uv_box = generate_uv_box(data);

    let data_id = data.content_hash();
    let mut vertex_buffer_mut = data.vertex_buffer.modify();
    let mut geometry_buffer_mut = data.geometry_buffer.modify();
    let (mut meshes, mut patch) = generate_uv_meshes(
        &uv_box,
        data_id,
        &mut vertex_buffer_mut,
        &mut geometry_buffer_mut,
    );
    drop(geometry_buffer_mut);

    // Step 4. Arrange and scale all meshes on uv map so it fits into [0;1] range.
    let area = meshes.iter().fold(0.0, |area, mesh| area + mesh.area());
    let square_side = area.sqrt() + spacing * meshes.len() as f32;

    meshes.sort_unstable_by(|a, b| b.area().partial_cmp(&a.area()).unwrap());

    let mut rects = vec![];

    let twice_spacing = spacing * 2.0;

    // Some empiric coefficient that large enough to make size big enough for all meshes.
    // This should be large enough to fit all meshes, but small to prevent losing of space.
    // We'll use iterative approach to pack everything as tight as possible: at each iteration
    // scale will be increased until packer is able to pack everything.
    let mut empiric_scale = 1.1;
    let mut scale = 1.0;
    let mut packer = RectPacker::new(1.0, 1.0);
    'try_loop: for _ in 0..100 {
        rects.clear();

        // Calculate size of atlas for packer, we'll scale it later on.
        scale = 1.0 / (square_side * empiric_scale);

        // We'll pack into 1.0 square, our UVs must be in [0;1] range, no wrapping is allowed.
        packer.clear();
        for mesh in meshes.iter() {
            if let Some(rect) = packer.find_free(
                mesh.width() * scale + twice_spacing,
                mesh.height() * scale + twice_spacing,
            ) {
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
            for (&vertex_index, &projection) in data.geometry_buffer[triangle_index]
                .indices()
                .iter()
                .zip(&uv_box.projections[triangle_index])
            {
                vertex_buffer_mut
                    .get_mut(vertex_index as usize)
                    .unwrap()
                    .write_2_f32(
                        VertexAttributeUsage::TexCoord1,
                        (projection - mesh.uv_min).scale(scale)
                            + Vector2::new(spacing, spacing)
                            + rect.position,
                    )?;
            }
        }
    }

    patch.triangles = data.geometry_buffer.triangles_ref().to_vec();

    for view in vertex_buffer_mut.iter() {
        patch
            .second_tex_coords
            .push(view.read_2_f32(VertexAttributeUsage::TexCoord1)?);
    }

    Ok(patch)
}

/// Generates UVs for a specified mesh.
pub fn generate_uvs_mesh(
    mesh: &Mesh,
    spacing: f32,
) -> Result<Vec<SurfaceDataPatch>, VertexFetchError> {
    let last = instant::Instant::now();

    let data_set = mesh.surfaces().iter().map(|s| s.data()).collect::<Vec<_>>();

    let patches = data_set
        .into_par_iter()
        .map(|data| generate_uvs(&mut data.lock(), spacing))
        .collect::<Result<Vec<SurfaceDataPatch>, VertexFetchError>>()?;

    println!("Generate UVs: {:?}", instant::Instant::now() - last);

    Ok(patches)
}
