use crate::{
    pool::{Handle, Pool},
    math::{
        aabb::AxisAlignedBoundingBox,
        vec3::Vec3,
        ray::Ray,
    },
};

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

#[derive(Default)]
pub struct Octree {
    nodes: Pool<OctreeNode>,
    root: Handle<OctreeNode>,
}

impl Octree {
    pub fn new(triangles: &[[Vec3; 3]], split_threshold: usize) -> Self {
        // Calculate bounds.
        let mut bounds = AxisAlignedBoundingBox::default();
        for triangle in triangles {
            for pt in triangle.iter() {
                bounds.add_point(*pt);
            }
        }

        // Get initial list of indices.
        let mut indices = Vec::new();
        for i in 0..triangles.len() {
            indices.push(i as u32);
        }

        let mut nodes = Pool::new();
        let root = build_recursive(&mut nodes, triangles, bounds, indices, split_threshold);

        Self {
            nodes,
            root,
        }
    }

    pub fn sphere_query(&self, position: Vec3, radius: f32, buffer: &mut Vec<u32>) {
        buffer.clear();
        self.sphere_recursive_query(self.root, position, radius, buffer);
    }

    fn sphere_recursive_query(&self, node: Handle<OctreeNode>, position: Vec3, radius: f32, buffer: &mut Vec<u32>) {
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

    fn aabb_recursive_query(&self, node: Handle<OctreeNode>, aabb: &AxisAlignedBoundingBox, buffer: &mut Vec<u32>) {
        match self.nodes.borrow(node) {
            OctreeNode::Leaf { indices, bounds } => {
                if bounds.intersect_aabb(aabb) {
                    buffer.extend_from_slice(indices)
                }
            }
            OctreeNode::Branch { bounds, leaves } => {
                if bounds.intersect_aabb(aabb) {
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
                if let Some(_) = ray.box_intersection(&bounds.min, &bounds.max) {
                    buffer.extend_from_slice(indices)
                }
            }
            OctreeNode::Branch { bounds, leaves } => {
                if let Some(_) = ray.box_intersection(&bounds.min, &bounds.max) {
                    for leaf in leaves {
                        self.ray_recursive_query(*leaf, ray, buffer)
                    }
                }
            }
        }
    }

    pub fn point_query(&self, point: Vec3, buffer: &mut Vec<u32>) {
        buffer.clear();
        self.point_recursive_query(self.root, point, buffer);
    }

    fn point_recursive_query(&self, node: Handle<OctreeNode>, point: Vec3, buffer: &mut Vec<u32>) {
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
    triangles: &[[Vec3; 3]],
    bounds: AxisAlignedBoundingBox,
    indices: Vec<u32>,
    split_threshold: usize,
) -> Handle<OctreeNode> {
    if indices.len() <= split_threshold {
        nodes.spawn(OctreeNode::Leaf {
            bounds,
            indices,
        })
    } else {
        let mut leaves = [Handle::NONE; 8];
        let leaf_bounds = split_bounds(bounds);

        for i in 0..8 {
            let mut leaf_indices = Vec::new();

            for index in indices.iter() {
                let index = *index;

                let triangle_bounds = AxisAlignedBoundingBox::from_points(&triangles[index as usize]);

                if triangle_bounds.intersect_aabb(&bounds) {
                    leaf_indices.push(index);
                }
            }

            leaves[i] = build_recursive(nodes, triangles, leaf_bounds[i], leaf_indices, split_threshold);
        }

        nodes.spawn(OctreeNode::Branch {
            leaves,
            bounds,
        })
    }
}

fn split_bounds(bounds: AxisAlignedBoundingBox) -> [AxisAlignedBoundingBox; 8] {
    let center = bounds.center();
    let min = &bounds.min;
    let max = &bounds.max;
    [
        AxisAlignedBoundingBox::from_min_max(Vec3::new(min.x, min.y, min.z), Vec3::new(center.x, center.y, center.z)),
        AxisAlignedBoundingBox::from_min_max(Vec3::new(center.x, min.y, min.z), Vec3::new(max.x, center.y, center.z)),
        AxisAlignedBoundingBox::from_min_max(Vec3::new(min.x, min.y, center.z), Vec3::new(center.x, center.y, max.z)),
        AxisAlignedBoundingBox::from_min_max(Vec3::new(center.x, min.y, center.z), Vec3::new(max.x, center.y, max.z)),
        AxisAlignedBoundingBox::from_min_max(Vec3::new(min.x, center.y, min.z), Vec3::new(center.x, max.y, center.z)),
        AxisAlignedBoundingBox::from_min_max(Vec3::new(center.x, center.y, min.z), Vec3::new(max.x, max.y, center.z)),
        AxisAlignedBoundingBox::from_min_max(Vec3::new(min.x, center.y, center.z), Vec3::new(center.x, max.y, max.z)),
        AxisAlignedBoundingBox::from_min_max(Vec3::new(center.x, center.y, center.z), Vec3::new(max.x, max.y, max.z))
    ]
}

#[cfg(test)]
mod test {
    use crate::octree::Octree;

    #[test]
    fn octree() {
        // TODO
        // It works fine in rusty-shooter game.
    }
}