use crate::command::{Command, CommandGroup};
use crate::fyrox::generic_animation::machine::Machine;
use crate::fyrox::generic_animation::AnimationContainer;
use crate::fyrox::graph::{PrefabData, SceneGraphNode};
use crate::fyrox::{
    core::pool::{ErasedHandle, Handle},
    generic_animation::{
        machine::{MachineLayer, PoseNode, State},
        Animation,
    },
    graph::{BaseSceneGraph, SceneGraph},
    gui::{
        border::BorderBuilder,
        message::{MessageDirection, UiMessage},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Thickness, UiNode, UserInterface,
    },
};
use crate::message::MessageSender;
use crate::{
    absm::{
        canvas::{AbsmCanvasBuilder, AbsmCanvasMessage},
        command::{
            blend::{
                SetBlendAnimationByIndexInputPoseSourceCommand,
                SetBlendAnimationsPoseSourceCommand, SetBlendSpacePoseSourceCommand,
            },
            MovePoseNodeCommand,
        },
        connection::{Connection, ConnectionBuilder},
        fetch_selection,
        node::{AbsmNode, AbsmNodeBuilder, AbsmNodeMessage},
        selection::{AbsmSelection, SelectedEntity},
        socket::{Socket, SocketBuilder, SocketDirection},
        state_viewer::context::{CanvasContextMenu, ConnectionContextMenu, NodeContextMenu},
        NORMAL_BACKGROUND, NORMAL_ROOT_COLOR, SELECTED_BACKGROUND, SELECTED_ROOT_COLOR,
    },
    scene::{commands::ChangeSelectionCommand, Selection},
    send_sync_message,
};
use std::cmp::Ordering;

mod context;

pub struct StateViewer {
    pub window: Handle<UiNode>,
    canvas: Handle<UiNode>,
    state: ErasedHandle,
    canvas_context_menu: CanvasContextMenu,
    node_context_menu: NodeContextMenu,
    connection_context_menu: ConnectionContextMenu,
    prev_absm: ErasedHandle,
    prev_layer: Option<usize>,
}

fn create_socket<N: 'static>(
    direction: SocketDirection,
    index: usize,
    show_index: bool,
    parent_node: Handle<PoseNode<Handle<N>>>,
    ui: &mut UserInterface,
) -> Handle<UiNode> {
    SocketBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(2.0)))
        .with_direction(direction)
        .with_parent_node(parent_node.into())
        .with_index(index)
        .with_show_index(show_index)
        .build(&mut ui.build_ctx())
}

fn create_sockets<N: 'static>(
    count: usize,
    direction: SocketDirection,
    parent_node: Handle<PoseNode<Handle<N>>>,
    ui: &mut UserInterface,
) -> Vec<Handle<UiNode>> {
    (0..count)
        .map(|index| create_socket(direction, index, true, parent_node, ui))
        .collect::<Vec<_>>()
}

fn fetch_pose_node_model_handle<N: 'static>(
    handle: Handle<UiNode>,
    ui: &UserInterface,
) -> Handle<PoseNode<Handle<N>>> {
    ui.node(handle)
        .query_component::<AbsmNode<PoseNode<Handle<N>>>>()
        .unwrap()
        .model_handle
}

fn fetch_socket_pose_node_model_handle<N: 'static>(
    handle: Handle<UiNode>,
    ui: &UserInterface,
) -> Handle<PoseNode<Handle<N>>> {
    ui.node(handle)
        .query_component::<Socket>()
        .unwrap()
        .parent_node
        .into()
}

fn make_play_animation_name<P, G, N>(
    animation_container: Option<&AnimationContainer<Handle<N>>>,
    animation: Handle<Animation<Handle<N>>>,
) -> String
where
    P: PrefabData<Graph = G>,
    G: SceneGraph<Node = N, Prefab = P>,
    N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
{
    if let Some(animation) = animation_container.and_then(|container| container.try_get(animation))
    {
        format!("Play Animation: {}", animation.name())
    } else {
        "Play Animation: <UNASSIGNED>".to_owned()
    }
}

