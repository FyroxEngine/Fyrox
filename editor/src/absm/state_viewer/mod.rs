use crate::absm::command::ChangeSelectionCommand;
use crate::absm::SelectedEntity;
use crate::{
    absm::{
        canvas::{AbsmCanvasBuilder, AbsmCanvasMessage},
        command::{AbsmCommand, CommandGroup, MovePoseNodeCommand},
        message::MessageSender,
        node::{AbsmNode, AbsmNodeBuilder},
        socket::SocketBuilder,
        state_viewer::context::{CanvasContextMenu, NodeContextMenu},
        AbsmDataModel,
    },
    send_sync_message,
};
use fyrox::{
    animation::machine::{node::PoseNodeDefinition, state::StateDefinition, MachineDefinition},
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

pub struct StateViewer {
    pub window: Handle<UiNode>,
    canvas: Handle<UiNode>,
    state: Handle<StateDefinition>,
    canvas_context_menu: CanvasContextMenu,
    node_context_menu: NodeContextMenu,
}

fn create_sockets(
    count: usize,
    parent_node: Handle<PoseNodeDefinition>,
    ui: &mut UserInterface,
) -> Vec<Handle<UiNode>> {
    (0..count)
        .map(|_| {
            SocketBuilder::new(WidgetBuilder::new())
                .with_parent_node(parent_node)
                .build(&mut ui.build_ctx())
        })
        .collect::<Vec<_>>()
}

fn fetch_pose_node_model_handle(
    handle: Handle<UiNode>,
    ui: &UserInterface,
) -> Handle<PoseNodeDefinition> {
    ui.node(handle)
        .query_component::<AbsmNode<PoseNodeDefinition>>()
        .unwrap()
        .model_handle
}

impl StateViewer {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let mut node_context_menu = NodeContextMenu::new(ctx);
        let mut canvas_context_menu = CanvasContextMenu::new(ctx);

        let canvas = AbsmCanvasBuilder::new(
            WidgetBuilder::new().with_context_menu(canvas_context_menu.menu),
        )
        .build(ctx);
        let window = WindowBuilder::new(WidgetBuilder::new())
            .can_close(false)
            .can_minimize(false)
            .with_title(WindowTitle::text("State Viewer"))
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
            state: Default::default(),
            canvas_context_menu,
            node_context_menu,
        }
    }

    pub fn set_state(&mut self, state: Handle<StateDefinition>) {
        self.state = state;
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
                    AbsmCanvasMessage::CommitDrag { entries } => {
                        let commands = entries
                            .iter()
                            .map(|e| {
                                let pose_handle = fetch_pose_node_model_handle(e.node, ui);
                                let new_position = ui.node(e.node).actual_local_position();

                                AbsmCommand::new(MovePoseNodeCommand::new(
                                    pose_handle,
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
                                        node_ref.query_component::<AbsmNode<PoseNodeDefinition>>()
                                    {
                                        Some(SelectedEntity::PoseNode(state_node.model_handle))
                                    } else {
                                        None
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

        self.node_context_menu.handle_ui_message(message, ui);
        self.canvas_context_menu
            .handle_ui_message(sender, message, self.state, ui);
    }

    pub fn sync_to_model(
        &mut self,
        definition: &MachineDefinition,
        ui: &mut UserInterface,
        data_model: &AbsmDataModel,
    ) {
        let canvas = ui.node(self.canvas);

        let mut views = canvas
            .children()
            .iter()
            .cloned()
            .filter(|h| {
                if let Some(pose_node) = ui
                    .node(*h)
                    .query_component::<AbsmNode<PoseNodeDefinition>>()
                {
                    definition.nodes[pose_node.model_handle].parent_state == self.state
                } else {
                    false
                }
            })
            .collect::<Vec<_>>();

        let models = definition
            .nodes
            .pair_iter()
            .filter_map(|(h, n)| {
                if n.parent_state == self.state {
                    Some(h)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        match views.len().cmp(&models.len()) {
            Ordering::Less => {
                // A node was added.
                for &pose_definition in models.iter() {
                    if views.iter().all(|v| {
                        ui.node(*v)
                            .query_component::<AbsmNode<PoseNodeDefinition>>()
                            .unwrap()
                            .model_handle
                            != pose_definition
                    }) {
                        let node_ref = &definition.nodes[pose_definition];

                        let (input_socket_count, name) = match node_ref {
                            PoseNodeDefinition::PlayAnimation(_) => {
                                // No input sockets
                                (0, "Play Animation")
                            }
                            PoseNodeDefinition::BlendAnimations(blend_animations) => {
                                (blend_animations.pose_sources.len(), "Blend Animations")
                            }
                            PoseNodeDefinition::BlendAnimationsByIndex(blend_animations) => {
                                (blend_animations.inputs.len(), "Blend Animations By Index")
                            }
                        };

                        // Every node has only one output socket.
                        let output_socket_count = 1;

                        let node_view = AbsmNodeBuilder::new(
                            WidgetBuilder::new()
                                .with_desired_position(node_ref.position)
                                .with_context_menu(self.node_context_menu.menu),
                        )
                        .with_name(name.to_owned())
                        .with_input_sockets(create_sockets(input_socket_count, pose_definition, ui))
                        .with_output_sockets(create_sockets(
                            output_socket_count,
                            pose_definition,
                            ui,
                        ))
                        .with_model_handle(pose_definition)
                        .build(&mut ui.build_ctx());

                        send_sync_message(
                            ui,
                            WidgetMessage::link(node_view, MessageDirection::ToWidget, self.canvas),
                        );

                        views.push(node_view);
                    }
                }
            }
            Ordering::Greater => {
                // A node was removed.
                for &view in views.clone().iter() {
                    let view_ref = ui
                        .node(view)
                        .query_component::<AbsmNode<PoseNodeDefinition>>()
                        .unwrap();

                    if definition
                        .nodes
                        .pair_iter()
                        .all(|(h, _)| view_ref.model_handle != h)
                    {
                        send_sync_message(
                            ui,
                            WidgetMessage::remove(view, MessageDirection::ToWidget),
                        );

                        if let Some(position) = views.iter().position(|s| *s == view) {
                            views.remove(position);
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
                SelectedEntity::Transition(_) | SelectedEntity::State(_) => {
                    // No such nodes possible to have on this canvas.
                    None
                }
                SelectedEntity::PoseNode(pose_node) => views.iter().cloned().find(|s| {
                    ui.node(*s)
                        .query_component::<AbsmNode<PoseNodeDefinition>>()
                        .unwrap()
                        .model_handle
                        == *pose_node
                }),
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
    }
}
