use crate::fyrox::graph::{PrefabData, SceneGraph, SceneGraphNode};
use crate::fyrox::{
    core::pool::{ErasedHandle, Handle},
    generic_animation::machine::{Machine, MachineLayer, State, Transition},
    graph::BaseSceneGraph,
    gui::{
        border::BorderBuilder,
        message::{MessageDirection, UiMessage},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowTitle},
        BuildContext, Thickness, UiNode, UserInterface,
    },
};
use crate::{
    absm::{
        canvas::{AbsmCanvas, AbsmCanvasBuilder, AbsmCanvasMessage},
        command::{AddTransitionCommand, MoveStateNodeCommand},
        fetch_selection,
        node::{AbsmNode, AbsmNodeBuilder, AbsmNodeMessage},
        selection::{AbsmSelection, SelectedEntity},
        state_graph::context::{CanvasContextMenu, NodeContextMenu, TransitionContextMenu},
        transition::{TransitionBuilder, TransitionMessage, TransitionView},
        NORMAL_BACKGROUND, NORMAL_ROOT_COLOR, SELECTED_BACKGROUND, SELECTED_ROOT_COLOR,
    },
    command::{Command, CommandGroup},
    message::MessageSender,
    scene::{commands::ChangeSelectionCommand, Selection},
    send_sync_message,
};
use std::cmp::Ordering;

mod context;

pub struct StateGraphViewer {
    pub window: Handle<UiNode>,
    pub canvas: Handle<UiNode>,
    canvas_context_menu: CanvasContextMenu,
    node_context_menu: NodeContextMenu,
    transition_context_menu: TransitionContextMenu,
    prev_absm: ErasedHandle,
    prev_layer: Option<usize>,
}

fn fetch_state_node_model_handle<N>(
    handle: Handle<UiNode>,
    ui: &UserInterface,
) -> Handle<State<Handle<N>>>
where
    N: 'static,
{
    ui.node(handle)
        .query_component::<AbsmNode<State<Handle<N>>>>()
        .unwrap()
        .model_handle
}

impl StateGraphViewer {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let mut node_context_menu = NodeContextMenu::new(ctx);
        let mut canvas_context_menu = CanvasContextMenu::new(ctx);
        let transition_context_menu = TransitionContextMenu::new(ctx);

        let canvas = AbsmCanvasBuilder::new(
            WidgetBuilder::new().with_context_menu(canvas_context_menu.menu.clone()),
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
        canvas_context_menu.node_context_menu = Some(node_context_menu.menu.clone());
        node_context_menu.canvas = canvas;

        Self {
            window,
            canvas,
            node_context_menu,
            canvas_context_menu,
            transition_context_menu,
            prev_absm: Default::default(),
            prev_layer: None,
        }
    }

    pub fn clear(&self, ui: &UserInterface) {
        for &child in ui.node(self.canvas).children() {
            ui.send_message(WidgetMessage::remove(child, MessageDirection::ToWidget));
        }
    }

    pub fn activate_transition<N>(
        &self,
        ui: &UserInterface,
        transition: Handle<Transition<Handle<N>>>,
    ) where
        N: 'static,
    {
        if let Some(view_handle) = ui.node(self.canvas).children().iter().cloned().find(|c| {
            ui.node(*c)
                .query_component::<TransitionView>()
                .map_or(false, |transition_view_ref| {
                    transition == transition_view_ref.model_handle.into()
                })
        }) {
            ui.send_message(TransitionMessage::activate(
                view_handle,
                MessageDirection::ToWidget,
            ));
        }
    }

    pub fn activate_state<N>(&self, ui: &UserInterface, state: Handle<State<Handle<N>>>)
    where
        N: 'static,
    {
        for (state_view_handle, state_view_ref) in ui
            .node(self.canvas)
            .children()
            .iter()
            .cloned()
            .filter_map(|c| {
                ui.node(c)
                    .query_component::<AbsmNode<State<Handle<N>>>>()
                    .map(|state_view_ref| (c, state_view_ref))
            })
        {
            ui.send_message(AbsmNodeMessage::set_active(
                state_view_handle,
                MessageDirection::ToWidget,
                state_view_ref.model_handle == state,
            ));
        }
    }

