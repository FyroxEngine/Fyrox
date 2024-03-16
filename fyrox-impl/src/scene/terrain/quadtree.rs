//! Special, terrain-specific quadtree.

use crate::{
    core::{
        algebra::{Matrix4, Vector2, Vector3},
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, frustum::Frustum},
    },
    scene::debug::SceneDrawingContext,
};

#[derive(Default, Debug, PartialEq)]
pub struct QuadTree {
    root: QuadTreeNode,
    pub max_level: u32,
}

#[derive(Debug, PartialEq)]
pub struct QuadTreeNode {
    pub size: Vector2<u32>,
    pub position: Vector2<u32>,
    pub kind: QuadTreeNodeKind,
    pub level: u32,
    pub min_height: f32,
    pub max_height: f32,
    pub persistent_index: usize,
}

impl Default for QuadTreeNode {
    fn default() -> Self {
        Self {
            size: Default::default(),
            position: Default::default(),
            kind: QuadTreeNodeKind::Leaf,
            level: 0,
            min_height: 0.0,
            max_height: 0.0,
            persistent_index: 0,
        }
    }
}

#[derive(Debug)]
pub struct SelectedNode {
    pub position: Vector2<u32>,
    pub size: Vector2<u32>,
    pub active_quadrants: [bool; 4],
    pub persistent_index: usize,
}

impl SelectedNode {
    pub fn is_draw_full(&self) -> bool {
        self.active_quadrants.iter().all(|s| *s)
    }
}

impl QuadTreeNode {
    pub fn new(
        height_map: &[f32],
        height_map_size: Vector2<u32>,
        position: Vector2<u32>,
        node_size: Vector2<u32>,
        max_size: Vector2<u32>,
        level: u32,
        index: &mut usize,
    ) -> Self {
        let mut min_height = f32::MAX;
        let mut max_height = f32::MIN;
        for y in position.y..((position.y + node_size.y).min(height_map_size.y)) {
            for x in position.x..((position.x + node_size.x).min(height_map_size.x)) {
                let height = height_map[(y * height_map_size.x + x) as usize];
                if height < min_height {
                    min_height = height;
                }
                if height > max_height {
                    max_height = height;
                }
            }
        }

        let kind = if node_size.x < max_size.x && node_size.y < max_size.y {
            QuadTreeNodeKind::Leaf
        } else {
            // Build children nodes recursively.
            let new_size = node_size / 2;
            let next_level = level + 1;
            QuadTreeNodeKind::Branch {
                leafs: [
                    Box::new(QuadTreeNode::new(
                        height_map,
                        height_map_size,
                        position,
                        new_size,
                        max_size,
                        next_level,
                        index,
                    )),
                    Box::new(QuadTreeNode::new(
                        height_map,
                        height_map_size,
                        position + Vector2::new(new_size.x, 0),
                        new_size,
                        max_size,
                        next_level,
                        index,
                    )),
                    Box::new(QuadTreeNode::new(
                        height_map,
                        height_map_size,
                        position + new_size,
                        new_size,
                        max_size,
                        next_level,
                        index,
                    )),
                    Box::new(QuadTreeNode::new(
                        height_map,
                        height_map_size,
                        position + Vector2::new(0, new_size.y),
                        new_size,
                        max_size,
                        next_level,
                        index,
                    )),
                ],
            }
        };

        let persistent_index = *index;
        *index += 1;

        Self {
            position,
            size: node_size,
            kind,
            level,
            min_height,
            max_height,
            persistent_index,
        }
    }

    pub fn aabb(
        &self,
        transform: &Matrix4<f32>,
        height_map_size: Vector2<u32>,
        physical_size: Vector2<f32>,
    ) -> AxisAlignedBoundingBox {
        let min_x = (self.position.x as f32 / height_map_size.x as f32) * physical_size.x;
        let min_y = (self.position.y as f32 / height_map_size.y as f32) * physical_size.y;

        let max_x =
            ((self.position.x + self.size.x) as f32 / height_map_size.x as f32) * physical_size.x;
        let max_y =
            ((self.position.y + self.size.y) as f32 / height_map_size.y as f32) * physical_size.y;

        let min = Vector3::new(min_x, self.min_height, min_y);
        let max = Vector3::new(max_x, self.max_height, max_y);

        AxisAlignedBoundingBox::from_min_max(min, max).transform(transform)
    }

    pub fn debug_draw(
        &self,
        transform: &Matrix4<f32>,
        height_map_size: Vector2<u32>,
        physical_size: Vector2<f32>,
        drawing_context: &mut SceneDrawingContext,
    ) {
        drawing_context.draw_aabb(
            &self.aabb(transform, height_map_size, physical_size),
            Color::RED,
        );

        if let QuadTreeNodeKind::Branch { ref leafs } = self.kind {
            for leaf in leafs {
                leaf.debug_draw(transform, height_map_size, physical_size, drawing_context);
            }
        }
    }

