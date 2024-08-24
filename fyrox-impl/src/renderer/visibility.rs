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

//! Volumetric visibility cache based on occlusion query.

#![allow(missing_docs)] // TODO

use crate::{
    core::{
        algebra::{Matrix4, Vector2, Vector3},
        array_as_u8_slice,
        arrayvec::ArrayVec,
        math::{OptionRect, Rect},
        pool::Handle,
        ImmutableString,
    },
    graph::BaseSceneGraph,
    renderer::{
        framework::{
            error::FrameworkError,
            framebuffer::{
                Attachment, AttachmentKind, BlendParameters, DrawParameters, FrameBuffer,
            },
            geometry_buffer::{GeometryBuffer, GeometryBufferKind},
            gpu_program::{GpuProgram, UniformLocation},
            gpu_texture::{
                Coordinate, GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter,
                PixelKind, WrapMode,
            },
            query::{Query, QueryKind, QueryResult},
            state::{BlendEquation, BlendFactor, BlendFunc, BlendMode, ColorMask, PipelineState},
        },
        storage::MatrixStorage,
    },
    scene::{graph::Graph, mesh::surface::SurfaceData, node::Node},
};
use bytemuck::{Pod, Zeroable};
use fxhash::FxHashMap;
use fyrox_core::color::Color;
use std::cmp::Ordering;
use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
struct PendingQuery {
    query: Query,
    observer_position: Vector3<f32>,
    node: Handle<Node>,
}

#[derive(Debug)]
enum Visibility {
    Undefined,
    Invisible,
    Visible,
}

type NodeVisibilityMap = FxHashMap<Handle<Node>, Visibility>;

/// Volumetric visibility cache based on occlusion query.
#[derive(Debug)]
pub struct ObserverVisibilityCache {
    cells: FxHashMap<Vector3<i32>, NodeVisibilityMap>,
    pending_queries: Vec<PendingQuery>,
    granularity: Vector3<u32>,
    distance_discard_threshold: f32,
}

fn world_to_grid(world_position: Vector3<f32>, granularity: Vector3<u32>) -> Vector3<i32> {
    Vector3::new(
        (world_position.x * (granularity.x as f32)).round() as i32,
        (world_position.y * (granularity.y as f32)).round() as i32,
        (world_position.z * (granularity.z as f32)).round() as i32,
    )
}

fn grid_to_world(grid_position: Vector3<i32>, granularity: Vector3<u32>) -> Vector3<f32> {
    Vector3::new(
        grid_position.x as f32 / (granularity.x as f32),
        grid_position.y as f32 / (granularity.y as f32),
        grid_position.z as f32 / (granularity.z as f32),
    )
}

impl ObserverVisibilityCache {
    /// Creates new visibility cache with the given granularity and distance discard threshold.
    /// Granularity in means how much the cache should subdivide the world. For example 2 means that
    /// 1 meter cell will be split into 8 blocks by 0.5 meters. Distance discard threshold means how
    /// far an observer can without discarding visibility info about distant objects.
    pub fn new(granularity: Vector3<u32>, distance_discard_threshold: f32) -> Self {
        Self {
            cells: Default::default(),
            pending_queries: Default::default(),
            granularity,
            distance_discard_threshold,
        }
    }

    /// Transforms the given world-space position into internal grid-space position.
    pub fn world_to_grid(&self, world_position: Vector3<f32>) -> Vector3<i32> {
        world_to_grid(world_position, self.granularity)
    }

    /// Transforms the given grid-space position into the world-space position.
    pub fn grid_to_world(&self, grid_position: Vector3<i32>) -> Vector3<f32> {
        grid_to_world(grid_position, self.granularity)
    }

    fn visibility_info(
        &self,
        observer_position: Vector3<f32>,
        node: Handle<Node>,
    ) -> Option<&Visibility> {
        let grid_position = self.world_to_grid(observer_position);

        self.cells
            .get(&grid_position)
            .and_then(|cell| cell.get(&node))
    }

