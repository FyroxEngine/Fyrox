use crate::{
    message::MessageSender,
    scene::Selection,
    ui_scene::{
        commands::{
            graph::{AddUiPrefabCommand, LinkWidgetsCommand},
            ChangeUiSelectionCommand, UiCommandGroup, UiSceneCommand,
        },
        selection::UiSelection,
    },
    world::WorldViewerDataProvider,
};
use fyrox::{
    asset::{manager::ResourceManager, untyped::UntypedResource},
    core::{
        futures::executor::block_on, make_pretty_type_name, make_relative_path, pool::ErasedHandle,
        pool::Handle, reflect::Reflect,
    },
    graph::{SceneGraph, SceneGraphNode},
    gui::{UiNode, UserInterface, UserInterfaceResourceExtension},
};
use std::{borrow::Cow, path::Path, path::PathBuf};

pub struct UiSceneWorldViewerDataProvider<'a> {
    pub ui: &'a mut UserInterface,
    pub path: Option<&'a Path>,
    pub selection: &'a Selection,
    pub sender: &'a MessageSender,
    pub resource_manager: &'a ResourceManager,
}

impl<'a> WorldViewerDataProvider for UiSceneWorldViewerDataProvider<'a> {
    fn root_node(&self) -> ErasedHandle {
        self.ui.root().into()
    }

    fn path(&self) -> Option<&Path> {
        self.path
    }

    fn children_of(&self, node: ErasedHandle) -> Vec<ErasedHandle> {
        self.ui
            .try_get(node.into())
            .map(|n| n.children.iter().map(|c| (*c).into()).collect::<Vec<_>>())
            .unwrap_or_default()
    }

    fn child_count_of(&self, node: ErasedHandle) -> usize {
        self.ui
            .try_get(node.into())
            .map(|n| n.children.len())
            .unwrap_or_default()
    }

    fn is_node_has_child(&self, node: ErasedHandle, child: ErasedHandle) -> bool {
        self.ui
            .try_get(node.into())
            .map_or(false, |n| n.children().iter().any(|c| *c == child.into()))
    }

    fn parent_of(&self, node: ErasedHandle) -> ErasedHandle {
        self.ui
            .try_get(node.into())
            .map(|n| n.parent().into())
            .unwrap_or_default()
    }

    fn name_of(&self, node: ErasedHandle) -> Option<Cow<str>> {
        self.ui.try_get(node.into()).map(|n| {
            Cow::Owned(format!(
                "{} [{}]",
                n.name(),
                make_pretty_type_name(Reflect::type_name(n))
            ))
        })
    }

    fn is_valid_handle(&self, node: ErasedHandle) -> bool {
        self.ui.try_get(node.into()).is_some()
    }

    fn icon_of(&self, _node: ErasedHandle) -> Option<UntypedResource> {
        // TODO
        None
    }

    fn is_instance(&self, node: ErasedHandle) -> bool {
        self.ui
            .try_get(node.into())
            .map_or(false, |n| n.resource().is_some())
    }

    fn selection(&self) -> Vec<ErasedHandle> {
        if let Selection::Ui(ref selection) = self.selection {
            selection
                .widgets
                .iter()
                .map(|h| ErasedHandle::from(*h))
                .collect::<Vec<_>>()
        } else {
            Default::default()
        }
    }

    fn on_change_hierarchy_request(&self, child: ErasedHandle, parent: ErasedHandle) {
        let child: Handle<UiNode> = child.into();
        let parent: Handle<UiNode> = parent.into();

        if let Selection::Ui(ref selection) = self.selection {
            if selection.widgets.contains(&child) {
                let mut commands = Vec::new();

                for &widget_handle in selection.widgets.iter() {
                    // Make sure we won't create any loops - child must not have parent in its
                    // descendants.
                    let mut attach = true;
                    let mut p = parent;
                    while p.is_some() {
                        if p == widget_handle {
                            attach = false;
                            break;
                        }
                        p = self.ui.node(p).parent();
                    }

                    if attach {
                        commands.push(UiSceneCommand::new(LinkWidgetsCommand::new(
                            widget_handle,
                            parent,
                        )));
                    }
                }

                if !commands.is_empty() {
                    self.sender
                        .do_ui_scene_command(UiCommandGroup::from(commands));
                }
            }
        }
    }

    fn on_asset_dropped(&mut self, path: PathBuf, node: ErasedHandle) {
        if let Ok(relative_path) = make_relative_path(path) {
            // No model was loaded yet, do it.
            if let Some(prefab) = self
                .resource_manager
                .try_request::<UserInterface>(relative_path)
                .and_then(|m| block_on(m).ok())
            {
                let (instance, _) = prefab.instantiate(self.ui);

                let sub_graph = self.ui.take_reserve_sub_graph(instance);

                let group = vec![
                    UiSceneCommand::new(AddUiPrefabCommand::new(sub_graph)),
                    UiSceneCommand::new(LinkWidgetsCommand::new(instance, node.into())),
                    // We also want to select newly instantiated model.
                    UiSceneCommand::new(ChangeUiSelectionCommand::new(
                        Selection::Ui(UiSelection::single_or_empty(instance)),
                        self.selection.clone(),
                    )),
                ];

                self.sender.do_ui_scene_command(UiCommandGroup::from(group));
            }
        }
    }

    fn validate(&self) -> Vec<(ErasedHandle, Result<(), String>)> {
        Default::default()
    }

    fn on_selection_changed(&self, selection: &[ErasedHandle]) {
        let mut new_selection = Selection::None;
        for &selected_item in selection {
            match new_selection {
                Selection::None => {
                    new_selection =
                        Selection::Ui(UiSelection::single_or_empty(selected_item.into()));
                }
                Selection::Ui(ref mut selection) => {
                    selection.insert_or_exclude(selected_item.into())
                }
                _ => (),
            }
        }

        if &new_selection != self.selection {
            self.sender
                .do_ui_scene_command(ChangeUiSelectionCommand::new(
                    new_selection,
                    self.selection.clone(),
                ));
        }
    }
}
