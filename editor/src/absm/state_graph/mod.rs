use crate::absm::selection::{AbsmSelection, SelectedEntity};
use crate::scene::commands::{ChangeSelectionCommand, CommandGroup, SceneCommand};
use crate::scene::{EditorScene, Selection};
use crate::{
    absm::{
        canvas::{AbsmCanvas, AbsmCanvasBuilder, AbsmCanvasMessage},
        command::{AddTransitionCommand, MoveStateNodeCommand},
        node::{AbsmNode, AbsmNodeBuilder, AbsmNodeMessage},
        state_graph::context::{CanvasContextMenu, NodeContextMenu, TransitionContextMenu},
        transition::{TransitionBuilder, TransitionMessage, TransitionView},
        NORMAL_BACKGROUND, NORMAL_ROOT_COLOR, SELECTED_BACKGROUND, SELECTED_ROOT_COLOR,
    },
    send_sync_message, Message,
};
use fyrox::animation::machine::{MachineLayer, State, Transition};
use fyrox::scene::animation::absm::AnimationBlendingStateMachine;
use fyrox::scene::node::Node;
use fyrox::{
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
use std::sync::mpsc::Sender;

mod context;

pub struct StateGraphViewer {
    pub window: Handle<UiNode>,
    pub canvas: Handle<UiNode>,
    canvas_context_menu: CanvasContextMenu,
    node_context_menu: NodeContextMenu,
    transition_context_menu: TransitionContextMenu,
}

fn fetch_state_node_model_handle(handle: Handle<UiNode>, ui: &UserInterface) -> Handle<State> {
    ui.node(handle)
        .query_component::<AbsmNode<State>>()
        .unwrap()
        .model_handle
}

impl StateGraphViewer {
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

    pub fn clear(&self, ui: &UserInterface) {
        for &child in ui.node(self.canvas).children() {
            ui.send_message(WidgetMessage::remove(child, MessageDirection::ToWidget));
        }
    }

    pub fn activate_transition(&self, ui: &UserInterface, transition: Handle<Transition>) {
        if let Some(view_handle) = ui.node(self.canvas).children().iter().cloned().find(|c| {
            ui.node(*c)
                .query_component::<TransitionView>()
                .map_or(false, |transition_view_ref| {
                    transition_view_ref.model_handle == transition
                })
        }) {
            ui.send_message(TransitionMessage::activate(
                view_handle,
                MessageDirection::ToWidget,
            ));
        }
    }

    pub fn activate_state(&self, ui: &UserInterface, state: Handle<State>) {
        for (state_view_handle, state_view_ref) in ui
            .node(self.canvas)
            .children()
            .iter()
            .cloned()
            .filter_map(|c| {
                ui.node(c)
                    .query_component::<AbsmNode<State>>()
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

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        sender: &Sender<Message>,
        absm_node_handle: Handle<Node>,
        absm_node: &AnimationBlendingStateMachine,
        layer_index: usize,
        editor_scene: &EditorScene,
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

                            sender
                                .send(Message::do_scene_command(AddTransitionCommand::new(
                                    absm_node_handle,
                                    layer_index,
                                    Transition::new("Transition", source, dest, 1.0, ""),
                                )))
                                .unwrap();
                        }
                    }
                    AbsmCanvasMessage::CommitDrag { entries } => {
                        let commands = entries
                            .iter()
                            .map(|e| {
                                let state_handle = fetch_state_node_model_handle(e.node, ui);
                                let new_position = ui.node(e.node).actual_local_position();

                                SceneCommand::new(MoveStateNodeCommand::new(
                                    absm_node_handle,
                                    state_handle,
                                    layer_index,
                                    e.initial_position,
                                    new_position,
                                ))
                            })
                            .collect::<Vec<_>>();

                        sender
                            .send(Message::do_scene_command(CommandGroup::from(commands)))
                            .unwrap();
                    }
                    AbsmCanvasMessage::SelectionChanged(selection) => {
                        if message.direction() == MessageDirection::FromWidget {
                            let selection = Selection::Absm(AbsmSelection {
                                absm_node_handle,
                                layer: layer_index,
                                entities: selection
                                    .iter()
                                    .filter_map(|n| {
                                        let node_ref = ui.node(*n);

                                        if let Some(state_node) =
                                            node_ref.query_component::<AbsmNode<State>>()
                                        {
                                            Some(SelectedEntity::State(state_node.model_handle))
                                        } else {
                                            node_ref.query_component::<TransitionView>().map(
                                                |state_node| {
                                                    SelectedEntity::Transition(
                                                        state_node.model_handle,
                                                    )
                                                },
                                            )
                                        }
                                    })
                                    .collect::<Vec<_>>(),
                            });

                            if !selection.is_empty() && selection != editor_scene.selection {
                                sender
                                    .send(Message::do_scene_command(ChangeSelectionCommand::new(
                                        selection,
                                        editor_scene.selection.clone(),
                                    )))
                                    .unwrap();
                            }
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
            absm_node,
            layer_index,
            editor_scene,
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
            editor_scene,
        );
    }

    pub fn sync_to_model(
        &mut self,
        machine_layer: &MachineLayer,
        ui: &mut UserInterface,
        editor_scene: &EditorScene,
    ) {
        let canvas = ui
            .node(self.canvas)
            .cast::<AbsmCanvas>()
            .expect("Must be AbsmCanvas!");

        let mut states = canvas
            .children()
            .iter()
            .cloned()
            .filter(|c| ui.node(*c).has_component::<AbsmNode<State>>())
            .collect::<Vec<_>>();

        let mut transitions = canvas
            .children()
            .iter()
            .cloned()
            .filter(|c| ui.node(*c).has_component::<TransitionView>())
            .collect::<Vec<_>>();

        match states
            .len()
            .cmp(&(machine_layer.states().alive_count() as usize))
        {
            Ordering::Less => {
                // A state was added.
                for (state_handle, state) in machine_layer.states().pair_iter() {
                    if states.iter().all(|state_view| {
                        ui.node(*state_view)
                            .query_component::<AbsmNode<State>>()
                            .unwrap()
                            .model_handle
                            != state_handle
                    }) {
                        let state_view_handle = AbsmNodeBuilder::new(
                            WidgetBuilder::new()
                                .with_context_menu(self.node_context_menu.menu)
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
                                .query_component::<AbsmNode<State>>()
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
                .query_component::<AbsmNode<State>>()
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
        ui.update(ui.screen_size(), 0.0);

        // Sync transitions.
        match transitions
            .len()
            .cmp(&(machine_layer.transitions().alive_count() as usize))
        {
            Ordering::Less => {
                // A transition was added.
                for (transition_handle, transition) in machine_layer.transitions().pair_iter() {
                    if transitions.iter().all(|transition_view| {
                        ui.node(*transition_view)
                            .query_component::<TransitionView>()
                            .unwrap()
                            .model_handle
                            != transition_handle
                    }) {
                        fn find_state_view(
                            state_handle: Handle<State>,
                            states: &[Handle<UiNode>],
                            ui: &UserInterface,
                        ) -> Handle<UiNode> {
                            states
                                .iter()
                                .find(|s| {
                                    ui.node(**s)
                                        .query_component::<AbsmNode<State>>()
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
                        .with_source(find_state_view(transition.source(), &states, ui))
                        .with_dest(find_state_view(transition.dest(), &states, ui))
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
                                .query_component::<TransitionView>()
                                .unwrap()
                                .model_handle,
                        )
                    })
                {
                    if machine_layer
                        .transitions()
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
        let new_selection = if let Selection::Absm(ref selection) = editor_scene.selection {
            selection
                .entities
                .iter()
                .filter_map(|entry| match entry {
                    SelectedEntity::Transition(transition) => {
                        transitions.iter().cloned().find(|t| {
                            ui.node(*t)
                                .query_component::<TransitionView>()
                                .unwrap()
                                .model_handle
                                == *transition
                        })
                    }
                    SelectedEntity::State(state) => states.iter().cloned().find(|s| {
                        ui.node(*s)
                            .query_component::<AbsmNode<State>>()
                            .unwrap()
                            .model_handle
                            == *state
                    }),
                    SelectedEntity::PoseNode(_) => {
                        // No such nodes possible to have on this canvas.
                        None
                    }
                })
                .collect::<Vec<_>>()
        } else {
            Default::default()
        };

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
