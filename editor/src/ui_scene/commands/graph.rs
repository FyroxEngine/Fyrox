//! Ui graph manipulation commands.

use crate::{
    scene::Selection,
    ui_scene::commands::{UiCommand, UiSceneContext},
    ui_scene::UiSelection,
    Message,
};
use fyrox::{
    core::pool::{Handle, Ticket},
    gui::UiNode,
};

#[derive(Debug)]
pub struct AddUiNodeCommand {
    ticket: Option<Ticket<UiNode>>,
    handle: Handle<UiNode>,
    node: Option<UiNode>,
    cached_name: String,
    parent: Handle<UiNode>,
    select_added: bool,
    prev_selection: Selection,
}

impl AddUiNodeCommand {
    pub fn new(node: UiNode, parent: Handle<UiNode>, select_added: bool) -> Self {
        Self {
            ticket: None,
            handle: Default::default(),
            cached_name: format!("Add Node {}", node.name()),
            node: Some(node),
            parent,
            select_added,
            prev_selection: Selection::None,
        }
    }
}

impl UiCommand for AddUiNodeCommand {
    fn name(&mut self, _context: &UiSceneContext) -> String {
        self.cached_name.clone()
    }

    fn execute(&mut self, context: &mut UiSceneContext) {
        match self.ticket.take() {
            None => {
                self.handle = context.ui.add_node(self.node.take().unwrap());
            }
            Some(ticket) => {
                let handle = context.ui.put_back(ticket, self.node.take().unwrap());
                assert_eq!(handle, self.handle);
            }
        }

        if self.select_added {
            self.prev_selection = std::mem::replace(
                context.selection,
                Selection::Ui(UiSelection::single_or_empty(self.handle)),
            );
            context.message_sender.send(Message::SelectionChanged {
                old_selection: self.prev_selection.clone(),
            });
        }

        context.ui.link_nodes(
            self.handle,
            if self.parent.is_none() {
                context.ui.root()
            } else {
                self.parent
            },
            false,
        )
    }

    fn revert(&mut self, context: &mut UiSceneContext) {
        // No need to unlink node from its parent because .take_reserve() does that for us.
        let (ticket, node) = context.ui.take_reserve(self.handle);
        self.ticket = Some(ticket);
        self.node = Some(node);

        if self.select_added {
            std::mem::swap(context.selection, &mut self.prev_selection);
            context.message_sender.send(Message::SelectionChanged {
                old_selection: self.prev_selection.clone(),
            });
        }
    }

    fn finalize(&mut self, context: &mut UiSceneContext) {
        if let Some(ticket) = self.ticket.take() {
            context.ui.forget_ticket(ticket, self.node.take().unwrap());
        }
    }
}
