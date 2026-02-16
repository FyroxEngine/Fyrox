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

use crate::fyrox::graph::SceneGraphNode;
use crate::fyrox::{
    asset::manager::ResourceManager,
    core::{
        futures::executor::block_on, make_pretty_type_name, make_relative_path, pool::ErasedHandle,
        pool::Handle, reflect::Reflect,
    },
    graph::SceneGraph,
    gui::{
        border::Border, button::Button, canvas::Canvas, check_box::CheckBox,
        file_browser::FileBrowser, grid::Grid, image::Image, inspector::Inspector,
        list_view::ListView, menu::Menu, messagebox::MessageBox, popup::Popup, screen::Screen,
        stack_panel::StackPanel, text::Text, window::Window, UiNode, UserInterface,
        UserInterfaceResourceExtension,
    },
};
use crate::world::SceneItemIcon;
use crate::{
    command::{Command, CommandGroup},
    message::MessageSender,
    scene::{commands::ChangeSelectionCommand, Selection},
    scene_item_icon,
    ui_scene::{
        commands::graph::{AddUiPrefabCommand, LinkWidgetsCommand, SetWidgetChildPosition},
        selection::UiSelection,
    },
    world::{item::DropAnchor, WorldViewerDataProvider},
};
use fyrox::core::color::Color;
use std::{borrow::Cow, path::Path, path::PathBuf};

pub struct UiSceneWorldViewerDataProvider<'a> {
    pub ui: &'a mut UserInterface,
    pub path: Option<&'a Path>,
    pub selection: &'a Selection,
    pub sender: &'a MessageSender,
    pub resource_manager: &'a ResourceManager,
}

impl WorldViewerDataProvider for UiSceneWorldViewerDataProvider<'_> {
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
        self.ui.try_get_node(node.into()).ok().is_some_and(|n| {
            n.children()
                .iter()
                .any(|c| *c == Handle::<UiNode>::from(child))
        })
    }

    fn parent_of(&self, node: ErasedHandle) -> ErasedHandle {
        self.ui
            .try_get_node(node.into())
            .map(|n| n.parent().into())
            .unwrap_or_default()
    }

    fn name_of(&self, node: ErasedHandle) -> Option<Cow<str>> {
        self.ui.try_get_node(node.into()).ok().map(|n| {
            Cow::Owned(format!(
                "{} [{}]",
                n.name(),
                make_pretty_type_name(Reflect::type_name(n))
            ))
        })
    }

    fn is_valid_handle(&self, node: ErasedHandle) -> bool {
        self.ui.try_get_node(node.into()).is_ok()
    }

    fn icon_of(&self, node: ErasedHandle) -> Option<SceneItemIcon> {
        let node: &UiNode = self.ui.try_get_node(node.into()).unwrap();

        // all icons are able to be used freely
        // todo: add more icons

        // Containers
        if node.cast::<Canvas>().is_some() {
            scene_item_icon!("../../resources/canvas-icon.png", Color::hex("#FFC312"))
        } else if node.cast::<Screen>().is_some() {
            scene_item_icon!("../../resources/screen-icon.png", Color::hex("#F79F1F"))
        } else if node.cast::<Grid>().is_some() {
            scene_item_icon!("../../resources/grid-icon.png", Color::hex("#EE5A24"))
        } else if node.cast::<StackPanel>().is_some() {
            scene_item_icon!("../../resources/stackPanel-icon.png", Color::hex("#EA2027"))
        } else if node.cast::<Window>().is_some() {
            scene_item_icon!("../../resources/window-icon.png", Color::hex("#C4E538"))
        } else if node.cast::<MessageBox>().is_some() {
            scene_item_icon!("../../resources/messageBox-icon.png", Color::hex("#A3CB38"))
        } else if node.cast::<Menu>().is_some() {
            scene_item_icon!("../../resources/menu-icon.png", Color::hex("#009432"))
        } else if node.cast::<Popup>().is_some() {
            scene_item_icon!("../../resources/popup-icon.png", Color::hex("#006266"))
        }
        // Visual
        else if node.cast::<Text>().is_some() {
            scene_item_icon!("../../resources/text-icon.png", Color::hex("#12CBC4"))
        } else if node.cast::<Image>().is_some() {
            scene_item_icon!("../../resources/image-icon.png", Color::hex("#1289A7"))
        } else if node.cast::<Border>().is_some() {
            scene_item_icon!("../../resources/border-icon.png", Color::hex("#0652DD"))
        }
        // Controls
        else if node.cast::<Button>().is_some() {
            scene_item_icon!("../../resources/button-icon.png", Color::hex("#FDA7DF"))
        } else if node.cast::<CheckBox>().is_some() {
            scene_item_icon!("../../resources/checkbox-icon.png", Color::hex("#D980FA"))
        } else if node.cast::<ListView>().is_some() {
            scene_item_icon!("../../resources/list-icon.png", Color::hex("#9980FA"))
        } else if node.cast::<FileBrowser>().is_some() {
            scene_item_icon!(
                "../../resources/fileBrowser-icon.png",
                Color::hex("#5758BB")
            )
        } else if node.cast::<Inspector>().is_some() {
            scene_item_icon!("../../resources/inspector-icon.png", Color::hex("#ED4C67"))
        } else {
            None
        }
    }

    fn is_instance(&self, node: ErasedHandle) -> bool {
        self.ui
            .try_get_node(node.into())
            .ok()
            .is_some_and(|n| n.resource().is_some())
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
                                if let Ok(node) = self.ui.try_get_node(widget_handle) {
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
