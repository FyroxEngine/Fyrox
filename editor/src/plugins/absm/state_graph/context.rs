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

use super::fetch_state_node_model_handle;
use crate::{
    command::{Command, CommandGroup},
    fyrox::{
        core::pool::Handle,
        generic_animation::machine::{Machine, State, Transition},
        graph::SceneGraph,
        gui::{
            menu::MenuItemMessage,
            message::UiMessage,
            popup::{Placement, PopupBuilder, PopupMessage},
            stack_panel::StackPanelBuilder,
            widget::WidgetBuilder,
            BuildContext, RcUiNodeHandle, UiNode, UserInterface,
        },
    },
    menu::create_menu_item,
    message::MessageSender,
    plugins::absm::{
        canvas::{AbsmCanvas, AbsmCanvasMessage, Mode},
        command::{
            AddStateCommand, AddTransitionCommand, DeleteStateCommand, DeleteTransitionCommand,
            SetMachineEntryStateCommand,
        },
        node::{AbsmNode, AbsmNodeMessage},
        selection::SelectedEntity,
        transition::TransitionView,
    },
    scene::{commands::ChangeSelectionCommand, Selection},
};

use fyrox::core::pool::HandlesArrayExtension;
use fyrox::core::reflect::Reflect;
use fyrox::core::{uuid, Uuid};
use fyrox::gui::menu::{ContextMenuBuilder, MenuItem};

pub struct CanvasContextMenu {
    create_state: Handle<MenuItem>,
    connect_all_nodes: Handle<MenuItem>,
    pub menu: RcUiNodeHandle,
    pub canvas: Handle<AbsmCanvas>,
    pub node_context_menu: Option<RcUiNodeHandle>,
}

impl CanvasContextMenu {
    pub const CREATE_STATE: Uuid = uuid!("6a3fafe3-0362-4964-a91c-f7a4b1e4e15e");
    pub const CONNECT_ALL: Uuid = uuid!("3db3f59f-e37a-439d-92fc-7785cf48ac49");

    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_state;
        let connect_all_nodes;
        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
                .with_content(
                    StackPanelBuilder::new(
                        WidgetBuilder::new().with_children(
                            [
                                {
                                    create_state = create_menu_item(
                                        "Create State",
                                        Self::CREATE_STATE,
                                        vec![],
                                        ctx,
                                    );
                                    create_state
                                },
                                {
                                    connect_all_nodes = create_menu_item(
                                        "Connect all nodes",
                                        Self::CONNECT_ALL,
                                        vec![],
                                        ctx,
                                    );
                                    connect_all_nodes
                                },
                            ]
                            .to_base(),
                        ),
                    )
                    .build(ctx),
                )
                .with_restrict_picking(false),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            create_state,
            connect_all_nodes,
            menu,
            canvas: Default::default(),
            node_context_menu: Default::default(),
        }
    }

    pub fn handle_ui_message<N: Reflect>(
        &mut self,
        sender: &MessageSender,
        message: &UiMessage,
        ui: &mut UserInterface,
        absm_node_handle: Handle<N>,
        layer_index: usize,
    ) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.create_state {
                let screen_position = ui.node(self.menu.handle()).screen_position();

                sender.do_command(AddStateCommand::new(
                    absm_node_handle,
                    layer_index,
                    State {
                        position: ui[self.canvas].screen_to_local(screen_position),
                        name: "New State".to_string(),
                        on_enter_actions: Default::default(),
                        on_leave_actions: Default::default(),
                        root: Default::default(),
                    },
                ));
            } else if message.destination() == self.connect_all_nodes {
                let canvas = &ui[self.canvas];
                let state_nodes = canvas.children();
                let mut states = Vec::default();
                for source in state_nodes {
                    for dest in state_nodes {
                        if source != dest {
                            states.push((source, dest));
                        }
                    }
                }

                let commands = states
                    .iter()
                    .map(|(source_node, dest_node)| {
                        let (source, dest) = (
                            fetch_state_node_model_handle(**source_node, ui),
                            fetch_state_node_model_handle(**dest_node, ui),
                        );
                        Command::new(AddTransitionCommand::new(
                            absm_node_handle,
                            layer_index,
                            Transition::new("Transition", source, dest, 1.0, ""),
                        ))
                    })
                    .collect::<Vec<_>>();
                sender.do_command(CommandGroup::from(commands));
            }
        }
    }
}

