use crate::fyrox::{
    core::pool::Handle,
    generic_animation::machine::{Machine, State, Transition},
    graph::BaseSceneGraph,
    gui::{
        menu::MenuItemMessage,
        message::{MessageDirection, UiMessage},
        popup::{Placement, PopupBuilder, PopupMessage},
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        BuildContext, RcUiNodeHandle, UiNode, UserInterface,
    },
};
use crate::{
    absm::{
        canvas::{AbsmCanvas, AbsmCanvasMessage, Mode},
        command::{
            AddStateCommand, AddTransitionCommand, DeleteStateCommand, DeleteTransitionCommand,
            SetMachineEntryStateCommand,
        },
        node::{AbsmNode, AbsmNodeMessage},
        selection::SelectedEntity,
        transition::TransitionView,
    },
    command::{Command, CommandGroup},
    menu::create_menu_item,
    message::MessageSender,
    scene::{commands::ChangeSelectionCommand, Selection},
};
use fyrox::gui::menu::ContextMenuBuilder;
use std::fmt::Debug;

use super::fetch_state_node_model_handle;

pub struct CanvasContextMenu {
    create_state: Handle<UiNode>,
    connect_all_nodes: Handle<UiNode>,
    pub menu: RcUiNodeHandle,
    pub canvas: Handle<UiNode>,
    pub node_context_menu: Option<RcUiNodeHandle>,
}

impl CanvasContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_state;
        let connect_all_nodes;
        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_visibility(false)).with_content(
                StackPanelBuilder::new(WidgetBuilder::new().with_children([
                    {
                        create_state = create_menu_item("Create State", vec![], ctx);
                        create_state
                    },
                    {
                        connect_all_nodes = create_menu_item("Connect all nodes", vec![], ctx);
                        connect_all_nodes
                    },
                ]))
                .build(ctx),
            ),
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

    pub fn handle_ui_message<N: Debug + 'static>(
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
                        position: ui.node(self.canvas).screen_to_local(screen_position),
                        name: "New State".to_string(),
                        on_enter_actions: Default::default(),
                        on_leave_actions: Default::default(),
                        root: Default::default(),
                    },
                ));
            } else if message.destination() == self.connect_all_nodes {
                let canvas = ui
                    .node(self.canvas)
                    .query_component::<AbsmCanvas>()
                    .unwrap();
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
    create_transition: Handle<UiNode>,
    remove: Handle<UiNode>,
    set_as_entry_state: Handle<UiNode>,
    enter_state: Handle<UiNode>,
    connect_to_all_nodes: Handle<UiNode>,
    pub menu: RcUiNodeHandle,
    pub canvas: Handle<UiNode>,
    placement_target: Handle<UiNode>,
}

impl NodeContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_transition;
        let remove;
        let set_as_entry_state;
        let enter_state;
        let connect_to_all_nodes;
        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_visibility(false)).with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            create_transition = create_menu_item("Create Transition", vec![], ctx);
                            create_transition
                        })
                        .with_child({
                            remove = create_menu_item("Remove", vec![], ctx);
                            remove
                        })
                        .with_child({
                            set_as_entry_state =
                                create_menu_item("Set As Entry State", vec![], ctx);
                            set_as_entry_state
                        })
                        .with_child({
                            enter_state = create_menu_item("Enter State", vec![], ctx);
                            enter_state
                        })
                        .with_child({
                            connect_to_all_nodes = create_menu_item(
                                "Create all transition from current state",
                                vec![],
                                ctx,
                            );
                            connect_to_all_nodes
                        }),
                )
                .build(ctx),
            ),
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

    pub fn handle_ui_message<N: Debug + 'static>(
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
                ui.send_message(AbsmCanvasMessage::switch_mode(
                    self.canvas,
                    MessageDirection::ToWidget,
                    Mode::CreateTransition {
                        source: self.placement_target,
                        source_pos: ui.node(self.placement_target).center(),
                        dest_pos: ui.node(self.canvas).screen_to_local(ui.cursor_position()),
                    },
                ))
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
                ui.send_message(AbsmNodeMessage::enter(
                    self.placement_target,
                    MessageDirection::FromWidget,
                ));
            } else if message.destination == self.connect_to_all_nodes {
                let canvas = ui
                    .node(self.canvas)
                    .cast::<AbsmCanvas>()
                    .expect("Must be absm canvas");

                let state_nodes = canvas
                    .children()
                    .iter()
                    .cloned()
                    .filter(|c| ui.node(*c).has_component::<AbsmNode<State<Handle<N>>>>())
                    .collect::<Vec<_>>();
                ui.send_message(AbsmCanvasMessage::commit_transition_to_all_nodes(
                    self.canvas,
                    MessageDirection::FromWidget,
                    self.placement_target,
                    state_nodes,
                ));
            }
        } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.menu.handle() {
                self.placement_target = *target;
            }
        }
    }
}

pub struct TransitionContextMenu {
    remove: Handle<UiNode>,
    pub menu: RcUiNodeHandle,
    placement_target: Handle<UiNode>,
}

impl TransitionContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let remove;
        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_visibility(false)).with_content(
                StackPanelBuilder::new(WidgetBuilder::new().with_child({
                    remove = create_menu_item("Remove Transition", vec![], ctx);
                    remove
                }))
                .build(ctx),
            ),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            menu,
            remove,
            placement_target: Default::default(),
        }
    }

    pub fn handle_ui_message<N: Debug + 'static>(
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