    /// Checks whether the given object needs an occlusion query for the given observer position.
    pub fn needs_occlusion_query(
        &self,
        observer_position: Vector3<f32>,
        node: Handle<Node>,
    ) -> bool {
        let Some(visibility) = self.visibility_info(observer_position, node) else {
            // There's no data about the visibility, so the occlusion query is needed.
            return true;
        };

        match visibility {
            Visibility::Undefined => {
                // There's already an occlusion query on GPU.
                false
            }
            Visibility::Invisible => {
                // The object could be invisible from one angle at the observer position, but visible
                // from another. Since we're using only position of the observer, we cannot be 100%
                // sure, that the object is invisible even if a previous query told us so.
                true
            }
            Visibility::Visible => {
                // Some pixels of the object is visible from the given observer position, so we don't
                // need a new occlusion query.
                false
            }
        }
    }

    /// Checks whether the object at the given handle is visible from the given observer position.
    /// This method returns `true` for non-completed occlusion queries, because occlusion query is
    /// async operation.
    pub fn is_visible(&self, observer_position: Vector3<f32>, node: Handle<Node>) -> bool {
        let Some(visibility_info) = self.visibility_info(observer_position, node) else {
            return false;
        };

        match *visibility_info {
            Visibility::Visible
            // Undefined visibility is treated like the object is visible, this is needed because
            // GPU queries are async, and we must still render the object to prevent popping light.
            | Visibility::Undefined => true,
            Visibility::Invisible => false,
        }
    }

    /// Begins a new visibility query (using occlusion query) for the object at the given handle from
    /// the given observer position.
    pub fn begin_query(
        &mut self,
        pipeline_state: &PipelineState,
        observer_position: Vector3<f32>,
        node: Handle<Node>,
    ) -> Result<(), FrameworkError> {
        let query = Query::new(pipeline_state)?;
        query.begin(QueryKind::AnySamplesPassed);
        self.pending_queries.push(PendingQuery {
            query,
            observer_position,
            node,
        });

        let grid_position = self.world_to_grid(observer_position);
        self.cells
            .entry(grid_position)
            .or_default()
            .entry(node)
            .or_insert(Visibility::Undefined);

        Ok(())
    }

    /// Ends the last visibility query.
    pub fn end_query(&mut self) {
        let last_pending_query = self
            .pending_queries
            .last()
            .expect("begin_query/end_query calls mismatch!");
        last_pending_query.query.end();
    }

    /// This method removes info about too distant objects and processes the pending visibility queries.
    pub fn update(&mut self, observer_position: Vector3<f32>) {
        self.pending_queries.retain_mut(|pending_query| {
            if let Some(QueryResult::AnySamplesPassed(query_result)) =
                pending_query.query.try_get_result()
            {
                let grid_position =
                    world_to_grid(pending_query.observer_position, self.granularity);

                let visibility = self
                    .cells
                    .get_mut(&grid_position)
                    .expect("grid cell must exist!")
                    .get_mut(&pending_query.node)
                    .expect("object visibility must be predefined!");

                match visibility {
                    Visibility::Undefined => match query_result {
                        true => {
                            *visibility = Visibility::Visible;
                        }
                        false => {
                            *visibility = Visibility::Invisible;
                        }
                    },
                    Visibility::Invisible => {
                        if query_result {
                            // Override "invisibility" - if any fragment of an object is visible, then
                            // it will remain visible forever. This is ok for non-moving objects only.
                            *visibility = Visibility::Visible;
                        }
                    }
                    Visibility::Visible => {
                        // Ignore the query result and keep the visibility.
                    }
                }

                false
            } else {
                true
            }
        });

        // Remove visibility info from the cache for distant cells.
        self.cells.retain(|grid_position, _| {
            let world_position = grid_to_world(*grid_position, self.granularity);

            world_position.metric_distance(&observer_position) < self.distance_discard_threshold
        });
    }
}

