use crate::absm::command::blend::SetBlendAnimationsPoseSourceCommand;
use crate::{
    absm::{
        canvas::{AbsmCanvasBuilder, AbsmCanvasMessage},
        command::{
            blend::SetBlendAnimationByIndexInputPoseSourceCommand, AbsmCommand,
            ChangeSelectionCommand, CommandGroup, MovePoseNodeCommand,
        },
        connection::{Connection, ConnectionBuilder},
        message::MessageSender,
        node::{AbsmNode, AbsmNodeBuilder, AbsmNodeMessage},
        socket::{Socket, SocketBuilder, SocketDirection},
        state_viewer::context::{CanvasContextMenu, ConnectionContextMenu, NodeContextMenu},
        AbsmDataModel, SelectedEntity, NORMAL_BACKGROUND, NORMAL_ROOT_COLOR, SELECTED_BACKGROUND,
        SELECTED_ROOT_COLOR,
    },
    send_sync_message,
};
use fyrox::{
    animation::machine::{node::PoseNodeDefinition, state::StateDefinition},
    core::pool::Handle,
    gui::{
        border::BorderBuilder,
        message::{MessageDirection, UiMessage},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
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
    connection_context_menu: ConnectionContextMenu,
}

fn create_socket(
    direction: SocketDirection,
    index: usize,
    parent_node: Handle<PoseNodeDefinition>,
    ui: &mut UserInterface,
) -> Handle<UiNode> {
    SocketBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(2.0)))
        .with_direction(direction)
        .with_parent_node(parent_node)
        .with_index(index)
        .build(&mut ui.build_ctx())
}

