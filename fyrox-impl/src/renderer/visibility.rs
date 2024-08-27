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
        algebra::{Matrix4, Vector2, Vector3, Vector4},
        array_as_u8_slice,
        arrayvec::ArrayVec,
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, OptionRect, Rect},
        pool::Handle,
        ImmutableString,
    },
    graph::BaseSceneGraph,
    renderer::{
        debug_renderer::{self, DebugRenderer},
        framework::{
            error::FrameworkError,
            framebuffer::{
                Attachment, AttachmentKind, BlendParameters, CullFace, DrawParameters, FrameBuffer,
            },
            geometry_buffer::{ElementRange, GeometryBuffer, GeometryBufferKind},
            gpu_program::{GpuProgram, UniformLocation},
            gpu_texture::{
                Coordinate, GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter,
                PixelKind, WrapMode,
            },
            query::{Query, QueryKind, QueryResult},
            state::{
                BlendEquation, BlendFactor, BlendFunc, BlendMode, ColorMask, CompareFunc,
                PipelineState,
            },
        },
        make_viewport_matrix,
        storage::MatrixStorage,
    },
    scene::{graph::Graph, mesh::surface::SurfaceData, node::Node},
};
use bytemuck::{Pod, Zeroable};
use fxhash::FxHashMap;
use std::{cell::RefCell, cmp::Ordering, rc::Rc};

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
    frame_buffer_height: UniformLocation,
    matrices: UniformLocation,
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
            matrices: program.uniform_location(state, &ImmutableString::new("matrices"))?,
            program,
        })
    }
}

struct VisibilityOptimizerShader {
    program: GpuProgram,
    view_projection: UniformLocation,
    tile_size: UniformLocation,
    visibility_buffer: UniformLocation,
}

impl VisibilityOptimizerShader {
    fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/visibility_optimizer_fs.glsl");
        let vertex_source = include_str!("shaders/visibility_optimizer_vs.glsl");
        let program = GpuProgram::from_source(
            state,
            "VisibilityOptimizerShader",
            vertex_source,
            fragment_source,
        )?;
        Ok(Self {
            view_projection: program
                .uniform_location(state, &ImmutableString::new("viewProjection"))?,
            tile_size: program.uniform_location(state, &ImmutableString::new("tileSize"))?,
            visibility_buffer: program
                .uniform_location(state, &ImmutableString::new("visibilityBuffer"))?,
            program,
        })
    }
}

struct VisibilityBufferOptimizer {
    framebuffer: FrameBuffer,
    optimized_visibility_buffer: Rc<RefCell<GpuTexture>>,
    shader: VisibilityOptimizerShader,
    w_tiles: usize,
    h_tiles: usize,
}

impl VisibilityBufferOptimizer {
    fn new(state: &PipelineState, w_tiles: usize, h_tiles: usize) -> Result<Self, FrameworkError> {
        let optimized_visibility_buffer = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle {
                width: w_tiles,
                height: h_tiles,
            },
            PixelKind::R32UI,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;

        let optimized_visibility_buffer = Rc::new(RefCell::new(optimized_visibility_buffer));

        Ok(Self {
            framebuffer: FrameBuffer::new(
                state,
                None,
                vec![Attachment {
                    kind: AttachmentKind::Color,
                    texture: optimized_visibility_buffer.clone(),
                }],
            )?,
            optimized_visibility_buffer,
            shader: VisibilityOptimizerShader::new(state)?,
            w_tiles,
            h_tiles,
        })
    }

    fn read_visibility_mask(&self, state: &PipelineState) -> Vec<u32> {
        self.optimized_visibility_buffer
            .borrow_mut()
            .bind_mut(state, 0)
            .get_image(0, state)
    }

    fn optimize(
        &mut self,
        state: &PipelineState,
        visibility_buffer: &Rc<RefCell<GpuTexture>>,
        unit_quad: &GeometryBuffer,
        tile_size: i32,
    ) -> Result<Vec<u32>, FrameworkError> {
        let viewport = Rect::new(0, 0, self.w_tiles as i32, self.h_tiles as i32);

        self.framebuffer
            .clear(state, viewport, Some(Color::TRANSPARENT), None, None);

        let matrix = make_viewport_matrix(viewport);

        self.framebuffer.draw(
            unit_quad,
            state,
            viewport,
            &self.shader.program,
            &DrawParameters {
                cull_face: None,
                color_write: ColorMask::all(true),
                depth_write: false,
                stencil_test: None,
                depth_test: false,
                blend: None,
                stencil_op: Default::default(),
            },
            ElementRange::Full,
            |mut program_binding| {
                program_binding
                    .set_matrix4(&self.shader.view_projection, &matrix)
                    .set_texture(&self.shader.visibility_buffer, visibility_buffer)
                    .set_i32(&self.shader.tile_size, tile_size);
            },
        )?;

        Ok(self.read_visibility_mask(state))
    }
}

