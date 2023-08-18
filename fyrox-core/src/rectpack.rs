//! Rectangle packer is used to pack set of smaller rectangles into one big, it
//! used in texture atlas packer.

use crate::{
    math::Rect,
    num_traits::Zero,
    pool::{Handle, Pool},
};
use nalgebra::Scalar;
use num_traits::NumAssign;

struct RectPackNode<T>
where
    T: NumAssign + Scalar + PartialOrd + Copy,
{
    filled: bool,
    split: bool,
    bounds: Rect<T>,
    left: Handle<RectPackNode<T>>,
    right: Handle<RectPackNode<T>>,
}

impl<T> RectPackNode<T>
where
    T: NumAssign + Scalar + PartialOrd + Copy,
{
    fn new(bounds: Rect<T>) -> Self {
        Self {
            bounds,
            filled: false,
            split: false,
            left: Handle::NONE,
            right: Handle::NONE,
        }
    }
}

/// See module docs.
pub struct RectPacker<T>
where
    T: NumAssign + Scalar + PartialOrd + Copy,
{
    nodes: Pool<RectPackNode<T>>,
    root: Handle<RectPackNode<T>>,
    width: T,
    height: T,
    unvisited: Vec<Handle<RectPackNode<T>>>,
}

impl<T> RectPacker<T>
where
    T: NumAssign + Scalar + PartialOrd + Copy,
{
    /// Creates new instance of rectangle packer with given bounds.
    ///
    /// # How to choose bounds
    ///
    /// If you have a set of rectangles and you need to calculate average side length of a square,
    /// then calculate total area of your triangles by sum of width*height and then take square
    /// root out of area. You'll get side length of a square which can be used as width and height
    /// parameters.
    pub fn new(w: T, h: T) -> Self {
        let mut nodes = Pool::new();
        let root = nodes.spawn(RectPackNode::new(Rect::new(
            Zero::zero(),
            Zero::zero(),
            w,
            h,
        )));
        Self {
            nodes,
            root,
            width: w,
            height: h,
            unvisited: Default::default(),
        }
    }

    /// Clears packer and prepares it for another run. It is much cheaper than create new packer,
    /// because it reuses previously allocated memory.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.unvisited.clear();
        self.root = self.nodes.spawn(RectPackNode::new(Rect::new(
            Zero::zero(),
            Zero::zero(),
            self.width,
            self.height,
        )));
    }

    /// Tries to find free place to put rectangle with given size. Returns None if there insufficient
    /// space.
    pub fn find_free(&mut self, w: T, h: T) -> Option<Rect<T>> {
        if self.unvisited.is_empty() {
            self.unvisited.push(self.root);
        }

        while let Some(node_handle) = self.unvisited.pop() {
            let node = self.nodes.borrow_mut(node_handle);
            if node.split {
                self.unvisited.push(node.right);
                self.unvisited.push(node.left);
            } else if !node.filled && node.bounds.w() >= w && node.bounds.h() >= h {
                if node.bounds.w() == w && node.bounds.h() == h {
                    node.filled = true;
                    return Some(node.bounds);
                }

                // Split and continue
                node.split = true;

                let (left_bounds, right_bounds) = if node.bounds.w() - w > node.bounds.h() - h {
                    (
                        Rect::new(node.bounds.x(), node.bounds.y(), w, node.bounds.h()),
                        Rect::new(
                            node.bounds.x() + w,
                            node.bounds.y(),
                            node.bounds.w() - w,
                            node.bounds.h(),
                        ),
                    )
                } else {
                    (
                        Rect::new(node.bounds.x(), node.bounds.y(), node.bounds.w(), h),
                        Rect::new(
                            node.bounds.x(),
                            node.bounds.y() + h,
                            node.bounds.w(),
                            node.bounds.h() - h,
                        ),
                    )
                };

                let left = self.nodes.spawn(RectPackNode::new(left_bounds));
                let right = self.nodes.spawn(RectPackNode::new(right_bounds));

                let node = self.nodes.borrow_mut(node_handle);
                node.left = left;
                node.right = right;

                self.unvisited.push(left);
            }
        }

        None
    }
}

#[cfg(test)]
mod test {
    use crate::{math::Rect, pool::Handle};

    use super::{RectPackNode, RectPacker};

    #[test]
    fn rect_pack_node_new() {
        let rect = Rect::new(0.0, 0.0, 1.0, 1.0);
        let node = RectPackNode::new(rect);

        assert!(!node.filled);
        assert!(!node.split);
        assert_eq!(node.bounds, rect);
        assert_eq!(node.left, Handle::NONE);
        assert_eq!(node.right, Handle::NONE);
    }

    #[test]
    fn rect_packer_new() {
        let rp = RectPacker::new(1.0, 1.0);

        assert_eq!(rp.width, 1.0);
        assert_eq!(rp.height, 1.0);
        assert_eq!(rp.unvisited, vec![]);
    }

    #[test]
    fn rect_packer_find_free() {
        let mut rp = RectPacker::new(10.0, 10.0);

        assert_eq!(rp.find_free(20.0, 20.0), None);
        assert_eq!(rp.find_free(1.0, 1.0), Some(Rect::new(0.0, 0.0, 1.0, 1.0)));
        assert_eq!(rp.find_free(9.0, 9.0), Some(Rect::new(0.0, 1.0, 9.0, 9.0)));
    }

    #[test]
    fn rect_packer_clear() {
        let mut rp = RectPacker::new(10.0, 10.0);

        rp.find_free(1.0, 1.0);
        rp.find_free(9.0, 9.0);
        assert_eq!(rp.nodes.alive_count(), 7);

        rp.clear();
        assert_eq!(rp.nodes.alive_count(), 1);
    }
}
