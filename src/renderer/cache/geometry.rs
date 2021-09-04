use crate::{
    core::scope_profile,
    engine::resource_manager::DEFAULT_RESOURCE_LIFETIME,
    renderer::{
        cache::CacheEntry,
        framework::{
            geometry_buffer::{
                BufferBuilder, ElementKind, GeometryBuffer, GeometryBufferBuilder,
                GeometryBufferKind,
            },
            state::PipelineState,
        },
    },
    scene::mesh::surface::SurfaceData,
};
use std::collections::HashMap;

#[derive(Default)]
pub struct GeometryCache {
    map: HashMap<usize, CacheEntry<GeometryBuffer>>,
}

impl GeometryCache {
    pub fn get(&mut self, state: &mut PipelineState, data: &SurfaceData) -> &mut GeometryBuffer {
        scope_profile!();

        let key = (data as *const _) as usize;
        let data_hash = data.content_hash();

        let geometry_buffer = self.map.entry(key).or_insert_with(|| {
            let geometry_buffer = GeometryBufferBuilder::new(ElementKind::Triangle)
                .with_buffer_builder(BufferBuilder::from_vertex_buffer(
                    &data.vertex_buffer,
                    GeometryBufferKind::StaticDraw,
                ))
                .build(state)
                .unwrap();

            geometry_buffer
                .bind(state)
                .set_triangles(data.geometry_buffer.triangles_ref());

            CacheEntry {
                value: geometry_buffer,
                time_to_live: DEFAULT_RESOURCE_LIFETIME,
                value_hash: data_hash,
            }
        });

        if data_hash != geometry_buffer.value_hash {
            // Content has changed, upload new content.
            geometry_buffer.set_buffer_data(state, 0, data.vertex_buffer.raw_data());
            geometry_buffer
                .bind(state)
                .set_triangles(data.geometry_buffer.triangles_ref());

            geometry_buffer.value_hash = data_hash;
        }

        geometry_buffer.time_to_live = DEFAULT_RESOURCE_LIFETIME;
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
