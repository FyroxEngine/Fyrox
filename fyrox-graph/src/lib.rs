use fyrox_core::{
    pool::{Handle, MultiBorrowContext, PayloadContainer, Pool},
    reflect::prelude::*,
    visitor::prelude::*,
};
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

#[derive(Reflect, Debug)]
pub struct HierarchicalData<N>
where
    N: Debug,
{
    pub parent: Handle<N>,
    pub children: Vec<Handle<N>>,
}

impl<N: Debug> Default for HierarchicalData<N> {
    fn default() -> Self {
        Self {
            parent: Default::default(),
            children: Default::default(),
        }
    }
}

impl<N: Debug> Clone for HierarchicalData<N> {
    fn clone(&self) -> Self {
        Self {
            parent: self.parent,
            children: self.children.clone(),
        }
    }
}

impl<N> Visit for HierarchicalData<N>
where
    N: Debug + 'static,
{
    fn visit(&mut self, _name: &str, visitor: &mut Visitor) -> VisitResult {
        self.parent.visit("Parent", visitor)?;
        self.children.visit("Children", visitor)?;
        Ok(())
    }
}

pub trait GraphNode<N>: Sized + Reflect + Visit + Debug
where
    N: Debug,
{
    fn children(&self) -> &[Handle<N>];
    fn parent(&self) -> Handle<N>;
    fn hierarchical_data_mut(&mut self) -> &mut HierarchicalData<N>;
}

#[derive(Reflect, Debug)]
pub struct Graph<N, P = Option<N>>
where
    N: GraphNode<N>,
    P: PayloadContainer<Element = N> + Debug + 'static,
{
    #[reflect(hidden)]
    pub root: Handle<N>,
    pub pool: Pool<N, P>,
    pub stack: Vec<Handle<N>>,
}

impl<N, P> Visit for Graph<N, P>
where
    N: GraphNode<N>,
    P: PayloadContainer<Element = N> + Debug + Visit + Default + 'static,
{
    fn visit(&mut self, _name: &str, visitor: &mut Visitor) -> VisitResult {
        self.root.visit("Root", visitor)?;
        self.pool.visit("Pool", visitor)?;
        Ok(())
    }
}

impl<N, P> Deref for Graph<N, P>
where
    N: GraphNode<N>,
    P: PayloadContainer<Element = N> + Debug + 'static,
{
    type Target = Pool<N, P>;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

impl<N, P> DerefMut for Graph<N, P>
where
    N: GraphNode<N>,
    P: PayloadContainer<Element = N> + Debug + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.pool
    }
}

impl<N, P> Graph<N, P>
where
    N: GraphNode<N>,
    P: PayloadContainer<Element = N> + Debug + 'static,
{
    pub fn new() -> Self {
        Self {
            root: Default::default(),
            pool: Pool::new(),
            stack: Default::default(),
        }
    }

    pub fn add_node(&mut self, mut node: N) -> Handle<N> {
        let children = std::mem::take(&mut node.hierarchical_data_mut().children);

        let handle = self.pool.spawn(node);

        if self.root.is_none() {
            self.root = handle;
        } else {
            self.link_nodes(handle, self.root);
        }

        for child in children {
            self.link_nodes(child, handle);
        }

        handle
    }

    #[inline]
    pub fn remove_node<'a, F>(&'a mut self, node_handle: Handle<N>, mut on_removed: F)
    where
        F: FnMut(Handle<N>, N, &MultiBorrowContext<'a, N, P>),
    {
        self.unlink(node_handle);

        let mbc = self.pool.begin_multi_borrow();
        self.stack.clear();
        self.stack.push(node_handle);
        while let Some(handle) = self.stack.pop() {
            if let Ok(node) = mbc.free(handle) {
                self.stack.extend_from_slice(node.children());
                on_removed(handle, node, &mbc);
            }
        }
    }

    #[inline]
    pub fn unlink(&mut self, node_handle: Handle<N>) {
        // Replace parent handle of a child.
        let parent_handle = std::mem::replace(
            &mut self.pool[node_handle].hierarchical_data_mut().parent,
            Handle::NONE,
        );

        // Remove the child from the parent's children list
        if let Some(parent) = self.pool.try_borrow_mut(parent_handle) {
            let hierarchical_data = parent.hierarchical_data_mut();
            if let Some(i) = hierarchical_data
                .children
                .iter()
                .position(|h| *h == node_handle)
            {
                hierarchical_data.children.remove(i);
            }
        }
    }

    /// Links specified child with specified parent.
    #[inline]
    pub fn link_nodes(&mut self, child: Handle<N>, parent: Handle<N>) {
        self.unlink(child);
        self.pool[child].hierarchical_data_mut().parent = parent;
        self.pool[parent]
            .hierarchical_data_mut()
            .children
            .push(child);
    }

    /// Searches for a node down the tree starting from the specified node using the specified closure. Returns a tuple
    /// with a handle and a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    pub fn find<C>(&self, root_node: Handle<N>, cmp: &mut C) -> Option<(Handle<N>, &N)>
    where
        C: FnMut(&N) -> bool,
    {
        self.pool.try_borrow(root_node).and_then(|root| {
            if cmp(root) {
                Some((root_node, root))
            } else {
                root.children().iter().find_map(|c| self.find(*c, cmp))
            }
        })
    }

    /// Searches for a node down the tree starting from the specified node using the specified closure. Returns a tuple
    /// with a handle and a reference to the mapped value. If nothing is found, it returns [`None`].
    #[inline]
    pub fn find_map<C, T>(&self, root_node: Handle<N>, cmp: &mut C) -> Option<(Handle<N>, &T)>
    where
        C: FnMut(&N) -> Option<&T>,
        T: ?Sized,
    {
        self.pool.try_borrow(root_node).and_then(|root| {
            if let Some(x) = cmp(root) {
                Some((root_node, x))
            } else {
                root.children().iter().find_map(|c| self.find_map(*c, cmp))
            }
        })
    }

    /// Searches for a node up the tree starting from the specified node using the specified closure. Returns a tuple
    /// with a handle and a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    pub fn find_up<C>(&self, root_node: Handle<N>, cmp: &mut C) -> Option<(Handle<N>, &N)>
    where
        C: FnMut(&N) -> bool,
    {
        let mut handle = root_node;
        while let Some(node) = self.pool.try_borrow(handle) {
            if cmp(node) {
                return Some((handle, node));
            }
            handle = node.parent();
        }
        None
    }

    /// Searches for a node up the tree starting from the specified node using the specified closure. Returns a tuple
    /// with a handle and a reference to the mapped value. If nothing is found, it returns [`None`].
    #[inline]
    pub fn find_up_map<C, T>(&self, root_node: Handle<N>, cmp: &mut C) -> Option<(Handle<N>, &T)>
    where
        C: FnMut(&N) -> Option<&T>,
        T: ?Sized,
    {
        let mut handle = root_node;
        while let Some(node) = self.pool.try_borrow(handle) {
            if let Some(x) = cmp(node) {
                return Some((handle, x));
            }
            handle = node.parent();
        }
        None
    }
}
