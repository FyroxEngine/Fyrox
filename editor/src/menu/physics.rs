use crate::{menu::create_menu_item, scene::commands::graph::AddNodeCommand, Message};
use rg3d::{
    core::{algebra::Vector3, pool::Handle},
    gui::{menu::MenuItemMessage, message::UiMessage, BuildContext, UiNode},
    scene::{base::BaseBuilder, collider::*, joint::*, rigidbody::RigidBodyBuilder},
};
use std::sync::mpsc::Sender;

pub struct PhysicsMenu {
    pub menu: Handle<UiNode>,
    create_rigid_body: Handle<UiNode>,
    create_revolute_joint: Handle<UiNode>,
    create_ball_joint: Handle<UiNode>,
    create_prismatic_joint: Handle<UiNode>,
    create_fixed_joint: Handle<UiNode>,
    create_cube_collider: Handle<UiNode>,
}

impl PhysicsMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_rigid_body;
        let create_cube_collider;
        let create_revolute_joint;
        let create_ball_joint;
        let create_prismatic_joint;
        let create_fixed_joint;
        let menu = create_menu_item(
            "Physics",
            vec![
                {
                    create_rigid_body = create_menu_item("Rigid Body", vec![], ctx);
                    create_rigid_body
                },
                {
                    create_cube_collider = create_menu_item("Cube Collider", vec![], ctx);
                    create_cube_collider
                },
                {
                    create_revolute_joint = create_menu_item("Revolute Joint", vec![], ctx);
                    create_revolute_joint
                },
                {
                    create_ball_joint = create_menu_item("Ball Joint", vec![], ctx);
                    create_ball_joint
                },
                {
                    create_prismatic_joint = create_menu_item("Prismatic Joint", vec![], ctx);
                    create_prismatic_joint
                },
                {
                    create_fixed_joint = create_menu_item("Fixed Joint", vec![], ctx);
                    create_fixed_joint
                },
            ],
            ctx,
        );

        Self {
            menu,
            create_rigid_body,
            create_revolute_joint,
            create_ball_joint,
            create_prismatic_joint,
            create_fixed_joint,
            create_cube_collider,
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, sender: &Sender<Message>) {
        if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if message.destination() == self.create_rigid_body {
                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(
                        RigidBodyBuilder::new(BaseBuilder::new().with_name("Rigid Body"))
                            .build_node(),
                    )))
                    .unwrap();
            } else if message.destination() == self.create_revolute_joint {
                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(
                        JointBuilder::new(BaseBuilder::new().with_name("Revolute Joint"))
                            .with_params(JointParams::RevoluteJoint(RevoluteJoint {
                                local_anchor1: Default::default(),
                                local_axis1: Vector3::y(),
                                local_anchor2: Default::default(),
                                local_axis2: Vector3::x(),
                            }))
                            .build_node(),
                    )))
                    .unwrap()
            } else if message.destination() == self.create_ball_joint {
                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(
                        JointBuilder::new(BaseBuilder::new().with_name("Ball Joint"))
                            .with_params(JointParams::BallJoint(BallJoint {
                                local_anchor1: Default::default(),
                                local_anchor2: Default::default(),
                            }))
                            .build_node(),
                    )))
                    .unwrap()
            } else if message.destination() == self.create_prismatic_joint {
                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(
                        JointBuilder::new(BaseBuilder::new().with_name("Prismatic Joint"))
                            .with_params(JointParams::PrismaticJoint(PrismaticJoint {
                                local_anchor1: Default::default(),
                                local_axis1: Vector3::y(),
                                local_anchor2: Default::default(),
                                local_axis2: Vector3::x(),
                            }))
                            .build_node(),
                    )))
                    .unwrap()
            } else if message.destination() == self.create_fixed_joint {
                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(
                        JointBuilder::new(BaseBuilder::new().with_name("Fixed Joint"))
                            .with_params(JointParams::FixedJoint(FixedJoint {
                                local_anchor1_translation: Default::default(),
                                local_anchor1_rotation: Default::default(),
                                local_anchor2_translation: Default::default(),
                                local_anchor2_rotation: Default::default(),
                            }))
                            .build_node(),
                    )))
                    .unwrap()
            } else if message.destination == self.create_cube_collider {
                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(
                        ColliderBuilder::new(BaseBuilder::new().with_name("Cuboid Collider"))
                            .with_shape(ColliderShape::Cuboid(Default::default()))
                            .build_node(),
                    )))
                    .unwrap();
            }
        }
    }
}
