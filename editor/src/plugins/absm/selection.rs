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
    command::{make_command, Command, SetPropertyCommand},
    fyrox::{
        core::log::Log,
        core::pool::ErasedHandle,
        core::pool::Handle,
        core::reflect::Reflect,
        core::variable::InheritableVariable,
        engine::Engine,
        generic_animation::machine::Machine,
        generic_animation::machine::{PoseNode, State, Transition},
        graph::{SceneGraph, SceneGraphNode},
        gui::inspector::PropertyChanged,
        scene::SceneContainer,
    },
    message::MessageSender,
    plugins::absm::command::fetch_machine,
    scene::controller::SceneController,
    scene::GameScene,
    scene::SelectionContainer,
    ui_scene::UiScene,
};
use std::fmt::{Debug, Formatter};
use std::ops::Deref;

#[derive(Eq)]
pub enum SelectedEntity<N: Reflect> {
    Transition(Handle<Transition<Handle<N>>>),
    State(Handle<State<Handle<N>>>),
    PoseNode(Handle<PoseNode<Handle<N>>>),
}

impl<N> Debug for SelectedEntity<N>
where
    N: Reflect,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transition(v) => write!(f, "{v}"),
            Self::State(v) => write!(f, "{v}"),
            Self::PoseNode(v) => write!(f, "{v}"),
        }
    }
}

impl<N> Clone for SelectedEntity<N>
where
    N: Reflect,
{
    fn clone(&self) -> Self {
        match self {
            Self::Transition(v) => Self::Transition(*v),
            Self::State(v) => Self::State(*v),
            Self::PoseNode(v) => Self::PoseNode(*v),
        }
    }
}

impl<N> PartialEq for SelectedEntity<N>
where
    N: Reflect,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Transition(a), Self::Transition(b)) => *a == *b,
            (Self::State(a), Self::State(b)) => *a == *b,
            (Self::PoseNode(a), Self::PoseNode(b)) => *a == *b,
            _ => false,
        }
    }
}

#[derive(Eq, Default)]
pub struct AbsmSelection<N: Reflect> {
    pub absm_node_handle: Handle<N>,
    pub layer: Option<usize>,
    pub entities: Vec<SelectedEntity<N>>,
}

impl<N> Debug for AbsmSelection<N>
where
    N: Reflect,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {:?} {:?}",
            self.absm_node_handle, self.layer, self.entities
        )
    }
}

impl<N> Clone for AbsmSelection<N>
where
    N: Reflect,
{
    fn clone(&self) -> Self {
        Self {
            absm_node_handle: self.absm_node_handle,
            layer: self.layer,
            entities: self.entities.clone(),
        }
    }
}

impl<N> PartialEq for AbsmSelection<N>
where
    N: Reflect,
{
    fn eq(&self, other: &Self) -> bool {
        self.entities == other.entities
            && self.layer == other.layer
            && self.absm_node_handle == other.absm_node_handle
    }
}

pub fn get_machine_ref<'a, N: Reflect>(
    controller: &'a dyn SceneController,
    node_handle: Handle<N>,
    scene_container: &'a SceneContainer,
) -> Option<&'a Machine<Handle<N>>> {
    if let Some(game_scene) = controller.downcast_ref::<GameScene>() {
        scene_container[game_scene.scene]
            .graph
            .node(ErasedHandle::from(node_handle).into())
            .component_ref::<InheritableVariable<Machine<Handle<N>>>>()
            .map(|v| v.deref())
    } else if let Some(ui) = controller.downcast_ref::<UiScene>() {
        ui.ui
            .node(ErasedHandle::from(node_handle).into())
            .component_ref::<InheritableVariable<Machine<Handle<N>>>>()
            .map(|v| v.deref())
    } else {
        None
    }
}

impl<N: Reflect> SelectionContainer for AbsmSelection<N> {
    fn len(&self) -> usize {
        self.entities.len()
    }

    fn first_selected_entity(
        &self,
        controller: &dyn SceneController,
        scenes: &SceneContainer,
        callback: &mut dyn FnMut(EntityInfo),
    ) {
        if let Some(machine) = get_machine_ref(controller, self.absm_node_handle, scenes) {
            if let Some(first) = self.entities.first() {
                if let Some(layer_index) = self.layer {
                    if let Some(layer) = machine.layers().get(layer_index) {
                        match first {
                            SelectedEntity::Transition(transition) => (callback)(
                                EntityInfo::with_no_parent(&layer.transitions()[*transition]),
                            ),
                            SelectedEntity::State(state) => (callback)(EntityInfo::with_no_parent(
                                &layer.states()[*state] as &dyn Reflect,
                            )),
                            SelectedEntity::PoseNode(pose) => (callback)(
                                EntityInfo::with_no_parent(&layer.nodes()[*pose] as &dyn Reflect),
                            ),
                        };
                    }
                }
            }
        }
    }

