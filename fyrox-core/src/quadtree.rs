use crate::{
    algebra::Vector2,
    math::Rect,
    pool::{Handle, Pool},
};
use arrayvec::ArrayVec;

pub enum QuadTreeNode<T> {
    Leaf {
        bounds: Rect<f32>,
        ids: Vec<T>,
    },
    Branch {
        bounds: Rect<f32>,
        leaves: [Handle<QuadTreeNode<T>>; 4],
    },
}

fn split_rect(rect: &Rect<f32>) -> [Rect<f32>; 4] {
    let half_size = rect.size.scale(0.5);
    [
        Rect {
            position: rect.position,
            size: half_size,
        },
        Rect {
            position: Vector2::new(rect.position.x + half_size.x, rect.position.y),
            size: half_size,
        },
        Rect {
            position: rect.position + half_size,
            size: half_size,
        },
        Rect {
            position: Vector2::new(rect.position.x, rect.position.y + half_size.y),
            size: half_size,
        },
    ]
}

pub struct QuadTree<T> {
    nodes: Pool<QuadTreeNode<T>>,
    root: Handle<QuadTreeNode<T>>,
    split_threshold: usize,
}

impl<T> Default for QuadTree<T> {
    fn default() -> Self {
        Self {
            nodes: Default::default(),
            root: Default::default(),
            split_threshold: 16,
        }
    }
}

pub trait BoundsProvider {
    type Id: Clone;

    fn bounds(&self) -> Rect<f32>;

    fn id(&self) -> Self::Id;
}

pub enum QuadTreeBuildError {
    /// It means that given split threshold is too low for an algorithm to build quad tree.
    /// Make it larger and try again. Also this might mean that your initial bounds are too small.
    ReachedRecursionLimit,
}

#[derive(Clone)]
struct Entry<I: Clone> {
    id: I,
    bounds: Rect<f32>,
}

fn build_recursive<I: Clone>(
    nodes: &mut Pool<QuadTreeNode<I>>,
    bounds: Rect<f32>,
    entries: &[Entry<I>],
    split_threshold: usize,
    depth: usize,
) -> Result<Handle<QuadTreeNode<I>>, QuadTreeBuildError> {
    if depth >= 64 {
        Err(QuadTreeBuildError::ReachedRecursionLimit)
    } else if entries.len() <= split_threshold {
        Ok(nodes.spawn(QuadTreeNode::Leaf {
            bounds,
            ids: entries.iter().map(|e| e.id.clone()).collect::<Vec<_>>(),
        }))
    } else {
        let leaf_bounds = split_rect(&bounds);
        let mut leaves = [Handle::NONE; 4];

        for (leaf, &leaf_bounds) in leaves.iter_mut().zip(leaf_bounds.iter()) {
            let leaf_entries = entries
                .iter()
                .filter_map(|e| {
                    if leaf_bounds.intersects(e.bounds) {
                        Some(e.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            *leaf = build_recursive(
                nodes,
                leaf_bounds,
                &leaf_entries,
                split_threshold,
                depth + 1,
            )?;
        }

        Ok(nodes.spawn(QuadTreeNode::Branch { bounds, leaves }))
    }
}

impl<I: Clone> QuadTree<I> {
    pub fn new<T: BoundsProvider<Id = I>>(
        root_bounds: Rect<f32>,
        objects: impl Iterator<Item = T>,
        split_threshold: usize,
    ) -> Result<Self, QuadTreeBuildError> {
        let entries = objects
            .filter_map(|o| {
                if root_bounds.intersects(o.bounds()) {
                    Some(Entry {
                        id: o.id(),
                        bounds: o.bounds(),
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let mut nodes = Pool::new();
        let root = build_recursive(&mut nodes, root_bounds, &entries, split_threshold, 0)?;
        Ok(Self {
            nodes,
            root,
            split_threshold,
        })
    }

    pub fn point_query<S: QueryStorage<Id = I>>(&self, point: Vector2<f32>, storage: &mut S) {
        self.point_query_recursive(self.root, point, storage)
    }

    fn point_query_recursive<S: QueryStorage<Id = I>>(
        &self,
        node: Handle<QuadTreeNode<I>>,
        point: Vector2<f32>,
        storage: &mut S,
    ) {
        if node.is_some() {
            match self.nodes.borrow(node) {
                QuadTreeNode::Leaf { bounds, ids } => {
                    if bounds.contains(point) {
                        for id in ids {
                            if !storage.try_push(id.clone()) {
                                return;
                            }
                        }
                    }
                }
                QuadTreeNode::Branch { bounds, leaves } => {
                    if bounds.contains(point) {
                        for &leaf in leaves {
                            self.point_query_recursive(leaf, point, storage)
                        }
                    }
                }
            }
        }
    }

    pub fn split_threshold(&self) -> usize {
        self.split_threshold
    }
}

pub trait QueryStorage {
    type Id;

    fn try_push(&mut self, id: Self::Id) -> bool;

    /// Clears the storage.
    fn clear(&mut self);
}

impl<I> QueryStorage for Vec<I> {
    type Id = I;

    fn try_push(&mut self, intersection: I) -> bool {
        self.push(intersection);
        true
    }

    fn clear(&mut self) {
        self.clear()
    }
}

impl<I, const CAP: usize> QueryStorage for ArrayVec<I, CAP> {
    type Id = I;

    fn try_push(&mut self, intersection: I) -> bool {
        self.try_push(intersection).is_ok()
    }

    fn clear(&mut self) {
        self.clear()
    }
}

#[cfg(test)]
mod test {
    use crate::math::Rect;
    use crate::quadtree::{BoundsProvider, QuadTree};

    struct TestObject {
        bounds: Rect<f32>,
        id: usize,
    }

    impl BoundsProvider for &TestObject {
        type Id = usize;

        fn bounds(&self) -> Rect<f32> {
            self.bounds
        }

        fn id(&self) -> Self::Id {
            self.id
        }
    }

    #[test]
    fn test_quad_tree() {
        let root_bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let objects = vec![
            TestObject {
                bounds: Rect::new(10.0, 10.0, 10.0, 10.0),
                id: 0,
            },
            TestObject {
                bounds: Rect::new(10.0, 10.0, 10.0, 10.0),
                id: 1,
            },
        ];
        // Infinite recursion prevention check (when there are multiple objects share same location).
        assert!(QuadTree::new(root_bounds, objects.iter(), 1).is_err());

        let objects = vec![
            TestObject {
                bounds: Rect::new(10.0, 10.0, 10.0, 10.0),
                id: 0,
            },
            TestObject {
                bounds: Rect::new(20.0, 20.0, 10.0, 10.0),
                id: 1,
            },
        ];
        assert!(QuadTree::new(root_bounds, objects.iter(), 1).is_ok());
    }
}
