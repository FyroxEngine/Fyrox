use fxhash::FxHashMap;
use fyrox_core::{
    pool::{Handle, MultiBorrowContext, PayloadContainer, Pool, Ticket},
    reflect::prelude::*,
    visitor::prelude::*,
    Uuid,
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    fmt::Debug,
    ops::{Deref, DerefMut, Index, IndexMut},
};

/// Unique id of a node, that could be used as a reliable "index" of the node. This id is mostly
/// useful for network games.
#[derive(
    Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug, Reflect, Serialize, Deserialize,
)]
#[repr(transparent)]
#[reflect(hide_all)]
pub struct NodeId(pub Uuid);

impl Default for NodeId {
    fn default() -> Self {
        // Generate new UUID everytime, instead of zero UUID.
        Self(Uuid::new_v4())
    }
}

impl PartialEq<Uuid> for NodeId {
    fn eq(&self, other: &Uuid) -> bool {
        &self.0 == other
    }
}

impl From<Uuid> for NodeId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl Visit for NodeId {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)
    }
}

#[derive(Reflect, Debug)]
pub struct BaseNode<N>
where
    N: Debug,
{
    pub name: String,

    pub instance_id: NodeId,

    #[reflect(hidden)]
    pub parent: Handle<N>,

    #[reflect(hidden)]
    pub children: Vec<Handle<N>>,
}

pub struct BaseNodeBuilder<N>
where
    N: Debug,
{
    name: String,

    instance_id: NodeId,

    children: Vec<Handle<N>>,
}

impl<N: Debug + 'static> Default for BaseNodeBuilder<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Debug + 'static> BaseNodeBuilder<N> {
    pub fn new() -> Self {
        Self {
            name: Default::default(),
            instance_id: Default::default(),
            children: Default::default(),
        }
    }

    /// Sets desired list of children nodes.
    #[inline]
    pub fn with_children<'a, I: IntoIterator<Item = &'a Handle<N>>>(mut self, children: I) -> Self {
        for &child in children.into_iter() {
            if child.is_some() {
                self.children.push(child)
            }
        }
        self
    }

    /// Sets new instance id.
    pub fn with_instance_id(mut self, id: Uuid) -> Self {
        self.instance_id = NodeId(id);
        self
    }

    /// Sets desired name.
    #[inline]
    pub fn with_name<P: AsRef<str>>(mut self, name: P) -> Self {
        self.name = name.as_ref().to_owned();
        self
    }

    pub fn build(self) -> BaseNode<N> {
        BaseNode {
            name: self.name,
            instance_id: self.instance_id,
            parent: Default::default(),
            children: self.children,
        }
    }
}

impl<N: Debug> BaseNode<N> {
    pub fn children(&self) -> &[Handle<N>] {
        &self.children
    }

    pub fn parent(&self) -> Handle<N> {
        self.parent
    }

    fn remove_child(&mut self, child: Handle<N>) {
        if let Some(i) = self.children.iter().position(|h| *h == child) {
            self.children.remove(i);
        }
    }
}

impl<N: Debug> Default for BaseNode<N> {
    fn default() -> Self {
        Self {
            name: Default::default(),
            parent: Default::default(),
            children: Default::default(),
            instance_id: Default::default(),
        }
    }
}

impl<N: Debug> Clone for BaseNode<N> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            parent: self.parent,
            children: self.children.clone(),
            // Copying **unique** id makes no sense, so generate new one.
            instance_id: Default::default(),
        }
    }
}

impl<N> Visit for BaseNode<N>
where
    N: Debug + 'static,
{
    fn visit(&mut self, _name: &str, visitor: &mut Visitor) -> VisitResult {
        self.parent.visit("Parent", visitor)?;
        self.children.visit("Children", visitor)?;
        let _ = self.instance_id.visit("InstanceId", visitor);

        if self.name.visit("Name", visitor).is_err() {
            // Name was wrapped into `InheritableVariable` previously, so we must maintain
            // backward compatibility here.
            let mut region = visitor.enter_region("Name")?;
            let mut value = String::default();
            value.visit("Value", &mut region)?;
            self.name = value;
        }

        Ok(())
    }
}