    fn on_property_changed(
        &mut self,
        _controller: &mut dyn SceneController,
        args: &PropertyChanged,
        _engine: &mut Engine,
        sender: &MessageSender,
    ) {
        let group = if let Some(layer_index) = self.layer {
            let absm_node_handle = self.absm_node_handle;
            self.entities
                .iter()
                .filter_map(|ent| match *ent {
                    SelectedEntity::Transition(transition) => make_command(args, move |ctx| {
                        let machine = fetch_machine(ctx, absm_node_handle);
                        machine
                            .layers_mut()
                            .get_mut(layer_index)?
                            .transitions_mut()
                            .try_borrow_mut(transition)
                            .ok()
                            .map(|t| t as &mut dyn Reflect)
                    }),
                    SelectedEntity::State(state) => make_command(args, move |ctx| {
                        let machine = fetch_machine(ctx, absm_node_handle);
                        machine
                            .layers_mut()
                            .get_mut(layer_index)?
                            .states_mut()
                            .try_borrow_mut(state)
                            .ok()
                            .map(|s| s as &mut dyn Reflect)
                    }),
                    SelectedEntity::PoseNode(pose) => make_command(args, move |ctx| {
                        let machine = fetch_machine(ctx, absm_node_handle);
                        machine
                            .layers_mut()
                            .get_mut(layer_index)?
                            .nodes_mut()
                            .try_borrow_mut(pose)
                            .ok()
                            .map(|p| p as &mut dyn Reflect)
                    }),
                })
                .collect()
        } else {
            vec![]
        };

        if group.is_empty() {
            if !args.is_inheritable() {
                Log::err(format!("Failed to handle a property {}", args.path()))
            }
        } else if group.len() == 1 {
            sender.do_command_group(group);
        }
    }

    fn paste_property(&mut self, path: &str, value: &dyn Reflect, sender: &MessageSender) {
        let group = if let Some(layer_index) = self.layer {
            let absm_node_handle = self.absm_node_handle;
            self.entities
                .iter()
                .filter_map(|ent| match *ent {
                    SelectedEntity::Transition(transition) => value.try_clone_box().map(|value| {
                        Command::new(SetPropertyCommand::new(
                            path.to_string(),
                            value,
                            move |ctx| {
                                let machine = fetch_machine(ctx, absm_node_handle);
                                machine
                                    .layers_mut()
                                    .get_mut(layer_index)?
                                    .transitions_mut()
                                    .try_borrow_mut(transition)
                                    .ok()
                                    .map(|t| t as &mut dyn Reflect)
                            },
                        ))
                    }),
                    SelectedEntity::State(state) => value.try_clone_box().map(|value| {
                        Command::new(SetPropertyCommand::new(
                            path.to_string(),
                            value,
                            move |ctx| {
                                let machine = fetch_machine(ctx, absm_node_handle);
                                machine
                                    .layers_mut()
                                    .get_mut(layer_index)?
                                    .states_mut()
                                    .try_borrow_mut(state)
                                    .ok()
                                    .map(|s| s as &mut dyn Reflect)
                            },
                        ))
                    }),
                    SelectedEntity::PoseNode(pose) => value.try_clone_box().map(|value| {
                        Command::new(SetPropertyCommand::new(
                            path.to_string(),
                            value,
                            move |ctx| {
                                let machine = fetch_machine(ctx, absm_node_handle);
                                machine
                                    .layers_mut()
                                    .get_mut(layer_index)?
                                    .nodes_mut()
                                    .try_borrow_mut(pose)
                                    .ok()
                                    .map(|n| n as &mut dyn Reflect)
                            },
                        ))
                    }),
                })
                .collect()
        } else {
            vec![]
        };

        if group.len() == 1 {
            sender.do_command_group(group);
        }
    }

    fn provide_docs(&self, _controller: &dyn SceneController, _engine: &Engine) -> Option<String> {
        None
    }
}