pub struct OcclusionTester {
    framebuffer: FrameBuffer,
    visibility_mask: Rc<RefCell<GpuTexture>>,
    tile_buffer: Rc<RefCell<GpuTexture>>,
    frame_size: Vector2<usize>,
    shader: Shader,
    tile_size: usize,
    w_tiles: usize,
    h_tiles: usize,
    cube: GeometryBuffer,
    visibility_buffer_optimizer: VisibilityBufferOptimizer,
    matrix_storage: MatrixStorage,
    objects_to_test: Vec<Handle<Node>>,
    view_projection: Matrix4<f32>,
    observer_position: Vector3<f32>,
    visibility_map: FxHashMap<Handle<Node>, bool>,
}

#[derive(Default, Pod, Zeroable, Copy, Clone, Debug)]
#[repr(C)]
struct GpuTile {
    objects: [u32; 32],
}

#[derive(Clone, Debug)]
struct Object {
    index: u32,
    depth: f32,
}

#[derive(Clone, Debug)]
struct Tile {
    objects: Vec<Object>,
    bounds: Rect<f32>,
}

const MAX_BITS: usize = u32::BITS as usize;

fn screen_space_rect(
    aabb: AxisAlignedBoundingBox,
    view_projection: &Matrix4<f32>,
    viewport: &Rect<i32>,
) -> Rect<f32> {
    let mut rect_builder = OptionRect::default();
    for corner in aabb.corners() {
        let clip_space = view_projection * Vector4::new(corner.x, corner.y, corner.z, 1.0);
        let ndc_space = clip_space.xyz() / clip_space.w.abs();
        let mut normalized_screen_space =
            Vector2::new((ndc_space.x + 1.0) / 2.0, (1.0 - ndc_space.y) / 2.0);
        normalized_screen_space.x = normalized_screen_space.x.clamp(0.0, 1.0);
        normalized_screen_space.y = normalized_screen_space.y.clamp(0.0, 1.0);
        let screen_space_corner = Vector2::new(
            (normalized_screen_space.x * viewport.size.x as f32) + viewport.position.x as f32,
            (normalized_screen_space.y * viewport.size.y as f32) + viewport.position.y as f32,
        );

        rect_builder.push(screen_space_corner);
    }
    rect_builder.unwrap()
}

