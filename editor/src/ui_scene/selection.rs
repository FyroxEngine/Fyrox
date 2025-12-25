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

use crate::scene::EntityInfo;
use crate::{
    command::{make_command, Command, CommandGroup, SetPropertyCommand},
    fyrox::{
        core::{pool::Handle, reflect::Reflect, some_or_return},
        engine::Engine,
        graph::{BaseSceneGraph, SceneGraphNode},
        gui::{inspector::PropertyChanged, UiNode, UserInterface},
        scene::SceneContainer,
    },
    message::MessageSender,
    scene::{commands::ChangeSelectionCommand, controller::SceneController, SelectionContainer},
    ui_scene::{
        commands::{
            graph::DeleteWidgetsSubGraphCommand, widget::RevertWidgetPropertyCommand,
            UiSceneContext,
        },
        UiScene,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UiSelection {
    pub widgets: Vec<Handle<UiNode>>,
}

impl SelectionContainer for UiSelection {
    fn len(&self) -> usize {
        self.widgets.len()
    }

    fn first_selected_entity(
        &self,
        controller: &dyn SceneController,
        _scenes: &SceneContainer,
        callback: &mut dyn FnMut(EntityInfo),
    ) {
        let ui_scene = some_or_return!(controller.downcast_ref::<UiScene>());
        if let Some(first) = self.widgets.first() {
            if let Ok(node) = ui_scene.ui.try_get_node(*first) {
                (callback)(EntityInfo {
                    entity: node as &dyn Reflect,
                    has_inheritance_parent: node.has_inheritance_parent(),
                    read_only: false,
                })
            }
        }
    }

    fn on_property_changed(
        &mut self,
        controller: &mut dyn SceneController,
        args: &PropertyChanged,
        _engine: &mut Engine,
        sender: &MessageSender,
    ) {
        let ui_scene = some_or_return!(controller.downcast_ref::<UiScene>());
        let group = self
            .widgets
            .iter()
            .filter_map(|&node_handle| {
                let node = ui_scene.ui.try_get_node(node_handle).ok()?;
                if args.is_inheritable() {
                    // Prevent reverting property value if there's no parent resource.
                    if node.resource().is_some() {
                        Some(Command::new(RevertWidgetPropertyCommand::new(
                            args.path(),
                            node_handle,
                        )))
                    } else {
                        None
                    }
                } else {
                    make_command(args, move |ctx| {
                        ctx.get_mut::<UiSceneContext>()
                            .ui
                            .try_get_node_mut(node_handle)
                            .ok()
                            .map(|n| n as &mut dyn Reflect)
                    })
                }
            })
            .collect::<Vec<_>>();

        sender.do_command_group_with_inheritance(group, args);
    }

    fn paste_property(&mut self, path: &str, value: &dyn Reflect, sender: &MessageSender) {
        let group = self
            .widgets
            .iter()
            .filter_map(|&node_handle| {
                value.try_clone_box().map(|value| {
                    Command::new(SetPropertyCommand::new(
                        path.to_string(),
                        value,
                        move |ctx| {
                            ctx.get_mut::<UiSceneContext>()
                                .ui
                                .try_get_node_mut(node_handle)
                                .ok()
                                .map(|n| n as &mut dyn Reflect)
                        },
                    ))
                })
            })
            .collect::<Vec<_>>();

        sender.do_command_group(group);
    }

    fn provide_docs(&self, controller: &dyn SceneController, _engine: &Engine) -> Option<String> {
        let ui_scene = controller.downcast_ref::<UiScene>()?;
        self.widgets.first().and_then(|h| {
            ui_scene
                .ui
                .try_get_node(*h)
                .ok()
                .map(|n| n.doc().to_string())
        })
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

    pub fn selection_to_delete(&self, ui: &UserInterface) -> UiSelection {
        let mut selection = self.clone();
        // UI's root is non-deletable.
        if let Some(root_position) = selection.widgets.iter().position(|&n| n == ui.root()) {
            selection.widgets.remove(root_position);
        }

        selection
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
        let selection = self.selection_to_delete(ui);

        // Change selection first.
        let mut command_group = CommandGroup::from(vec![Command::new(
            ChangeSelectionCommand::new(Default::default()),
        )]);

        let root_nodes = selection.root_widgets(ui);

        for root_node in root_nodes {
            command_group.push(DeleteWidgetsSubGraphCommand::new(root_node));
        }

        Command::new(command_group)
    }
}