#[derive(Debug)]
struct ObserverData {
    position: Vector3<f32>,
    visibility_cache: ObserverVisibilityCache,
}

/// Visibility cache that caches visibility info for multiple cameras.
#[derive(Default, Debug)]
pub struct VisibilityCache {
    observers: FxHashMap<Handle<Node>, ObserverData>,
}

impl VisibilityCache {
    /// Gets or adds new storage for the given observer.
    pub fn get_or_register(
        &mut self,
        graph: &Graph,
        observer: Handle<Node>,
    ) -> &mut ObserverVisibilityCache {
        &mut self
            .observers
            .entry(observer)
            .or_insert_with(|| ObserverData {
                position: graph[observer].global_position(),
                visibility_cache: ObserverVisibilityCache::new(Vector3::repeat(2), 100.0),
            })
            .visibility_cache
    }

    /// Updates the cache by removing unused data.
    pub fn update(&mut self, graph: &Graph) {
        self.observers.retain(|observer, data| {
            let Some(observer_ref) = graph.try_get(*observer) else {
                return false;
            };

            data.position = observer_ref.global_position();

            data.visibility_cache.update(data.position);

            true
        });
    }
}

struct Shader {
    program: GpuProgram,
    view_projection: UniformLocation,
    tile_size: UniformLocation,
    tile_buffer: UniformLocation,
    instance_matrices: UniformLocation,
    frame_buffer_height: UniformLocation,
}

impl Shader {
    fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/visibility_fs.glsl");
        let vertex_source = include_str!("shaders/visibility_vs.glsl");
        let program =
            GpuProgram::from_source(state, "VisibilityShader", vertex_source, fragment_source)?;
        Ok(Self {
            view_projection: program
                .uniform_location(state, &ImmutableString::new("viewProjection"))?,
            tile_size: program.uniform_location(state, &ImmutableString::new("tileSize"))?,
            frame_buffer_height: program
                .uniform_location(state, &ImmutableString::new("frameBufferHeight"))?,
            tile_buffer: program.uniform_location(state, &ImmutableString::new("tileBuffer"))?,
            instance_matrices: program
                .uniform_location(state, &ImmutableString::new("instanceMatrices"))?,
            program,
        })
    }
}

pub struct OcclusionTester {
    framebuffer: FrameBuffer,
    visibility_mask: Rc<RefCell<GpuTexture>>,
    instance_matrices_buffer: MatrixStorage,
    tile_buffer: Rc<RefCell<GpuTexture>>,
    frame_size: Vector2<usize>,
    cube: GeometryBuffer,
    shader: Shader,
    tile_size: usize,
    w_tiles: usize,
    h_tiles: usize,
}

#[derive(Default, Pod, Zeroable, Copy, Clone)]
#[repr(C)]
struct GpuTile {
    objects: [u32; 32],
}

#[derive(Clone)]
struct Object {
    index: u32,
    depth: f32,
}

#[derive(Default, Clone)]
struct Tile {
    objects: Vec<Object>,
}

const MAX_BITS: usize = u32::BITS as usize;

