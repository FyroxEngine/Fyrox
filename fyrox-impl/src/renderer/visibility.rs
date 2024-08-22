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

use crate::{
    core::{
        algebra::Matrix4, algebra::Vector2, algebra::Vector3, math::wrap_angle, math::Rect,
        pool::Handle,
    },
    graph::BaseSceneGraph,
    renderer::{
        flat_shader::FlatShader,
        framework::{
            error::FrameworkError,
            framebuffer::{DrawParameters, FrameBuffer},
            geometry_buffer::{DrawCallStatistics, ElementRange, GeometryBuffer},
            gpu_texture::GpuTexture,
            query::{Query, QueryKind, QueryResult},
            state::{ColorMask, PipelineState},
        },
    },
    scene::{graph::Graph, node::Node},
};
use fxhash::FxHashMap;
use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
struct PendingQuery {
    query: Query,
    observer_info: ObserverInfo,
    node: Handle<Node>,
}

#[derive(Debug, Copy, Clone)]
pub enum Visibility {
    Undefined,
    Invisible,
    Visible,
}

impl Visibility {
    pub fn needs_rendering(self) -> bool {
        match self {
            Visibility::Visible
            // Undefined visibility is treated like the object is visible, this is needed because
            // GPU queries are async, and we must still render the object to prevent popping light.
            | Visibility::Undefined => true,
            Visibility::Invisible => false,
        }
    }
}

type NodeVisibilityMap = FxHashMap<Handle<Node>, Visibility>;

fn dir_to_angles(dir: Vector3<f32>, fov: f32) -> Vector2<i32> {
    let granularity = fov.to_degrees() / 2.0;
    let dir = dir.try_normalize(f32::EPSILON).unwrap_or_default();
    let theta =
        wrap_angle((dir.x * dir.x + dir.z * dir.z).sqrt().atan2(dir.y)).to_degrees() / granularity;
    let phi = wrap_angle(dir.x.atan2(dir.z)).to_degrees() / granularity;
    Vector2::new(theta.round() as i32, phi.round() as i32)
}

#[derive(Debug, Default)]
struct Cell {
    angles: FxHashMap<Vector2<i32>, NodeVisibilityMap>,
}

/// Volumetric visibility cache based on occlusion query.
#[derive(Debug)]
pub struct ObserverVisibilityCache {
    cells: FxHashMap<Vector3<i32>, Cell>,
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

#[derive(Debug, Clone)]
pub struct ObserverInfo {
    position: Vector3<f32>,
    grid_position: Vector3<i32>,
    rotation: Vector2<i32>,
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

    pub fn observer_info(
        &self,
        position: Vector3<f32>,
        look_dir: Vector3<f32>,
        fov: f32,
    ) -> ObserverInfo {
        let grid_position = self.world_to_grid(position);
        let rotation = dir_to_angles(look_dir, fov);
        ObserverInfo {
            position,
            grid_position,
            rotation,
        }
    }

    /// Tries to find visibility info about the object for the given observer position.
    pub fn visibility_info(
        &self,
        observer_info: &ObserverInfo,
        node: Handle<Node>,
    ) -> Option<&Visibility> {
        self.cells
            .get(&observer_info.grid_position)
            .and_then(|cell| cell.angles.get(&observer_info.rotation))
            .and_then(|cell| cell.get(&node))
    }

    /// Checks whether the given object needs an occlusion query for the given observer position.
    pub fn needs_occlusion_query(&self, observer_info: &ObserverInfo, node: Handle<Node>) -> bool {
        self.visibility_info(observer_info, node).is_none()
    }

    /// Checks whether the object at the given handle is visible from the given observer position.
    /// This method returns `true` for non-completed occlusion queries, because occlusion query is
    /// async operation.
    pub fn is_visible(&self, observer_info: &ObserverInfo, node: Handle<Node>) -> bool {
        let Some(visibility_info) = self.visibility_info(observer_info, node) else {
            return false;
        };

        visibility_info.needs_rendering()
    }

