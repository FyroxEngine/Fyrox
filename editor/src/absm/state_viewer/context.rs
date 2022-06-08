use crate::{
    absm::{
        command::{
            blend::{
                SetBlendAnimationByIndexInputPoseSourceCommand, SetBlendAnimationsPoseSourceCommand,
            },
            AbsmCommand, AddPoseNodeCommand, ChangeSelectionCommand, CommandGroup,
            DeletePoseNodeCommand, SetStateRootPoseCommand,
        },
        connection::Connection,
        message::MessageSender,
        node::AbsmNode,
        SelectedEntity,
    },
    menu::create_menu_item,
};
use fyrox::{
    animation::machine::{
        node::{
            blend::{BlendAnimationsByIndexDefinition, BlendAnimationsDefinition},
            play::PlayAnimationDefinition,
            BasePoseNodeDefinition, PoseNodeDefinition,
        },
        state::StateDefinition,
        MachineDefinition,
    },
    core::pool::Handle,
    gui::{
        menu::MenuItemMessage,
        message::UiMessage,
        popup::{Placement, PopupBuilder, PopupMessage},
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        BuildContext, UiNode, UserInterface,
    },
};

pub struct CanvasContextMenu {
    create_play_animation: Handle<UiNode>,
    create_blend_animations: Handle<UiNode>,
    create_blend_by_index: Handle<UiNode>,
    pub menu: Handle<UiNode>,
    pub canvas: Handle<UiNode>,
    pub node_context_menu: Handle<UiNode>,
}

impl CanvasContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_play_animation;
        let create_blend_animations;
        let create_blend_by_index;
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
                    }),
            )
            .build(ctx),
        )
        .build(ctx);

        Self {
            create_play_animation,
            create_blend_animations,
            create_blend_by_index,
            menu,
            canvas: Default::default(),
            node_context_menu: Default::default(),
        }
    }

    pub fn handle_ui_message(
        &mut self,
        sender: &MessageSender,
        message: &UiMessage,
        current_state: Handle<StateDefinition>,
        ui: &mut UserInterface,
    ) {
        if let Some(MenuItemMessage::Click) = message.data() {
            let position = ui
                .node(self.canvas)
                .screen_to_local(ui.node(self.menu).screen_position());

            let pose_node = if message.destination() == self.create_play_animation {
                Some(PoseNodeDefinition::PlayAnimation(PlayAnimationDefinition {
                    base: BasePoseNodeDefinition {
                        position,
                        parent_state: current_state,
                    },
                    animation: Default::default(),
                    speed: 1.0,
                    time_slice: None,
                }))
            } else if message.destination() == self.create_blend_animations {
                Some(PoseNodeDefinition::BlendAnimations(
                    BlendAnimationsDefinition {
                        base: BasePoseNodeDefinition {
                            position,
                            parent_state: current_state,
                        },
                        pose_sources: Default::default(),
                    },
                ))
            } else if message.destination() == self.create_blend_by_index {
                Some(PoseNodeDefinition::BlendAnimationsByIndex(
                    BlendAnimationsByIndexDefinition {
                        base: BasePoseNodeDefinition {
                            position,
                            parent_state: current_state,
                        },
                        index_parameter: "".to_string(),
                        inputs: Default::default(),
                    },
                ))
            } else {
                None
            };

            if let Some(pose_node) = pose_node {
                sender.do_command(AddPoseNodeCommand::new(pose_node));
            }
        }
    }
}

pub struct NodeContextMenu {
    remove: Handle<UiNode>,
    set_as_root: Handle<UiNode>,
    pub menu: Handle<UiNode>,
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
        selection: &[SelectedEntity],
        definition: &MachineDefinition,
        sender: &MessageSender,
        ui: &UserInterface,
    ) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.remove {
                let mut group = vec![AbsmCommand::new(ChangeSelectionCommand {
                    selection: vec![],
                })];

                group.extend(selection.iter().filter_map(|entry| {
                    if let SelectedEntity::PoseNode(pose_node) = entry {
                        Some(AbsmCommand::new(DeletePoseNodeCommand::new(*pose_node)))
                    } else {
                        None
                    }
                }));

                sender.do_command(CommandGroup::from(group));
            } else if message.destination() == self.set_as_root {
                let root = ui
                    .node(self.placement_target)
                    .query_component::<AbsmNode<PoseNodeDefinition>>()
                    .unwrap()
                    .model_handle;

                sender.do_command(SetStateRootPoseCommand {
                    handle: definition.nodes[root].parent_state,
                    value: root,
                })
            }
        } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.menu {
                self.placement_target = *target;
            }
        }
    }
}

pub struct ConnectionContextMenu {
    remove: Handle<UiNode>,
    pub menu: Handle<UiNode>,
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
        definition: &MachineDefinition,
    ) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination == self.remove {
                let connection_ref = ui
                    .node(self.placement_target)
                    .query_component::<Connection>()
                    .unwrap();

                let dest_node_ref = ui
                    .node(connection_ref.dest_node)
                    .query_component::<AbsmNode<PoseNodeDefinition>>()
                    .unwrap();

                let index = dest_node_ref
                    .base
                    .input_sockets
                    .iter()
                    .position(|s| *s == connection_ref.segment.dest)
                    .unwrap();

                let model_handle = dest_node_ref.model_handle;
                match definition.nodes[model_handle] {
                    PoseNodeDefinition::PlayAnimation(_) => {
                        // No connections
                    }
                    PoseNodeDefinition::BlendAnimations(_) => {
                        sender.do_command(SetBlendAnimationsPoseSourceCommand {
                            handle: model_handle,
                            index,
                            value: Default::default(),
                        })
                    }
                    PoseNodeDefinition::BlendAnimationsByIndex(_) => {
                        sender.do_command(SetBlendAnimationByIndexInputPoseSourceCommand {
                            handle: model_handle,
                            index,
                            value: Default::default(),
                        })
                    }
                }
            }
        } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.menu {
                self.placement_target = *target;
            }
        }
    }
}
