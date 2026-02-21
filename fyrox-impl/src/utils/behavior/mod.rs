// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

#![warn(missing_docs)]

//! Everything related to AI behavior and behavior trees.
//!
//! Behavior trees are simple but very powerful mechanism to implement artificial intelligence for
//! games. The main concept is in its name. Tree is a set of connected nodes, where each node could
//! have single parent and zero or more children nodes. Execution path of the tree is defined by the
//! actions of the nodes. Behavior tree has a set of hard coded nodes as well as leaf nodes with
//! user-defined logic. Hard coded nodes are: Sequence, Selector, Leaf. Leaf is special - it has
//! custom method `tick` that can contain any logic you want.
//!
//! For more info see:
//! - [Wikipedia article](https://en.wikipedia.org/wiki/Behavior_tree_(artificial_intelligence,_robotics_and_control))
//! - [Gamasutra](https://www.gamasutra.com/blogs/ChrisSimpson/20140717/221339/Behavior_trees_for_AI_How_they_work.php)

use crate::plugin::error::GameError;
use crate::{
    core::{
        pool::{Handle, Pool},
        visitor::prelude::*,
    },
    utils::behavior::{
        composite::{CompositeNode, CompositeNodeKind},
        inverter::Inverter,
        leaf::LeafNode,
    },
};
use fyrox_core::pool::PoolError;
use std::{
    fmt::Debug,
    ops::{Index, IndexMut},
};

pub mod composite;
pub mod inverter;
pub mod leaf;

/// Status of execution of behavior tree node.
pub enum Status {
    /// Action was successful.
    Success,
    /// Failed to perform an action.
    Failure,
    /// Need another iteration to perform an action.
    Running,
}

/// Base trait for all behaviors. This trait has auto-impl.
pub trait BaseBehavior: Visit + Default + PartialEq + Debug + Clone + 'static {}
impl<T: Visit + Default + PartialEq + Debug + Clone + 'static> BaseBehavior for T {}

/// A trait for user-defined actions for behavior tree.
pub trait Behavior<'a>: BaseBehavior {
    /// A context in which the behavior will be performed.
    type Context;

    /// A function that will be called each frame depending on
    /// the current execution path of the behavior tree it belongs
    /// to.
    fn tick(&mut self, context: &mut Self::Context) -> Result<Status, GameError>;
}

/// Root node of the tree.
#[derive(Debug, PartialEq, Visit, Eq, Clone)]
pub struct RootNode<B>
where
    B: BaseBehavior,
{
    child: Handle<BehaviorNode<B>>,
}

impl<B> Default for RootNode<B>
where
    B: BaseBehavior,
{
    fn default() -> Self {
        Self {
            child: Default::default(),
        }
    }
}

/// Possible variations of behavior nodes.
#[derive(Debug, PartialEq, Visit, Eq, Clone, Default)]
pub enum BehaviorNode<B>
where
    B: BaseBehavior,
{
    #[doc(hidden)]
    #[default]
    Unknown,
    /// Root node of the tree.
    Root(RootNode<B>),
    /// Composite (sequence or selector) node of the tree.
    Composite(CompositeNode<B>),
    /// A node with custom logic.
    Leaf(LeafNode<B>),
    /// A node, that inverts its child state ([`Status::Failure`] becomes [`Status::Success`] and vice versa, [`Status::Running`] remains
    /// unchanged)
    Inverter(Inverter<B>),
}

/// See module docs.
#[derive(Debug, PartialEq, Visit, Clone)]
pub struct BehaviorTree<B>
where
    B: BaseBehavior,
{
    nodes: Pool<BehaviorNode<B>>,
    root: Handle<BehaviorNode<B>>,
}

impl<B> Default for BehaviorTree<B>
where
    B: BaseBehavior,
{
    fn default() -> Self {
        Self {
            nodes: Default::default(),
            root: Default::default(),
        }
    }
}

