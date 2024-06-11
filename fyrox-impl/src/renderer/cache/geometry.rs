use crate::{
    renderer::{
        cache::{TemporaryCache, TimeToLive},
        framework::{
            error::FrameworkError,
            geometry_buffer::{GeometryBuffer, GeometryBufferKind},
            state::PipelineState,
        },
    },
    scene::mesh::surface::{SurfaceData, SurfaceResource},
};
use fyrox_core::log::Log;

struct SurfaceRenderData {
    buffer: GeometryBuffer,
    vertex_modifications_count: u64,
    triangles_modifications_count: u64,
    layout_hash: u64,
}

#[derive(Default)]
pub struct GeometryCache {
    buffer: TemporaryCache<SurfaceRenderData>,
}

fn create_geometry_buffer(
    data: &SurfaceData,
    state: &PipelineState,
) -> Result<SurfaceRenderData, FrameworkError> {
    let geometry_buffer =
        GeometryBuffer::from_surface_data(data, GeometryBufferKind::StaticDraw, state)?;

    Ok(SurfaceRenderData {
        buffer: geometry_buffer,
        vertex_modifications_count: data.vertex_buffer.modifications_count(),
        triangles_modifications_count: data.geometry_buffer.modifications_count(),
        layout_hash: data.vertex_buffer.layout_hash(),
    })
}

impl GeometryCache {
    pub fn get<'a>(
        &'a mut self,
        state: &PipelineState,
        data: &SurfaceResource,
        time_to_live: TimeToLive,
    ) -> Option<&'a mut GeometryBuffer> {
        let data = data.data_ref();

        match self
            .buffer
            .get_entry_mut_or_insert_with(&data.cache_index, time_to_live, || {
                create_geometry_buffer(&data, state)
            }) {
            Ok(entry) => {
                // We also must check if buffer's layout changed, and if so - recreate the entire
                // buffer.
                if entry.layout_hash == data.vertex_buffer.layout_hash() {
                    if data.vertex_buffer.modifications_count() != entry.vertex_modifications_count
                    {
                        // Vertices has changed, upload the new content.
                        entry
                            .buffer
                            .set_buffer_data(state, 0, data.vertex_buffer.raw_data());

                        entry.vertex_modifications_count = data.vertex_buffer.modifications_count();
                    }

                    if data.geometry_buffer.modifications_count()
                        != entry.triangles_modifications_count
                    {
                        // Triangles has changed, upload the new content.
                        entry
                            .buffer
                            .bind(state)
                            .set_triangles(data.geometry_buffer.triangles_ref());

                        entry.triangles_modifications_count =
                            data.geometry_buffer.modifications_count();
                    }
                }
                Some(&mut entry.buffer)
            }
            Err(err) => {
                Log::err(err.to_string());
                None
            }
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.buffer.update(dt);
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}
