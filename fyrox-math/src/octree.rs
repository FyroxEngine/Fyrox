use crate::{aabb::AxisAlignedBoundingBox, ray::Ray};
use arrayvec::ArrayVec;
use nalgebra::Vector3;

#[derive(Clone, Debug)]
pub enum OctreeNode {
    Leaf {
        indices: Vec<u32>,
        bounds: AxisAlignedBoundingBox,
    },
    Branch {
        bounds: AxisAlignedBoundingBox,
        leaves: [usize; 8],
    },
}

#[derive(Default, Clone, Debug)]
pub struct Octree {
    nodes: Vec<OctreeNode>,
    root: usize,
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

        let mut nodes = Vec::new();
        let root = build_recursive(&mut nodes, triangles, bounds, indices, split_threshold);

        Self { nodes, root }
    }

    pub fn sphere_query(&self, position: Vector3<f32>, radius: f32, buffer: &mut Vec<u32>) {
        buffer.clear();
        self.sphere_recursive_query(self.root, position, radius, buffer);
    }

    fn sphere_recursive_query(
        &self,
        node: usize,
        position: Vector3<f32>,
        radius: f32,
        buffer: &mut Vec<u32>,
    ) {
        match &self.nodes[node] {
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
        node: usize,
        aabb: &AxisAlignedBoundingBox,
        buffer: &mut Vec<u32>,
    ) {
        match &self.nodes[node] {
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

    fn ray_recursive_query(&self, node: usize, ray: &Ray, buffer: &mut Vec<u32>) {
        match &self.nodes[node] {
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

    pub fn node(&self, handle: usize) -> &OctreeNode {
        &self.nodes[handle]
    }

    pub fn nodes(&self) -> &Vec<OctreeNode> {
        &self.nodes
    }

    pub fn ray_query_static<const CAP: usize>(&self, ray: &Ray, buffer: &mut ArrayVec<usize, CAP>) {
        buffer.clear();
        self.ray_recursive_query_static(self.root, ray, buffer);
    }

    fn ray_recursive_query_static<const CAP: usize>(
        &self,
        node: usize,
        ray: &Ray,
        buffer: &mut ArrayVec<usize, CAP>,
    ) {
        match &self.nodes[node] {
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

    pub fn point_query<C>(&self, point: Vector3<f32>, mut callback: C)
    where
        C: FnMut(&[u32]),
    {
        self.point_recursive_query(self.root, point, &mut callback);
    }

    fn point_recursive_query<C>(&self, node: usize, point: Vector3<f32>, callback: &mut C)
    where
        C: FnMut(&[u32]),
    {
        match &self.nodes[node] {
            OctreeNode::Leaf { indices, bounds } => {
                if bounds.is_contains_point(point) {
                    (callback)(indices)
                }
            }
            OctreeNode::Branch { bounds, leaves } => {
                if bounds.is_contains_point(point) {
                    for leaf in leaves {
                        self.point_recursive_query(*leaf, point, callback)
                    }
                }
            }
        }
    }
}

fn build_recursive(
    nodes: &mut Vec<OctreeNode>,
    triangles: &[[Vector3<f32>; 3]],
    bounds: AxisAlignedBoundingBox,
    indices: Vec<u32>,
    split_threshold: usize,
) -> usize {
    if indices.len() <= split_threshold {
        let index = nodes.len();
        nodes.push(OctreeNode::Leaf { bounds, indices });
        index
    } else {
        let mut leaves = [0; 8];
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

        let index = nodes.len();
        nodes.push(OctreeNode::Branch { leaves, bounds });
        index
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn get_six_triangles() -> [[Vector3<f32>; 3]; 6] {
        [
            [
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(1.0, 0.0, 0.0),
                Vector3::new(0.0, 1.0, 0.0),
            ],
            [
                Vector3::new(1.0, 1.0, 0.0),
                Vector3::new(1.0, 0.0, 0.0),
                Vector3::new(0.0, 1.0, 0.0),
            ],
            [
                Vector3::new(0.0, 1.0, 0.0),
                Vector3::new(1.0, 1.0, 0.0),
                Vector3::new(0.0, 2.0, 0.0),
            ],
            [
                Vector3::new(1.0, 2.0, 0.0),
                Vector3::new(1.0, 1.0, 0.0),
                Vector3::new(0.0, 2.0, 0.0),
            ],
            [
                Vector3::new(0.0, 2.0, 0.0),
                Vector3::new(1.0, 2.0, 0.0),
                Vector3::new(0.0, 3.0, 0.0),
            ],
            [
                Vector3::new(1.0, 3.0, 0.0),
                Vector3::new(1.0, 2.0, 0.0),
                Vector3::new(0.0, 3.0, 0.0),
            ],
        ]
    }

    #[test]
    fn octree_new() {
        let tree = Octree::new(&get_six_triangles(), 5);

        assert_eq!(tree.root, 72);
        assert_eq!(tree.nodes().len(), 73);
    }

    #[test]
    fn default_for_octree() {
        let tree = Octree::default();
        assert_eq!(tree.root, 0);
        assert_eq!(tree.nodes.len(), 0);
    }

    #[test]
    fn octree_point_query() {
        let tree = Octree::new(&get_six_triangles(), 5);
        let mut buffer = Vec::new();
        tree.point_query(Vector3::new(0.0, 0.0, 0.0), |triangles| {
            buffer.extend_from_slice(triangles)
        });

        assert_eq!(buffer, [0, 1, 2, 3, 0, 1, 2, 3]);
    }

    #[test]
    fn octree_sphere_query() {
        let tree = Octree::new(&get_six_triangles(), 5);
        let mut buffer = Vec::new();
        tree.sphere_query(Vector3::new(0.0, 0.0, 0.0), 1.0, &mut buffer);

        assert_eq!(
            buffer,
            [
                0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3,
                0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3,
                0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3,
                0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3
            ]
        );
    }

    #[test]
    fn octree_aabb_query() {
        let tree = Octree::new(&get_six_triangles(), 5);
        let mut buffer = Vec::new();
        tree.aabb_query(
            &AxisAlignedBoundingBox {
                min: Vector3::new(0.0, 0.0, 0.0),
                max: Vector3::new(0.5, 0.5, 0.5),
            },
            &mut buffer,
        );

        assert_eq!(
            buffer,
            [
                0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3,
                0, 1, 2, 3, 0, 1, 2, 3
            ]
        );
    }

    #[test]
    fn octree_ray_query() {
        let tree = Octree::new(&get_six_triangles(), 5);
        let mut buffer = Vec::new();
        tree.ray_query(
            &Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 0.0)),
            &mut buffer,
        );

        assert_eq!(
            buffer,
            [
                0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3,
                0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3
            ]
        );
    }

    #[test]
    fn octree_ray_query_static() {
        const CAP: usize = 10;
        let tree = Octree::new(&get_six_triangles(), 5);
        let mut buffer = ArrayVec::<usize, CAP>::new();
        tree.ray_query_static::<CAP>(
            &Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 0.0)),
            &mut buffer,
        );

        assert_eq!(buffer.as_slice(), [2, 3, 11, 15, 16, 18, 19, 27, 31, 32,]);
    }
}
