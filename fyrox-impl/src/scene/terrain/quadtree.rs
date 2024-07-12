//! Special, terrain-specific quadtree.

use crate::{
    core::{
        algebra::{Matrix4, Vector2, Vector3},
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, frustum::Frustum},
    },
    scene::debug::SceneDrawingContext,
};

/// A QuadTree represents the geometry of a chunk of a height map.
/// It allows the chunk to be repeatedly split into four quadrants for increasing levels of detail.
/// Whether a particular level of detail will be used in rendering is determined by its distance
/// to the camera and whether it intersects the camera's frustrum.
/// These distances and intersections are determined by the AABBs stored in the nodes of this tree.
#[derive(Default, Debug, PartialEq)]
pub struct QuadTree {
    root: QuadTreeNode,
    pub max_level: u32,
    height_mod_count: u64,
}

/// Each QuadTreeNode is primarily responsible for storing the AABB data for a particular
/// area of terrain and pointers to the four children of the node.
#[derive(Debug, PartialEq)]
pub struct QuadTreeNode {
    /// The size of the 2D area that this node represents.
    pub size: Vector2<u32>,
    /// The position of the area that this node represents.
    pub position: Vector2<u32>,
    /// The children of this node.
    pub kind: QuadTreeNodeKind,
    /// The level of detail of this node.
    /// This determines whether we should render this node directly (if `level` is high enough)
    /// or whether we should render this node's children (if `level` is too low).
    pub level: u32,
    /// The minimum of all terrain height data within the area this node represents.
    pub min_height: f32,
    /// The maximum of all terrain height data within the area this node represents.
    pub max_height: f32,
    /// A number that is unique to each node in the tree, increment as the tree is constructed
    /// so that each constructed node gets a value one greater than the previous node.
    /// It is used to create a [PersistentIdentifier](crate::renderer::bundle::PersistentIdentifier)
    /// for each instance of the terrain geometry used to render the terrain.
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

/// The relevant details of a QuadTreeNode that needs to be rendered.
/// These are generated based upon the camera position, view frustrum, and other details
/// that are provided as arguments to [QuadTree::select].
#[derive(Debug)]
pub struct SelectedNode {
    /// The position of the selected [QuadTreeNode].
    /// This determines the translaiton of the [TerrainGeometry](crate::scene::terrain::geometry::TerrainGeometry) instance.
    pub position: Vector2<u32>,
    /// The size of the selected [QuadTreeNode].
    /// This determines the scaling of the [TerrainGeometry](crate::scene::terrain::geometry::TerrainGeometry) instance,
    /// as it may be re-sized as needed to cover the area of the node.
    pub size: Vector2<u32>,
    /// An array of four flags that marks which of the node's four quadrants need to be rendered.
    /// If any quadrants are active, then an instance of the terrain geometry is put at the
    /// [SelectedNode::position] and [SelectedNode::size].
    /// The active_quadrants determine which elements go in the
    /// [SurfaceInstanceData::element_range](crate::renderer::bundle::SurfaceInstanceData::element_range).
    pub active_quadrants: [bool; 4],
    /// The [persistent_index](QuadTreeNode::persistent_index) of the selected [QuadTreeNode].
    /// This is used to create a [PersistentIdentifier](crate::renderer::bundle::PersistentIdentifier) for
    /// the geometry of this node.
    pub persistent_index: usize,
}

impl SelectedNode {
    /// Do all four quadrants need to be drawn? If so, then we can render this entire node
    /// with a single instance of terrain geometry.
    pub fn is_draw_full(&self) -> bool {
        self.active_quadrants.iter().all(|s| *s)
    }
}

impl QuadTreeNode {
    /// * height_map: The height data as an array of f32.
    /// * height_map_size: The number of rows and columns of the height data.
    /// * position: The position of the area represented by this node within the data.
    /// * node_size: The size of the area represented by this ndoe within the data.
    /// Each node should overlap with its neighbors along the edges by one pixel.
    /// * max_size: Any node below this size will be a leaf.
    /// * level: The level of detail of this node.
    /// * index: The mutable pointer to the current persistent index.
    /// It will be recursively passed to each of the children and incremented by each child,
    /// then it's value will be copied into [QuadTreeNode::persistent_index] of this node
    /// and then incremented for the next node.
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
        let x_max = position.x + node_size.x;
        let y_max = position.y + node_size.y;
        assert!(
            x_max <= height_map_size.x,
            "position.x({}) + node_size.x({}) = {} > {} (height_map_size.x)",
            position.x,
            node_size.x,
            x_max,
            height_map_size.x
        );
        assert!(
            y_max <= height_map_size.y,
            "position.y({}) + node_size.y({}) = {} > {} (height_map_size.y)",
            position.y,
            node_size.y,
            y_max,
            height_map_size.y
        );
        for y in position.y..y_max {
            for x in position.x..x_max {
                let height = height_map[(y * height_map_size.x + x) as usize];
                if height < min_height {
                    min_height = height;
                }
                if height > max_height {
                    max_height = height;
                }
            }
        }

