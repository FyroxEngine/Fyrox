use crate::{
    absm::{
        canvas::{AbsmCanvas, AbsmCanvasBuilder, AbsmCanvasMessage},
        command::{
            AbsmCommand, AddTransitionCommand, ChangeSelectionCommand, CommandGroup,
            MoveStateNodeCommand,
        },
        message::MessageSender,
        node::{AbsmNode, AbsmNodeBuilder, AbsmNodeMessage},
        state_graph::context::{CanvasContextMenu, NodeContextMenu, TransitionContextMenu},
        transition::{Transition, TransitionBuilder},
        AbsmDataModel, SelectedEntity, NORMAL_BACKGROUND, NORMAL_ROOT_COLOR, SELECTED_BACKGROUND,
        SELECTED_ROOT_COLOR,
    },
    send_sync_message,
};
use fyrox::{
    animation::machine::{state::StateDefinition, transition::TransitionDefinition},
    core::pool::Handle,
    gui::{
        border::BorderBuilder,
        message::{MessageDirection, UiMessage},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowTitle},
        BuildContext, Thickness, UiNode, UserInterface,
    },
};
use std::cmp::Ordering;

mod context;

pub struct Document {
    pub window: Handle<UiNode>,
    pub canvas: Handle<UiNode>,
    canvas_context_menu: CanvasContextMenu,
    node_context_menu: NodeContextMenu,
    transition_context_menu: TransitionContextMenu,
}

fn fetch_state_node_model_handle(
    handle: Handle<UiNode>,
    ui: &UserInterface,
) -> Handle<StateDefinition> {
    ui.node(handle)
        .query_component::<AbsmNode<StateDefinition>>()
        .unwrap()
        .model_handle
}

impl Document {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let mut node_context_menu = NodeContextMenu::new(ctx);
        let mut canvas_context_menu = CanvasContextMenu::new(ctx);
        let transition_context_menu = TransitionContextMenu::new(ctx);