impl OcclusionTester {
    pub fn new(
        state: &PipelineState,
        width: usize,
        height: usize,
        tile_size: usize,
    ) -> Result<Self, FrameworkError> {
        let mut depth_stencil_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::D24S8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        depth_stencil_texture
            .bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let visibility_mask = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::R32F,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;

        let w_tiles = width / tile_size + 1;
        let h_tiles = height / tile_size + 1;
        let tile_buffer = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle {
                width: w_tiles * MAX_BITS,
                height: h_tiles,
            },
            PixelKind::R32UI,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;

        let depth_stencil = Rc::new(RefCell::new(depth_stencil_texture));
        let visibility_mask = Rc::new(RefCell::new(visibility_mask));
        let tile_buffer = Rc::new(RefCell::new(tile_buffer));

        Ok(Self {
            framebuffer: FrameBuffer::new(
                state,
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
            cube: GeometryBuffer::from_surface_data(
                &SurfaceData::make_cube(Matrix4::identity()),
                GeometryBufferKind::StaticDraw,
                state,
            )?,
            instance_matrices_buffer: MatrixStorage::new(state)?,
            shader: Shader::new(state)?,
            tile_size,
            w_tiles,
            tile_buffer,
            h_tiles,
        })
    }

    fn read_visibility_mask(&self, state: &PipelineState) -> Vec<f32> {
        // TODO: Replace with double buffering to prevent GPU stalls.
        self.visibility_mask
            .borrow_mut()
            .bind_mut(state, 0)
            .read_pixels_of_type(state)
    }

    fn visibility_map(
        &self,
        state: &PipelineState,
        objects: &[Handle<Node>],
        tiles: &[Tile],
    ) -> FxHashMap<Handle<Node>, bool> {
        // TODO: This must be done using compute shader, but it is not available on WebGL2 so this
        // code should still be used on WASM targets.
        let visibility_buffer = self.read_visibility_mask(state);
        let mut visibility_map = FxHashMap::default();
        for (index, pixel) in visibility_buffer.into_iter().enumerate() {
            let x = index % self.frame_size.x;
            let y = index / self.frame_size.y;
            let tx = x / self.tile_size;
            let ty = y / self.tile_size;
            let bits = pixel as u32;
            if let Some(tile) = tiles.get(ty * self.w_tiles + tx) {
                'bit_loop: for bit in 0..u32::BITS {
                    if let Some(object) = tile.objects.get(bit as usize) {
                        let is_visible = (bits & bit) != 0;
                        let visibility = visibility_map
                            .entry(objects[object.index as usize])
                            .or_insert(is_visible);
                        if is_visible {
                            *visibility = true;
                        }
                    } else {
                        break 'bit_loop;
                    }
                }
            }
        }
        visibility_map
    }

    fn screen_space_to_tile_space(
        &self,
        pos: Vector2<f32>,
        viewport: &Rect<i32>,
    ) -> Vector2<usize> {
        let x = (pos.x.clamp(
            viewport.position.x as f32,
            (viewport.position.x + viewport.size.x) as f32,
        ) / (self.tile_size as f32)) as usize;
        let y = (pos.y.clamp(
            viewport.position.y as f32,
            (viewport.position.y + viewport.size.y) as f32,
        ) / (self.tile_size as f32)) as usize;
        Vector2::new(x, y)
    }

    fn prepare_tiles(
        &self,
        state: &PipelineState,
        graph: &Graph,
        observer_position: Vector3<f32>,
        view_projection: &Matrix4<f32>,
        viewport: Rect<i32>,
        objects: &[Handle<Node>],
    ) -> Result<Vec<Tile>, FrameworkError> {
        let mut tiles = vec![Tile::default(); self.w_tiles * self.h_tiles];

        for (object_index, object) in objects.iter().enumerate() {
            let aabb = graph[*object].world_bounding_box();
            let mut rect_builder = OptionRect::default();
            for corner in aabb.corners() {
                let ndc_space = view_projection.transform_point(&corner.into());
                let screen_space_corner = Vector2::new(
                    (ndc_space.x + 1.0) * (viewport.size.x as f32) / 2.0
                        + viewport.position.x as f32,
                    (ndc_space.y + 1.0) * (viewport.size.y as f32) / 2.0
                        + viewport.position.y as f32,
                );
                rect_builder.push(screen_space_corner);
            }
            let rect = rect_builder.unwrap();

            let average_depth = observer_position.metric_distance(&aabb.center());
            let min = self.screen_space_to_tile_space(rect.left_top_corner(), &viewport);
            let max = self.screen_space_to_tile_space(rect.right_bottom_corner(), &viewport);
            let size = (max - min).sup(&Vector2::repeat(1));
            for y in min.y..(min.y + size.y) {
                for x in min.x..(min.x + size.x) {
                    let tile = &mut tiles[y * self.w_tiles + x];
                    tile.objects.push(Object {
                        index: object_index as u32,
                        depth: average_depth,
                    });
                }
            }
        }

        let mut gpu_tiles = Vec::with_capacity(tiles.len());
        for tile in tiles.iter_mut() {
            tile.objects
                .sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap_or(Ordering::Less));

            let mut gpu_tile = GpuTile {
                objects: tile
                    .objects
                    .iter()
                    .map(|obj| obj.index)
                    .chain([u32::MAX; MAX_BITS])
                    .take(MAX_BITS)
                    .collect::<ArrayVec<u32, MAX_BITS>>()
                    .into_inner()
                    .unwrap(),
            };

            gpu_tile.objects.sort();

            gpu_tiles.push(gpu_tile);
        }

