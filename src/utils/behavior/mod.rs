#![allow(missing_docs)] // TODO

use crate::{
    core::{
        pool::{Handle, Pool},
        visitor::prelude::*,
    },
    utils::behavior::{
        composite::{CompositeNode, CompositeNodeKind},
        leaf::LeafNode,
    },
};
use std::fmt::Debug;

pub mod composite;
pub mod leaf;

pub enum Status {
    Success,
    Failure,
    Running,
}

pub trait Behavior: Visit + Default + PartialEq + Debug {
    type Context;

    fn tick(&mut self, context: &mut Self::Context) -> Status;
}

#[derive(Debug, PartialEq, Visit)]
pub struct RootNode<B> {
    child: Handle<BehaviorNode<B>>,
}

impl<B> Default for RootNode<B> {
    fn default() -> Self {
        Self {
            child: Default::default(),
        }
    }
}

#[derive(Debug, PartialEq, Visit)]
pub enum BehaviorNode<B> {
    Unknown,
    Root(RootNode<B>),
    Composite(CompositeNode<B>),
    Leaf(LeafNode<B>),
}

impl<B> Default for BehaviorNode<B> {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, PartialEq, Visit)]
pub struct BehaviorTree<B> {
    nodes: Pool<BehaviorNode<B>>,
    root: Handle<BehaviorNode<B>>,
}

impl<B> Default for BehaviorTree<B> {
    fn default() -> Self {
        Self {
            nodes: Default::default(),
            root: Default::default(),
        }
    }
}

impl<B> BehaviorTree<B> {
    pub fn new() -> Self {
        let mut nodes = Pool::new();
        let root = nodes.spawn(BehaviorNode::Root(RootNode {
            child: Default::default(),
        }));
        Self { nodes, root }
    }

    pub fn add_node(&mut self, node: BehaviorNode<B>) -> Handle<BehaviorNode<B>> {
        self.nodes.spawn(node)
    }

    pub fn set_entry_node(&mut self, entry: Handle<BehaviorNode<B>>) {
        if let BehaviorNode::Root(root) = &mut self.nodes[self.root] {
            root.child = entry;
        } else {
            unreachable!("must be root")
        }
    }

    fn tick_recursive<Ctx>(&self, handle: Handle<BehaviorNode<B>>, context: &mut Ctx) -> Status
    where
        B: Behavior<Context = Ctx>,
    {
        match self.nodes[handle] {
            BehaviorNode::Root(ref root) => {
                if root.child.is_some() {
                    self.tick_recursive(root.child, context)
                } else {
                    Status::Success
                }
            }
            BehaviorNode::Composite(ref composite) => match composite.kind {
                CompositeNodeKind::Sequence => {
                    let mut all_succeeded = true;
                    for child in composite.children.iter() {
                        match self.tick_recursive(*child, context) {
                            Status::Failure => all_succeeded = false,
                            Status::Running => {
                                return Status::Running;
                            }
                            _ => (),
                        }
                    }
                    if all_succeeded {
                        Status::Success
                    } else {
                        Status::Failure
                    }
                }
                CompositeNodeKind::Selector => {
                    for child in composite.children.iter() {
                        match self.tick_recursive(*child, context) {
                            Status::Success => return Status::Success,
                            Status::Running => return Status::Running,
                            _ => (),
                        }
                    }
                    Status::Failure
                }
            },
            BehaviorNode::Leaf(ref leaf) => {
                leaf.behavior.as_ref().unwrap().borrow_mut().tick(context)
            }
            BehaviorNode::Unknown => {
                unreachable!()
            }
        }
    }

    pub fn tick<Ctx>(&self, context: &mut Ctx) -> Status
    where
        B: Behavior<Context = Ctx>,
    {
        self.tick_recursive(self.root, context)
    }
}