impl<B> BehaviorTree<B>
where
    B: BaseBehavior,
{
    /// Creates new behavior tree with single root node.
    pub fn new() -> Self {
        let mut nodes = Pool::new();
        let root = nodes.spawn(BehaviorNode::Root(RootNode {
            child: Default::default(),
        }));
        Self { nodes, root }
    }

    /// Adds a node to the tree, returns its handle.
    pub fn add_node(&mut self, node: BehaviorNode<B>) -> Handle<BehaviorNode<B>> {
        self.nodes.spawn(node)
    }

    /// Sets entry nodes of the tree. Execution will start from this node.
    pub fn set_entry_node(&mut self, entry: Handle<BehaviorNode<B>>) {
        if let BehaviorNode::Root(root) = &mut self.nodes[self.root] {
            root.child = entry;
        } else {
            unreachable!("must be root")
        }
    }

    fn tick_recursive<'a, Ctx>(
        &self,
        handle: Handle<BehaviorNode<B>>,
        context: &mut Ctx,
    ) -> Result<Status, GameError>
    where
        B: Behavior<'a, Context = Ctx>,
    {
        match self.nodes[handle] {
            BehaviorNode::Root(ref root) => {
                if root.child.is_some() {
                    self.tick_recursive(root.child, context)
                } else {
                    Ok(Status::Success)
                }
            }
            BehaviorNode::Composite(ref composite) => match composite.kind {
                CompositeNodeKind::Sequence => {
                    let mut all_succeeded = true;
                    for child in composite.children.iter() {
                        match self.tick_recursive(*child, context)? {
                            Status::Failure => {
                                all_succeeded = false;
                                break;
                            }
                            Status::Running => {
                                return Ok(Status::Running);
                            }
                            _ => (),
                        }
                    }
                    if all_succeeded {
                        Ok(Status::Success)
                    } else {
                        Ok(Status::Failure)
                    }
                }
                CompositeNodeKind::Selector => {
                    for child in composite.children.iter() {
                        match self.tick_recursive(*child, context)? {
                            Status::Success => return Ok(Status::Success),
                            Status::Running => return Ok(Status::Running),
                            _ => (),
                        }
                    }
                    Ok(Status::Failure)
                }
            },
            BehaviorNode::Leaf(ref leaf) => {
                leaf.behavior.as_ref().unwrap().borrow_mut().tick(context)
            }
            BehaviorNode::Inverter(ref inverter) => {
                match self.tick_recursive(inverter.child, context)? {
                    Status::Success => Ok(Status::Failure),
                    Status::Failure => Ok(Status::Success),
                    Status::Running => Ok(Status::Running),
                }
            }
            BehaviorNode::Unknown => {
                unreachable!()
            }
        }
    }

    /// Tries to get a shared reference to a node by given handle.
    pub fn node(&self, handle: Handle<BehaviorNode<B>>) -> Result<&BehaviorNode<B>, PoolError> {
        self.nodes.try_borrow(handle)
    }

    /// Tries to get a mutable reference to a node by given handle.
    pub fn node_mut(
        &mut self,
        handle: Handle<BehaviorNode<B>>,
    ) -> Result<&mut BehaviorNode<B>, PoolError> {
        self.nodes.try_borrow_mut(handle)
    }

    /// Performs a single update tick with given context.
    pub fn tick<'a, Ctx>(&self, context: &mut Ctx) -> Result<Status, GameError>
    where
        B: Behavior<'a, Context = Ctx>,
    {
        self.tick_recursive(self.root, context)
    }
}

impl<B: BaseBehavior> Index<Handle<BehaviorNode<B>>> for BehaviorTree<B> {
    type Output = BehaviorNode<B>;

    fn index(&self, index: Handle<BehaviorNode<B>>) -> &Self::Output {
        &self.nodes[index]
    }
}

impl<B: BaseBehavior> IndexMut<Handle<BehaviorNode<B>>> for BehaviorTree<B> {
    fn index_mut(&mut self, index: Handle<BehaviorNode<B>>) -> &mut Self::Output {
        &mut self.nodes[index]
    }
}

/// Creates a new sequence.
pub fn sequence<B, const N: usize>(
    children: [Handle<BehaviorNode<B>>; N],
    tree: &mut BehaviorTree<B>,
) -> Handle<BehaviorNode<B>>
where
    B: BaseBehavior,
{
    CompositeNode::new_sequence(children.to_vec()).add_to(tree)
}

/// Creates a new selector.
pub fn selector<B, const N: usize>(
    children: [Handle<BehaviorNode<B>>; N],
    tree: &mut BehaviorTree<B>,
) -> Handle<BehaviorNode<B>>
where
    B: BaseBehavior,
{
    CompositeNode::new_selector(children.to_vec()).add_to(tree)
}

/// Creates a new leaf.
pub fn leaf<B>(behavior: B, tree: &mut BehaviorTree<B>) -> Handle<BehaviorNode<B>>
where
    B: BaseBehavior,
{
    LeafNode::new(behavior).add_to(tree)
}

/// Creates a new inverter.
pub fn inverter<B>(
    child: Handle<BehaviorNode<B>>,
    tree: &mut BehaviorTree<B>,
) -> Handle<BehaviorNode<B>>
where
    B: BaseBehavior,
{
    Inverter::new(child).add_to(tree)
}

#[macro_export]
macro_rules! dispatch_behavior_variants {
    ($type_name:ident, $context:ty, $($variants:ident),*) => {
        impl<'a> $crate::utils::behavior::Behavior<'a> for $type_name {
            type Context = $context;

            fn tick(&mut self, context: &mut Self::Context)
                -> Result<$crate::utils::behavior::Status, $crate::plugin::error::GameError>
            {
                match self {
                    $(
                        $type_name::$variants(v) => v.tick(context)
                    ),*
                }
            }
        }
    };
}