    /// `level_ranges` contains a list of distances for every lod in farthest-to-closest direction (first will be the
    /// most distant range).
    pub fn select(
        &self,
        transform: &Matrix4<f32>,
        height_map_size: Vector2<u32>,
        physical_size: Vector2<f32>,
        frustum: Option<&Frustum>,
        camera_position: Vector3<f32>,
        level_ranges: &[f32],
        selection: &mut Vec<SelectedNode>,
    ) -> bool {
        let aabb = self.aabb(transform, height_map_size, physical_size);

        let current_level = self.level as usize;

        if !frustum.map_or(true, |f| f.is_intersects_aabb(&aabb))
            || !aabb.is_intersects_sphere(camera_position, level_ranges[current_level])
        {
            return false;
        }

        if level_ranges
            .get(current_level + 1)
            .map_or(false, |next_range| {
                aabb.is_intersects_sphere(camera_position, *next_range)
            })
        {
            match self.kind {
                QuadTreeNodeKind::Branch { ref leafs } => {
                    let mut active_quadrants = [false; 4];

                    for (leaf, is_active) in leafs.iter().zip(active_quadrants.iter_mut()) {
                        *is_active = !leaf.select(
                            transform,
                            height_map_size,
                            physical_size,
                            frustum,
                            camera_position,
                            level_ranges,
                            selection,
                        );
                    }

                    selection.push(SelectedNode {
                        position: self.position,
                        size: self.size,
                        active_quadrants,
                        persistent_index: self.persistent_index,
                    });
                }
                QuadTreeNodeKind::Leaf => {
                    selection.push(SelectedNode {
                        position: self.position,
                        size: self.size,
                        active_quadrants: [true; 4],
                        persistent_index: self.persistent_index,
                    });
                }
            }
        } else {
            selection.push(SelectedNode {
                position: self.position,
                size: self.size,
                active_quadrants: [true; 4],
                persistent_index: self.persistent_index,
            });
        }

        true
    }

    fn max_level(&self, max_level: &mut u32) {
        if self.level > *max_level {
            *max_level = self.level
        }

        if let QuadTreeNodeKind::Branch { ref leafs } = self.kind {
            for leaf in leafs {
                leaf.max_level(max_level);
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum QuadTreeNodeKind {
    Leaf,
    Branch { leafs: [Box<QuadTreeNode>; 4] },
}

impl QuadTree {
    pub fn new(
        height_map: &[f32],
        height_map_size: Vector2<u32>,
        block_size: Vector2<u32>,
    ) -> Self {
        let mut index = 0;
        let root = QuadTreeNode::new(
            height_map,
            height_map_size,
            Vector2::new(0, 0),
            height_map_size,
            block_size,
            0,
            &mut index,
        );
        let mut max_level = 0;
        root.max_level(&mut max_level);
        Self { max_level, root }
    }

    /// `level_ranges` contains a list of distances for every lod in farthest-to-closest direction (first will be the
    /// most distant range).
    pub fn select(
        &self,
        transform: &Matrix4<f32>,
        height_map_size: Vector2<u32>,
        physical_size: Vector2<f32>,
        frustum: Option<&Frustum>,
        camera_position: Vector3<f32>,
        level_ranges: &[f32],
        selection: &mut Vec<SelectedNode>,
    ) {
        self.root.select(
            transform,
            height_map_size,
            physical_size,
            frustum,
            camera_position,
            level_ranges,
            selection,
        );
    }

    pub fn debug_draw(
        &self,
        transform: &Matrix4<f32>,
        height_map_size: Vector2<u32>,
        physical_size: Vector2<f32>,
        drawing_context: &mut SceneDrawingContext,
    ) {
        self.root
            .debug_draw(transform, height_map_size, physical_size, drawing_context);
    }
}

#[cfg(test)]
mod test {
    use crate::{
        core::{
            algebra::{Matrix4, Point3, Vector2, Vector3},
            math::frustum::Frustum,
        },
        scene::terrain::quadtree::QuadTree,
    };

    #[test]
    fn test_terrain_quad_tree_selection() {
        let block_size = Vector2::new(16, 16);
        let height_map_size = Vector2::<u32>::new(64, 64);
        let physical_size = Vector2::new(100.0, 100.0);
        let heightmap = vec![0.0; (height_map_size.x * height_map_size.y) as usize];
        let z_far = 100.0;

        let quadtree = QuadTree::new(&heightmap, height_map_size, block_size);

        let levels = (0..quadtree.max_level)
            .map(|n| z_far * (n as f32 / quadtree.max_level as f32))
            .collect::<Vec<_>>();

        for (iteration, (position, target)) in [
            (
                Point3::new(physical_size.x * 0.5, 0.0, 0.0),
                Point3::new(0.0, 0.0, 1.0),
            ),
            (
                Point3::new(physical_size.x * 0.5, 0.0, -f32::EPSILON),
                // Facing away from terrain.
                Point3::new(0.0, 0.0, -1.0),
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let view_matrix = Matrix4::look_at_rh(&position, &target, &Vector3::y());
            let projection = Matrix4::new_perspective(1.0, 90.0f32.to_radians(), 0.025, z_far);
            let frustum = Frustum::from_view_projection_matrix(projection * view_matrix).unwrap();

            let mut selection = Vec::new();
            quadtree.select(
                &Matrix4::identity(),
                height_map_size,
                physical_size,
                Some(&frustum),
                position.coords,
                &levels,
                &mut selection,
            );

            dbg!(iteration, &selection);

            if iteration == 1 {
                assert!(selection.is_empty());
            }
        }
    }
}