pub trait GraphNode<N>: Sized + Reflect + Visit + Debug
where
    N: Debug,
{
    fn as_base_node_mut(&mut self) -> &mut BaseNode<N>;
    fn as_base_node(&self) -> &BaseNode<N>;
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
    pub instance_id_map: FxHashMap<NodeId, Handle<N>>,
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

/// Sub-graph is a piece of graph that was extracted from a graph. It has ownership
/// over its nodes. It is used to temporarily take ownership of a sub-graph. This could
/// be used if you making a scene editor with a command stack - once you reverted a command,
/// that created a complex nodes hierarchy (for example you loaded a model) you must store
/// all added nodes somewhere to be able put nodes back into graph when user decide to re-do
/// command. Sub-graph allows you to do this without invalidating handles to nodes.
#[derive(Debug)]
pub struct SubGraph<N> {
    /// A root node and its [ticket](Ticket)
    pub root: (Ticket<N>, N),

    /// A set of descendant nodes with their tickets.
    pub descendants: Vec<(Ticket<N>, N)>,

    /// A handle to the parent node from which the sub-graph was extracted (it it parent node of
    /// the root of this sub-graph).
    pub parent: Handle<N>,
}

impl<N, P> Graph<N, P>
where
    N: GraphNode<N>,
    P: PayloadContainer<Element = N> + Debug + Default + Visit + 'static,
{
    #[inline]
    pub fn new_empty() -> Self {
        Self {
            root: Default::default(),
            pool: Pool::new(),
            stack: Default::default(),
            instance_id_map: Default::default(),
        }
    }

    #[inline]
    pub fn add_node(&mut self, mut node: N) -> Handle<N> {
        let children = std::mem::take(&mut node.as_base_node_mut().children);

        let instance_id = node.as_base_node().instance_id;

        let handle = self.pool.spawn(node);

        self.instance_id_map.insert(instance_id, handle);

        if self.root.is_none() {
            self.root = handle;
        } else {
            self.link_nodes(handle, self.root, false);
        }

        for child in children {
            self.link_nodes(child, handle, false);
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
                let base = node.as_base_node();
                self.instance_id_map.remove(&base.instance_id);
                self.stack.extend_from_slice(base.children());
                on_removed(handle, node, &mbc);
            }
        }
    }

    #[inline]
    pub fn unlink(&mut self, node_handle: Handle<N>) {
        // Replace parent handle of a child.
        let parent_handle = std::mem::replace(
            &mut self.pool[node_handle].as_base_node_mut().parent,
            Handle::NONE,
        );

        // Remove the child from the parent's children list
        if let Some(parent) = self.pool.try_borrow_mut(parent_handle) {
            parent.as_base_node_mut().remove_child(node_handle);
        }
    }

    /// Links specified child with specified parent.
    #[inline]
    pub fn link_nodes(&mut self, child: Handle<N>, parent: Handle<N>, in_front: bool) {
        self.unlink(child);

        self.pool[child].as_base_node_mut().parent = parent;

        let children = &mut self.pool[parent].as_base_node_mut().children;
        if in_front {
            children.insert(0, child);
        } else {
            children.push(child);
        }
    }

    /// Searches for a node down the tree starting from the specified node using the specified closure. Returns a tuple
    /// with a handle and a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    pub fn find<C>(&self, root_node: Handle<N>, mut cmp: C) -> Option<(Handle<N>, &N)>
    where
        C: FnMut(&N) -> bool,
    {
        fn find<'a, N, P, C>(
            pool: &'a Pool<N, P>,
            root_node: Handle<N>,
            cmp: &mut C,
        ) -> Option<(Handle<N>, &'a N)>
        where
            C: FnMut(&N) -> bool,
            N: GraphNode<N>,
            P: PayloadContainer<Element = N> + Debug + 'static,
        {
            pool.try_borrow(root_node).and_then(|root| {
                if cmp(root) {
                    Some((root_node, root))
                } else {
                    root.as_base_node()
                        .children()
                        .iter()
                        .find_map(|c| find(pool, *c, cmp))
                }
            })
        }

        find(self, root_node, &mut cmp)
    }

    /// Searches for a node down the tree starting from the specified node using the specified closure. Returns a tuple
    /// with a handle and a reference to the mapped value. If nothing is found, it returns [`None`].
    #[inline]
    pub fn find_map<T: ?Sized>(
        &self,
        root_node: Handle<N>,
        mut cmp: impl FnMut(&N) -> Option<&T>,
    ) -> Option<(Handle<N>, &T)> {
        fn find_map<'a, T, C, N, P>(
            pool: &'a Pool<N, P>,
            root_node: Handle<N>,
            cmp: &mut C,
        ) -> Option<(Handle<N>, &'a T)>
        where
            C: FnMut(&'a N) -> Option<&'a T>,
            N: GraphNode<N>,
            P: PayloadContainer<Element = N> + Debug + 'static,
            T: ?Sized,
        {
            pool.try_borrow(root_node).and_then(|root| {
                if let Some(x) = cmp(root) {
                    Some((root_node, x))
                } else {
                    root.as_base_node()
                        .children()
                        .iter()
                        .find_map(|c| find_map(pool, *c, cmp))
                }
            })
        }

        find_map(self, root_node, &mut cmp)
    }

    /// Searches for a node up the tree starting from the specified node using the specified closure. Returns a tuple
    /// with a handle and a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    pub fn find_up(
        &self,
        root_node: Handle<N>,
        mut cmp: impl FnMut(&N) -> bool,
    ) -> Option<(Handle<N>, &N)> {
        let mut handle = root_node;
        while let Some(node) = self.pool.try_borrow(handle) {
            if cmp(node) {
                return Some((handle, node));
            }
            handle = node.as_base_node().parent();
        }
        None
    }

    /// Searches for a node up the tree starting from the specified node using the specified closure. Returns a tuple
    /// with a handle and a reference to the mapped value. If nothing is found, it returns [`None`].
    #[inline]
    pub fn find_up_map<T: ?Sized>(
        &self,
        root_node: Handle<N>,
        mut cmp: impl FnMut(&N) -> Option<&T>,
    ) -> Option<(Handle<N>, &T)> {
        let mut handle = root_node;
        while let Some(node) = self.pool.try_borrow(handle) {
            if let Some(x) = cmp(node) {
                return Some((handle, x));
            }
            handle = node.as_base_node().parent();
        }
        None
    }

    /// Searches node using specified compare closure starting from root. Returns a tuple with a handle and
    /// a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    pub fn find_from_root(&self, cmp: impl FnMut(&N) -> bool) -> Option<(Handle<N>, &N)> {
        self.find(self.root, cmp)
    }

    /// Searches for a node with the specified name down the tree starting from the specified node. Returns a tuple with
    /// a handle and a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    pub fn find_by_name(&self, root_node: Handle<N>, name: &str) -> Option<(Handle<N>, &N)> {
        self.find(root_node, |node| node.as_base_node().name == name)
    }

    /// Searches for a node with the specified name up the tree starting from the specified node. Returns a tuple with a
    /// handle and a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    pub fn find_up_by_name(&self, root_node: Handle<N>, name: &str) -> Option<(Handle<N>, &N)> {
        self.find_up(root_node, |node| node.as_base_node().name == name)
    }

    /// Searches for a node with the specified name down the tree starting from the graph root. Returns a tuple with a
    /// handle and a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    pub fn find_by_name_from_root(&self, name: &str) -> Option<(Handle<N>, &N)> {
        self.find_by_name(self.root, name)
    }

    // Puts node at the end of children list of a parent node.
    #[inline]
    fn move_child(&mut self, handle: Handle<N>, in_front: bool) {
        let Some(node) = self.pool.try_borrow(handle) else {
            return;
        };
        let Some(parent) = self.pool.try_borrow_mut(node.as_base_node().parent()) else {
            return;
        };
        let parent_base_node = parent.as_base_node_mut();
        parent_base_node.remove_child(handle);
        if in_front {
            parent_base_node.children.insert(0, handle)
        } else {
            parent_base_node.children.push(handle)
        };
    }

    #[inline]
    pub fn make_topmost(&mut self, handle: Handle<N>) {
        self.move_child(handle, false)
    }

    #[inline]
    pub fn make_lowermost(&mut self, handle: Handle<N>) {
        self.move_child(handle, true)
    }

    #[inline]
    pub fn sort_children<F>(&mut self, handle: Handle<N>, mut cmp: F)
    where
        F: FnMut(&N, &N) -> Ordering,
    {
        let mbc = self.pool.begin_multi_borrow();
        if let Ok(mut parent) = mbc.try_get_mut(handle) {
            parent
                .as_base_node_mut()
                .children
                .sort_by(|a, b| cmp(&mbc.get(*a), &mbc.get(*b)));
        };
    }

    /// Extracts node from graph and reserves its handle. It is used to temporarily take
    /// ownership over node, and then put node back using given ticket. Extracted node is
    /// detached from its parent!
    #[inline]
    pub fn take_reserve_node(&mut self, handle: Handle<N>) -> (Ticket<N>, N) {
        self.unlink(handle);
        let (ticket, node) = self.pool.take_reserve(handle);
        self.instance_id_map
            .remove(&node.as_base_node().instance_id);
        (ticket, node)
    }

    /// Puts node back by given ticket. Attaches back to root node of graph.
    #[inline]
    pub fn put_node_back(&mut self, ticket: Ticket<N>, node: N) -> Handle<N> {
        let instance_id = node.as_base_node().instance_id;
        let handle = self.pool.put_back(ticket, node);
        self.instance_id_map.insert(instance_id, handle);
        self.link_nodes(handle, self.root, false);
        handle
    }

    /// Makes node handle vacant again.
    #[inline]
    pub fn forget_node_ticket(&mut self, ticket: Ticket<N>, node: N) -> N {
        self.pool.forget_ticket(ticket);
        node
    }

    /// Extracts sub-graph starting from a given node. All handles to extracted nodes
    /// becomes reserved and will be marked as "occupied", an attempt to borrow a node
    /// at such handle will result in panic!. Please note that root node will be
    /// detached from its parent!
    #[inline]
    pub fn take_reserve_sub_graph(&mut self, root: Handle<N>) -> SubGraph<N> {
        // Take out descendants first.
        let mut descendants = Vec::new();
        let root_ref = self.pool[root].as_base_node();
        let mut stack = root_ref.children().to_vec();
        let parent = root_ref.parent();
        while let Some(handle) = stack.pop() {
            stack.extend_from_slice(self.pool[handle].as_base_node().children());
            descendants.push(self.pool.take_reserve(handle));
        }

        SubGraph {
            // Root must be extracted with detachment from its parent (if any).
            root: self.take_reserve(root),
            descendants,
            parent,
        }
    }

    /// Puts previously extracted sub-graph into graph. Handles to nodes will become valid
    /// again. After that you probably want to re-link returned handle with its previous
    /// parent.
    #[inline]
    pub fn put_sub_graph_back(&mut self, sub_graph: SubGraph<N>) -> Handle<N> {
        for (ticket, node) in sub_graph.descendants {
            self.pool.put_back(ticket, node);
        }

        let (ticket, node) = sub_graph.root;
        let root_handle = self.pool.put_back(ticket, node);

        self.link_nodes(root_handle, sub_graph.parent, false);

        root_handle
    }

    /// Forgets the entire sub-graph making handles to nodes invalid.
    #[inline]
    pub fn forget_sub_graph(&mut self, sub_graph: SubGraph<N>) {
        for (ticket, _) in sub_graph.descendants {
            self.pool.forget_ticket(ticket);
        }
        let (ticket, _) = sub_graph.root;
        self.pool.forget_ticket(ticket);
    }

    /// Returns a handle of the node that has the given id.
    pub fn id_to_node_handle(&self, id: NodeId) -> Option<&Handle<N>> {
        self.instance_id_map.get(&id)
    }

    /// Tries to borrow a node by its id.
    pub fn node_by_id(&self, id: NodeId) -> Option<(Handle<N>, &N)> {
        self.instance_id_map
            .get(&id)
            .and_then(|h| self.pool.try_borrow(*h).map(|n| (*h, n)))
    }

    /// Tries to borrow a node by its id.
    pub fn node_by_id_mut(&mut self, id: NodeId) -> Option<(Handle<N>, &mut N)> {
        self.instance_id_map
            .get(&id)
            .and_then(|h| self.pool.try_borrow_mut(*h).map(|n| (*h, n)))
    }

    /// Sets new root of the graph and attaches the old root to the new root. Old root becomes a child
    /// node of the new root.
    pub fn change_root(&mut self, root: N) {
        let prev_root = self.root;
        self.root = Handle::NONE;
        let handle = self.add_node(root);
        assert_eq!(self.root, handle);
        self.link_nodes(prev_root, handle, false);
    }

    /// Makes a node in the graph the new root of the graph. All children nodes of the previous root will
    /// become children nodes of the new root. Old root will become a child node of the new root.
    pub fn change_root_inplace(&mut self, new_root: Handle<N>) {
        let prev_root = self.root;
        self.unlink(new_root);
        let prev_root_children = self
            .pool
            .try_borrow(prev_root)
            .map(|r| r.as_base_node().children.clone())
            .unwrap_or_default();
        for child in prev_root_children {
            self.link_nodes(child, new_root, false);
        }
        if prev_root.is_some() {
            self.link_nodes(prev_root, new_root, false);
        }
        self.root = new_root;
    }

    #[inline]
    pub fn visit(
        &mut self,
        root_name: &str,
        pool_name: &str,
        visitor: &mut Visitor,
    ) -> VisitResult {
        self.root.visit(root_name, visitor)?;
        self.pool.visit(pool_name, visitor)?;
        Ok(())
    }
}