#[cfg(test)]
mod test {
    use crate::{
        core::{futures::executor::block_on, visitor::prelude::*},
        plugin::error::GameError,
        utils::behavior::{leaf, sequence, Behavior, BehaviorTree, Status},
    };
    use std::{env, fs::File, io::Write, path::PathBuf};

    #[derive(Debug, PartialEq, Default, Visit, Clone)]
    struct WalkAction;

    impl Behavior<'_> for WalkAction {
        type Context = Environment;

        fn tick(&mut self, context: &mut Self::Context) -> Result<Status, GameError> {
            if context.distance_to_door <= 0.0 {
                Ok(Status::Success)
            } else {
                context.distance_to_door -= 0.1;
                println!(
                    "Approaching door, remaining distance: {}",
                    context.distance_to_door
                );
                Ok(Status::Running)
            }
        }
    }

    #[derive(Debug, PartialEq, Default, Visit, Clone)]
    struct OpenDoorAction;

    impl Behavior<'_> for OpenDoorAction {
        type Context = Environment;

        fn tick(&mut self, context: &mut Self::Context) -> Result<Status, GameError> {
            if !context.door_opened {
                context.door_opened = true;
                println!("Door was opened!");
            }
            Ok(Status::Success)
        }
    }

    #[derive(Debug, PartialEq, Default, Visit, Clone)]
    struct StepThroughAction;

    impl Behavior<'_> for StepThroughAction {
        type Context = Environment;

        fn tick(&mut self, context: &mut Self::Context) -> Result<Status, GameError> {
            if context.distance_to_door < -1.0 {
                Ok(Status::Success)
            } else {
                context.distance_to_door -= 0.1;
                println!(
                    "Stepping through doorway, remaining distance: {}",
                    -1.0 - context.distance_to_door
                );
                Ok(Status::Running)
            }
        }
    }

    #[derive(Debug, PartialEq, Default, Visit, Clone)]
    struct CloseDoorAction;

    impl Behavior<'_> for CloseDoorAction {
        type Context = Environment;

        fn tick(&mut self, context: &mut Self::Context) -> Result<Status, GameError> {
            if context.door_opened {
                context.door_opened = false;
                context.done = true;
                println!("Door was closed");
            }
            Ok(Status::Success)
        }
    }

    #[derive(Debug, PartialEq, Visit, Clone)]
    enum BotBehavior {
        Walk(WalkAction),
        OpenDoor(OpenDoorAction),
        StepThrough(StepThroughAction),
        CloseDoor(CloseDoorAction),
    }

    impl Default for BotBehavior {
        fn default() -> Self {
            Self::Walk(Default::default())
        }
    }

    #[derive(Default, Visit)]
    struct Environment {
        // > 0 - door in front of
        // < 0 - door is behind
        distance_to_door: f32,
        door_opened: bool,
        done: bool,
    }

    dispatch_behavior_variants!(
        BotBehavior,
        Environment,
        Walk,
        OpenDoor,
        StepThrough,
        CloseDoor
    );

    fn create_tree() -> BehaviorTree<BotBehavior> {
        let mut tree = BehaviorTree::new();

        let entry = sequence(
            [
                leaf(BotBehavior::Walk(WalkAction), &mut tree),
                leaf(BotBehavior::OpenDoor(OpenDoorAction), &mut tree),
                leaf(BotBehavior::StepThrough(StepThroughAction), &mut tree),
                leaf(BotBehavior::CloseDoor(CloseDoorAction), &mut tree),
            ],
            &mut tree,
        );

        tree.set_entry_node(entry);

        tree
    }

    #[test]
    fn test_behavior() {
        let tree = create_tree();

        let mut ctx = Environment {
            distance_to_door: 3.0,
            door_opened: false,
            done: false,
        };

        while !ctx.done {
            tree.tick(&mut ctx).unwrap();
        }
    }

    #[test]
    fn test_behavior_save_load() {
        let (bin, txt) = {
            let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
            let root = PathBuf::from(manifest_dir).join("test_output");
            if !root.exists() {
                std::fs::create_dir(&root).unwrap();
            }
            (
                root.join(format!("{}.bin", "behavior_save_load")),
                root.join(format!("{}.txt", "behavior_save_load")),
            )
        };

        // Save
        let mut saved_tree = create_tree();
        let mut visitor = Visitor::new();
        saved_tree.visit("Tree", &mut visitor).unwrap();
        visitor.save_binary_to_file(bin.clone()).unwrap();
        let mut file = File::create(txt).unwrap();
        file.write_all(visitor.save_ascii_to_string().as_bytes())
            .unwrap();

        // Load
        let mut visitor = block_on(Visitor::load_from_file(bin)).unwrap();
        let mut loaded_tree = BehaviorTree::<BotBehavior>::default();
        loaded_tree.visit("Tree", &mut visitor).unwrap();

        assert_eq!(saved_tree, loaded_tree);
    }
}
