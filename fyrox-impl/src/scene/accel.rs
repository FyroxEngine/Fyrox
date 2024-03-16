#![allow(missing_docs)] // TODO

use crate::{
    core::{
        algebra::Vector3,
        math::aabb::AxisAlignedBoundingBox,
        pool::{Handle, Pool},
    },
    graph::SceneGraph,
    scene::{graph::Graph, node::Node},
};

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Entry {
    node: Handle<Node>,
    world_aabb: AxisAlignedBoundingBox,
}

#[derive(Clone, Debug)]
pub enum OctreeNode {
    Leaf {
        entries: Vec<Entry>,
        bounds: AxisAlignedBoundingBox,
    },
    Branch {
        bounds: AxisAlignedBoundingBox,
        leaves: [Handle<OctreeNode>; 8],
    },
}

#[derive(Default, Clone, Debug)]
pub struct Octree {
    nodes: Pool<OctreeNode>,
    root: Handle<OctreeNode>,
}

impl Octree {
    pub fn new(graph: &Graph, split_threshold: usize) -> Self {
        // Calculate bounds.
        let mut bounds = AxisAlignedBoundingBox::default();

        let mut entries = Vec::new();
        for (handle, node) in graph.pair_iter() {
            let aabb = node.world_bounding_box();
            entries.push(Entry {
                node: handle,
                world_aabb: aabb,
            });
            bounds.add_box(aabb);
        }

        // Inflate initial bounds by very low value to fix floating-point calculation
        // issues when splitting and checking intersection later on.
        let inflation = 2.0 * f32::EPSILON;
        bounds.inflate(Vector3::new(inflation, inflation, inflation));

        let mut nodes = Pool::new();

        let root = build_recursive(&mut nodes, entries, bounds, split_threshold);

        Self { nodes, root }
    }

    pub fn sphere_query(&self, position: Vector3<f32>, radius: f32, buffer: &mut Vec<Entry>) {
        buffer.clear();
        self.sphere_recursive_query(self.root, position, radius, buffer);
    }

    fn sphere_recursive_query(
        &self,
        node: Handle<OctreeNode>,
        position: Vector3<f32>,
        radius: f32,
        buffer: &mut Vec<Entry>,
    ) {
        match self.nodes.borrow(node) {
            OctreeNode::Leaf { entries, bounds } => {
                if bounds.is_intersects_sphere(position, radius) {
                    buffer.extend_from_slice(entries)
                }
            }
            OctreeNode::Branch { bounds, leaves } => {
                if bounds.is_intersects_sphere(position, radius) {
                    for leaf in leaves {
                        self.sphere_recursive_query(*leaf, position, radius, buffer)
                    }
                }
            }
        }
    }

    pub fn aabb_query(&self, aabb: &AxisAlignedBoundingBox, buffer: &mut Vec<Entry>) {
        buffer.clear();
        self.aabb_recursive_query(self.root, aabb, buffer);
    }

    fn aabb_recursive_query(
        &self,
        node: Handle<OctreeNode>,
        aabb: &AxisAlignedBoundingBox,
        buffer: &mut Vec<Entry>,
    ) {
        match self.nodes.borrow(node) {
            OctreeNode::Leaf { entries, bounds } => {
                if bounds.is_intersects_aabb(aabb) {
                    buffer.extend_from_slice(entries)
                }
            }
            OctreeNode::Branch { bounds, leaves } => {
                if bounds.is_intersects_aabb(aabb) {
                    for leaf in leaves {
                        self.aabb_recursive_query(*leaf, aabb, buffer)
                    }
                }
            }
        }
    }

    pub fn node(&self, handle: Handle<OctreeNode>) -> &OctreeNode {
        &self.nodes[handle]
    }

    pub fn nodes(&self) -> &Pool<OctreeNode> {
        &self.nodes
    }

    pub fn point_query(&self, point: Vector3<f32>, buffer: &mut Vec<Entry>) {
        buffer.clear();
        self.point_recursive_query(self.root, point, buffer);
    }

    fn point_recursive_query(
        &self,
        node: Handle<OctreeNode>,
        point: Vector3<f32>,
        buffer: &mut Vec<Entry>,
    ) {
        match self.nodes.borrow(node) {
            OctreeNode::Leaf { entries, bounds } => {
                if bounds.is_contains_point(point) {
                    buffer.extend_from_slice(entries)
                }
            }
            OctreeNode::Branch { bounds, leaves } => {
                if bounds.is_contains_point(point) {
                    for leaf in leaves {
                        self.point_recursive_query(*leaf, point, buffer)
                    }
                }
            }
        }
    }
}

fn build_recursive(
    nodes: &mut Pool<OctreeNode>,
    entries: Vec<Entry>,
    bounds: AxisAlignedBoundingBox,
    split_threshold: usize,
) -> Handle<OctreeNode> {
    if entries.len() <= split_threshold {
        nodes.spawn(OctreeNode::Leaf { bounds, entries })
    } else {
        let mut leaves = [Handle::NONE; 8];

        let leaf_bounds = bounds.split();

        for (leaf, leaf_bounds) in leaves.iter_mut().zip(leaf_bounds) {
            let mut leaf_entries = Vec::new();

            leaf_entries.extend(
                entries
                    .iter()
                    .filter(|entry| entry.world_aabb.is_intersects_aabb(&bounds))
                    .cloned(),
            );

            *leaf = build_recursive(nodes, leaf_entries, leaf_bounds, split_threshold);
        }

        nodes.spawn(OctreeNode::Branch { leaves, bounds })
    }
}
