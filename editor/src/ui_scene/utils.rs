use crate::{
    message::MessageSender, scene::Selection, ui_scene::commands::ChangeUiSelectionCommand,
    ui_scene::selection::UiSelection, world::WorldViewerDataProvider,
};
use fyrox::{
    core::{make_pretty_type_name, pool::ErasedHandle, reflect::Reflect},
    gui::{draw::SharedTexture, UserInterface},
};
use std::{borrow::Cow, path::Path};

pub struct UiSceneWorldViewerDataProvider<'a> {
    pub ui: &'a UserInterface,
    pub path: Option<&'a Path>,
    pub selection: &'a Selection,
    pub sender: &'a MessageSender,
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
            .try_get_node(node.into())
            .map(|n| n.children.iter().map(|c| (*c).into()).collect::<Vec<_>>())
            .unwrap_or_default()
    }

    fn child_count_of(&self, node: ErasedHandle) -> usize {
        self.ui
            .try_get_node(node.into())
            .map(|n| n.children.len())
            .unwrap_or_default()
    }

    fn is_node_has_child(&self, node: ErasedHandle, child: ErasedHandle) -> bool {
        self.ui
            .try_get_node(node.into())
            .map_or(false, |n| n.children().iter().any(|c| *c == child.into()))
    }

    fn parent_of(&self, node: ErasedHandle) -> ErasedHandle {
        self.ui
            .try_get_node(node.into())
            .map(|n| n.parent().into())
            .unwrap_or_default()
    }

    fn name_of(&self, node: ErasedHandle) -> Option<Cow<str>> {
        self.ui.try_get_node(node.into()).map(|n| {
            Cow::Owned(format!(
                "{} [{}]",
                n.name(),
                make_pretty_type_name(Reflect::type_name(n))
            ))
        })
    }

    fn is_valid_handle(&self, node: ErasedHandle) -> bool {
        self.ui.try_get_node(node.into()).is_some()
    }

    fn icon_of(&self, _node: ErasedHandle) -> Option<SharedTexture> {
        // TODO
        None
    }

    fn is_instance(&self, _node: ErasedHandle) -> bool {
        false
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

    fn on_drop(&self, _child: ErasedHandle, _parent: ErasedHandle) {
        // TODO: Add link widgets command
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