fn make_pose_node_name<P, G, N>(
    model_ref: &PoseNode<Handle<N>>,
    animation_container: Option<&AnimationContainer<Handle<N>>>,
) -> String
where
    P: PrefabData<Graph = G>,
    G: SceneGraph<Node = N, Prefab = P>,
    N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
{
    match model_ref {
        PoseNode::PlayAnimation(play_animation) => {
            make_play_animation_name(animation_container, play_animation.animation)
        }
        PoseNode::BlendAnimations(blend_animations) => {
            format!("Blend {} Animations", blend_animations.pose_sources.len())
        }
        PoseNode::BlendAnimationsByIndex(blend_animations_by_index) => format!(
            "Blend {} Animations By Index",
            blend_animations_by_index.inputs.len()
        ),
        PoseNode::BlendSpace(blend_space) => {
            format!("Blend Space: {:?} animations", blend_space.points().len())
        }
    }
}

impl StateViewer {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let mut node_context_menu = NodeContextMenu::new(ctx);
        let mut canvas_context_menu = CanvasContextMenu::new(ctx);
        let connection_context_menu = ConnectionContextMenu::new(ctx);

        let canvas = AbsmCanvasBuilder::new(
            WidgetBuilder::new().with_context_menu(canvas_context_menu.menu.clone()),
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
        canvas_context_menu.node_context_menu = Some(node_context_menu.menu.clone());
        node_context_menu.canvas = canvas;

        Self {
            window,
            canvas,
            state: Default::default(),
            canvas_context_menu,
            node_context_menu,
            connection_context_menu,
            prev_absm: Default::default(),
            prev_layer: None,
        }
    }