        let kind = if node_size.x <= max_size.x && node_size.y <= max_size.y {
            QuadTreeNodeKind::Leaf
        } else {
            // Build children nodes recursively.
            // Convert pixel size into mesh size, counting the edges between vertices instead of counting vertices.
            let real_size = Vector2::new(node_size.x - 1, node_size.y - 1);
            // Calculate child size by taking half of the real size and adding 1 to convert back into pixel size.
            let new_size = Vector2::new(real_size.x / 2 + 1, real_size.y / 2 + 1);
            // The first pixel of the next node starts on the last pixel of the previous node, not on the first pixel beyond the previous node.
            // Therefore we position the node at node_size.x - 1 instead of node_size.x.
            let center_pos = Vector2::new(new_size.x - 1, new_size.y - 1);
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
                        position + Vector2::new(center_pos.x, 0),
                        new_size,
                        max_size,
                        next_level,
                        index,
                    )),
                    Box::new(QuadTreeNode::new(
                        height_map,
                        height_map_size,
                        position + center_pos,
                        new_size,
                        max_size,
                        next_level,
                        index,
                    )),
                    Box::new(QuadTreeNode::new(
                        height_map,
                        height_map_size,
                        position + Vector2::new(0, center_pos.y),
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

    /// Construct an AABB for the node.
    /// * transform: Transformation matrix to apply to the AABB just before it is returned.
    /// * height_map_size: The overall size of the whole of the height map data that this node is a part of.
    /// * physical_size: The size of the whole of the height map data in world units.
    /// Note that the sizes of these arguments are only for the chunk of this [QuadTree].
    /// Other chunks are not included since they have entirely separate height data.
    pub fn aabb(
        &self,
        transform: &Matrix4<f32>,
        height_map_size: Vector2<u32>,
        physical_size: Vector2<f32>,
    ) -> AxisAlignedBoundingBox {
        // Convert sizes form pixel sizes to mesh sizes.
        // For calculating AABB, we do not care about the number of vertices;
        // we care about the number of edges between vertices, which is one fewer.
        let real_map_size = Vector2::new(height_map_size.x - 1, height_map_size.y - 1);
        let real_node_size = Vector2::new(self.size.x - 1, self.size.y - 1);
        let min_x = (self.position.x as f32 / real_map_size.x as f32) * physical_size.x;
        let min_y = (self.position.y as f32 / real_map_size.y as f32) * physical_size.y;

        let max_x = ((self.position.x + real_node_size.x) as f32 / real_map_size.x as f32)
            * physical_size.x;
        let max_y = ((self.position.y + real_node_size.y) as f32 / real_map_size.y as f32)
            * physical_size.y;

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

    /// Determine the size and position of terrain geometry instances that are needed in order to
    /// render the part of the chunk that is represented by this node of the [QuadTree].
    /// Return true if new elements have been added to `selection`.
    /// * `transform`: The matrix transformation to apply to the rendered height map geometry.
    /// * `height_map_size`: The size of the height data of this QuadTree's chunk in rows and columns.
    /// * `physical_size`: The size of the chunk in local units before the transform is applied.
    /// * `frustrum`: The camera frustrum in world space. Intersections with this frustrum are tested after `transform` is applied.
    /// * `camera_position`: The camera position in world space. Distances from this position are calculated after `transform` is applied.
    /// * `level_ranges`: a list of distances for every LOD in farthest-to-closest direction (first will be the
    /// most distant range).
    /// * `selection`: a mutable list that will store the list of QuadTreeNodes that need to be rendered.
    ///
    /// Note that being in the `selection` list does not mean that the node will be rendered directly.
    /// It may be the node's children that will be directly rendered.
    /// A node can be included in the `selection` list with all four of its quadrants set to inactive
    /// in [SelectedNode::active_quadrants].
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
            // This node is out of range, so add nothing to `selection` and return.
            // If this node is rendered at all, it will need an active quadrant of the parent node.
            return false;
        }

        // Get the range for the LOD above the LOD of this node.
        // Check whether any part of the AABB of this node is within that range.
        // If the list has no LOD range above the LOD of this node,
        // then we are at the maximum LOD and the children of this node are to be ignored.
        if level_ranges
            .get(current_level + 1)
            .map_or(false, |next_range| {
                aabb.is_intersects_sphere(camera_position, *next_range)
            })
        {
            // We are close enough to the camera that we need to try to render a higher LOD,
            // so examine the children of this node, if any.
            match self.kind {
                QuadTreeNodeKind::Branch { ref leafs } => {
                    let mut active_quadrants = [false; 4];

                    // Recursively go through the child for each quadrant to determine whether we need to
                    // render that quadrant directly, or let the child render the quadrant.
                    for (leaf, is_active) in leafs.iter().zip(active_quadrants.iter_mut()) {
                        // Activate the quadrant if the child has added nothing to the selection list.
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

                    // Push the position of this node onto the list, even if `active_quadrants` is all false.
                    selection.push(SelectedNode {
                        position: self.position,
                        size: self.size,
                        active_quadrants,
                        persistent_index: self.persistent_index,
                    });
                }
                QuadTreeNodeKind::Leaf => {
                    // A leaf has no children, so push the node into the list with all four quadrants active.
                    selection.push(SelectedNode {
                        position: self.position,
                        size: self.size,
                        active_quadrants: [true; 4],
                        persistent_index: self.persistent_index,
                    });
                }
            }
        } else {
            // Are far enough from the camera that we should ignore this node's children due to LOD.
            // Just render this node with all four quadrants active.
            selection.push(SelectedNode {
                position: self.position,
                size: self.size,
                active_quadrants: [true; 4],
                persistent_index: self.persistent_index,
            });
        }
        // At this point we are guaranteed to have added something to the selection list.
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
        height_mod_count: u64,
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
        Self {
            max_level,
            root,
            height_mod_count,
        }
    }

    pub fn height_mod_count(&self) -> u64 {
        self.height_mod_count
    }

    /// Determine the size and position of terrain geometry instances that are needed in order to render the chunk of this QuadTree.
    /// * `transform`: The matrix transformation to apply to the rendered height map geometry.
    /// * `height_map_size`: The size of the height data of this QuadTree's chunk in rows and columns.
    /// * `physical_size`: The size of the chunk in local units before the transform is applied.
    /// * `frustrum`: The camera frustrum in world space. Intersections with this frustrum are tested after `transform` is applied.
    /// * `camera_position`: The camera position in world space. Distances from this position are calculated after `transform` is applied.
    /// * `level_ranges`: a list of distances for every LOD in farthest-to-closest direction (first will be the
    /// most distant range).
    /// * `selection`: a mutable list that will store the list of QuadTreeNodes that need to be rendered.
    ///
    /// Note that being in the `selection` list does not mean that the node will be rendered directly.
    /// It may be the node's children that will be directly rendered.
    /// A node can be included in the `selection` list with all four of its quadrants set to inactive
    /// in [SelectedNode::active_quadrants].
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

        let quadtree = QuadTree::new(&heightmap, height_map_size, block_size, 0);

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
