use crate::{
    physics::Joint,
    physics::{Collider, RigidBody},
    scene::{
        commands::{
            physics::{
                AddColliderCommand, DeleteBodyCommand, DeleteColliderCommand, DeleteJointCommand,
                SetJointBody1Command, SetJointBody2Command,
            },
            CommandGroup, SceneCommand,
        },
        EditorScene,
    },
    world::graph::item::SceneItem,
    Message,
};
use rg3d::{
    core::{
        algebra::{Vector2, Vector3},
        pool::Handle,
    },
    gui::{
        menu::{MenuItemBuilder, MenuItemContent},
        message::{MenuItemMessage, PopupMessage, UiMessage, UiMessageData},
        popup::{Placement, PopupBuilder},
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        BuildContext, UiNode, UserInterface,
    },
    physics3d::desc::{
        BallDesc, CapsuleDesc, ColliderShapeDesc, ConeDesc, CuboidDesc, CylinderDesc,
        HeightfieldDesc, RoundCylinderDesc, SegmentDesc, TriangleDesc, TrimeshDesc,
    },
};
use std::sync::mpsc::Sender;

pub struct RigidBodyContextMenu {
    pub menu: Handle<UiNode>,
    pub delete: Handle<UiNode>,
    pub add_ball_collider: Handle<UiNode>,
    pub add_cylinder_collider: Handle<UiNode>,
    pub add_round_cylinder_collider: Handle<UiNode>,
    pub add_cone_collider: Handle<UiNode>,
    pub add_cuboid_collider: Handle<UiNode>,
    pub add_capsule_collider: Handle<UiNode>,
    pub add_segment_collider: Handle<UiNode>,
    pub add_triangle_collider: Handle<UiNode>,
    pub add_trimesh_collider: Handle<UiNode>,
    pub add_heightfield_collider: Handle<UiNode>,
    /// A rigid body node above which the menu was opened.
    pub target: Handle<UiNode>,
}

fn make_menu_item(ctx: &mut BuildContext, text: &str) -> Handle<UiNode> {
    MenuItemBuilder::new(WidgetBuilder::new().with_min_size(Vector2::new(120.0, 20.0)))
        .with_content(MenuItemContent::text(text))
        .build(ctx)
}

