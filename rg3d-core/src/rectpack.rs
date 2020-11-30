//! Rectangle packer is used to pack set of smaller rectangles into one big, it
//! used in texture atlas packer.

use crate::{
    math::Rect,
    pool::{Handle, Pool},
};
use nalgebra::Scalar;
use std::ops::{Add, Mul, Sub};

struct RectPackNode<T: Scalar> {
    filled: bool,
    split: bool,
    bounds: Rect<T>,
    left: Handle<RectPackNode<T>>,
    right: Handle<RectPackNode<T>>,
}

impl<T: Scalar> RectPackNode<T> {
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
pub struct RectPacker<T: Scalar> {
    nodes: Pool<RectPackNode<T>>,
    root: Handle<RectPackNode<T>>,
    width: T,
    height: T,
    unvisited: Vec<Handle<RectPackNode<T>>>,
}

impl<T> RectPacker<T>
where
    T: Add<Output = T> + Sub<Output = T> + Scalar + Mul<Output = T> + PartialOrd + Default + Copy,
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
            Default::default(),
            Default::default(),
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
            Default::default(),
            Default::default(),
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
