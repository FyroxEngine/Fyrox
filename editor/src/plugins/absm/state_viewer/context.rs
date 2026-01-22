// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::fyrox::graph::SceneGraphNode;
use crate::fyrox::{
    core::{algebra::Vector2, pool::Handle},
    generic_animation::machine::{
        node::{blendspace::BlendSpace, blendspace::BlendSpacePoint, BasePoseNode},
        BlendAnimations, BlendAnimationsByIndex, MachineLayer, PlayAnimation, PoseNode, State,
    },
    graph::SceneGraph,
    gui::{
        menu::MenuItemMessage,
        message::UiMessage,
        popup::{Placement, PopupBuilder, PopupMessage},
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        BuildContext, RcUiNodeHandle, UiNode, UserInterface,
    },
};
use crate::plugins::absm::canvas::AbsmCanvas;
use crate::plugins::absm::{
    command::{
        blend::{
            SetBlendAnimationByIndexInputPoseSourceCommand, SetBlendAnimationsPoseSourceCommand,
            SetBlendSpacePoseSourceCommand,
        },
        AddPoseNodeCommand, DeletePoseNodeCommand, SetStateRootPoseCommand,
    },
    connection::Connection,
    node::AbsmNode,
    selection::SelectedEntity,
};
use crate::{
    command::{Command, CommandGroup},
    menu::create_menu_item,
    message::MessageSender,
    scene::{commands::ChangeSelectionCommand, Selection},
};
use fyrox::core::{uuid, Uuid};
use fyrox::gui::menu::{ContextMenuBuilder, MenuItem};

pub struct CanvasContextMenu {
    create_play_animation: Handle<MenuItem>,
    create_blend_animations: Handle<MenuItem>,
    create_blend_by_index: Handle<MenuItem>,
    create_blend_space: Handle<MenuItem>,
    pub menu: RcUiNodeHandle,
    pub canvas: Handle<AbsmCanvas>,
    pub node_context_menu: Option<RcUiNodeHandle>,
}

impl CanvasContextMenu {
    pub const PLAY_ANIMATION: Uuid = uuid!("6a151c9d-4d3e-49e7-b229-d9ffe536102e");
    pub const BLEND_ANIMATIONS: Uuid = uuid!("c923a357-ed22-46f2-9188-bf639095c1cf");
    pub const BLEND_BY_INDEX: Uuid = uuid!("2a656cac-20b9-4576-af95-c2a1b87e8304");
    pub const BLEND_SPACE: Uuid = uuid!("94a92a0a-a59f-44a8-bc8a-98d89f6aff80");

    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_play_animation;
        let create_blend_animations;
        let create_blend_by_index;
        let create_blend_space;
        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(
                WidgetBuilder::new()
                    .with_enabled(false) // Disabled by default.
                    .with_visibility(false),
            )
            .with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            create_play_animation = create_menu_item(
                                "Play Animation",
                                Self::PLAY_ANIMATION,
                                vec![],
                                ctx,
                            );
                            create_play_animation
                        })
                        .with_child({
                            create_blend_animations = create_menu_item(
                                "Blend Animations",
                                Self::BLEND_ANIMATIONS,
                                vec![],
                                ctx,
                            );
                            create_blend_animations
                        })
                        .with_child({
                            create_blend_by_index = create_menu_item(
                                "Blend By Index",
                                Self::BLEND_BY_INDEX,
                                vec![],
                                ctx,
                            );
                            create_blend_by_index
                        })
                        .with_child({
                            create_blend_space =
                                create_menu_item("Blend Space", Self::BLEND_SPACE, vec![], ctx);
                            create_blend_space
                        }),
                )
                .build(ctx),
            )
            .with_restrict_picking(false),
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

    pub fn handle_ui_message<N: SceneGraphNode>(
        &mut self,
        sender: &MessageSender,
        message: &UiMessage,
        current_state: Handle<State<Handle<N>>>,
        ui: &mut UserInterface,
        absm_node_handle: Handle<N>,
        layer_index: usize,
    ) {
        if let Some(MenuItemMessage::Click) = message.data() {
            let position =
                ui[self.canvas].screen_to_local(ui.node(self.menu.handle()).screen_position());

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
                        position: Vector2::new(0.0, 0.0),
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
                sender.do_command(AddPoseNodeCommand::new(
                    absm_node_handle,
                    layer_index,
                    pose_node,
                ));
            }
        }
    }
}

pub struct NodeContextMenu {
    remove: Handle<MenuItem>,
    set_as_root: Handle<MenuItem>,
    pub menu: RcUiNodeHandle,
    pub canvas: Handle<AbsmCanvas>,
    placement_target: Handle<UiNode>,
}

