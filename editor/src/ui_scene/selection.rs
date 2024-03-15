use crate::fyrox::{
    core::pool::Handle,
    graph::BaseSceneGraph,
    gui::{UiNode, UserInterface},
};
use crate::{
    command::{Command, CommandGroup},
    scene::{commands::ChangeSelectionCommand, SelectionContainer},
    ui_scene::commands::graph::DeleteWidgetsSubGraphCommand,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UiSelection {
    pub widgets: Vec<Handle<UiNode>>,
}

impl SelectionContainer for UiSelection {
    fn len(&self) -> usize {
        self.widgets.len()
    }
}

impl UiSelection {
    /// Creates new selection as single if node handle is not none, and empty if it is.
    pub fn single_or_empty(node: Handle<UiNode>) -> Self {
        if node.is_none() {
            Self {
                widgets: Default::default(),
            }
        } else {
            Self {
                widgets: vec![node],
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.widgets.is_empty()
    }

    pub fn len(&self) -> usize {
        self.widgets.len()
    }

    pub fn insert_or_exclude(&mut self, handle: Handle<UiNode>) {
        if let Some(position) = self.widgets.iter().position(|&h| h == handle) {
            self.widgets.remove(position);
        } else {
            self.widgets.push(handle);
        }
    }

    pub fn root_widgets(&self, ui: &UserInterface) -> Vec<Handle<UiNode>> {
        // Helper function.
        fn is_descendant_of(
            handle: Handle<UiNode>,
            other: Handle<UiNode>,
            ui: &UserInterface,
        ) -> bool {
            for &child in ui.node(other).children() {
                if child == handle {
                    return true;
                }

                let inner = is_descendant_of(handle, child, ui);
                if inner {
                    return true;
                }
            }
            false
        }

        let mut root_widgets = Vec::new();
        for &node in self.widgets.iter() {
            let mut descendant = false;
            for &other_node in self.widgets.iter() {
                if is_descendant_of(node, other_node, ui) {
                    descendant = true;
                    break;
                }
            }
            if !descendant {
                root_widgets.push(node);
            }
        }
        root_widgets
    }

    pub fn make_deletion_command(&self, ui: &UserInterface) -> Command {
        // Change selection first.
        let mut command_group = CommandGroup::from(vec![Command::new(
            ChangeSelectionCommand::new(Default::default()),
        )]);

        let root_nodes = self.root_widgets(ui);

        for root_node in root_nodes {
            command_group.push(DeleteWidgetsSubGraphCommand::new(root_node));
        }

        Command::new(command_group)
    }
}