fn inflated_world_aabb(graph: &Graph, object: Handle<Node>) -> AxisAlignedBoundingBox {
    let mut aabb = graph[object].world_bounding_box();
    aabb.inflate(Vector3::repeat(0.05));
    aabb
}

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
            shader: Shader::new(state)?,
            tile_size,
            w_tiles,
            tile_buffer,
            h_tiles,
            cube: GeometryBuffer::from_surface_data(
                &SurfaceData::make_cube(Matrix4::identity()),
                GeometryBufferKind::StaticDraw,
                state,
            )?,
            visibility_buffer_optimizer: VisibilityBufferOptimizer::new(state, w_tiles, h_tiles)?,
            matrix_storage: MatrixStorage::new(state)?,
            objects_to_test: Default::default(),
            view_projection: Default::default(),
            observer_position: Default::default(),
            visibility_map: Default::default(),
        })
    }

    #[inline(never)]
    fn prepare_visibility_map(
        &mut self,
        state: &PipelineState,
        tiles: &[Tile],
        graph: &Graph,
        unit_quad: &GeometryBuffer,
    ) -> Result<(), FrameworkError> {
        let visibility_buffer = self.visibility_buffer_optimizer.optimize(
            state,
            &self.visibility_mask,
            unit_quad,
            self.tile_size as i32,
        )?;

        let mut objects_visibility = vec![false; self.objects_to_test.len()];
        for y in 0..self.h_tiles {
            let img_y = self.h_tiles.saturating_sub(1) - y;
            for x in 0..self.w_tiles {
                let tile = &tiles[y * self.w_tiles + x];
                let bits = visibility_buffer[img_y * self.w_tiles + x];
                for bit in 0..u32::BITS {
                    if let Some(object) = tile.objects.get(bit as usize) {
                        let visibility = &mut objects_visibility[object.index as usize];
                        let mask = 1 << bit;
                        let is_visible = (bits & mask) == mask;
                        if is_visible {
                            *visibility = true;
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        self.visibility_map.clear();
        self.visibility_map.extend(
            self.objects_to_test
                .iter()
                .cloned()
                .zip(objects_visibility.iter().cloned()),
        );

        for (object, visibility) in self.visibility_map.iter_mut() {
            if inflated_world_aabb(graph, *object).is_contains_point(self.observer_position) {
                *visibility = true;
            }
        }

        Ok(())
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
        viewport: &Rect<i32>,
        debug_renderer: Option<&mut DebugRenderer>,
    ) -> Result<Vec<Tile>, FrameworkError> {
        let mut tiles = Vec::with_capacity(self.w_tiles * self.h_tiles);
        for y in 0..self.h_tiles {
            for x in 0..self.w_tiles {
                tiles.push(Tile {
                    objects: vec![],
                    bounds: Rect::new(
                        (x * self.tile_size) as f32,
                        (y * self.tile_size) as f32,
                        self.tile_size as f32,
                        self.tile_size as f32,
                    ),
                })
            }
        }
        let mut lines = Vec::new();
        for (object_index, object) in self.objects_to_test.iter().enumerate() {
            let node_ref = &graph[*object];
            let aabb = node_ref.world_bounding_box();
            let rect = screen_space_rect(aabb, &self.view_projection, viewport);

            if debug_renderer.is_some() {
                debug_renderer::draw_rect(&rect, &mut lines, Color::WHITE);
            }

            let average_depth = self.observer_position.metric_distance(&aabb.center());
            let min = self.screen_space_to_tile_space(rect.left_top_corner(), viewport);
            let max = self.screen_space_to_tile_space(rect.right_bottom_corner(), viewport);
            let size = max - min;
            for y in min.y..=(min.y + size.y) {
                for x in min.x..=(min.x + size.x) {
                    let tile = &mut tiles[y * self.w_tiles + x];
                    tile.objects.push(Object {
                        index: object_index as u32,
                        depth: average_depth,
                    });
                }
            }
        }

        if let Some(debug_renderer) = debug_renderer {
            for tile in tiles.iter() {
                debug_renderer::draw_rect(
                    &tile.bounds,
                    &mut lines,
                    Color::COLORS[tile.objects.len() + 2],
                );
            }

            debug_renderer.set_lines(state, &lines);
        }

        let mut gpu_tiles = Vec::with_capacity(tiles.len());
        for tile in tiles.iter_mut() {
            tile.objects
                .sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap_or(Ordering::Less));

            gpu_tiles.push(GpuTile {
                objects: tile
                    .objects
                    .iter()
                    .map(|obj| obj.index)
                    .chain([u32::MAX; MAX_BITS])
                    .take(MAX_BITS)
                    .collect::<ArrayVec<u32, MAX_BITS>>()
                    .into_inner()
                    .unwrap(),
            });
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

    pub fn upload_data<'a>(
        &mut self,
        state: &PipelineState,
        objects_to_test: impl Iterator<Item = &'a Handle<Node>>,
        prev_framebuffer: &FrameBuffer,
        observer_position: Vector3<f32>,
        view_projection: Matrix4<f32>,
    ) {
        let w = self.frame_size.x as i32;
        let h = self.frame_size.y as i32;
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

        self.objects_to_test.clear();
        self.objects_to_test.extend(objects_to_test);

        self.view_projection = view_projection;
        self.observer_position = observer_position;
    }

    pub fn run_visibility_test(
        &mut self,
        state: &PipelineState,
        graph: &Graph,
        debug_renderer: Option<&mut DebugRenderer>,
        unit_quad: &GeometryBuffer,
    ) -> Result<(), FrameworkError> {
        if self.objects_to_test.is_empty() {
            return Ok(());
        }

        let w = self.frame_size.x as i32;
        let h = self.frame_size.y as i32;
        let viewport = Rect::new(0, 0, w, h);

        self.framebuffer
            .clear(state, viewport, Some(Color::TRANSPARENT), None, None);

        let tiles = self.prepare_tiles(state, graph, &viewport, debug_renderer)?;

        self.matrix_storage.upload(
            state,
            self.objects_to_test.iter().map(|h| {
                let aabb = inflated_world_aabb(graph, *h);
                let s = aabb.max - aabb.min;
                Matrix4::new_translation(&aabb.center()) * Matrix4::new_nonuniform_scaling(&s)
            }),
            0,
        )?;

        state.set_depth_func(CompareFunc::LessOrEqual);
        let shader = &self.shader;
        self.framebuffer.draw_instances(
            self.objects_to_test.len(),
            &self.cube,
            state,
            viewport,
            &self.shader.program,
            &DrawParameters {
                cull_face: Some(CullFace::Back),
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
                    .set_texture(&shader.matrices, self.matrix_storage.texture())
                    .set_i32(&shader.tile_size, self.tile_size as i32)
                    .set_f32(&shader.frame_buffer_height, self.frame_size.y as f32)
                    .set_matrix4(&shader.view_projection, &self.view_projection);
            },
        );

        self.prepare_visibility_map(state, &tiles, graph, unit_quad)
    }

    pub fn is_visible(&self, object: Handle<Node>) -> bool {
        self.visibility_map.get(&object).cloned().unwrap_or(true)
    }
}
