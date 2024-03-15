//! Ui graph manipulation commands.

use crate::command::{CommandContext, CommandTrait};
use crate::fyrox::graph::{BaseSceneGraph, LinkScheme, SceneGraphNode};
use crate::fyrox::{
    core::pool::Handle,
    gui::{SubGraph, UiNode, UserInterface},
};
use crate::ui_scene::clipboard::DeepCloneResult;
use crate::{
    scene::Selection,
    ui_scene::{commands::UiSceneContext, UiSelection},
    Message,
};

#[derive(Debug)]
pub struct AddWidgetCommand {
    sub_graph: Option<SubGraph>,
    handle: Handle<UiNode>,
    parent: Handle<UiNode>,
    select_added: bool,
    prev_selection: Selection,
}

impl AddWidgetCommand {
    pub fn new(sub_graph: SubGraph, parent: Handle<UiNode>, select_added: bool) -> Self {
        Self {
            sub_graph: Some(sub_graph),
            handle: Default::default(),
            parent,
            select_added,
            prev_selection: Selection::new_empty(),
        }
    }
}

impl CommandTrait for AddWidgetCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Add Widget".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<UiSceneContext>();

        self.handle = context
            .ui
            .put_sub_graph_back(self.sub_graph.take().unwrap());

        if self.select_added {
            self.prev_selection = std::mem::replace(
                context.selection,
                Selection::new(UiSelection::single_or_empty(self.handle)),
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

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<UiSceneContext>();

        // No need to unlink node from its parent because .take_reserve_sub_graph() does that for us.
        self.sub_graph = Some(context.ui.take_reserve_sub_graph(self.handle));

        if self.select_added {
            std::mem::swap(context.selection, &mut self.prev_selection);
            context.message_sender.send(Message::SelectionChanged {
                old_selection: self.prev_selection.clone(),
            });
        }
    }

    fn finalize(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<UiSceneContext>();

        if let Some(sub_graph) = self.sub_graph.take() {
            context.ui.forget_sub_graph(sub_graph)
        }
    }
}

#[derive(Debug)]
pub struct LinkWidgetsCommand {
    child: Handle<UiNode>,
    parent: Handle<UiNode>,
}

impl LinkWidgetsCommand {
    pub fn new(child: Handle<UiNode>, parent: Handle<UiNode>) -> Self {
        Self { child, parent }
    }

    fn link(&mut self, ui: &mut UserInterface) {
        let old_parent = ui.node(self.child).parent();
        ui.link_nodes(self.child, self.parent, false);
        self.parent = old_parent;
    }
}

impl CommandTrait for LinkWidgetsCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Link Widgets".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.link(context.get_mut::<UiSceneContext>().ui);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.link(context.get_mut::<UiSceneContext>().ui);
    }
}

#[derive(Debug)]
pub struct DeleteWidgetsSubGraphCommand {
    sub_graph_root: Handle<UiNode>,
    sub_graph: Option<SubGraph>,
    parent: Handle<UiNode>,
}

impl DeleteWidgetsSubGraphCommand {
    pub fn new(sub_graph_root: Handle<UiNode>) -> Self {
        Self {
            sub_graph_root,
            sub_graph: None,
            parent: Handle::NONE,
        }
    }
}

impl CommandTrait for DeleteWidgetsSubGraphCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Delete Sub Graph".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<UiSceneContext>();

        self.parent = context.ui.node(self.sub_graph_root).parent();
        self.sub_graph = Some(context.ui.take_reserve_sub_graph(self.sub_graph_root));
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<UiSceneContext>();

        context
            .ui
            .put_sub_graph_back(self.sub_graph.take().unwrap());
        context
            .ui
            .link_nodes(self.sub_graph_root, self.parent, false);
    }

    fn finalize(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<UiSceneContext>();

        if let Some(sub_graph) = self.sub_graph.take() {
            context.ui.forget_sub_graph(sub_graph)
        }
    }
}

#[derive(Debug)]
enum PasteWidgetCommandState {
    Undefined,
    NonExecuted,
    Reverted {
        subgraphs: Vec<SubGraph>,
        selection: Selection,
    },
    Executed {
        paste_result: DeepCloneResult,
        last_selection: Selection,
    },
}

#[derive(Debug)]
pub struct PasteWidgetCommand {
    parent: Handle<UiNode>,
    state: PasteWidgetCommandState,
}

impl PasteWidgetCommand {
    pub fn new(parent: Handle<UiNode>) -> Self {
        Self {
            parent,
            state: PasteWidgetCommandState::NonExecuted,
        }
    }
}