    pub fn handle_ui_message<P, G, N>(
        &mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        sender: &MessageSender,
        absm_node_handle: Handle<N>,
        machine: &Machine<Handle<N>>,
        layer_index: usize,
        editor_selection: &Selection,
    ) where
        P: PrefabData<Graph = G>,
        G: SceneGraph<Node = N, Prefab = P>,
        N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
    {
        if message.destination() == self.canvas {
            if let Some(msg) = message.data::<AbsmCanvasMessage>() {
                match msg {
                    AbsmCanvasMessage::CommitTransition {
                        source_node,
                        dest_node,
                    } => {
                        if message.direction() == MessageDirection::FromWidget {
                            let source = fetch_state_node_model_handle(*source_node, ui);
                            let dest = fetch_state_node_model_handle(*dest_node, ui);
                            sender.do_command(AddTransitionCommand::new(
                                absm_node_handle,
                                layer_index,
                                Transition::new("Transition", source, dest, 1.0, ""),
                            ));
                        }
                    }
                    AbsmCanvasMessage::CommitDrag { entries } => {
                        let commands = entries
                            .iter()
                            .map(|e| {
                                let state_handle = fetch_state_node_model_handle(e.node, ui);
                                let new_position = ui.node(e.node).actual_local_position();

                                Command::new(MoveStateNodeCommand::new(
                                    absm_node_handle,
                                    state_handle,
                                    layer_index,
                                    e.initial_position,
                                    new_position,
                                ))
                            })
                            .collect::<Vec<_>>();

                        sender.do_command(CommandGroup::from(commands));
                    }
                    AbsmCanvasMessage::SelectionChanged(selection) => {
                        if message.direction() == MessageDirection::FromWidget {
                            let selection = Selection::new(AbsmSelection {
                                absm_node_handle,
                                layer: Some(layer_index),
                                entities: selection
                                    .iter()
                                    .filter_map(|n| {
                                        let node_ref = ui.node(*n);

                                        if let Some(state_node) =
                                            node_ref.query_component::<AbsmNode<State<Handle<N>>>>()
                                        {
                                            Some(SelectedEntity::State(state_node.model_handle))
                                        } else {
                                            node_ref.query_component::<TransitionView>().map(
                                                |state_node| {
                                                    SelectedEntity::Transition(
                                                        state_node.model_handle.into(),
                                                    )
                                                },
                                            )
                                        }
                                    })
                                    .collect::<Vec<_>>(),
                            });

                            if !selection.is_empty() && &selection != editor_selection {
                                sender.do_command(ChangeSelectionCommand::new(selection));
                            }
                        }
                    }
                    AbsmCanvasMessage::CommitTransitionToAllNodes {
                        source_node,
                        dest_nodes,
                    } => {
                        if message.direction() == MessageDirection::FromWidget {
                            let source = fetch_state_node_model_handle(*source_node, ui);
                            let commands = dest_nodes
                                .iter()
                                .map(|node| {
                                    let dest_state = fetch_state_node_model_handle(*node, ui);
                                    Command::new(AddTransitionCommand::new(
                                        absm_node_handle,
                                        layer_index,
                                        Transition::new("Transition", source, dest_state, 1.0, ""),
                                    ))
                                })
                                .collect::<Vec<_>>();

                            sender.do_command(CommandGroup::from(commands));
                        }
                    }
                    _ => (),
                }
            }
        }

        self.node_context_menu.handle_ui_message(
            message,
            ui,
            sender,
            absm_node_handle,
            machine,
            layer_index,
            editor_selection,
        );
        self.canvas_context_menu.handle_ui_message(
            sender,
            message,
            ui,
            absm_node_handle,
            layer_index,
        );
        self.transition_context_menu.handle_ui_message(
            message,
            ui,
            sender,
            absm_node_handle,
            layer_index,
            editor_selection,
        );
    }

