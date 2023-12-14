//! Ui graph manipulation commands.

use crate::{
    scene::Selection,
    ui_scene::{
        commands::{UiCommand, UiSceneContext},
        UiSelection,
    },
    Message,
};
use fyrox::{
    core::pool::Handle,
    gui::{SubGraph, UiNode},
};

#[derive(Debug)]
pub struct AddUiNodeCommand {
    sub_graph: Option<SubGraph>,
    handle: Handle<UiNode>,
    parent: Handle<UiNode>,
    select_added: bool,
    prev_selection: Selection,
}

impl AddUiNodeCommand {
    pub fn new(sub_graph: SubGraph, parent: Handle<UiNode>, select_added: bool) -> Self {
        Self {
            sub_graph: Some(sub_graph),
            handle: Default::default(),
            parent,
            select_added,
            prev_selection: Selection::None,
        }
    }
}

impl UiCommand for AddUiNodeCommand {
    fn name(&mut self, _context: &UiSceneContext) -> String {
        "Add Ui Node".to_string()
    }

    fn execute(&mut self, context: &mut UiSceneContext) {
        self.handle = context
            .ui
            .put_sub_graph_back(self.sub_graph.take().unwrap());

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
        // No need to unlink node from its parent because .take_reserve_sub_graph() does that for us.
        self.sub_graph = Some(context.ui.take_reserve_sub_graph(self.handle));

        if self.select_added {
            std::mem::swap(context.selection, &mut self.prev_selection);
            context.message_sender.send(Message::SelectionChanged {
                old_selection: self.prev_selection.clone(),
            });
        }
    }

    fn finalize(&mut self, context: &mut UiSceneContext) {
        if let Some(sub_graph) = self.sub_graph.take() {
            context.ui.forget_sub_graph(sub_graph)
        }
    }
}