impl RigidBodyContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let delete;
        let add_ball_collider;
        let add_cylinder_collider;
        let add_round_cylinder_collider;
        let add_cone_collider;
        let add_cuboid_collider;
        let add_capsule_collider;
        let add_segment_collider;
        let add_triangle_collider;
        let add_trimesh_collider;
        let add_heightfield_collider;
        let menu = PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
            .with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            delete = MenuItemBuilder::new(
                                WidgetBuilder::new().with_min_size(Vector2::new(120.0, 20.0)),
                            )
                            .with_content(MenuItemContent::Text {
                                text: "Delete",
                                shortcut: "Del",
                                icon: Default::default(),
                            })
                            .build(ctx);
                            delete
                        })
                        .with_child({
                            add_ball_collider = make_menu_item(ctx, "Add Ball Collider");
                            add_ball_collider
                        })
                        .with_child({
                            add_cylinder_collider = make_menu_item(ctx, "Add Cylinder Collider");
                            add_cylinder_collider
                        })
                        .with_child({
                            add_round_cylinder_collider =
                                make_menu_item(ctx, "Add Round Cylinder Collider");
                            add_round_cylinder_collider
                        })
                        .with_child({
                            add_cone_collider = make_menu_item(ctx, "Add Cone Collider");
                            add_cone_collider
                        })
                        .with_child({
                            add_cuboid_collider = make_menu_item(ctx, "Add Cuboid Collider");
                            add_cuboid_collider
                        })
                        .with_child({
                            add_capsule_collider = make_menu_item(ctx, "Add Capsule Collider");
                            add_capsule_collider
                        })
                        .with_child({
                            add_segment_collider = make_menu_item(ctx, "Add Segment Collider");
                            add_segment_collider
                        })
                        .with_child({
                            add_triangle_collider = make_menu_item(ctx, "Add Triangle Collider");
                            add_triangle_collider
                        })
                        .with_child({
                            add_trimesh_collider = make_menu_item(ctx, "Add Trimesh Collider");
                            add_trimesh_collider
                        })
                        .with_child({
                            add_heightfield_collider =
                                make_menu_item(ctx, "Add Height Field Collider");
                            add_heightfield_collider
                        }),
                )
                .build(ctx),
            )
            .build(ctx);

        Self {
            menu,
            delete,
            add_ball_collider,
            add_cylinder_collider,
            add_round_cylinder_collider,
            add_cone_collider,
            add_cuboid_collider,
            add_capsule_collider,
            add_segment_collider,
            add_triangle_collider,
            add_trimesh_collider,
            add_heightfield_collider,
            target: Default::default(),
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        sender: &Sender<Message>,
        editor_scene: &EditorScene,
        ui: &UserInterface,
    ) {
        match message.data() {
            UiMessageData::Popup(PopupMessage::Placement(Placement::Cursor(target))) => {
                if message.destination() == self.menu {
                    self.target = *target;
                }
            }
            UiMessageData::MenuItem(MenuItemMessage::Click) => {
                if let Some(rigid_body_view_ref) = ui.try_get_node(self.target).map(|n| {
                    n.cast::<SceneItem<RigidBody>>()
                        .expect("Rigid body context menu must have SceneItem<RigidBody> target!")
                }) {
                    let rigid_body_handle = rigid_body_view_ref.entity_handle;

                    if message.destination() == self.delete {
                        let mut group = Vec::new();

                        for collider in editor_scene.physics.bodies[rigid_body_handle]
                            .colliders
                            .iter()
                            .map(|h| Handle::<Collider>::from(*h))
                        {
                            group.push(SceneCommand::new(DeleteColliderCommand::new(collider)));
                        }

                        group.push(SceneCommand::new(DeleteBodyCommand::new(rigid_body_handle)));

                        for (joint_handle, joint_ref) in editor_scene.physics.joints.pair_iter() {
                            if joint_ref.body1 == rigid_body_handle.into() {
                                group.push(SceneCommand::new(SetJointBody1Command::new(
                                    joint_handle,
                                    Default::default(),
                                )));
                            } else if joint_ref.body2 == rigid_body_handle.into() {
                                group.push(SceneCommand::new(SetJointBody2Command::new(
                                    joint_handle,
                                    Default::default(),
                                )));
                            }
                        }

                        sender
                            .send(Message::do_scene_command(CommandGroup::from(group)))
                            .unwrap();
                    }

                    // Handle <add x collider> items
                    let shape = if message.destination() == self.add_ball_collider {
                        Some(ColliderShapeDesc::Ball(BallDesc { radius: 0.5 }))
                    } else if message.destination() == self.add_cylinder_collider {
                        Some(ColliderShapeDesc::Cylinder(CylinderDesc {
                            half_height: 0.5,
                            radius: 0.5,
                        }))
                    } else if message.destination() == self.add_round_cylinder_collider {
                        Some(ColliderShapeDesc::RoundCylinder(RoundCylinderDesc {
                            half_height: 0.5,
                            radius: 0.5,
                            border_radius: 0.1,
                        }))
                    } else if message.destination() == self.add_cone_collider {
                        Some(ColliderShapeDesc::Cone(ConeDesc {
                            half_height: 0.5,
                            radius: 0.5,
                        }))
                    } else if message.destination() == self.add_cuboid_collider {
                        Some(ColliderShapeDesc::Cuboid(CuboidDesc {
                            half_extents: Vector3::new(0.5, 0.5, 0.5),
                        }))
                    } else if message.destination() == self.add_capsule_collider {
                        Some(ColliderShapeDesc::Capsule(CapsuleDesc {
                            begin: Vector3::new(0.0, 0.0, 0.0),
                            end: Vector3::new(0.0, 1.0, 0.0),
                            radius: 0.5,
                        }))
                    } else if message.destination() == self.add_segment_collider {
                        Some(ColliderShapeDesc::Segment(SegmentDesc {
                            begin: Vector3::new(0.0, 0.0, 0.0),
                            end: Vector3::new(1.0, 0.0, 0.0),
                        }))
                    } else if message.destination() == self.add_triangle_collider {
                        Some(ColliderShapeDesc::Triangle(TriangleDesc {
                            a: Vector3::new(0.0, 0.0, 0.0),
                            b: Vector3::new(1.0, 0.0, 0.0),
                            c: Vector3::new(1.0, 0.0, 1.0),
                        }))
                    } else if message.destination() == self.add_trimesh_collider {
                        Some(ColliderShapeDesc::Trimesh(TrimeshDesc))
                    } else if message.destination() == self.add_heightfield_collider {
                        Some(ColliderShapeDesc::Heightfield(HeightfieldDesc))
                    } else {
                        None
                    };

                    if let Some(shape) = shape {
                        sender
                            .send(Message::do_scene_command(AddColliderCommand::new(
                                rigid_body_handle,
                                Collider {
                                    shape,
                                    ..Default::default()
                                },
                            )))
                            .unwrap();
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct DeletableSceneItemContextMenu {
    pub menu: Handle<UiNode>,
    pub delete: Handle<UiNode>,
    pub target: Handle<UiNode>,
}

impl DeletableSceneItemContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let delete;
        let menu = PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
            .with_content(
                StackPanelBuilder::new(WidgetBuilder::new().with_child({
                    delete = MenuItemBuilder::new(
                        WidgetBuilder::new().with_min_size(Vector2::new(120.0, 20.0)),
                    )
                    .with_content(MenuItemContent::Text {
                        text: "Delete",
                        shortcut: "Del",
                        icon: Default::default(),
                    })
                    .build(ctx);
                    delete
                }))
                .build(ctx),
            )
            .build(ctx);

        Self {
            menu,
            delete,
            target: Default::default(),
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        ui: &UserInterface,
        sender: &Sender<Message>,
    ) {
        match message.data() {
            UiMessageData::Popup(PopupMessage::Placement(Placement::Cursor(target))) => {
                if message.destination() == self.menu {
                    self.target = *target;
                }
            }
            UiMessageData::MenuItem(MenuItemMessage::Click) => {
                if message.destination() == self.delete {
                    if let Some(collider_item) = ui.node(self.target).cast::<SceneItem<Collider>>()
                    {
                        sender
                            .send(Message::do_scene_command(DeleteColliderCommand::new(
                                collider_item.entity_handle,
                            )))
                            .unwrap();
                    } else if let Some(collider_item) =
                        ui.node(self.target).cast::<SceneItem<Joint>>()
                    {
                        sender
                            .send(Message::do_scene_command(DeleteJointCommand::new(
                                collider_item.entity_handle,
                            )))
                            .unwrap();
                    }
                }
            }
            _ => {}
        }
    }
}