        let canvas = AbsmCanvasBuilder::new(
            WidgetBuilder::new().with_context_menu(canvas_context_menu.menu),
        )
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::text("State Graph"))
            .can_close(false)
            .can_minimize(false)
            .with_content(
                BorderBuilder::new(
                    WidgetBuilder::new()
                        .with_margin(Thickness::uniform(1.0))
                        .with_child(canvas),
                )
                .build(ctx),
            )
            .build(ctx);

        canvas_context_menu.canvas = canvas;
        canvas_context_menu.node_context_menu = node_context_menu.menu;
        node_context_menu.canvas = canvas;

        Self {
            window,
            canvas,
            node_context_menu,
            canvas_context_menu,
            transition_context_menu,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        sender: &MessageSender,
        data_model: &AbsmDataModel,
    ) {
        if message.destination() == self.canvas {
            if let Some(msg) = message.data::<AbsmCanvasMessage>() {
                match msg {
                    AbsmCanvasMessage::CommitTransition {
                        source_node: source,
                        dest_node: dest,
                    } => {
                        if message.direction() == MessageDirection::FromWidget {
                            let source = fetch_state_node_model_handle(*source, ui);
                            let dest = fetch_state_node_model_handle(*dest, ui);

                            sender.do_command(AddTransitionCommand::new(TransitionDefinition {
                                name: "Transition".to_string(),
                                transition_time: 1.0,
                                source,
                                dest,
                                rule: "".to_string(),
                            }));
                        }
                    }
                    AbsmCanvasMessage::CommitDrag { entries } => {
                        let commands = entries
                            .iter()
                            .map(|e| {
                                let state_handle = fetch_state_node_model_handle(e.node, ui);
                                let new_position = ui.node(e.node).actual_local_position();

                                AbsmCommand::new(MoveStateNodeCommand::new(
                                    state_handle,
                                    e.initial_position,
                                    new_position,
                                ))
                            })
                            .collect::<Vec<_>>();

                        sender.do_command(CommandGroup::from(commands));
                    }
                    AbsmCanvasMessage::SelectionChanged(selection) => {
                        if message.direction() == MessageDirection::FromWidget {
                            let selection = selection
                                .iter()
                                .filter_map(|n| {
                                    let node_ref = ui.node(*n);

                                    if let Some(state_node) =
                                        node_ref.query_component::<AbsmNode<StateDefinition>>()
                                    {
                                        Some(SelectedEntity::State(state_node.model_handle))
                                    } else {
                                        node_ref.query_component::<Transition>().map(|state_node| {
                                            SelectedEntity::Transition(state_node.model_handle)
                                        })
                                    }
                                })
                                .collect();

                            if selection != data_model.selection {
                                sender.do_command(ChangeSelectionCommand { selection });
                            }
                        }
                    }
                    _ => (),
                }
            }
        }

        self.node_context_menu
            .handle_ui_message(message, ui, data_model, sender);
        self.canvas_context_menu
            .handle_ui_message(sender, message, ui);
        self.transition_context_menu
            .handle_ui_message(message, ui, sender);
    }

    pub fn sync_to_model(&mut self, data_model: &AbsmDataModel, ui: &mut UserInterface) {
        let definition = &data_model.absm_definition;

        let canvas = ui
            .node(self.canvas)
            .cast::<AbsmCanvas>()
            .expect("Must be AbsmCanvas!");

        let mut states = canvas
            .children()
            .iter()
            .cloned()
            .filter(|c| ui.node(*c).has_component::<AbsmNode<StateDefinition>>())
            .collect::<Vec<_>>();

        let mut transitions = canvas
            .children()
            .iter()
            .cloned()
            .filter(|c| ui.node(*c).has_component::<Transition>())
            .collect::<Vec<_>>();

        match states
            .len()
            .cmp(&(definition.states.alive_count() as usize))
        {
            Ordering::Less => {
                // A state was added.
                for (state_handle, state) in definition.states.pair_iter() {
                    if states.iter().all(|state_view| {
                        ui.node(*state_view)
                            .query_component::<AbsmNode<StateDefinition>>()
                            .unwrap()
                            .model_handle
                            != state_handle
                    }) {
                        let state_view_handle = AbsmNodeBuilder::new(
                            WidgetBuilder::new()
                                .with_context_menu(self.node_context_menu.menu)
                                .with_desired_position(state.position),
                        )
                        .with_normal_color(if state_handle == definition.entry_state {
                            NORMAL_ROOT_COLOR
                        } else {
                            NORMAL_BACKGROUND
                        })
                        .with_selected_color(if state_handle == definition.entry_state {
                            SELECTED_ROOT_COLOR
                        } else {
                            SELECTED_BACKGROUND
                        })
                        .with_model_handle(state_handle)
                        .with_name(state.name.clone())
                        .build(&mut ui.build_ctx());

                        states.push(state_view_handle);

                        send_sync_message(
                            ui,
                            WidgetMessage::link(
                                state_view_handle,
                                MessageDirection::ToWidget,
                                self.canvas,
                            ),
                        );
                    }
                }
            }
            Ordering::Greater => {
                // A state was removed.
                for (state_view_handle, state_model_handle) in
                    states.clone().iter().cloned().map(|state_view| {
                        (
                            state_view,
                            ui.node(state_view)
                                .query_component::<AbsmNode<StateDefinition>>()
                                .unwrap()
                                .model_handle,
                        )
                    })
                {
                    if definition
                        .states
                        .pair_iter()
                        .all(|(h, _)| h != state_model_handle)
                    {
                        send_sync_message(
                            ui,
                            WidgetMessage::remove(state_view_handle, MessageDirection::ToWidget),
                        );

                        if let Some(position) = states.iter().position(|s| *s == state_view_handle)
                        {
                            states.remove(position);
                        }
                    }
                }
            }
            _ => (),
        }

        // Sync state nodes.
        for state in states.iter() {
            let state_node = ui
                .node(*state)
                .query_component::<AbsmNode<StateDefinition>>()
                .unwrap();
            let state_model_handle = state_node.model_handle;
            let state_model_ref = &definition.states[state_node.model_handle];

            if state_model_ref.name != state_node.name_value {
                send_sync_message(
                    ui,
                    AbsmNodeMessage::name(
                        *state,
                        MessageDirection::ToWidget,
                        state_model_ref.name.clone(),
                    ),
                );
            }

            send_sync_message(
                ui,
                WidgetMessage::desired_position(
                    *state,
                    MessageDirection::ToWidget,
                    state_model_ref.position,
                ),
            );

            send_sync_message(
                ui,
                AbsmNodeMessage::normal_color(
                    *state,
                    MessageDirection::ToWidget,
                    if state_model_handle == definition.entry_state {
                        NORMAL_ROOT_COLOR
                    } else {
                        NORMAL_BACKGROUND
                    },
                ),
            );
            send_sync_message(
                ui,
                AbsmNodeMessage::selected_color(
                    *state,
                    MessageDirection::ToWidget,
                    if state_model_handle == definition.entry_state {
                        SELECTED_ROOT_COLOR
                    } else {
                        SELECTED_BACKGROUND
                    },
                ),
            );
        }

        // Force update layout to be able to fetch positions of nodes for transitions.
        ui.update(ui.screen_size(), 0.0);

        // Sync transitions.
        match transitions
            .len()
            .cmp(&(definition.transitions.alive_count() as usize))
        {
            Ordering::Less => {
                // A transition was added.
                for (transition_handle, transition) in definition.transitions.pair_iter() {
                    if transitions.iter().all(|transition_view| {
                        ui.node(*transition_view)
                            .query_component::<Transition>()
                            .unwrap()
                            .model_handle
                            != transition_handle
                    }) {
                        fn find_state_view(
                            state_handle: Handle<StateDefinition>,
                            states: &[Handle<UiNode>],
                            ui: &UserInterface,
                        ) -> Handle<UiNode> {
                            states
                                .iter()
                                .find(|s| {
                                    ui.node(**s)
                                        .query_component::<AbsmNode<StateDefinition>>()
                                        .unwrap()
                                        .model_handle
                                        == state_handle
                                })
                                .cloned()
                                .unwrap_or_default()
                        }

                        let transition_view = TransitionBuilder::new(
                            WidgetBuilder::new()
                                .with_context_menu(self.transition_context_menu.menu),
                        )
                        .with_source(find_state_view(transition.source, &states, ui))
                        .with_dest(find_state_view(transition.dest, &states, ui))
                        .build(transition_handle, &mut ui.build_ctx());

                        send_sync_message(
                            ui,
                            WidgetMessage::link(
                                transition_view,
                                MessageDirection::ToWidget,
                                self.canvas,
                            ),
                        );

                        send_sync_message(
                            ui,
                            WidgetMessage::lowermost(transition_view, MessageDirection::ToWidget),
                        );

                        transitions.push(transition_view);
                    }
                }
            }

            Ordering::Greater => {
                // A transition was removed.
                for (transition_view_handle, transition_model_handle) in
                    transitions.clone().iter().cloned().map(|transition_view| {
                        (
                            transition_view,
                            ui.node(transition_view)
                                .query_component::<Transition>()
                                .unwrap()
                                .model_handle,
                        )
                    })
                {
                    if definition
                        .transitions
                        .pair_iter()
                        .all(|(h, _)| h != transition_model_handle)
                    {
                        send_sync_message(
                            ui,
                            WidgetMessage::remove(
                                transition_view_handle,
                                MessageDirection::ToWidget,
                            ),
                        );

                        if let Some(position) = transitions
                            .iter()
                            .position(|s| *s == transition_view_handle)
                        {
                            transitions.remove(position);
                        }
                    }
                }
            }
            Ordering::Equal => {}
        }

        // Sync selection.
        let new_selection = data_model
            .selection
            .iter()
            .filter_map(|entry| match entry {
                SelectedEntity::Transition(transition) => transitions.iter().cloned().find(|t| {
                    ui.node(*t)
                        .query_component::<Transition>()
                        .unwrap()
                        .model_handle
                        == *transition
                }),
                SelectedEntity::State(state) => states.iter().cloned().find(|s| {
                    ui.node(*s)
                        .query_component::<AbsmNode<StateDefinition>>()
                        .unwrap()
                        .model_handle
                        == *state
                }),
                SelectedEntity::PoseNode(_) => {
                    // No such nodes possible to have on this canvas.
                    None
                }
            })
            .collect::<Vec<_>>();
        send_sync_message(
            ui,
            AbsmCanvasMessage::selection_changed(
                self.canvas,
                MessageDirection::ToWidget,
                new_selection,
            ),
        );

        send_sync_message(
            ui,
            AbsmCanvasMessage::force_sync_dependent_objects(
                self.canvas,
                MessageDirection::ToWidget,
            ),
        );
    }
}