        self.tile_buffer.borrow_mut().bind_mut(state, 0).set_data(
            GpuTextureKind::Rectangle {
                width: self.w_tiles * MAX_BITS,
                height: self.h_tiles,
            },
            PixelKind::R32UI,
            1,
            Some(array_as_u8_slice(&gpu_tiles)),
        )?;

        Ok(tiles)
    }

    pub fn check(
        &mut self,
        observer_position: Vector3<f32>,
        view_projection: &Matrix4<f32>,
        state: &PipelineState,
        prev_framebuffer: &FrameBuffer,
        graph: &Graph,
        objects: &[Handle<Node>],
    ) -> Result<FxHashMap<Handle<Node>, bool>, FrameworkError> {
        let w = self.frame_size.x as i32;
        let h = self.frame_size.y as i32;
        let viewport = Rect::new(0, 0, w, h);

        self.framebuffer.clear(
            state,
            viewport,
            Some(Color::TRANSPARENT),
            Some(1.0),
            Some(0),
        );

        state.blit_framebuffer(
            prev_framebuffer.id(),
            self.framebuffer.id(),
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

        let tiles = self.prepare_tiles(
            state,
            graph,
            observer_position,
            view_projection,
            viewport,
            objects,
        )?;

        self.instance_matrices_buffer.upload(
            state,
            objects.iter().map(|h| {
                let mut aabb = graph[*h].world_bounding_box();
                aabb.inflate(Vector3::repeat(0.01));
                let s = aabb.max - aabb.min;
                Matrix4::new_translation(&aabb.center()) * Matrix4::new_nonuniform_scaling(&s)
            }),
            0,
        )?;

        let shader = &self.shader;
        self.framebuffer.draw_instances(
            objects.len(),
            &self.cube,
            state,
            viewport,
            &self.shader.program,
            &DrawParameters {
                cull_face: None,
                color_write: ColorMask::all(true),
                depth_write: false,
                stencil_test: None,
                depth_test: true,
                blend: Some(BlendParameters {
                    func: BlendFunc::new(BlendFactor::One, BlendFactor::One),
                    equation: BlendEquation {
                        rgb: BlendMode::Add,
                        alpha: BlendMode::Add,
                    },
                }),
                stencil_op: Default::default(),
            },
            |mut program_binding| {
                program_binding
                    .set_texture(&shader.tile_buffer, &self.tile_buffer)
                    .set_texture(
                        &shader.instance_matrices,
                        self.instance_matrices_buffer.texture(),
                    )
                    .set_i32(&shader.tile_size, self.tile_size as i32)
                    .set_f32(&shader.frame_buffer_height, self.frame_size.y as f32)
                    .set_matrix4(&shader.view_projection, view_projection);
            },
        );

        Ok(self.visibility_map(state, objects, &tiles))
    }
}

#[cfg(test)]
mod tests {}
