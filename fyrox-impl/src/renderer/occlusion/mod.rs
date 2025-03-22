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

//! Full algorithm explained - <https://fyrox.rs/blog/post/tile-based-occlusion-culling/>

mod grid;
mod optimizer;

use crate::renderer::FallbackResources;
use crate::{
    core::{
        algebra::{Matrix4, Vector2, Vector3},
        array_as_u8_slice,
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, Rect, Vector3Ext},
        pool::Handle,
        ImmutableString,
    },
    graph::BaseSceneGraph,
    renderer::{
        cache::shader::{binding, property, PropertyGroup, RenderMaterial, RenderPassContainer},
        cache::uniform::UniformBufferCache,
        debug_renderer::{self, DebugRenderer},
        framework::{
            buffer::BufferUsage,
            error::FrameworkError,
            framebuffer::GpuFrameBuffer,
            framebuffer::{Attachment, AttachmentKind},
            geometry_buffer::GpuGeometryBuffer,
            gpu_texture::GpuTexture,
            gpu_texture::{GpuTextureKind, PixelKind},
            server::GraphicsServer,
            stats::RenderPassStatistics,
            GeometryBufferExt,
        },
        occlusion::{
            grid::{GridCache, Visibility},
            optimizer::VisibilityBufferOptimizer,
        },
        storage::MatrixStorage,
    },
    scene::{graph::Graph, mesh::surface::SurfaceData, node::Node},
};
use bytemuck::{Pod, Zeroable};

pub struct OcclusionTester {
    framebuffer: GpuFrameBuffer,
    visibility_mask: GpuTexture,
    tile_buffer: GpuTexture,
    frame_size: Vector2<usize>,
    shader: RenderPassContainer,
    tile_size: usize,
    w_tiles: usize,
    h_tiles: usize,
    cube: GpuGeometryBuffer,
    visibility_buffer_optimizer: VisibilityBufferOptimizer,
    matrix_storage: MatrixStorage,
    objects_to_test: Vec<Handle<Node>>,
    view_projection: Matrix4<f32>,
    observer_position: Vector3<f32>,
    pub grid_cache: GridCache,
    tiles: TileBuffer,
}

const MAX_BITS: usize = u32::BITS as usize;

#[derive(Default, Pod, Zeroable, Copy, Clone, Debug)]
#[repr(C)]
struct Tile {
    count: u32,
    objects: [u32; MAX_BITS],
}

impl Tile {
    fn add(&mut self, index: u32) {
        let count = self.count as usize;
        if count < self.objects.len() {
            self.objects[count] = index;
            self.count += 1;
        }
    }
}

#[derive(Default, Debug)]
struct TileBuffer {
    tiles: Vec<Tile>,
}

impl TileBuffer {
    fn new(width: usize, height: usize) -> Self {
        Self {
            tiles: vec![Default::default(); width * height],
        }
    }

    fn clear(&mut self) {
        for tile in self.tiles.iter_mut() {
            tile.count = 0;
        }
    }
}

fn inflated_world_aabb(graph: &Graph, object: Handle<Node>) -> Option<AxisAlignedBoundingBox> {
    let mut aabb = graph
        .try_get(object)
        .map(|node_ref| node_ref.world_bounding_box())?;
    aabb.inflate(Vector3::repeat(0.01));
    Some(aabb)
}