    pub fn set_state<P, G, N>(
        &mut self,
        state: Handle<State<Handle<N>>>,
        machine: &Machine<Handle<N>>,
        layer_index: usize,
        ui: &UserInterface,
    ) where
        P: PrefabData<Graph = G>,
        G: SceneGraph<Node = N, Prefab = P>,
        N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
    {
        assert!(state.is_some());

        self.state = state.into();

        let (state_name, exists) = machine.layers()[layer_index]
            .states()
            .try_borrow(state)
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
            self.canvas_context_menu.menu.handle(),
            MessageDirection::ToWidget,
            exists,
        ));
    }

    pub fn clear(&mut self, ui: &UserInterface) {
        self.state = Default::default();

        for &child in ui.node(self.canvas).children() {
            ui.send_message(WidgetMessage::remove(child, MessageDirection::ToWidget));
        }

        ui.send_message(WindowMessage::title(
            self.window,
            MessageDirection::ToWidget,
            WindowTitle::text("State Viewer - No State"),
        ));

        ui.send_message(WidgetMessage::enabled(
            self.canvas_context_menu.menu.handle(),
            MessageDirection::ToWidget,
            false,
        ));
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
        if let Some(layer) = machine.layers().get(layer_index) {
            if message.destination() == self.canvas {
                if let Some(msg) = message.data::<AbsmCanvasMessage>() {
                    match msg {
                        AbsmCanvasMessage::CommitDrag { entries } => {
                            let commands = entries
                                .iter()
                                .map(|e| {
                                    let pose_handle = fetch_pose_node_model_handle(e.node, ui);
                                    let new_position = ui.node(e.node).actual_local_position();

                                    Command::new(MovePoseNodeCommand::new(
                                        absm_node_handle,
                                        pose_handle,
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

                                            node_ref
                                                .query_component::<AbsmNode<PoseNode<Handle<N>>>>()
                                                .map(|state_node| {
                                                    SelectedEntity::PoseNode(
                                                        state_node.model_handle,
                                                    )
                                                })
                                        })
                                        .collect::<Vec<_>>(),
                                });

                                if !selection.is_empty() && &selection != editor_selection {
                                    sender.do_command(ChangeSelectionCommand::new(selection));
                                }
                            }
                        }
                        AbsmCanvasMessage::CommitConnection {
                            source_socket,
                            dest_socket,
                        } => {
                            let source_node =
                                fetch_socket_pose_node_model_handle(*source_socket, ui);

                            let dest_socket_ref =
                                ui.node(*dest_socket).query_component::<Socket>().unwrap();
                            let dest_node = fetch_socket_pose_node_model_handle(*dest_socket, ui);

                            let dest_node_ref = &layer.nodes()[dest_node];
                            match dest_node_ref {
                                PoseNode::PlayAnimation(_) => {}
                                PoseNode::BlendAnimations(_) => {
                                    sender.do_command(SetBlendAnimationsPoseSourceCommand {
                                        node_handle: absm_node_handle,
                                        layer_index,
                                        handle: dest_node,
                                        index: dest_socket_ref.index,
                                        value: source_node,
                                    });
                                }
                                PoseNode::BlendAnimationsByIndex(_) => {
                                    sender.do_command(
                                        SetBlendAnimationByIndexInputPoseSourceCommand {
                                            node_handle: absm_node_handle,
                                            layer_index,
                                            handle: dest_node,
                                            index: dest_socket_ref.index,
                                            value: source_node,
                                        },
                                    );
                                }
                                PoseNode::BlendSpace(_) => {
                                    sender.do_command(SetBlendSpacePoseSourceCommand {
                                        node_handle: absm_node_handle,
                                        layer_index,
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
                layer,
                sender,
                ui,
                editor_selection,
                absm_node_handle,
                layer_index,
            );
            self.canvas_context_menu.handle_ui_message(
                sender,
                message,
                self.state.into(),
                ui,
                absm_node_handle,
                layer_index,
            );
            self.connection_context_menu.handle_ui_message(
                message,
                ui,
                sender,
                layer,
                absm_node_handle,
                layer_index,
            );
        }
    }

    pub fn sync_to_model<P, G, N>(
        &mut self,
        ui: &mut UserInterface,
        machine_layer: &MachineLayer<Handle<N>>,
        editor_selection: &Selection,
        animation_container: Option<&AnimationContainer<Handle<N>>>,
    ) where
        P: PrefabData<Graph = G>,
        G: SceneGraph<Node = N, Prefab = P>,
        N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
    {
        if let Some(parent_state_ref) = machine_layer.states().try_borrow(self.state.into()) {
            let current_selection = fetch_selection(editor_selection);

            let mut views = Vec::new();
            if self.prev_layer != current_selection.layer
                || current_selection.absm_node_handle != self.prev_absm.into()
            {
                self.prev_layer = current_selection.layer;
                self.prev_absm = current_selection.absm_node_handle.into();
                self.clear(ui);
            } else {
                views = ui
                    .node(self.canvas)
                    .children()
                    .iter()
                    .cloned()
                    .filter(|h| {
                        if let Some(pose_node) = ui
                            .node(*h)
                            .query_component::<AbsmNode<PoseNode<Handle<N>>>>()
                        {
                            if machine_layer
                                .nodes()
                                .try_borrow(pose_node.model_handle)
                                .map_or(false, |node| node.parent_state == self.state.into())
                            {
                                true
                            } else {
                                // Remove every node that does not belong to a state or its data model was
                                // removed.
                                send_sync_message(
                                    ui,
                                    WidgetMessage::remove(*h, MessageDirection::ToWidget),
                                );

                                false
                            }
                        } else {
                            false
                        }
                    })
                    .collect::<Vec<_>>();
            }

            let models = machine_layer
                .nodes()
                .pair_iter()
                .filter_map(|(h, n)| {
                    if n.parent_state == self.state.into() {
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
                                .query_component::<AbsmNode<PoseNode<Handle<N>>>>()
                                .unwrap()
                                .model_handle
                                != pose_definition
                        }) {
                            let node_ref = &machine_layer.nodes()[pose_definition];

                            let (input_socket_count, name, can_add_sockets, editable) =
                                match node_ref {
                                    PoseNode::PlayAnimation(_) => {
                                        // No input sockets
                                        (0, "Play Animation", false, false)
                                    }
                                    PoseNode::BlendAnimations(blend_animations) => (
                                        blend_animations.pose_sources.len(),
                                        "Blend Animations",
                                        true,
                                        false,
                                    ),
                                    PoseNode::BlendAnimationsByIndex(blend_animations) => (
                                        blend_animations.inputs.len(),
                                        "Blend Animations By Index",
                                        true,
                                        false,
                                    ),
                                    PoseNode::BlendSpace(blend_space) => {
                                        (blend_space.points().len(), "Blend Space", true, true)
                                    }
                                };

                            let node_view = AbsmNodeBuilder::new(
                                WidgetBuilder::new()
                                    .with_desired_position(node_ref.position)
                                    .with_context_menu(self.node_context_menu.menu.clone()),
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
                                false,
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
                            .with_editable(editable)
                            .with_model_handle(pose_definition)
                            .build(&mut ui.build_ctx());

                            send_sync_message(
                                ui,
                                WidgetMessage::link(
                                    node_view,
                                    MessageDirection::ToWidget,
                                    self.canvas,
                                ),
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
                            .query_component::<AbsmNode<PoseNode<Handle<N>>>>()
                            .unwrap();

                        if machine_layer
                            .nodes()
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
                    .query_component::<AbsmNode<PoseNode<Handle<N>>>>()
                    .unwrap();
                let model_handle = view_ref.model_handle;
                let model_ref = &machine_layer.nodes()[model_handle];
                let children = model_ref.children();
                let position = view_ref.actual_local_position();

                let new_name = make_pose_node_name(model_ref, animation_container);
                if new_name != view_ref.name_value {
                    send_sync_message(
                        ui,
                        AbsmNodeMessage::name(view, MessageDirection::ToWidget, new_name),
                    );
                }

                if view_ref.base.input_sockets.len() != children.len() {
                    let input_sockets = create_sockets(
                        children.len(),
                        SocketDirection::Input,
                        view_ref.model_handle,
                        ui,
                    );

                    send_sync_message(
                        ui,
                        AbsmNodeMessage::input_sockets(
                            view,
                            MessageDirection::ToWidget,
                            input_sockets,
                        ),
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

                if model_ref.parent_state == self.state.into() {
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
            ui.update_layout(ui.screen_size());

            // Sync connections - remove old ones and create new. Since there is no separate data model
            // for connection we can't find which connection has changed and sync only it, instead we
            // removing every connection and create new.
            for child in ui.node(self.canvas).children().iter().cloned() {
                if ui.node(child).has_component::<Connection>() {
                    send_sync_message(ui, WidgetMessage::remove(child, MessageDirection::ToWidget));
                }
            }

            for model in models.iter().cloned() {
                let dest_ref = views
                    .iter()
                    .filter_map(|v| {
                        ui.node(*v)
                            .query_component::<AbsmNode<PoseNode<Handle<N>>>>()
                    })
                    .find(|v| v.model_handle == model)
                    .unwrap();
                let dest_handle = dest_ref.handle();
                let input_sockets = dest_ref.base.input_sockets.clone();

                let model_ref = &machine_layer.nodes()[model];
                for (i, child) in model_ref.children().into_iter().enumerate() {
                    // Sanity check.
                    assert_ne!(child, model);

                    if machine_layer.nodes().is_valid_handle(child) {
                        let source = views
                            .iter()
                            .filter_map(|v| {
                                ui.node(*v)
                                    .query_component::<AbsmNode<PoseNode<Handle<N>>>>()
                            })
                            .find(|v| v.model_handle == child)
                            .unwrap();

                        let connection = ConnectionBuilder::new(
                            WidgetBuilder::new()
                                .with_context_menu(self.connection_context_menu.menu.clone()),
                        )
                        .with_source_socket(source.base.output_socket)
                        .with_source_node(source.handle())
                        .with_dest_socket(input_sockets[i])
                        .with_dest_node(dest_handle)
                        .build(self.canvas, &mut ui.build_ctx());

                        send_sync_message(
                            ui,
                            WidgetMessage::link(
                                connection,
                                MessageDirection::ToWidget,
                                self.canvas,
                            ),
                        );
                        send_sync_message(
                            ui,
                            WidgetMessage::lowermost(connection, MessageDirection::ToWidget),
                        );
                    }
                }
            }

            // Sync selection.
            let new_selection = current_selection
                .entities
                .iter()
                .filter_map(|entry| match entry {
                    SelectedEntity::Transition(_) | SelectedEntity::State(_) => {
                        // No such nodes possible to have on this canvas.
                        None
                    }
                    SelectedEntity::PoseNode(pose_node) => views.iter().cloned().find(|s| {
                        ui.node(*s)
                            .query_component::<AbsmNode<PoseNode<Handle<N>>>>()
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
        } else {
            // Clean the canvas.
            for child in ui.node(self.canvas).children() {
                send_sync_message(
                    ui,
                    WidgetMessage::remove(*child, MessageDirection::ToWidget),
                );
            }
        }
    }
}
