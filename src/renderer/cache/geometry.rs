use crate::core::sparse::SparseBuffer;
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

#[derive(Default)]
pub struct GeometryCache {
    buffer: SparseBuffer<CacheEntry<GeometryBuffer>>,
}

impl GeometryCache {
    pub fn get<'a>(
        &'a mut self,
        state: &mut PipelineState,
        data: &SurfaceData,
    ) -> &'a mut GeometryBuffer {
        scope_profile!();

        let data_hash = data.content_hash();

        if self.buffer.is_index_valid(&data.cache_entry) {
            let entry = self.buffer.get_mut(&data.cache_entry).unwrap();

            if data_hash != entry.value_hash {
                // Content has changed, upload new content.
                entry.set_buffer_data(state, 0, data.vertex_buffer.raw_data());
                entry
                    .bind(state)
                    .set_triangles(data.geometry_buffer.triangles_ref());

                entry.value_hash = data_hash;
            }

            entry.time_to_live = DEFAULT_RESOURCE_LIFETIME;
            entry
        } else {
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

            let index = self.buffer.spawn(CacheEntry {
                value: geometry_buffer,
                time_to_live: DEFAULT_RESOURCE_LIFETIME,
                value_hash: data_hash,
            });

            data.cache_entry.set(index.get());

            self.buffer.get_mut(&index).unwrap()
        }
    }

    pub fn update(&mut self, dt: f32) {
        scope_profile!();

        for entry in self.buffer.iter_mut() {
            entry.time_to_live -= dt;
        }

        for i in 0..self.buffer.len() {
            if let Some(entry) = self.buffer.get_raw(i) {
                if entry.time_to_live <= 0.0 {
                    self.buffer.free_raw(i);
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}
