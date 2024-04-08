use crate::fyrox::graph::SceneGraphNode;
use crate::fyrox::{
    asset::{manager::ResourceManager, untyped::UntypedResource},
    core::{
        futures::executor::block_on, make_pretty_type_name, make_relative_path, pool::ErasedHandle,
        pool::Handle, reflect::Reflect,
    },
    graph::{BaseSceneGraph, SceneGraph},
    gui::{
        border::Border, button::Button, canvas::Canvas, check_box::CheckBox,
        file_browser::FileBrowser, grid::Grid, image::Image, inspector::Inspector,
        list_view::ListView, menu::Menu, messagebox::MessageBox, popup::Popup, screen::Screen,
        stack_panel::StackPanel, text::Text, window::Window, UiNode, UserInterface,
        UserInterfaceResourceExtension,
    },
};
use crate::{
    command::{Command, CommandGroup},
    load_image,
    message::MessageSender,
    scene::{commands::ChangeSelectionCommand, Selection},
    ui_scene::{
        commands::graph::{AddUiPrefabCommand, LinkWidgetsCommand, SetWidgetChildPosition},
        selection::UiSelection,
    },
    world::{graph::item::DropAnchor, WorldViewerDataProvider},
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

    fn nth_child(&self, node: ErasedHandle, i: usize) -> ErasedHandle {
        self.ui
            .node(node.into())
            .children()
            .get(i)
            .cloned()
            .unwrap_or_default()
            .into()
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

    fn icon_of(&self, node: ErasedHandle) -> Option<UntypedResource> {
        let node: &UiNode = self.ui.try_get(node.into()).unwrap();

        // all icons are able to be used freely
        // todo: add more icons

        // Containers
        if node.cast::<Canvas>().is_some() {
            load_image(include_bytes!("../../resources/canvas-icon.png"))
        } else if node.cast::<Screen>().is_some() {
            load_image(include_bytes!("../../resources/screen-icon.png"))
        } else if node.cast::<Grid>().is_some() {
            load_image(include_bytes!("../../resources/grid-icon.png"))
        } else if node.cast::<StackPanel>().is_some() {
            load_image(include_bytes!("../../resources/stackPanel-icon.png"))
        } else if node.cast::<Window>().is_some() {
            load_image(include_bytes!("../../resources/window-icon.png"))
        } else if node.cast::<MessageBox>().is_some() {
            load_image(include_bytes!("../../resources/messageBox-icon.png"))
        } else if node.cast::<Menu>().is_some() {
            load_image(include_bytes!("../../resources/menu-icon.png"))
        } else if node.cast::<Popup>().is_some() {
            load_image(include_bytes!("../../resources/popup-icon.png"))
        }
        // Visual
        else if node.cast::<Text>().is_some() {
            load_image(include_bytes!("../../resources/text-icon.png"))
        } else if node.cast::<Image>().is_some() {
            load_image(include_bytes!("../../resources/image-icon.png"))
        } else if node.cast::<Border>().is_some() {
            load_image(include_bytes!("../../resources/border-icon.png"))
        }
        // Controls
        else if node.cast::<Button>().is_some() {
            load_image(include_bytes!("../../resources/button-icon.png"))
        } else if node.cast::<CheckBox>().is_some() {
            load_image(include_bytes!("../../resources/checkbox-icon.png"))
        } else if node.cast::<ListView>().is_some() {
            load_image(include_bytes!("../../resources/list-icon.png"))
        } else if node.cast::<FileBrowser>().is_some() {
            load_image(include_bytes!("../../resources/fileBrowser-icon.png"))
        } else if node.cast::<Inspector>().is_some() {
            load_image(include_bytes!("../../resources/inspector-icon.png"))
        } else {
            None
        }
    }

    fn is_instance(&self, node: ErasedHandle) -> bool {
        self.ui
            .try_get(node.into())
            .map_or(false, |n| n.resource().is_some())
    }

    fn selection(&self) -> Vec<ErasedHandle> {
        if let Some(selection) = self.selection.as_ui() {
            selection
                .widgets
                .iter()
                .map(|h| ErasedHandle::from(*h))
                .collect::<Vec<_>>()
        } else {
            Default::default()
        }
    }

    fn on_change_hierarchy_request(
        &self,
        child: ErasedHandle,
        parent: ErasedHandle,
        anchor: DropAnchor,
    ) {
        let child: Handle<UiNode> = child.into();
        let parent: Handle<UiNode> = parent.into();

        if let Some(selection) = self.selection.as_ui() {
            if selection.widgets.contains(&child) {
                let mut commands = CommandGroup::default();

                'selection_loop: for &widget_handle in selection.widgets.iter() {
                    // Make sure we won't create any loops - child must not have parent in its
                    // descendants.
                    let mut p = parent;
                    while p.is_some() {
                        if p == widget_handle {
                            continue 'selection_loop;
                        }
                        p = self.ui.node(p).parent();
                    }

                    match anchor {
                        DropAnchor::Side { index_offset, .. } => {
                            if let Some((parents_parent, position)) =
                                self.ui.relative_position(parent, index_offset)
                            {
                                if let Some(node) = self.ui.try_get(widget_handle) {
                                    if node.parent() != parents_parent {
                                        commands.push(LinkWidgetsCommand::new(
                                            widget_handle,
                                            parents_parent,
                                        ));
                                    }
                                }

                                commands.push(SetWidgetChildPosition {
                                    node: parents_parent,
                                    child: widget_handle,
                                    position,
                                });
                            }
                        }
                        DropAnchor::OnTop => {
                            commands.push(LinkWidgetsCommand::new(widget_handle, parent));
                        }
                    }
                }

                if !commands.is_empty() {
                    self.sender.do_command(commands);
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
                    Command::new(AddUiPrefabCommand::new(sub_graph)),
                    Command::new(LinkWidgetsCommand::new(instance, node.into())),
                    // We also want to select newly instantiated model.
                    Command::new(ChangeSelectionCommand::new(Selection::new(
                        UiSelection::single_or_empty(instance),
                    ))),
                ];

                self.sender.do_command(CommandGroup::from(group));
            }
        }
    }

    fn validate(&self) -> Vec<(ErasedHandle, Result<(), String>)> {
        Default::default()
    }

    fn on_selection_changed(&self, selection: &[ErasedHandle]) {
        let mut new_selection = Selection::new_empty();
        for &selected_item in selection {
            match new_selection.as_ui_mut() {
                Some(selection) => selection.insert_or_exclude(selected_item.into()),
                None => {
                    new_selection =
                        Selection::new(UiSelection::single_or_empty(selected_item.into()));
                }
            }
        }

        if &new_selection != self.selection {
            self.sender
                .do_command(ChangeSelectionCommand::new(new_selection));
        }
    }
}