pub struct NodeContextMenu {
    create_transition: Handle<MenuItem>,
    remove: Handle<MenuItem>,
    set_as_entry_state: Handle<MenuItem>,
    enter_state: Handle<MenuItem>,
    connect_to_all_nodes: Handle<MenuItem>,
    pub menu: RcUiNodeHandle,
    pub canvas: Handle<AbsmCanvas>,
    placement_target: Handle<UiNode>,
}

impl NodeContextMenu {
    pub const CREATE_TRANSITION: Uuid = uuid!("4a76fffd-ed28-4451-96f1-72ff9459fc64");
    pub const REMOVE: Uuid = uuid!("23b98c44-2cbb-499b-98b2-46fde9438c8f");
    pub const SET_AS_ENTRY_STATE: Uuid = uuid!("bf43a306-58b9-46c8-bd8a-108bdfd1c67b");
    pub const ENTER_STATE: Uuid = uuid!("78c899a7-b11f-4c25-969d-019272a440a3");
    pub const CREATE_ALL_TRANSITIONS: Uuid = uuid!("75b69ca2-e11c-4d00-94e6-3e548ec3d526");

    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_transition;
        let remove;
        let set_as_entry_state;
        let enter_state;
        let connect_to_all_nodes;
        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
                .with_content(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_child({
                                create_transition = create_menu_item(
                                    "Create Transition",
                                    Self::CREATE_TRANSITION,
                                    vec![],
                                    ctx,
                                );
                                create_transition
                            })
                            .with_child({
                                remove = create_menu_item("Remove", Self::REMOVE, vec![], ctx);
                                remove
                            })
                            .with_child({
                                set_as_entry_state = create_menu_item(
                                    "Set As Entry State",
                                    Self::SET_AS_ENTRY_STATE,
                                    vec![],
                                    ctx,
                                );
                                set_as_entry_state
                            })
                            .with_child({
                                enter_state =
                                    create_menu_item("Enter State", Self::ENTER_STATE, vec![], ctx);
                                enter_state
                            })
                            .with_child({
                                connect_to_all_nodes = create_menu_item(
                                    "Create all transition from current state",
                                    Self::CREATE_ALL_TRANSITIONS,
                                    vec![],
                                    ctx,
                                );
                                connect_to_all_nodes
                            }),
                    )
                    .build(ctx),
                )
                .with_restrict_picking(false),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            create_transition,
            menu,
            remove,
            canvas: Default::default(),
            placement_target: Default::default(),
            set_as_entry_state,
            enter_state,
            connect_to_all_nodes,
        }
    }

    pub fn handle_ui_message<N: Reflect>(
        &mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        sender: &MessageSender,
        absm_node_handle: Handle<N>,
        machine: &Machine<Handle<N>>,
        layer_index: usize,
        editor_selection: &Selection,
    ) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.create_transition {
                ui.send(
                    self.canvas,
                    AbsmCanvasMessage::SwitchMode(Mode::CreateTransition {
                        source: self.placement_target,
                        source_pos: ui.node(self.placement_target).center(),
                        dest_pos: ui[self.canvas].screen_to_local(ui.cursor_position()),
                    }),
                )
            } else if message.destination == self.remove {
                if let Some(selection) = editor_selection.as_absm() {
                    let states_to_remove = selection
                        .entities
                        .iter()
                        .cloned()
                        .filter_map(|e| {
                            if let SelectedEntity::State(handle) = e {
                                Some(handle)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();

                    // Gather every transition that leads from/to any of states to remove.
                    let transitions_to_remove = machine.layers()[layer_index]
                        .transitions()
                        .pair_iter()
                        .filter_map(|(handle, transition)| {
                            if states_to_remove.iter().cloned().any(|state_to_remove| {
                                state_to_remove == transition.source()
                                    || state_to_remove == transition.dest()
                            }) {
                                Some(handle)
                            } else {
                                None
                            }
                        });

                    let mut new_selection = selection.clone();
                    new_selection.entities.clear();

                    let mut group = vec![Command::new(ChangeSelectionCommand::new(
                        Selection::new(new_selection),
                    ))];

                    group.extend(transitions_to_remove.map(|transition| {
                        Command::new(DeleteTransitionCommand::new(
                            absm_node_handle,
                            layer_index,
                            transition,
                        ))
                    }));

                    group.extend(states_to_remove.into_iter().map(|state| {
                        Command::new(DeleteStateCommand::new(
                            absm_node_handle,
                            layer_index,
                            state,
                        ))
                    }));

                    sender.do_command(CommandGroup::from(group));
                }
            } else if message.destination() == self.set_as_entry_state {
                sender.do_command(SetMachineEntryStateCommand {
                    node_handle: absm_node_handle,
                    layer: layer_index,
                    entry: ui
                        .node(self.placement_target)
                        .query_component::<AbsmNode<State<Handle<N>>>>()
                        .unwrap()
                        .model_handle,
                });
            } else if message.destination == self.enter_state {
                ui.post(self.placement_target, AbsmNodeMessage::Enter);
            } else if message.destination == self.connect_to_all_nodes {
                let canvas = &ui[self.canvas];
                let state_nodes = canvas
                    .children()
                    .iter()
                    .cloned()
                    .filter(|c| ui.node(*c).has_component::<AbsmNode<State<Handle<N>>>>())
                    .collect::<Vec<_>>();
                ui.post(
                    self.canvas,
                    AbsmCanvasMessage::CommitTransitionToAllNodes {
                        source_node: self.placement_target,
                        dest_nodes: state_nodes,
                    },
                );
            }
        } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.menu.handle() {
                self.placement_target = *target;
            }
        }
    }
}