impl OcclusionTester {
    pub fn new(
        server: &dyn GraphicsServer,
        width: usize,
        height: usize,
        tile_size: usize,
    ) -> Result<Self, FrameworkError> {
        let depth_stencil = server.create_2d_render_target(PixelKind::D24S8, width, height)?;
        let visibility_mask = server.create_2d_render_target(PixelKind::RGBA8, width, height)?;
        let w_tiles = width / tile_size + 1;
        let h_tiles = height / tile_size + 1;
        let tile_buffer =
            server.create_2d_render_target(PixelKind::R32UI, w_tiles * (MAX_BITS + 1), h_tiles)?;

        Ok(Self {
            framebuffer: server.create_frame_buffer(
                Some(Attachment {
                    kind: AttachmentKind::DepthStencil,
                    texture: depth_stencil,
                }),
                vec![Attachment {
                    kind: AttachmentKind::Color,
                    texture: visibility_mask.clone(),
                }],
            )?,
            visibility_mask,
            frame_size: Vector2::new(width, height),
            shader: RenderPassContainer::from_str(
                server,
                include_str!("../shaders/visibility.shader"),
            )?,
            tile_size,
            w_tiles,
            tile_buffer,
            h_tiles,
            cube: GpuGeometryBuffer::from_surface_data(
                &SurfaceData::make_cube(Matrix4::identity()),
                BufferUsage::StaticDraw,
                server,
            )?,
            visibility_buffer_optimizer: VisibilityBufferOptimizer::new(server, w_tiles, h_tiles)?,
            matrix_storage: MatrixStorage::new(server)?,
            objects_to_test: Default::default(),
            view_projection: Default::default(),
            observer_position: Default::default(),
            grid_cache: GridCache::new(Vector3::repeat(1)),
            tiles: TileBuffer::new(w_tiles, h_tiles),
        })
    }

    pub fn try_query_visibility_results(&mut self, graph: &Graph) {
        let Some(visibility_buffer) = self.visibility_buffer_optimizer.read_visibility_mask()
        else {
            return;
        };

        let mut objects_visibility = vec![false; self.objects_to_test.len()];
        for y in 0..self.h_tiles {
            let img_y = self.h_tiles.saturating_sub(1) - y;
            let tile_offset = y * self.w_tiles;
            let img_offset = img_y * self.w_tiles;
            for x in 0..self.w_tiles {
                let tile = &self.tiles.tiles[tile_offset + x];
                let bits = visibility_buffer[img_offset + x];
                let count = tile.count as usize;
                for bit in 0..count {
                    let object_index = tile.objects[bit];
                    let visibility = &mut objects_visibility[object_index as usize];
                    let mask = 1 << bit;
                    let is_visible = (bits & mask) == mask;
                    if is_visible {
                        *visibility = true;
                    }
                }
            }
        }

        let cell = self.grid_cache.get_or_insert_cell(self.observer_position);
        for (obj, vis) in self.objects_to_test.iter().zip(objects_visibility.iter()) {
            cell.mark(*obj, (*vis).into());
        }

        for (object, visibility) in cell.iter_mut() {
            let Some(aabb) = inflated_world_aabb(graph, *object) else {
                continue;
            };
            if aabb.is_contains_point(self.observer_position) {
                *visibility = Visibility::Visible;
            }
        }
    }

    fn screen_space_to_tile_space(&self, pos: Vector2<f32>) -> Vector2<usize> {
        let x = (pos.x / (self.tile_size as f32)) as usize;
        let y = (pos.y / (self.tile_size as f32)) as usize;
        Vector2::new(x, y)
    }

    fn prepare_tiles(
        &mut self,
        graph: &Graph,
        viewport: &Rect<i32>,
        debug_renderer: Option<&mut DebugRenderer>,
    ) -> Result<(), FrameworkError> {
        self.tiles.clear();

        let mut lines = Vec::new();
        for (object_index, object) in self.objects_to_test.iter().enumerate() {
            let object_index = object_index as u32;
            let Some(node_ref) = graph.try_get(*object) else {
                continue;
            };

            let aabb = node_ref.world_bounding_box();
            let rect = aabb.project(&self.view_projection, viewport);

            if debug_renderer.is_some() {
                debug_renderer::draw_rect(&rect, &mut lines, Color::WHITE);
            }

            let min = self.screen_space_to_tile_space(rect.left_top_corner());
            let max = self.screen_space_to_tile_space(rect.right_bottom_corner());
            for y in min.y..=max.y {
                let offset = y * self.w_tiles;
                for x in min.x..=max.x {
                    self.tiles.tiles[offset + x].add(object_index);
                }
            }
        }

        if let Some(debug_renderer) = debug_renderer {
            for (tile_index, tile) in self.tiles.tiles.iter().enumerate() {
                let x = (tile_index % self.w_tiles) * self.tile_size;
                let y = (tile_index / self.w_tiles) * self.tile_size;
                let bounds = Rect::new(
                    x as f32,
                    y as f32,
                    self.tile_size as f32,
                    self.tile_size as f32,
                );

                debug_renderer::draw_rect(
                    &bounds,
                    &mut lines,
                    Color::COLORS[tile.objects.len() + 2],
                );
            }

            debug_renderer.set_lines(&lines);
        }

        self.tile_buffer.set_data(
            GpuTextureKind::Rectangle {
                width: self.w_tiles * (MAX_BITS + 1),
                height: self.h_tiles,
            },
            PixelKind::R32UI,
            1,
            Some(array_as_u8_slice(self.tiles.tiles.as_slice())),
        )?;

        Ok(())
    }