impl NodeContextMenu {
    pub const SET_AS_ROOT: Uuid = uuid!("b0dbe561-6d30-4cd3-a900-8cb4105c9c77");
    pub const REMOVE: Uuid = uuid!("2936d699-bdea-43d9-b601-b5159ea056cf");

    pub fn new(ctx: &mut BuildContext) -> Self {
        let remove;
        let set_as_root;
        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
                .with_content(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_child({
                                set_as_root =
                                    create_menu_item("Set As Root", Self::SET_AS_ROOT, vec![], ctx);
                                set_as_root
                            })
                            .with_child({
                                remove = create_menu_item("Remove", Self::REMOVE, vec![], ctx);
                                remove
                            }),
                    )
                    .build(ctx),
                )
                .with_restrict_picking(false),
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

    pub fn handle_ui_message<N: SceneGraphNode>(
        &mut self,
        message: &UiMessage,
        machine_layer: &MachineLayer<Handle<N>>,
        sender: &MessageSender,
        ui: &UserInterface,
        editor_selection: &Selection,
        absm_node_handle: Handle<N>,
        layer_index: usize,
    ) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.remove {
                if let Some(selection) = editor_selection.as_absm() {
                    let mut new_selection = selection.clone();
                    new_selection.entities.clear();

                    let mut group = vec![Command::new(ChangeSelectionCommand::new(
                        Selection::new(new_selection),
                    ))];

                    group.extend(selection.entities.iter().filter_map(|entry| {
                        if let SelectedEntity::PoseNode(pose_node) = entry {
                            Some(Command::new(DeletePoseNodeCommand::new(
                                absm_node_handle,
                                layer_index,
                                *pose_node,
                            )))
                        } else {
                            None
                        }
                    }));

                    sender.do_command(CommandGroup::from(group));
                }
            } else if message.destination() == self.set_as_root {
                let root = ui
                    .node(self.placement_target)
                    .query_component::<AbsmNode<PoseNode<Handle<N>>>>()
                    .unwrap()
                    .model_handle;

                sender.do_command(SetStateRootPoseCommand {
                    node_handle: absm_node_handle,
                    layer_index,
                    handle: machine_layer.node(root).parent_state,
                    value: root,
                });
            }
        } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.menu.handle() {
                self.placement_target = *target;
            }
        }
    }
}

pub struct ConnectionContextMenu {
    remove: Handle<MenuItem>,
    pub menu: RcUiNodeHandle,
    placement_target: Handle<UiNode>,
}

impl ConnectionContextMenu {
    pub const REMOVE_CONNECTION: Uuid = uuid!("4b91ee45-02ba-47d2-b3dd-e05962f323d9");

    pub fn new(ctx: &mut BuildContext) -> Self {
        let remove;
        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
                .with_content(
                    StackPanelBuilder::new(WidgetBuilder::new().with_child({
                        remove = create_menu_item(
                            "Remove Connection",
                            Self::REMOVE_CONNECTION,
                            vec![],
                            ctx,
                        );
                        remove
                    }))
                    .build(ctx),
                )
                .with_restrict_picking(false),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            menu,
            remove,
            placement_target: Default::default(),
        }
    }

    pub fn handle_ui_message<N: SceneGraphNode>(
        &mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        sender: &MessageSender,
        machine_layer: &MachineLayer<Handle<N>>,
        absm_node_handle: Handle<N>,
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
                    .query_component::<AbsmNode<PoseNode<Handle<N>>>>()
                    .unwrap();

                let index = dest_node_ref
                    .base
                    .input_sockets
                    .iter()
                    .position(|s| connection_ref.segment.dest == *s)
                    .unwrap();

                let model_handle = dest_node_ref.model_handle;
                match machine_layer.node(model_handle) {
                    PoseNode::PlayAnimation(_) => {
                        // No connections
                    }
                    PoseNode::BlendAnimations(_) => {
                        sender.do_command(SetBlendAnimationsPoseSourceCommand {
                            node_handle: absm_node_handle,
                            layer_index,
                            handle: model_handle,
                            index,
                            value: Default::default(),
                        })
                    }
                    PoseNode::BlendAnimationsByIndex(_) => {
                        sender.do_command(SetBlendAnimationByIndexInputPoseSourceCommand {
                            node_handle: absm_node_handle,
                            layer_index,
                            handle: model_handle,
                            index,
                            value: Default::default(),
                        })
                    }
                    PoseNode::BlendSpace(_) => sender.do_command(SetBlendSpacePoseSourceCommand {
                        node_handle: absm_node_handle,
                        layer_index,
                        handle: model_handle,
                        index,
                        value: Default::default(),
                    }),
                }
            }
        } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.menu.handle() {
                self.placement_target = *target;
            }
        }
    }
}
