use crate::{
    core::{
        algebra::{Vector2, Vector3},
        math::TriangleDefinition,
    },
    renderer::framework::geometry_buffer::ElementRange,
    scene::mesh::{
        buffer::{TriangleBuffer, VertexBuffer},
        surface::{SurfaceData, SurfaceSharedData},
        vertex::StaticVertex,
    },
};

#[derive(Default, Debug, Clone)]
pub struct TerrainGeometry {
    pub data: SurfaceSharedData,
    /// Triangle ranges for each quadrant (in clockwise order; left-top -> right-top -> right-bottom -> left-bottom)  
    pub quadrants: [ElementRange; 4],
}

impl TerrainGeometry {
    pub fn new(mesh_size: Vector2<u32>) -> Self {
        let mut surface_data = SurfaceData::new(
            VertexBuffer::new::<StaticVertex>(0, vec![]).unwrap(),
            TriangleBuffer::default(),
            false,
        );

        let mut vertex_buffer_mut = surface_data.vertex_buffer.modify();
        // Form vertex buffer.
        for iy in 0..mesh_size.y {
            let kz = iy as f32 / ((mesh_size.y - 1) as f32);
            for x in 0..mesh_size.x {
                let kx = x as f32 / ((mesh_size.x - 1) as f32);

                vertex_buffer_mut
                    .push_vertex(&StaticVertex {
                        position: Vector3::new(kx, 0.0, kz),
                        tex_coord: Vector2::new(kx, kz),
                        // Normals and tangents will be calculated later.
                        normal: Default::default(),
                        tangent: Default::default(),
                    })
                    .unwrap();
            }
        }
        drop(vertex_buffer_mut);

        let mut geometry_buffer_mut = surface_data.geometry_buffer.modify();

        let half_size = mesh_size / 2;

        let mut quadrants = [ElementRange::Full; 4];
        for ((x_range, y_range), quadrant) in [
            (0..(half_size.x + 1), 0..(half_size.y + 1)),
            ((half_size.x - 1)..mesh_size.x, 0..(half_size.y + 1)),
            (
                (half_size.x - 1)..mesh_size.x,
                (half_size.y - 1)..mesh_size.y,
            ),
            (0..(half_size.x + 1), (half_size.y - 1)..mesh_size.y),
        ]
        .into_iter()
        .zip(&mut quadrants)
        {
            let offset = geometry_buffer_mut.len();

            for iy in y_range.start..y_range.end - 1 {
                let iy_next = iy + 1;
                for x in x_range.start..x_range.end - 1 {
                    let x_next = x + 1;

                    let i0 = iy * mesh_size.x + x;
                    let i1 = iy_next * mesh_size.x + x;
                    let i2 = iy_next * mesh_size.x + x_next;
                    let i3 = iy * mesh_size.x + x_next;

                    geometry_buffer_mut.push(TriangleDefinition([i0, i1, i2]));
                    geometry_buffer_mut.push(TriangleDefinition([i2, i3, i0]));
                }
            }

            *quadrant = ElementRange::Specific {
                offset,
                count: geometry_buffer_mut.len() - offset,
            };
        }
        drop(geometry_buffer_mut);

        surface_data.calculate_normals().unwrap();
        surface_data.calculate_tangents().unwrap();

        Self {
            data: SurfaceSharedData::new(surface_data),
            quadrants,
        }
    }
}