    fn upload_data<'a>(
        &mut self,
        graph: &Graph,
        objects_to_test: impl Iterator<Item = &'a Handle<Node>>,
        prev_framebuffer: &GpuFrameBuffer,
        observer_position: Vector3<f32>,
        view_projection: Matrix4<f32>,
    ) {
        self.view_projection = view_projection;
        self.observer_position = observer_position;
        let w = self.frame_size.x as i32;
        let h = self.frame_size.y as i32;
        prev_framebuffer.blit_to(
            &self.framebuffer,
            0,
            0,
            w,
            h,
            0,
            0,
            w,
            h,
            false,
            true,
            false,
        );

        self.objects_to_test.clear();
        if let Some(cell) = self.grid_cache.cell(self.observer_position) {
            for object in objects_to_test {
                if cell.needs_occlusion_query(*object) {
                    self.objects_to_test.push(*object);
                }
            }
        }

        self.objects_to_test.sort_unstable_by_key(|a| {
            (graph[*a].global_position().sqr_distance(&observer_position) * 1000.0) as u64
        });
    }

    pub fn try_run_visibility_test<'a>(
        &mut self,
        graph: &Graph,
        debug_renderer: Option<&mut DebugRenderer>,
        unit_quad: &GpuGeometryBuffer,
        objects_to_test: impl Iterator<Item = &'a Handle<Node>>,
        prev_framebuffer: &GpuFrameBuffer,
        observer_position: Vector3<f32>,
        view_projection: Matrix4<f32>,
        uniform_buffer_cache: &mut UniformBufferCache,
        fallback_resources: &FallbackResources,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();

        if self.visibility_buffer_optimizer.is_reading_from_gpu() {
            return Ok(stats);
        }

        self.upload_data(
            graph,
            objects_to_test,
            prev_framebuffer,
            observer_position,
            view_projection,
        );

        let w = self.frame_size.x as i32;
        let h = self.frame_size.y as i32;
        let viewport = Rect::new(0, 0, w, h);

        self.framebuffer
            .clear(viewport, Some(Color::TRANSPARENT), None, None);

        self.prepare_tiles(graph, &viewport, debug_renderer)?;

        self.matrix_storage
            .upload(self.objects_to_test.iter().filter_map(|h| {
                let aabb = inflated_world_aabb(graph, *h)?;
                let s = aabb.max - aabb.min;
                Some(Matrix4::new_translation(&aabb.center()) * Matrix4::new_nonuniform_scaling(&s))
            }))?;

        let tile_size = self.tile_size as i32;
        let frame_buffer_height = self.frame_size.y as f32;
        let properties = PropertyGroup::from([
            property("viewProjection", &self.view_projection),
            property("tileSize", &tile_size),
            property("frameBufferHeight", &frame_buffer_height),
        ]);
        let material = RenderMaterial::from([
            binding(
                "matrices",
                (
                    self.matrix_storage.texture(),
                    &fallback_resources.nearest_clamp_sampler,
                ),
            ),
            binding(
                "tileBuffer",
                (&self.tile_buffer, &fallback_resources.nearest_clamp_sampler),
            ),
            binding("properties", &properties),
        ]);

        stats += self.shader.run_pass(
            self.objects_to_test.len(),
            &ImmutableString::new("Primary"),
            &self.framebuffer,
            &self.cube,
            viewport,
            &material,
            uniform_buffer_cache,
            Default::default(),
            None,
        )?;

        self.visibility_buffer_optimizer.optimize(
            &self.visibility_mask,
            unit_quad,
            self.tile_size as i32,
            uniform_buffer_cache,
            fallback_resources,
        )?;

        Ok(stats)
    }
}
