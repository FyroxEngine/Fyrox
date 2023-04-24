use crate::{
    asset::container::entry::DEFAULT_RESOURCE_LIFETIME,
    core::{scope_profile, sparse::SparseBuffer},
    renderer::framework::{
        geometry_buffer::{GeometryBuffer, GeometryBufferKind},
        state::PipelineState,
    },
    scene::mesh::surface::{SurfaceData, SurfaceSharedData},
};
use fyrox_core::sparse::AtomicIndex;

struct CacheEntry {
    buffer: GeometryBuffer,
    data_hash: u64,
    layout_hash: u64,
    time_to_live: f32,
}

#[derive(Default)]
pub struct GeometryCache {
    buffer: SparseBuffer<CacheEntry>,
}

fn create_geometry_buffer(
    data: &SurfaceData,
    state: &mut PipelineState,
    buffer: &mut SparseBuffer<CacheEntry>,
) -> AtomicIndex {
    let geometry_buffer =
        GeometryBuffer::from_surface_data(data, GeometryBufferKind::StaticDraw, state);

    let index = buffer.spawn(CacheEntry {
        buffer: geometry_buffer,
        time_to_live: DEFAULT_RESOURCE_LIFETIME,
        data_hash: data.content_hash(),
        layout_hash: data.vertex_buffer.layout_hash(),
    });

    data.cache_entry.set(index.get());

    index
}

impl GeometryCache {
    pub fn get<'a>(
        &'a mut self,
        state: &mut PipelineState,
        data: &SurfaceSharedData,
    ) -> &'a mut GeometryBuffer {
        scope_profile!();

        let data = data.lock();

        if let Some(entry) = self.buffer.get_mut(&data.cache_entry) {
            // We also must check if buffer's layout changed, and if so - recreate the entire
            // buffer.
            if entry.layout_hash == data.vertex_buffer.layout_hash() {
                let data_hash = data.content_hash();
                if data_hash != entry.data_hash {
                    // Content has changed, upload new content.
                    entry
                        .buffer
                        .set_buffer_data(state, 0, data.vertex_buffer.raw_data());
                    entry
                        .buffer
                        .bind(state)
                        .set_triangles(data.geometry_buffer.triangles_ref());

                    entry.data_hash = data_hash;
                }

                entry.time_to_live = DEFAULT_RESOURCE_LIFETIME;

                &mut self.buffer.get_mut(&data.cache_entry).unwrap().buffer
            } else {
                let index = create_geometry_buffer(&data, state, &mut self.buffer);
                &mut self.buffer.get_mut(&index).unwrap().buffer
            }
        } else {
            let index = create_geometry_buffer(&data, state, &mut self.buffer);
            &mut self.buffer.get_mut(&index).unwrap().buffer
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