    /// Tries to begin a new visibility query (using occlusion query) for the object at the given handle from
    /// the given observer position. The query will not be started if the observer is inside the object's
    /// bounding box, because this is an edge case where the object is always considered visible.
    pub fn begin_conditional_query(
        &mut self,
        pipeline_state: &PipelineState,
        observer_info: &ObserverInfo,
        graph: &Graph,
        node: Handle<Node>,
    ) -> Result<bool, FrameworkError> {
        let Some(node_ref) = graph.try_get(node) else {
            return Ok(false);
        };

        let cell = self
            .cells
            .entry(observer_info.grid_position)
            .or_default()
            .angles
            .entry(observer_info.rotation)
            .or_default();

        if node_ref
            .world_bounding_box()
            .is_contains_point(observer_info.position)
        {
            cell.entry(node).or_insert(Visibility::Visible);

            Ok(false)
        } else {
            let query = Query::new(pipeline_state)?;
            query.begin(QueryKind::AnySamplesPassed);
            self.pending_queries.push(PendingQuery {
                query,
                observer_info: observer_info.clone(),
                node,
            });

            cell.entry(node).or_insert(Visibility::Undefined);

            Ok(true)
        }
    }

    /// Begins a new visibility query (using occlusion query) for the object at the given handle from
    /// the given observer position.
    pub fn begin_non_conditional_query(
        &mut self,
        pipeline_state: &PipelineState,
        observer_info: &ObserverInfo,
        node: Handle<Node>,
    ) -> Result<(), FrameworkError> {
        let query = Query::new(pipeline_state)?;
        query.begin(QueryKind::AnySamplesPassed);
        self.pending_queries.push(PendingQuery {
            query,
            observer_info: observer_info.clone(),
            node,
        });

        self.cells
            .entry(observer_info.grid_position)
            .or_default()
            .angles
            .entry(observer_info.rotation)
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
                let visibility = self
                    .cells
                    .get_mut(&pending_query.observer_info.grid_position)
                    .expect("grid cell must exist!")
                    .angles
                    .get_mut(&pending_query.observer_info.rotation)
                    .expect("angle must exist!")
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

    pub fn run_query(
        &mut self,
        state: &PipelineState,
        graph: &Graph,
        frame_buffer: &mut FrameBuffer,
        viewport: Rect<i32>,
        unit_cube: &GeometryBuffer,
        flat_shader: &FlatShader,
        white_dummy: &Rc<RefCell<GpuTexture>>,
        observer_info: &ObserverInfo,
        view_projection_matrix: Matrix4<f32>,
        node: Handle<Node>,
    ) -> Result<DrawCallStatistics, FrameworkError> {
        let Some(node_ref) = graph.try_get(node) else {
            return Ok(Default::default());
        };
        if self.needs_occlusion_query(observer_info, node)
            && self.begin_conditional_query(state, observer_info, graph, node)?
        {
            let mut aabb = node_ref.world_bounding_box();
            aabb.inflate(Vector3::repeat(0.05));
            let s = aabb.max - aabb.min;
            let matrix =
                Matrix4::new_translation(&aabb.center()) * Matrix4::new_nonuniform_scaling(&s);
            let mvp_matrix = view_projection_matrix * matrix;
            let stats = frame_buffer.draw(
                unit_cube,
                state,
                viewport,
                &flat_shader.program,
                &DrawParameters {
                    cull_face: None,
                    color_write: ColorMask::all(false),
                    depth_write: false,
                    stencil_test: None,
                    depth_test: true,
                    blend: None,
                    stencil_op: Default::default(),
                },
                ElementRange::Full,
                |mut program_binding| {
                    program_binding
                        .set_matrix4(&flat_shader.wvp_matrix, &mvp_matrix)
                        .set_texture(&flat_shader.diffuse_texture, white_dummy);
                },
            )?;
            self.end_query();
            Ok(stats)
        } else {
            Ok(Default::default())
        }
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
