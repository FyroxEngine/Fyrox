use crate::{
    core::{
        algebra::{Matrix4, Vector2},
        color::Color,
        math::TriangleDefinition,
        scope_profile,
    },
    engine::resource_manager::TimedEntry,
    renderer::framework::{
        geometry_buffer::{
            AttributeDefinition, AttributeKind, BufferBuilder, ElementKind, GeometryBuffer,
            GeometryBufferBuilder, GeometryBufferKind,
        },
        state::PipelineState,
    },
};
use fxhash::FxHashMap;
use rg3d_core::algebra::Vector3;

#[repr(C)]
pub struct Vertex {
    position: Vector3<f32>,
    tex_coord: Vector2<f32>,
}

pub struct Mesh {
    vertices: Vec<Vertex>,
    triangles: Vec<TriangleDefinition>,
}

impl Mesh {
    pub fn new_unit_quad() -> Self {
        let vertices = vec![
            Vertex {
                position: Vector3::new(-0.5, 0.5, 0.0),
                tex_coord: Vector2::new(0.0, 0.0),
            },
            Vertex {
                position: Vector3::new(0.5, 0.5, 0.0),
                tex_coord: Vector2::new(1.0, 0.0),
            },
            Vertex {
                position: Vector3::new(0.5, -0.5, 0.0),
                tex_coord: Vector2::new(1.0, 1.0),
            },
            Vertex {
                position: Vector3::new(-0.5, -0.5, 0.0),
                tex_coord: Vector2::new(0.0, 1.0),
            },
        ];

        let triangles = vec![TriangleDefinition([0, 1, 2]), TriangleDefinition([0, 2, 3])];

        Self {
            vertices,
            triangles,
        }
    }
}

#[derive(Default)]
pub(in crate) struct GeometryCache {
    map: FxHashMap<usize, TimedEntry<GeometryBuffer>>,
}

#[derive(Clone)]
#[repr(C)]
pub(in crate) struct InstanceData {
    pub color: Color,
    pub world_matrix: Matrix4<f32>,
}

impl GeometryCache {
    pub fn get(&mut self, state: &mut PipelineState, data: &Mesh) -> &mut GeometryBuffer {
        scope_profile!();

        let key = (data as *const _) as usize;

        let geometry_buffer = self.map.entry(key).or_insert_with(|| {
            let geometry_buffer = GeometryBufferBuilder::new(ElementKind::Triangle)
                .with_buffer_builder(
                    BufferBuilder::new(
                        GeometryBufferKind::StaticDraw,
                        Some(data.vertices.as_slice()),
                    )
                    .with_attribute(AttributeDefinition {
                        location: 0,
                        divisor: 0,
                        kind: AttributeKind::Float3,
                        normalized: false,
                    })
                    .with_attribute(AttributeDefinition {
                        location: 1,
                        divisor: 0,
                        kind: AttributeKind::Float2,
                        normalized: false,
                    }),
                )
                // Buffer for instance data.
                .with_buffer_builder(
                    BufferBuilder::new::<InstanceData>(GeometryBufferKind::DynamicDraw, None)
                        // Color
                        .with_attribute(AttributeDefinition {
                            location: 2,
                            kind: AttributeKind::UnsignedByte4,
                            normalized: true,
                            divisor: 1,
                        })
                        // World Matrix
                        .with_attribute(AttributeDefinition {
                            location: 3,
                            kind: AttributeKind::Float4,
                            normalized: false,
                            divisor: 1,
                        })
                        .with_attribute(AttributeDefinition {
                            location: 4,
                            kind: AttributeKind::Float4,
                            normalized: false,
                            divisor: 1,
                        })
                        .with_attribute(AttributeDefinition {
                            location: 5,
                            kind: AttributeKind::Float4,
                            normalized: false,
                            divisor: 1,
                        })
                        .with_attribute(AttributeDefinition {
                            location: 6,
                            kind: AttributeKind::Float4,
                            normalized: false,
                            divisor: 1,
                        }),
                )
                .build(state)
                .unwrap();

            geometry_buffer.bind(state).set_triangles(&data.triangles);

            TimedEntry {
                value: geometry_buffer,
                time_to_live: 20.0,
            }
        });

        geometry_buffer.time_to_live = 20.0;
        geometry_buffer
    }

    pub fn update(&mut self, dt: f32) {
        scope_profile!();

        for entry in self.map.values_mut() {
            entry.time_to_live -= dt;
        }
        self.map.retain(|_, v| v.time_to_live > 0.0);
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }
}