impl CommandTrait for PasteWidgetCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Paste".to_owned()
    }

    fn execute(&mut self, ctx: &mut dyn CommandContext) {
        let ctx = ctx.get_mut::<UiSceneContext>();

        match std::mem::replace(&mut self.state, PasteWidgetCommandState::Undefined) {
            PasteWidgetCommandState::NonExecuted => {
                let paste_result = ctx.clipboard.paste(ctx.ui);

                for &handle in paste_result.root_nodes.iter() {
                    ctx.ui.link_nodes(handle, self.parent, false);
                }

                let mut selection = Selection::new(UiSelection {
                    widgets: paste_result.root_nodes.clone(),
                });
                std::mem::swap(ctx.selection, &mut selection);

                self.state = PasteWidgetCommandState::Executed {
                    paste_result,
                    last_selection: selection,
                };
            }
            PasteWidgetCommandState::Reverted {
                subgraphs,
                mut selection,
            } => {
                let mut paste_result = DeepCloneResult {
                    ..Default::default()
                };

                for subgraph in subgraphs {
                    paste_result
                        .root_nodes
                        .push(ctx.ui.put_sub_graph_back(subgraph));
                }

                for &handle in paste_result.root_nodes.iter() {
                    ctx.ui.link_nodes(handle, self.parent, false);
                }

                std::mem::swap(ctx.selection, &mut selection);
                self.state = PasteWidgetCommandState::Executed {
                    paste_result,
                    last_selection: selection,
                };
            }
            _ => unreachable!(),
        }
    }

    fn revert(&mut self, ctx: &mut dyn CommandContext) {
        let ctx = ctx.get_mut::<UiSceneContext>();

        if let PasteWidgetCommandState::Executed {
            paste_result,
            mut last_selection,
        } = std::mem::replace(&mut self.state, PasteWidgetCommandState::Undefined)
        {
            let mut subgraphs = Vec::new();
            for root_node in paste_result.root_nodes {
                subgraphs.push(ctx.ui.take_reserve_sub_graph(root_node));
            }

            std::mem::swap(ctx.selection, &mut last_selection);

            self.state = PasteWidgetCommandState::Reverted {
                subgraphs,
                selection: last_selection,
            };
        }
    }

    fn finalize(&mut self, ctx: &mut dyn CommandContext) {
        let ctx = ctx.get_mut::<UiSceneContext>();

        if let PasteWidgetCommandState::Reverted { subgraphs, .. } =
            std::mem::replace(&mut self.state, PasteWidgetCommandState::Undefined)
        {
            for subgraph in subgraphs {
                ctx.ui.forget_sub_graph(subgraph);
            }
        }
    }
}

#[derive(Debug)]
pub struct AddUiPrefabCommand {
    model: Handle<UiNode>,
    sub_graph: Option<SubGraph>,
}

impl AddUiPrefabCommand {
    pub fn new(sub_graph: SubGraph) -> Self {
        Self {
            model: Default::default(),
            sub_graph: Some(sub_graph),
        }
    }
}

impl CommandTrait for AddUiPrefabCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Instantiate Prefab".to_owned()
    }

    fn execute(&mut self, ctx: &mut dyn CommandContext) {
        let ctx = ctx.get_mut::<UiSceneContext>();
        // A model was loaded, but change was reverted and here we must put all nodes
        // back to graph.
        self.model = ctx.ui.put_sub_graph_back(self.sub_graph.take().unwrap());
    }

    fn revert(&mut self, ctx: &mut dyn CommandContext) {
        let ctx = ctx.get_mut::<UiSceneContext>();
        self.sub_graph = Some(ctx.ui.take_reserve_sub_graph(self.model));
    }

    fn finalize(&mut self, ctx: &mut dyn CommandContext) {
        let ctx = ctx.get_mut::<UiSceneContext>();
        if let Some(sub_graph) = self.sub_graph.take() {
            ctx.ui.forget_sub_graph(sub_graph)
        }
    }
}

#[derive(Debug)]
pub struct SetUiRootCommand {
    pub root: Handle<UiNode>,
    pub link_scheme: LinkScheme<UiNode>,
}

impl CommandTrait for SetUiRootCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Root".to_string()
    }

    fn execute(&mut self, ctx: &mut dyn CommandContext) {
        let ctx = ctx.get_mut::<UiSceneContext>();
        let prev_root = ctx.ui.root();
        self.link_scheme = ctx.ui.change_hierarchy_root(prev_root, self.root);
        self.root = prev_root;
    }

    fn revert(&mut self, ctx: &mut dyn CommandContext) {
        let ctx = ctx.get_mut::<UiSceneContext>();
        ctx.ui
            .apply_link_scheme(std::mem::take(&mut self.link_scheme));
        self.root = self.link_scheme.root;
    }
}

#[derive(Debug)]
pub struct SetWidgetChildPosition {
    pub node: Handle<UiNode>,
    pub child: Handle<UiNode>,
    pub position: usize,
}

impl SetWidgetChildPosition {
    fn swap(&mut self, context: &mut dyn CommandContext) {
        let prev_pos = context
            .get_mut::<UiSceneContext>()
            .ui
            .node_mut(self.node)
            .set_child_position(self.child, self.position)
            .unwrap();
        self.position = prev_pos;
    }
}

impl CommandTrait for SetWidgetChildPosition {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Widget Position".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }
}
