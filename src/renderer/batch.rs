//! The module responsible for batch generation for rendering optimizations.

use crate::{
    core::{algebra::Matrix4, math::frustum::Frustum, sstorage::ImmutableString},
    material::SharedMaterial,
    scene::{
        graph::Graph,
        mesh::{surface::SurfaceSharedData, RenderPath},
    },
};
use fxhash::{FxBuildHasher, FxHashMap, FxHasher};
use std::{
    fmt::{Debug, Formatter},
    hash::Hasher,
};

pub struct RenderContext<'a> {
    pub view_matrix: &'a Matrix4<f32>,
    pub projection_matrix: &'a Matrix4<f32>,
    pub frustum: &'a Frustum,
    pub storage: &'a mut RenderDataBatchStorage,
    pub graph: &'a Graph,
    pub render_pass_name: &'a ImmutableString,
}

/// A set of data of a surface for rendering.  
pub struct SurfaceInstanceData {
    /// A world matrix.
    pub world_transform: Matrix4<f32>,
    /// A set of bone matrices.
    pub bone_matrices: Vec<Matrix4<f32>>,
    /// A depth-hack value.
    pub depth_offset: f32,
    /// A set of weights for each blend shape in the surface.
    pub blend_shapes_weights: Vec<f32>,
}

/// A set of surface instances that share the same vertex/index data and a material.
pub struct RenderDataBatch {
    /// A pointer to shared surface data.
    pub data: SurfaceSharedData,
    /// A set of instances.
    pub instances: Vec<SurfaceInstanceData>,
    /// A material that is shared across all instances.
    pub material: SharedMaterial,
    /// Whether the batch is using GPU skinning or not.
    pub is_skinned: bool,
    /// A render path of the batch.
    pub render_path: RenderPath,
    /// A decal layer index of the batch.
    pub decal_layer_index: u8,
    sort_index: u64,
}

impl Debug for RenderDataBatch {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Batch {}: {} instances",
            self.data.key(),
            self.instances.len()
        )
    }
}

/// Batch storage handles batch generation for a scene before rendering. It is used to optimize
/// rendering by reducing amount of state changes of OpenGL context.
#[derive(Default)]
pub struct RenderDataBatchStorage {
    batch_map: FxHashMap<u64, usize>,
    pub batches: Vec<RenderDataBatch>,
}

impl RenderDataBatchStorage {
    pub fn from_graph(
        graph: &Graph,
        view_matrix: Matrix4<f32>,
        projection_matrix: Matrix4<f32>,
        render_pass_name: ImmutableString,
    ) -> Self {
        // Aim for the worst-case scenario when every node has unique render data.
        let capacity = graph.node_count() as usize;
        let mut storage = Self {
            batch_map: FxHashMap::with_capacity_and_hasher(capacity, FxBuildHasher::default()),
            batches: Vec::with_capacity(capacity),
        };

        let frustum = Frustum::from(projection_matrix * view_matrix).unwrap_or_default();

        for node in graph.linear_iter() {
            node.collect_render_data(&mut RenderContext {
                view_matrix: &view_matrix,
                projection_matrix: &projection_matrix,
                frustum: &frustum,
                storage: &mut storage,
                graph,
                render_pass_name: &render_pass_name,
            });
        }

        storage.sort();

        storage
    }

    pub fn push(
        &mut self,
        data: &SurfaceSharedData,
        material: &SharedMaterial,
        render_path: RenderPath,
        decal_layer_index: u8,
        sort_index: u64,
        instance_data: SurfaceInstanceData,
    ) {
        let is_skinned = !instance_data.bone_matrices.is_empty();

        let mut hasher = FxHasher::default();
        hasher.write_u64(material.key());
        hasher.write_u64(data.key());
        hasher.write_u8(if is_skinned { 1 } else { 0 });
        hasher.write_u8(decal_layer_index);
        hasher.write_u32(render_path as u32);
        let key = hasher.finish();

        let batch = if let Some(&batch_index) = self.batch_map.get(&key) {
            self.batches.get_mut(batch_index).unwrap()
        } else {
            self.batch_map.insert(key, self.batches.len());
            self.batches.push(RenderDataBatch {
                data: data.clone(),
                sort_index,
                instances: Default::default(),
                material: material.clone(),
                is_skinned,
                render_path,
                decal_layer_index,
            });
            self.batches.last_mut().unwrap()
        };

        batch.instances.push(instance_data)
    }

    pub fn sort(&mut self) {
        self.batches.sort_unstable_by_key(|b| b.sort_index);
    }
}
