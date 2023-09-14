use crate::{
    absm::{
        command::{
            blend::{
                SetBlendAnimationByIndexInputPoseSourceCommand,
                SetBlendAnimationsPoseSourceCommand, SetBlendSpacePoseSourceCommand,
            },
            AddPoseNodeCommand, DeletePoseNodeCommand, SetStateRootPoseCommand,
        },
        connection::Connection,
        node::AbsmNode,
        selection::SelectedEntity,
    },
    menu::create_menu_item,
    message::MessageSender,
    scene::{
        commands::{ChangeSelectionCommand, CommandGroup, SceneCommand},
        EditorScene, Selection,
    },
};
use fyrox::{
    animation::machine::{
        node::{
            blendspace::{BlendSpace, BlendSpacePoint},
            BasePoseNode,
        },
        BlendAnimations, BlendAnimationsByIndex, MachineLayer, PlayAnimation, PoseNode, State,
    },
    core::{algebra::Vector2, pool::Handle},
    gui::{
        menu::MenuItemMessage,
        message::UiMessage,
        popup::{Placement, PopupBuilder, PopupMessage},
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        BuildContext, RcUiNodeHandle, UiNode, UserInterface,
    },
    scene::node::Node,
};

pub struct CanvasContextMenu {
    create_play_animation: Handle<UiNode>,
    create_blend_animations: Handle<UiNode>,
    create_blend_by_index: Handle<UiNode>,
    create_blend_space: Handle<UiNode>,
    pub menu: RcUiNodeHandle,
    pub canvas: Handle<UiNode>,
    pub node_context_menu: Option<RcUiNodeHandle>,
}

impl CanvasContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_play_animation;
        let create_blend_animations;
        let create_blend_by_index;
        let create_blend_space;
        let menu = PopupBuilder::new(
            WidgetBuilder::new()
                .with_enabled(false) // Disabled by default.
                .with_visibility(false),
        )
        .with_content(
            StackPanelBuilder::new(
                WidgetBuilder::new()
                    .with_child({
                        create_play_animation = create_menu_item("Play Animation", vec![], ctx);
                        create_play_animation
                    })
                    .with_child({
                        create_blend_animations = create_menu_item("Blend Animations", vec![], ctx);
                        create_blend_animations
                    })
                    .with_child({
                        create_blend_by_index = create_menu_item("Blend By Index", vec![], ctx);
                        create_blend_by_index
                    })
                    .with_child({
                        create_blend_space = create_menu_item("Blend Space", vec![], ctx);
                        create_blend_space
                    }),
            )
            .build(ctx),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            create_play_animation,
            create_blend_animations,
            create_blend_by_index,
            create_blend_space,
            menu,
            canvas: Default::default(),
            node_context_menu: Default::default(),
        }
    }

    pub fn handle_ui_message(
        &mut self,
        sender: &MessageSender,
        message: &UiMessage,
        current_state: Handle<State>,
        ui: &mut UserInterface,
        absm_node_handle: Handle<Node>,
        layer_index: usize,
    ) {
        if let Some(MenuItemMessage::Click) = message.data() {
            let position = ui
                .node(self.canvas)
                .screen_to_local(ui.node(*self.menu).screen_position());

            let pose_node = if message.destination() == self.create_play_animation {
                Some(PoseNode::PlayAnimation(PlayAnimation {
                    base: BasePoseNode {
                        position,
                        parent_state: current_state,
                    },
                    animation: Default::default(),
                    output_pose: Default::default(),
                }))
            } else if message.destination() == self.create_blend_animations {
                Some(PoseNode::BlendAnimations(BlendAnimations {
                    base: BasePoseNode {
                        position,
                        parent_state: current_state,
                    },
                    pose_sources: Default::default(),
                    output_pose: Default::default(),
                }))
            } else if message.destination() == self.create_blend_by_index {
                Some(PoseNode::BlendAnimationsByIndex(BlendAnimationsByIndex {
                    base: BasePoseNode {
                        position,
                        parent_state: current_state,
                    },
                    index_parameter: "".to_string(),
                    inputs: Default::default(),
                    prev_index: Default::default(),
                    blend_time: Default::default(),
                    output_pose: Default::default(),
                }))
            } else if message.destination() == self.create_blend_space {
                let mut blend_space = BlendSpace::default();

                blend_space.position = position;
                blend_space.parent_state = current_state;
                blend_space.set_points(vec![
                    BlendSpacePoint {
                        position: Vector2::zeros(),
                        pose_source: Default::default(),
                    },
                    BlendSpacePoint {
                        position: Vector2::new(1.0, 0.0),
                        pose_source: Default::default(),
                    },
                    BlendSpacePoint {
                        position: Vector2::new(1.0, 1.0),
                        pose_source: Default::default(),
                    },
                    BlendSpacePoint {
                        position: Vector2::new(0.5, 0.5),
                        pose_source: Default::default(),
                    },
                ]);

                Some(PoseNode::BlendSpace(blend_space))
            } else {
                None
            };

            if let Some(pose_node) = pose_node {
                sender.do_scene_command(AddPoseNodeCommand::new(
                    absm_node_handle,
                    layer_index,
                    pose_node,
                ));
            }
        }
    }
}