impl<N, P> Index<Handle<N>> for Graph<N, P>
where
    N: GraphNode<N>,
    P: PayloadContainer<Element = N> + Debug + 'static,
{
    type Output = N;

    #[inline]
    fn index(&self, index: Handle<N>) -> &Self::Output {
        &self.pool[index]
    }
}

impl<N, P> IndexMut<Handle<N>> for Graph<N, P>
where
    N: GraphNode<N>,
    P: PayloadContainer<Element = N> + Debug + 'static,
{
    #[inline]
    fn index_mut(&mut self, index: Handle<N>) -> &mut Self::Output {
        &mut self.pool[index]
    }
}

#[cfg(test)]
mod test {
    use crate::{BaseNode, BaseNodeBuilder, Graph, GraphNode};
    use fyrox_core::{pool::Handle, reflect::prelude::*, visitor::prelude::*};

    #[derive(Debug, Reflect, Clone, Visit, Default)]
    struct MyNode {
        base: BaseNode<MyNode>,
    }

    impl GraphNode<MyNode> for MyNode {
        fn as_base_node_mut(&mut self) -> &mut BaseNode<MyNode> {
            &mut self.base
        }

        fn as_base_node(&self) -> &BaseNode<MyNode> {
            &self.base
        }
    }

    struct MyNodeBuilder {
        base_builder: BaseNodeBuilder<MyNode>,
    }

