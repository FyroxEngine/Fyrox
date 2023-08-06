use crate::algebra::Vector3;
use crate::{
    math::{aabb::AxisAlignedBoundingBox, ray::Ray},
    pool::{Handle, Pool},
};
use arrayvec::ArrayVec;

#[derive(Clone, Debug)]
pub enum OctreeNode {
    Leaf {
        indices: Vec<u32>,
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
    pub fn new(triangles: &[[Vector3<f32>; 3]], split_threshold: usize) -> Self {
        // Calculate bounds.
        let mut bounds = AxisAlignedBoundingBox::default();
        for triangle in triangles {
            for pt in triangle.iter() {
                bounds.add_point(*pt);
            }
        }

        // Inflate initial bounds by very low value to fix floating-point calculation
        // issues when splitting and checking intersection later on.
        let inflation = 2.0 * f32::EPSILON;
        bounds.inflate(Vector3::new(inflation, inflation, inflation));

        // Get initial list of indices.
        let mut indices = Vec::new();
        for i in 0..triangles.len() {
            indices.push(i as u32);
        }

        let mut nodes = Pool::new();
        let root = build_recursive(&mut nodes, triangles, bounds, indices, split_threshold);

        Self { nodes, root }
    }

    pub fn sphere_query(&self, position: Vector3<f32>, radius: f32, buffer: &mut Vec<u32>) {
        buffer.clear();
        self.sphere_recursive_query(self.root, position, radius, buffer);
    }

    fn sphere_recursive_query(
        &self,
        node: Handle<OctreeNode>,
        position: Vector3<f32>,
        radius: f32,
        buffer: &mut Vec<u32>,
    ) {
        match self.nodes.borrow(node) {
            OctreeNode::Leaf { indices, bounds } => {
                if bounds.is_intersects_sphere(position, radius) {
                    buffer.extend_from_slice(indices)
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

    pub fn aabb_query(&self, aabb: &AxisAlignedBoundingBox, buffer: &mut Vec<u32>) {
        buffer.clear();
        self.aabb_recursive_query(self.root, aabb, buffer);
    }

    fn aabb_recursive_query(
        &self,
        node: Handle<OctreeNode>,
        aabb: &AxisAlignedBoundingBox,
        buffer: &mut Vec<u32>,
    ) {
        match self.nodes.borrow(node) {
            OctreeNode::Leaf { indices, bounds } => {
                if bounds.is_intersects_aabb(aabb) {
                    buffer.extend_from_slice(indices)
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

    pub fn ray_query(&self, ray: &Ray, buffer: &mut Vec<u32>) {
        buffer.clear();
        self.ray_recursive_query(self.root, ray, buffer);
    }

    fn ray_recursive_query(&self, node: Handle<OctreeNode>, ray: &Ray, buffer: &mut Vec<u32>) {
        match self.nodes.borrow(node) {
            OctreeNode::Leaf { indices, bounds } => {
                if ray.box_intersection(&bounds.min, &bounds.max).is_some() {
                    buffer.extend_from_slice(indices)
                }
            }
            OctreeNode::Branch { bounds, leaves } => {
                if ray.box_intersection(&bounds.min, &bounds.max).is_some() {
                    for leaf in leaves {
                        self.ray_recursive_query(*leaf, ray, buffer)
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

    pub fn ray_query_static<const CAP: usize>(
        &self,
        ray: &Ray,
        buffer: &mut ArrayVec<Handle<OctreeNode>, CAP>,
    ) {
        buffer.clear();
        self.ray_recursive_query_static(self.root, ray, buffer);
    }

    fn ray_recursive_query_static<const CAP: usize>(
        &self,
        node: Handle<OctreeNode>,
        ray: &Ray,
        buffer: &mut ArrayVec<Handle<OctreeNode>, CAP>,
    ) {
        match self.nodes.borrow(node) {
            OctreeNode::Leaf { bounds, .. } => {
                if ray.box_intersection(&bounds.min, &bounds.max).is_some() {
                    buffer.push(node);
                }
            }
            OctreeNode::Branch { bounds, leaves } => {
                if ray.box_intersection(&bounds.min, &bounds.max).is_some() {
                    for leaf in leaves {
                        self.ray_recursive_query_static(*leaf, ray, buffer)
                    }
                }
            }
        }
    }

    pub fn point_query(&self, point: Vector3<f32>, buffer: &mut Vec<u32>) {
        buffer.clear();
        self.point_recursive_query(self.root, point, buffer);
    }

    fn point_recursive_query(
        &self,
        node: Handle<OctreeNode>,
        point: Vector3<f32>,
        buffer: &mut Vec<u32>,
    ) {
        match self.nodes.borrow(node) {
            OctreeNode::Leaf { indices, bounds } => {
                if bounds.is_contains_point(point) {
                    buffer.extend_from_slice(indices)
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
    triangles: &[[Vector3<f32>; 3]],
    bounds: AxisAlignedBoundingBox,
    indices: Vec<u32>,
    split_threshold: usize,
) -> Handle<OctreeNode> {
    if indices.len() <= split_threshold {
        nodes.spawn(OctreeNode::Leaf { bounds, indices })
    } else {
        let mut leaves = [Handle::NONE; 8];
        let leaf_bounds = bounds.split();

        for i in 0..8 {
            let mut leaf_indices = Vec::new();

            for index in indices.iter() {
                let index = *index;

                let triangle_bounds =
                    AxisAlignedBoundingBox::from_points(&triangles[index as usize]);

                if triangle_bounds.is_intersects_aabb(&bounds) {
                    leaf_indices.push(index);
                }
            }

            leaves[i] = build_recursive(
                nodes,
                triangles,
                leaf_bounds[i],
                leaf_indices,
                split_threshold,
            );
        }

        nodes.spawn(OctreeNode::Branch { leaves, bounds })
    }
}
