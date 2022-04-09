use crate::{
    absm::{
        canvas::AbsmCanvasBuilder,
        node::{AbsmNode, AbsmNodeBuilder},
        socket::SocketBuilder,
    },
    send_sync_message,
};
use fyrox::{
    animation::machine::{node::PoseNodeDefinition, state::StateDefinition, MachineDefinition},
    core::pool::Handle,
    gui::{
        border::BorderBuilder,
        message::MessageDirection,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowTitle},
        BuildContext, Thickness, UiNode, UserInterface,
    },
};
use std::cmp::Ordering;

pub struct StateViewer {
    pub window: Handle<UiNode>,
    canvas: Handle<UiNode>,
    state: Handle<StateDefinition>,
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

impl StateViewer {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let canvas = AbsmCanvasBuilder::new(WidgetBuilder::new()).build(ctx);
        let window = WindowBuilder::new(WidgetBuilder::new())
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

        Self {
            window,
            canvas,
            state: Default::default(),
        }
    }

    pub fn set_state(&mut self, state: Handle<StateDefinition>) {
        self.state = state;
    }

    pub fn sync_to_model(&mut self, definition: &MachineDefinition, ui: &mut UserInterface) {
        let canvas = ui.node(self.canvas);

        let views = canvas
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
                        let input_socket_count = match &definition.nodes[pose_definition] {
                            PoseNodeDefinition::PlayAnimation(_) => {
                                // No input sockets
                                0
                            }
                            PoseNodeDefinition::BlendAnimations(blend_animations) => {
                                blend_animations.pose_sources.len()
                            }
                            PoseNodeDefinition::BlendAnimationsByIndex(blend_animations) => {
                                blend_animations.inputs.len()
                            }
                        };

                        // Every node has only one output socket.
                        let output_socket_count = 1;

                        AbsmNodeBuilder::new(WidgetBuilder::new())
                            .with_input_sockets(create_sockets(
                                input_socket_count,
                                pose_definition,
                                ui,
                            ))
                            .with_output_sockets(create_sockets(
                                output_socket_count,
                                pose_definition,
                                ui,
                            ))
                            .with_model_handle(pose_definition)
                            .build(&mut ui.build_ctx());
                    }
                }
            }
            Ordering::Greater => {
                // A node was removed.
                for &view in views.iter() {
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
                    }
                }
            }
            Ordering::Equal => {}
        }
    }
}