    impl MyNodeBuilder {
        fn new(base_builder: BaseNodeBuilder<MyNode>) -> Self {
            Self { base_builder }
        }

        pub fn build(self, graph: &mut Graph<MyNode>) -> Handle<MyNode> {
            graph.add_node(MyNode {
                base: self.base_builder.build(),
            })
        }
    }

    #[test]
    fn graph_init_test() {
        let graph = Graph::<MyNode>::new_empty();
        assert_eq!(graph.root, Handle::NONE);
        assert_eq!(graph.pool.alive_count(), 0);
    }

    #[test]
    fn graph_node_test() {
        let mut graph = Graph::<MyNode>::new_empty();
        graph.add_node(MyNode::default());
        graph.add_node(MyNode::default());
        graph.add_node(MyNode::default());
        assert_eq!(graph.pool.alive_count(), 3);
    }

    #[test]
    fn test_graph_structure() {
        let mut graph = Graph::new_empty();

        let root = graph.add_node(MyNode::default());

        // Root_
        //      |_A_
        //          |_B
        //          |_C_
        //             |_D
        let b;
        let c;
        let d;
        let a = MyNodeBuilder::new(BaseNodeBuilder::new().with_name("A").with_children(&[
            {
                b = MyNodeBuilder::new(BaseNodeBuilder::new().with_name("B")).build(&mut graph);
                b
            },
            {
                c = MyNodeBuilder::new(BaseNodeBuilder::new().with_name("C").with_children(&[{
                    d = MyNodeBuilder::new(BaseNodeBuilder::new().with_name("D")).build(&mut graph);
                    d
                }]))
                .build(&mut graph);
                c
            },
        ]))
        .build(&mut graph);

        assert_eq!(graph.root, root);
        assert_eq!(graph[root].as_base_node().children, vec![a]);
        assert_eq!(graph[a].as_base_node().children, vec![b, c]);
        assert!(graph[b].as_base_node().children.is_empty());
        assert_eq!(graph[c].as_base_node().children, vec![d]);
    }