    pub fn sync_to_model<P, G, N>(
        &mut self,
        machine_layer: &MachineLayer<Handle<N>>,
        ui: &mut UserInterface,
        editor_selection: &Selection,
    ) where
        P: PrefabData<Graph = G>,
        G: SceneGraph<Node = N, Prefab = P>,
        N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
    {
        let canvas = ui
            .node(self.canvas)
            .cast::<AbsmCanvas>()
            .expect("Must be AbsmCanvas!");

        let current_selection = fetch_selection(editor_selection);

        let mut states = Vec::new();
        let mut transitions = Vec::new();
        if self.prev_layer != current_selection.layer
            || current_selection.absm_node_handle != self.prev_absm.into()
        {
            self.prev_layer = current_selection.layer;
            self.prev_absm = current_selection.absm_node_handle.into();
            // Remove content of the previous layer/absm.
            self.clear(ui);
        } else {
            states = canvas
                .children()
                .iter()
                .cloned()
                .filter(|c| ui.node(*c).has_component::<AbsmNode<State<Handle<N>>>>())
                .collect::<Vec<_>>();

            transitions = canvas
                .children()
                .iter()
                .cloned()
                .filter(|c| ui.node(*c).has_component::<TransitionView>())
                .collect::<Vec<_>>();
        }

        match states
            .len()
            .cmp(&(machine_layer.states().alive_count() as usize))
        {
            Ordering::Less => {
                // A state was added.
                for (state_handle, state) in machine_layer.states().pair_iter() {
                    if states.iter().all(|state_view| {
                        ui.node(*state_view)
                            .query_component::<AbsmNode<State<Handle<N>>>>()
                            .unwrap()
                            .model_handle
                            != state_handle
                    }) {
                        let state_view_handle = AbsmNodeBuilder::new(
                            WidgetBuilder::new()
                                .with_context_menu(self.node_context_menu.menu.clone())
                                .with_desired_position(state.position),
                        )
                        .with_normal_color(if state_handle == machine_layer.entry_state() {
                            NORMAL_ROOT_COLOR
                        } else {
                            NORMAL_BACKGROUND
                        })
                        .with_selected_color(if state_handle == machine_layer.entry_state() {
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
                                .query_component::<AbsmNode<State<Handle<N>>>>()
                                .unwrap()
                                .model_handle,
                        )
                    })
                {
                    if machine_layer
                        .states()
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
                .query_component::<AbsmNode<State<Handle<N>>>>()
                .unwrap();
            let state_model_handle = state_node.model_handle;
            let state_model_ref = &machine_layer.states()[state_node.model_handle];

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
                    if state_model_handle == machine_layer.entry_state() {
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
                    if state_model_handle == machine_layer.entry_state() {
                        SELECTED_ROOT_COLOR
                    } else {
                        SELECTED_BACKGROUND
                    },
                ),
            );
        }

        // Force update layout to be able to fetch positions of nodes for transitions.
        ui.update_layout(ui.screen_size());

        // Sync transitions.
        match transitions
            .len()
            .cmp(&(machine_layer.transitions().alive_count() as usize))
        {
            Ordering::Less => {
                // A transition was added.
                for (transition_handle, transition) in machine_layer.transitions().pair_iter() {
                    if transitions.iter().all(|transition_view| {
                        transition_handle
                            != ui
                                .node(*transition_view)
                                .query_component::<TransitionView>()
                                .unwrap()
                                .model_handle
                                .into()
                    }) {
                        fn find_state_view<N: 'static>(
                            state_handle: Handle<State<Handle<N>>>,
                            states: &[Handle<UiNode>],
                            ui: &UserInterface,
                        ) -> Handle<UiNode> {
                            states
                                .iter()
                                .find(|s| {
                                    ui.node(**s)
                                        .query_component::<AbsmNode<State<Handle<N>>>>()
                                        .unwrap()
                                        .model_handle
                                        == state_handle
                                })
                                .cloned()
                                .unwrap_or_default()
                        }

                        let transition_view = TransitionBuilder::new(
                            WidgetBuilder::new()
                                .with_context_menu(self.transition_context_menu.menu.clone()),
                        )
                        .with_source(find_state_view(transition.source(), &states, ui))
                        .with_dest(find_state_view(transition.dest(), &states, ui))
                        .build(transition_handle.into(), &mut ui.build_ctx());

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
                                .query_component::<TransitionView>()
                                .unwrap()
                                .model_handle,
                        )
                    })
                {
                    if machine_layer
                        .transitions()
                        .pair_iter()
                        .all(|(h, _)| h != transition_model_handle.into())
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
        let new_selection = current_selection
            .entities
            .iter()
            .filter_map(|entry| match entry {
                SelectedEntity::Transition(transition) => transitions.iter().cloned().find(|t| {
                    *transition
                        == ui
                            .node(*t)
                            .query_component::<TransitionView>()
                            .unwrap()
                            .model_handle
                            .into()
                }),
                SelectedEntity::State(state) => states.iter().cloned().find(|s| {
                    ui.node(*s)
                        .query_component::<AbsmNode<State<Handle<N>>>>()
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