#[cfg(test)]
mod test {
    use crate::core::futures::executor::block_on;
    use crate::{
        core::visitor::prelude::*,
        utils::behavior::{
            composite::{CompositeNode, CompositeNodeKind},
            leaf::LeafNode,
            Behavior, BehaviorTree, Status,
        },
    };
    use std::env;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;

    #[derive(Debug, PartialEq, Default, Visit)]
    struct WalkAction;

    impl Behavior for WalkAction {
        type Context = Environment;

        fn tick(&mut self, context: &mut Self::Context) -> Status {
            if context.distance_to_door <= 0.0 {
                Status::Success
            } else {
                context.distance_to_door -= 0.1;
                println!(
                    "Approaching door, remaining distance: {}",
                    context.distance_to_door
                );
                Status::Running
            }
        }
    }

    #[derive(Debug, PartialEq, Default, Visit)]
    struct OpenDoorAction;

    impl Behavior for OpenDoorAction {
        type Context = Environment;

        fn tick(&mut self, context: &mut Self::Context) -> Status {
            if !context.door_opened {
                context.door_opened = true;
                println!("Door was opened!");
            }
            Status::Success
        }
    }

    #[derive(Debug, PartialEq, Default, Visit)]
    struct StepThroughAction;

    impl Behavior for StepThroughAction {
        type Context = Environment;

        fn tick(&mut self, context: &mut Self::Context) -> Status {
            if context.distance_to_door < -1.0 {
                Status::Success
            } else {
                context.distance_to_door -= 0.1;
                println!(
                    "Stepping through doorway, remaining distance: {}",
                    -1.0 - context.distance_to_door
                );
                Status::Running
            }
        }
    }

    #[derive(Debug, PartialEq, Default, Visit)]
    struct CloseDoorAction;

    impl Behavior for CloseDoorAction {
        type Context = Environment;

        fn tick(&mut self, context: &mut Self::Context) -> Status {
            if context.door_opened {
                context.door_opened = false;
                context.done = true;
                println!("Door was closed");
            }
            Status::Success
        }
    }

    #[derive(Debug, PartialEq, Visit)]
    enum BotBehavior {
        None,
        Walk(WalkAction),
        OpenDoor(OpenDoorAction),
        StepThrough(StepThroughAction),
        CloseDoor(CloseDoorAction),
    }

    impl Default for BotBehavior {
        fn default() -> Self {
            Self::None
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

    impl Behavior for BotBehavior {
        type Context = Environment;

        fn tick(&mut self, context: &mut Self::Context) -> Status {
            match self {
                BotBehavior::None => unreachable!(),
                BotBehavior::Walk(v) => v.tick(context),
                BotBehavior::OpenDoor(v) => v.tick(context),
                BotBehavior::StepThrough(v) => v.tick(context),
                BotBehavior::CloseDoor(v) => v.tick(context),
            }
        }
    }

    fn create_tree() -> BehaviorTree<BotBehavior> {
        let mut tree = BehaviorTree::new();

        let entry = CompositeNode::new(
            CompositeNodeKind::Sequence,
            vec![
                LeafNode::new(BotBehavior::Walk(WalkAction)).add(&mut tree),
                LeafNode::new(BotBehavior::OpenDoor(OpenDoorAction)).add(&mut tree),
                LeafNode::new(BotBehavior::StepThrough(StepThroughAction)).add(&mut tree),
                LeafNode::new(BotBehavior::CloseDoor(CloseDoorAction)).add(&mut tree),
            ],
        )
        .add(&mut tree);

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
            tree.tick(&mut ctx);
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
        visitor.save_binary(bin.clone()).unwrap();
        let mut file = File::create(&txt).unwrap();
        file.write(visitor.save_text().as_bytes()).unwrap();

        // Load
        let mut visitor = block_on(Visitor::load_binary(bin)).unwrap();
        let mut loaded_tree = BehaviorTree::<BotBehavior>::default();
        loaded_tree.visit("Tree", &mut visitor).unwrap();

        assert_eq!(saved_tree, loaded_tree);
    }
}
