// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::renderer::framework::GeometryBufferExt;
use crate::{
    renderer::{
        cache::{TemporaryCache, TimeToLive},
        framework::{error::FrameworkError, server::GraphicsServer},
    },
    scene::mesh::surface::{SurfaceData, SurfaceResource},
};
use fyrox_graphics::buffer::BufferUsage;
use fyrox_graphics::geometry_buffer::GpuGeometryBuffer;

struct SurfaceRenderData {
    buffer: GpuGeometryBuffer,
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
    server: &dyn GraphicsServer,
) -> Result<SurfaceRenderData, FrameworkError> {
    let geometry_buffer =
        GpuGeometryBuffer::from_surface_data(data, BufferUsage::StaticDraw, server)?;

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
        server: &dyn GraphicsServer,
        data: &SurfaceResource,
        time_to_live: TimeToLive,
    ) -> Result<&'a GpuGeometryBuffer, FrameworkError> {
        let data = data.data_ref();

        match self
            .buffer
            .get_entry_mut_or_insert_with(&data.cache_index, time_to_live, || {
                create_geometry_buffer(&data, server)
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
                            .set_buffer_data(0, data.vertex_buffer.raw_data());

                        entry.vertex_modifications_count = data.vertex_buffer.modifications_count();
                    }

                    if data.geometry_buffer.modifications_count()
                        != entry.triangles_modifications_count
                    {
                        // Triangles has changed, upload the new content.
                        entry
                            .buffer
                            .set_triangles(data.geometry_buffer.triangles_ref());

                        entry.triangles_modifications_count =
                            data.geometry_buffer.modifications_count();
                    }
                }
                Ok(&entry.buffer)
            }
            Err(err) => Err(err),
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.buffer.update(dt);
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn alive_count(&self) -> usize {
        self.buffer.alive_count()
    }
}