fn create_sockets(
    count: usize,
    direction: SocketDirection,
    parent_node: Handle<PoseNodeDefinition>,
    ui: &mut UserInterface,
) -> Vec<Handle<UiNode>> {
    (0..count)
        .map(|index| create_socket(direction, index, parent_node, ui))
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

fn fetch_socket_pose_node_model_handle(
    handle: Handle<UiNode>,
    ui: &UserInterface,
) -> Handle<PoseNodeDefinition> {
    ui.node(handle)
        .query_component::<Socket>()
        .unwrap()
        .parent_node
}

impl StateViewer {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let mut node_context_menu = NodeContextMenu::new(ctx);
        let mut canvas_context_menu = CanvasContextMenu::new(ctx);
        let connection_context_menu = ConnectionContextMenu::new(ctx);

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
            connection_context_menu,
        }
    }

    pub fn set_state(
        &mut self,
        state: Handle<StateDefinition>,
        data_model: &AbsmDataModel,
        ui: &UserInterface,
    ) {
        assert!(state.is_some());

        self.state = state;

        let (state_name, exists) = data_model
            .resource
            .data_ref()
            .absm_definition
            .states
            .try_borrow(self.state)
            .map(|state| {
                (
                    format!(
                        "{} ({}:{})",
                        state.name,
                        self.state.index(),
                        self.state.generation()
                    ),
                    true,
                )
            })
            .unwrap_or_else(|| (String::from("<No State>"), false));

        ui.send_message(WindowMessage::title(
            self.window,
            MessageDirection::ToWidget,
            WindowTitle::text(format!("State Viewer - {}", state_name)),
        ));

        ui.send_message(WidgetMessage::enabled(
            self.canvas_context_menu.menu,
            MessageDirection::ToWidget,
            exists,
        ));
    }

    pub fn clear(&mut self, ui: &UserInterface) {
        self.state = Handle::NONE;

        for &child in ui.node(self.canvas).children() {
            ui.send_message(WidgetMessage::remove(child, MessageDirection::ToWidget));
        }

        ui.send_message(WindowMessage::title(
            self.window,
            MessageDirection::ToWidget,
            WindowTitle::text("State Viewer - No State"),
        ));

        ui.send_message(WidgetMessage::enabled(
            self.canvas_context_menu.menu,
            MessageDirection::ToWidget,
            false,
        ));
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        sender: &MessageSender,
        data_model: &AbsmDataModel,
    ) {
        let definition = &data_model.resource.data_ref().absm_definition;

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

                                    node_ref
                                        .query_component::<AbsmNode<PoseNodeDefinition>>()
                                        .map(|state_node| {
                                            SelectedEntity::PoseNode(state_node.model_handle)
                                        })
                                })
                                .collect::<Vec<_>>();

                            if !selection.is_empty() && selection != data_model.selection {
                                sender.do_command(ChangeSelectionCommand { selection });
                            }
                        }
                    }
                    AbsmCanvasMessage::CommitConnection {
                        source_socket,
                        dest_socket,
                    } => {
                        let source_node = fetch_socket_pose_node_model_handle(*source_socket, ui);

                        let dest_socket_ref =
                            ui.node(*dest_socket).query_component::<Socket>().unwrap();
                        let dest_node = fetch_socket_pose_node_model_handle(*dest_socket, ui);

                        let dest_node_ref = &definition.nodes[dest_node];
                        match dest_node_ref {
                            PoseNodeDefinition::PlayAnimation(_) => {}
                            PoseNodeDefinition::BlendAnimations(_) => {
                                sender.do_command(SetBlendAnimationsPoseSourceCommand {
                                    handle: dest_node,
                                    index: dest_socket_ref.index,
                                    value: source_node,
                                });
                            }
                            PoseNodeDefinition::BlendAnimationsByIndex(_) => {
                                sender.do_command(SetBlendAnimationByIndexInputPoseSourceCommand {
                                    handle: dest_node,
                                    index: dest_socket_ref.index,
                                    value: source_node,
                                });
                            }
                        }
                    }
                    _ => (),
                }
            }
        }

        self.node_context_menu.handle_ui_message(
            message,
            &data_model.selection,
            definition,
            sender,
            ui,
        );
        self.canvas_context_menu
            .handle_ui_message(sender, message, self.state, ui);
        self.connection_context_menu
            .handle_ui_message(message, ui, sender, definition);
    }

    pub fn sync_to_model(&mut self, ui: &mut UserInterface, data_model: &AbsmDataModel) {
        if self.state.is_none() {
            return;
        }

        let definition = &data_model.resource.data_ref().absm_definition;

        let parent_state_ref = &definition.states[self.state];

        let mut views = ui
            .node(self.canvas)
            .children()
            .iter()
            .cloned()
            .filter(|h| {
                if let Some(pose_node) = ui
                    .node(*h)
                    .query_component::<AbsmNode<PoseNodeDefinition>>()
                {
                    if definition
                        .nodes
                        .try_borrow(pose_node.model_handle)
                        .map_or(false, |node| node.parent_state == self.state)
                    {
                        true
                    } else {
                        // Remove every node that does not belong to a state or its data model was
                        // removed.
                        ui.send_message(WidgetMessage::remove(*h, MessageDirection::ToWidget));

                        false
                    }
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

                        let (input_socket_count, name, can_add_sockets) = match node_ref {
                            PoseNodeDefinition::PlayAnimation(_) => {
                                // No input sockets
                                (0, "Play Animation", false)
                            }
                            PoseNodeDefinition::BlendAnimations(blend_animations) => (
                                blend_animations.pose_sources.len(),
                                "Blend Animations",
                                true,
                            ),
                            PoseNodeDefinition::BlendAnimationsByIndex(blend_animations) => (
                                blend_animations.inputs.len(),
                                "Blend Animations By Index",
                                true,
                            ),
                        };

                        let node_view = AbsmNodeBuilder::new(
                            WidgetBuilder::new()
                                .with_desired_position(node_ref.position)
                                .with_context_menu(self.node_context_menu.menu),
                        )
                        .with_name("".to_owned())
                        .with_title(name.to_owned())
                        .with_can_add_sockets(can_add_sockets)
                        .with_input_sockets(create_sockets(
                            input_socket_count,
                            SocketDirection::Input,
                            pose_definition,
                            ui,
                        ))
                        .with_output_socket(create_socket(
                            SocketDirection::Output,
                            0,
                            pose_definition,
                            ui,
                        ))
                        .with_normal_color(if pose_definition == parent_state_ref.root {
                            NORMAL_ROOT_COLOR
                        } else {
                            NORMAL_BACKGROUND
                        })
                        .with_selected_color(if pose_definition == parent_state_ref.root {
                            SELECTED_ROOT_COLOR
                        } else {
                            SELECTED_BACKGROUND
                        })
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

        // Sync nodes.
        for &view in &views {
            let view_ref = ui
                .node(view)
                .query_component::<AbsmNode<PoseNodeDefinition>>()
                .unwrap();
            let model_handle = view_ref.model_handle;
            let model_ref = &definition.nodes[model_handle];
            let children = model_ref.children();
            let position = view_ref.actual_local_position();

            if view_ref.base.input_sockets.len() != children.len() {
                let input_sockets = create_sockets(
                    children.len(),
                    SocketDirection::Input,
                    view_ref.model_handle,
                    ui,
                );

                send_sync_message(
                    ui,
                    AbsmNodeMessage::input_sockets(view, MessageDirection::ToWidget, input_sockets),
                );
            }

            if position != model_ref.position {
                send_sync_message(
                    ui,
                    WidgetMessage::desired_position(
                        view,
                        MessageDirection::ToWidget,
                        model_ref.position,
                    ),
                );
            }

            if model_ref.parent_state == self.state {
                send_sync_message(
                    ui,
                    AbsmNodeMessage::normal_color(
                        view,
                        MessageDirection::ToWidget,
                        if model_handle == parent_state_ref.root {
                            NORMAL_ROOT_COLOR
                        } else {
                            NORMAL_BACKGROUND
                        },
                    ),
                );
                send_sync_message(
                    ui,
                    AbsmNodeMessage::selected_color(
                        view,
                        MessageDirection::ToWidget,
                        if model_handle == parent_state_ref.root {
                            SELECTED_ROOT_COLOR
                        } else {
                            SELECTED_BACKGROUND
                        },
                    ),
                );
            }
        }

        // Force update layout to be able to fetch positions of nodes for transitions.
        ui.update(ui.screen_size(), 0.0);

        // Sync connections - remove old ones and create new. Since there is no separate data model
        // for connection we can't find which connection has changed and sync only it, instead we
        // removing every connection and create new.
        for child in ui.node(self.canvas).children().iter().cloned() {
            if ui.node(child).has_component::<Connection>() {
                ui.send_message(WidgetMessage::remove(child, MessageDirection::ToWidget));
            }
        }

        for model in models.iter().cloned() {
            let dest_ref = views
                .iter()
                .filter_map(|v| {
                    ui.node(*v)
                        .query_component::<AbsmNode<PoseNodeDefinition>>()
                })
                .find(|v| v.model_handle == model)
                .unwrap();
            let dest_handle = dest_ref.handle();
            let input_sockets = dest_ref.base.input_sockets.clone();

            let model_ref = &definition.nodes[model];
            for (i, child) in model_ref.children().into_iter().enumerate() {
                // Sanity check.
                assert_ne!(child, model);

                if definition.nodes.is_valid_handle(child) {
                    let source = views
                        .iter()
                        .filter_map(|v| {
                            ui.node(*v)
                                .query_component::<AbsmNode<PoseNodeDefinition>>()
                        })
                        .find(|v| v.model_handle == child)
                        .unwrap();

                    let connection = ConnectionBuilder::new(
                        WidgetBuilder::new().with_context_menu(self.connection_context_menu.menu),
                    )
                    .with_source_socket(source.base.output_socket)
                    .with_source_node(source.handle())
                    .with_dest_socket(input_sockets[i])
                    .with_dest_node(dest_handle)
                    .build(self.canvas, &mut ui.build_ctx());

                    send_sync_message(
                        ui,
                        WidgetMessage::link(connection, MessageDirection::ToWidget, self.canvas),
                    );
                    send_sync_message(
                        ui,
                        WidgetMessage::lowermost(connection, MessageDirection::ToWidget),
                    );
                }
            }
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

        send_sync_message(
            ui,
            AbsmCanvasMessage::force_sync_dependent_objects(
                self.canvas,
                MessageDirection::ToWidget,
            ),
        );
    }
}