    #[test]
    fn test_graph_search() {
        let mut graph = Graph::new_empty();

        let _root = graph.add_node(MyNode::default());

        // Root_
        //      |_A_
        //          |_B
        //          |_C_
        //             |_D
        let b;
        let c;
        let d;
        let a = MyNodeBuilder::new(BaseNodeBuilder::new().with_name("A").with_children(&[
            {
                b = MyNodeBuilder::new(BaseNodeBuilder::new().with_name("B")).build(&mut graph);
                b
            },
            {
                c = MyNodeBuilder::new(BaseNodeBuilder::new().with_name("C").with_children(&[{
                    d = MyNodeBuilder::new(BaseNodeBuilder::new().with_name("D")).build(&mut graph);
                    d
                }]))
                .build(&mut graph);
                c
            },
        ]))
        .build(&mut graph);

        // Test down search.
        assert!(graph.find_by_name(a, "X").is_none());
        assert_eq!(graph.find_by_name(a, "A").unwrap().0, a);
        assert_eq!(graph.find_by_name(a, "D").unwrap().0, d);

        let result = graph
            .find_map(a, |n| {
                if n.as_base_node().name == "D" {
                    Some("D")
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(result.0, d);
        assert_eq!(result.1, "D");

        // Test up search.
        assert!(graph.find_up_by_name(d, "X").is_none());
        assert_eq!(graph.find_up_by_name(d, "D").unwrap().0, d);
        assert_eq!(graph.find_up_by_name(d, "A").unwrap().0, a);

        let result = graph
            .find_up_map(d, |n| {
                if n.as_base_node().name == "A" {
                    Some("A")
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(result.0, a);
        assert_eq!(result.1, "A");
    }

    #[test]
    fn test_change_root() {
        let mut graph = Graph::new_empty();

        let root = graph.add_node(MyNode::default());

        // Root_
        //      |_A_
        //          |_B
        //          |_C_
        //             |_D
        let b;
        let c;
        let d;
        let a = MyNodeBuilder::new(BaseNodeBuilder::new().with_children(&[
            {
                b = MyNodeBuilder::new(BaseNodeBuilder::new()).build(&mut graph);
                b
            },
            {
                c = MyNodeBuilder::new(BaseNodeBuilder::new().with_children(&[{
                    d = MyNodeBuilder::new(BaseNodeBuilder::new()).build(&mut graph);
                    d
                }]))
                .build(&mut graph);
                c
            },
        ]))
        .build(&mut graph);

        dbg!(root, a, b, c, d);

        graph.change_root_inplace(c);

        // C_
        //      |_D
        //      |_A_
        //          |_B
        //      |_Root
        assert_eq!(graph.root, c);

        assert_eq!(graph[graph.root].as_base_node().parent, Handle::NONE);
        assert_eq!(graph[graph.root].as_base_node().children.len(), 3);

        assert_eq!(graph[graph.root].as_base_node().children[0], d);
        assert_eq!(graph[d].as_base_node().parent, graph.root);
        assert!(graph[d].as_base_node().children.is_empty());

        assert_eq!(graph[graph.root].as_base_node().children[1], a);
        assert_eq!(graph[a].as_base_node().parent, graph.root);

        assert_eq!(graph[graph.root].as_base_node().children[2], root);
        assert_eq!(graph[root].as_base_node().parent, graph.root);

        assert_eq!(graph[a].as_base_node().children.len(), 1);
        assert_eq!(graph[a].as_base_node().children[0], b);
        assert_eq!(graph[b].as_base_node().parent, a);

        assert!(graph[b].as_base_node().children.is_empty());
    }
}