pub struct NodeContextMenu {
    remove: Handle<UiNode>,
    set_as_root: Handle<UiNode>,
    pub menu: RcUiNodeHandle,
    pub canvas: Handle<UiNode>,
    placement_target: Handle<UiNode>,
}

impl NodeContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let remove;
        let set_as_root;
        let menu = PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
            .with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            set_as_root = create_menu_item("Set As Root", vec![], ctx);
                            set_as_root
                        })
                        .with_child({
                            remove = create_menu_item("Remove", vec![], ctx);
                            remove
                        }),
                )
                .build(ctx),
            )
            .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            remove,
            set_as_root,
            menu,
            canvas: Default::default(),
            placement_target: Default::default(),
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        machine_layer: &MachineLayer,
        sender: &MessageSender,
        ui: &UserInterface,
        editor_scene: &EditorScene,
        absm_node_handle: Handle<Node>,
        layer_index: usize,
    ) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.remove {
                if let Selection::Absm(ref selection) = editor_scene.selection {
                    let mut new_selection = selection.clone();
                    new_selection.entities.clear();

                    let mut group = vec![SceneCommand::new(ChangeSelectionCommand::new(
                        Selection::Absm(new_selection),
                        editor_scene.selection.clone(),
                    ))];

                    group.extend(selection.entities.iter().filter_map(|entry| {
                        if let SelectedEntity::PoseNode(pose_node) = entry {
                            Some(SceneCommand::new(DeletePoseNodeCommand::new(
                                absm_node_handle,
                                layer_index,
                                *pose_node,
                            )))
                        } else {
                            None
                        }
                    }));

                    sender.do_scene_command(CommandGroup::from(group));
                }
            } else if message.destination() == self.set_as_root {
                let root = ui
                    .node(self.placement_target)
                    .query_component::<AbsmNode<PoseNode>>()
                    .unwrap()
                    .model_handle;

                sender.do_scene_command(SetStateRootPoseCommand {
                    node_handle: absm_node_handle,
                    layer_index,
                    handle: machine_layer.node(root).parent_state,
                    value: root,
                });
            }
        } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == *self.menu {
                self.placement_target = *target;
            }
        }
    }
}

pub struct ConnectionContextMenu {
    remove: Handle<UiNode>,
    pub menu: RcUiNodeHandle,
    placement_target: Handle<UiNode>,
}

impl ConnectionContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let remove;
        let menu = PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
            .with_content(
                StackPanelBuilder::new(WidgetBuilder::new().with_child({
                    remove = create_menu_item("Remove Connection", vec![], ctx);
                    remove
                }))
                .build(ctx),
            )
            .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            menu,
            remove,
            placement_target: Default::default(),
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        sender: &MessageSender,
        machine_layer: &MachineLayer,
        absm_node_handle: Handle<Node>,
        layer_index: usize,
    ) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination == self.remove {
                let connection_ref = ui
                    .node(self.placement_target)
                    .query_component::<Connection>()
                    .unwrap();

                let dest_node_ref = ui
                    .node(connection_ref.dest_node)
                    .query_component::<AbsmNode<PoseNode>>()
                    .unwrap();

                let index = dest_node_ref
                    .base
                    .input_sockets
                    .iter()
                    .position(|s| *s == connection_ref.segment.dest)
                    .unwrap();

                let model_handle = dest_node_ref.model_handle;
                match machine_layer.node(model_handle) {
                    PoseNode::PlayAnimation(_) => {
                        // No connections
                    }
                    PoseNode::BlendAnimations(_) => {
                        sender.do_scene_command(SetBlendAnimationsPoseSourceCommand {
                            node_handle: absm_node_handle,
                            layer_index,
                            handle: model_handle,
                            index,
                            value: Default::default(),
                        })
                    }
                    PoseNode::BlendAnimationsByIndex(_) => {
                        sender.do_scene_command(SetBlendAnimationByIndexInputPoseSourceCommand {
                            node_handle: absm_node_handle,
                            layer_index,
                            handle: model_handle,
                            index,
                            value: Default::default(),
                        })
                    }
                    PoseNode::BlendSpace(_) => {
                        sender.do_scene_command(SetBlendSpacePoseSourceCommand {
                            node_handle: absm_node_handle,
                            layer_index,
                            handle: model_handle,
                            index,
                            value: Default::default(),
                        })
                    }
                }
            }
        } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == *self.menu {
                self.placement_target = *target;
            }
        }
    }
}