pub struct TransitionContextMenu {
    remove: Handle<MenuItem>,
    pub menu: RcUiNodeHandle,
    placement_target: Handle<UiNode>,
}

impl TransitionContextMenu {
    pub const REMOVE_TRANSITION: Uuid = uuid!("57a73a49-7734-421a-bcd4-8caf44bc3a71");

    pub fn new(ctx: &mut BuildContext) -> Self {
        let remove;
        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
                .with_content(
                    StackPanelBuilder::new(WidgetBuilder::new().with_child({
                        remove = create_menu_item(
                            "Remove Transition",
                            Self::REMOVE_TRANSITION,
                            vec![],
                            ctx,
                        );
                        remove
                    }))
                    .build(ctx),
                )
                .with_restrict_picking(false),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            menu,
            remove,
            placement_target: Default::default(),
        }
    }

    pub fn handle_ui_message<N: Reflect>(
        &mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        sender: &MessageSender,
        absm_node_handle: Handle<N>,
        layer_index: usize,
        editor_selection: &Selection,
    ) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination == self.remove {
                if let Some(selection) = editor_selection.as_absm::<N>() {
                    let mut new_selection = selection.clone();
                    new_selection.entities.clear();

                    let transition_ref = ui
                        .node(self.placement_target)
                        .query_component::<TransitionView>()
                        .unwrap();

                    let group = vec![
                        Command::new(ChangeSelectionCommand::new(Selection::new(new_selection))),
                        Command::new(DeleteTransitionCommand::new(
                            absm_node_handle,
                            layer_index,
                            transition_ref.model_handle.into(),
                        )),
                    ];

                    sender.do_command(CommandGroup::from(group));
                }
            }
        } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.menu.handle() {
                self.placement_target = *target;
            }
        }
    }
}
