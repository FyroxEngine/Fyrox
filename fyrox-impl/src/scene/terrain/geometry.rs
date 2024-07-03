use crate::{
    core::{
        algebra::{Vector2, Vector3, Vector4},
        math::TriangleDefinition,
    },
    renderer::framework::geometry_buffer::ElementRange,
    scene::mesh::{
        buffer::{TriangleBuffer, VertexBuffer},
        surface::{SurfaceData, SurfaceResource},
        vertex::StaticVertex,
    },
};
use fyrox_resource::untyped::ResourceKind;

/// The [SurfaceSharedData](crate::scene::mesh::surface::SurfaceResource) of a grid mesh for use
/// in rendering a terrain.
#[derive(Default, Debug, Clone)]
pub struct TerrainGeometry {
    pub data: SurfaceResource,
    /// Triangle ranges for each quadrant (in clockwise order; left-top -> right-top -> right-bottom -> left-bottom).
    /// This is used when creating an instance to render just one particular quadrant of a
    /// [QuadTreeNode](crate::scene::terrain::quadtree::QuadTree).
    pub quadrants: [ElementRange; 4],
}

impl TerrainGeometry {
    /// Create a grid mesh with the given number of rows and columns of vertices.
    /// For example, if mesh_size were (3,3), the resulting mesh would have 9 vertices
    /// and 4 quads, each made of two triangles, for a total of 8 triangles.
    ///
    /// Because the mesh will be divided into quadrants, the mesh_size *must not* be smaller than 3
    /// in either dimension.
    ///
    /// Quadrants are calculated by dividing each dimension of the mesh by 2.
    /// This will only produce equal-sized quadrants if (mesh_size.x - 1) is even
    /// and (mesh_size.y - 1) is even.
    ///
    /// If mesh_size is (3,3), then the 4 quads will be split evenly between
    /// the four quadrants, one quad per quadrant.
    /// If mesh_size is (4,4), then the 9 quads will be split unevenly, with one
    /// quadrant getting 1 quad, two quadrants getting 2 quads, and one quardrant getting 4 quads.
    /// If mesh_size is (5,5), then the 16 quads will be split evenly, with each quardant getting 4 quads.
    pub fn new(mesh_size: Vector2<u32>) -> Self {
        let mut surface_data = SurfaceData::new(
            VertexBuffer::new::<StaticVertex>(0, vec![]).unwrap(),
            TriangleBuffer::default(),
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
                        normal: Vector3::new(0.0, 1.0, 0.0),
                        tangent: Vector4::new(1.0, 0.0, 0.0, -1.0),
                    })
                    .unwrap();
            }
        }
        drop(vertex_buffer_mut);

        let mut geometry_buffer_mut = surface_data.geometry_buffer.modify();

        let half_size = Vector2::new(mesh_size.x - 1, mesh_size.y - 1) / 2;

        let mut quadrants = [ElementRange::Full; 4];
        for ((x_range, y_range), quadrant) in [
            (0..half_size.x, 0..half_size.y),
            (half_size.x..mesh_size.x - 1, 0..half_size.y),
            (half_size.x..mesh_size.x - 1, half_size.y..mesh_size.y - 1),
            (0..half_size.x, half_size.y..mesh_size.y - 1),
        ]
        .into_iter()
        .zip(&mut quadrants)
        {
            let offset = geometry_buffer_mut.len();

            for iy in y_range {
                let iy_next = iy + 1;
                for x in x_range.clone() {
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

        // There is no need to calculate normals and tangents when they will always be the same for
        // all vertices.
        //surface_data.calculate_normals().unwrap();
        //surface_data.calculate_tangents().unwrap();

        Self {
            data: SurfaceResource::new_ok(ResourceKind::Embedded, surface_data),
            quadrants,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::scene::mesh::buffer::VertexAttributeUsage;

    use super::*;

    #[test]
    fn geometry_3x3() {
        let g = TerrainGeometry::new(Vector2::new(3, 3));
        for (i, q) in g.quadrants.iter().copied().enumerate() {
            let ElementRange::Specific { count, .. } = q else {
                panic!("Quadrant is full.")
            };
            assert_eq!(count, 2, "Quadrant: {}", i);
        }
    }
    #[test]
    fn geometry_4x4() {
        let g = TerrainGeometry::new(Vector2::new(4, 4));
        let counts = [2, 4, 8, 4];
        for (i, (q, c)) in g.quadrants.iter().copied().zip(counts).enumerate() {
            let ElementRange::Specific { count, .. } = q else {
                panic!("Quadrant is full.")
            };
            assert_eq!(count, c, "Quadrant: {}", i);
        }
    }
    #[test]
    fn geometry_5x5() {
        let g = TerrainGeometry::new(Vector2::new(5, 5));
        for (i, q) in g.quadrants.iter().copied().enumerate() {
            let ElementRange::Specific { count, .. } = q else {
                panic!("Quadrant is full.")
            };
            assert_eq!(count, 8, "Quadrant: {}", i);
        }
    }
    #[test]
    fn normals() {
        let g = TerrainGeometry::new(Vector2::new(3, 3));
        let vertices = &g.data.data_ref().vertex_buffer;
        let attr_view = vertices
            .attribute_view::<Vector3<f32>>(VertexAttributeUsage::Normal)
            .unwrap();
        for i in 0..vertices.vertex_count() as usize {
            assert_eq!(
                attr_view.get(i).unwrap().clone(),
                Vector3::<f32>::new(0.0, 1.0, 0.0)
            );
        }
    }
    #[test]
    fn tangents() {
        let g = TerrainGeometry::new(Vector2::new(3, 3));
        let vertices = &g.data.data_ref().vertex_buffer;
        let attr_view = vertices
            .attribute_view::<Vector4<f32>>(VertexAttributeUsage::Tangent)
            .unwrap();
        for i in 0..vertices.vertex_count() as usize {
            assert_eq!(
                attr_view.get(i).unwrap().clone(),
                Vector4::<f32>::new(1.0, 0.0, 0.0, -1.0)
            );
        }
    }
}
