//! Rectangle packer is used to pack set of smaller rectangles into one big, it
//! used in texture atlas packer.

use crate::{
    math::Rect,
    pool::{Handle, Pool},
};
use std::ops::{Add, Mul, Sub};

struct RectPackNode<T> {
    filled: bool,
    split: bool,
    bounds: Rect<T>,
    left: Handle<RectPackNode<T>>,
    right: Handle<RectPackNode<T>>,
}

impl<T> RectPackNode<T> {
    fn new(bounds: Rect<T>) -> RectPackNode<T> {
        RectPackNode {
            bounds,
            filled: false,
            split: false,
            left: Handle::NONE,
            right: Handle::NONE,
        }
    }
}

//! See module docs.
pub struct RectPacker<T> {
    nodes: Pool<RectPackNode<T>>,
    root: Handle<RectPackNode<T>>,
}

impl<T> RectPacker<T>
where
    T: Add<Output = T> + Sub<Output = T> + Copy + Clone + Default + PartialOrd + Mul<Output = T>,
{
    /// Creates new instance of rectangle packer with given bounds.
    ///
    /// # How to choose bounds
    ///
    /// If you have a set of rectangles and you need to calculate average side length of a square,
    /// then calculate total area of your triangles by sum of width*height and then take square
    /// root out of area. You'll get side length of a square which can be used as width and height
    /// parameters.
    pub fn new(w: T, h: T) -> RectPacker<T> {
        let mut nodes = Pool::new();
        let root = nodes.spawn(RectPackNode::new(Rect::new(
            Default::default(),
            Default::default(),
            w,
            h,
        )));
        RectPacker { nodes, root }
    }

    /// Tries to find free place to put rectangle with given size. Returns None if there insufficient
    /// space.
    pub fn find_free(&mut self, w: T, h: T) -> Option<Rect<T>> {
        let mut unvisited = vec![self.root];
        while let Some(node_handle) = unvisited.pop() {
            let left_bounds;
            let right_bounds;

            let node = self.nodes.borrow_mut(node_handle);
            if node.split {
                unvisited.push(node.right);
                unvisited.push(node.left);
                continue;
            } else {
                if node.filled || node.bounds.w < w || node.bounds.h < h {
                    continue;
                }

                if node.bounds.w == w && node.bounds.h == h {
                    node.filled = true;
                    return Some(node.bounds);
                }

                // Split and continue
                node.split = true;
                if node.bounds.w - w > node.bounds.h - h {
                    left_bounds = Rect::new(node.bounds.x, node.bounds.y, w, node.bounds.h);
                    right_bounds = Rect::new(
                        node.bounds.x + w,
                        node.bounds.y,
                        node.bounds.w - w,
                        node.bounds.h,
                    );
                } else {
                    left_bounds = Rect::new(node.bounds.x, node.bounds.y, node.bounds.w, h);
                    right_bounds = Rect::new(
                        node.bounds.x,
                        node.bounds.y + h,
                        node.bounds.w,
                        node.bounds.h - h,
                    );
                }
            }

            let left = self.nodes.spawn(RectPackNode::new(left_bounds));
            self.nodes.borrow_mut(node_handle).left = left;

            let right = self.nodes.spawn(RectPackNode::new(right_bounds));
            self.nodes.borrow_mut(node_handle).right = right;

            unvisited.push(left);
        }

        None
    }
}
