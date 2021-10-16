use crate::{
    menu::create_menu_item,
    physics::{Joint, RigidBody},
    scene::commands::physics::{AddJointCommand, CreateRigidBodyCommand},
    Message,
};
use rg3d::{
    core::{algebra::Vector3, pool::Handle},
    gui::{
        message::{MenuItemMessage, UiMessage, UiMessageData},
        BuildContext, UiNode,
    },
    physics3d::desc::{
        BallJointDesc, FixedJointDesc, JointParamsDesc, PrismaticJointDesc, RevoluteJointDesc,
    },
};
use std::sync::mpsc::Sender;

pub struct PhysicsMenu {
    pub menu: Handle<UiNode>,
    create_rigid_body: Handle<UiNode>,
    create_revolute_joint: Handle<UiNode>,
    create_ball_joint: Handle<UiNode>,
    create_prismatic_joint: Handle<UiNode>,
    create_fixed_joint: Handle<UiNode>,
}

impl PhysicsMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_rigid_body;
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
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, sender: &Sender<Message>) {
        if let UiMessageData::MenuItem(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.create_rigid_body {
                sender
                    .send(Message::do_scene_command(CreateRigidBodyCommand::new(
                        RigidBody::default(),
                    )))
                    .unwrap();
            } else if message.destination() == self.create_revolute_joint {
                sender
                    .send(Message::do_scene_command(AddJointCommand::new(Joint {
                        body1: Default::default(),
                        body2: Default::default(),
                        params: JointParamsDesc::RevoluteJoint(RevoluteJointDesc {
                            local_anchor1: Default::default(),
                            local_axis1: Vector3::y(),
                            local_anchor2: Default::default(),
                            local_axis2: Vector3::x(),
                        }),
                    })))
                    .unwrap();
            } else if message.destination() == self.create_ball_joint {
                sender
                    .send(Message::do_scene_command(AddJointCommand::new(Joint {
                        body1: Default::default(),
                        body2: Default::default(),
                        params: JointParamsDesc::BallJoint(BallJointDesc {
                            local_anchor1: Default::default(),
                            local_anchor2: Default::default(),
                        }),
                    })))
                    .unwrap();
            } else if message.destination() == self.create_prismatic_joint {
                sender
                    .send(Message::do_scene_command(AddJointCommand::new(Joint {
                        body1: Default::default(),
                        body2: Default::default(),
                        params: JointParamsDesc::PrismaticJoint(PrismaticJointDesc {
                            local_anchor1: Default::default(),
                            local_axis1: Vector3::y(),
                            local_anchor2: Default::default(),
                            local_axis2: Vector3::x(),
                        }),
                    })))
                    .unwrap();
            } else if message.destination() == self.create_fixed_joint {
                sender
                    .send(Message::do_scene_command(AddJointCommand::new(Joint {
                        body1: Default::default(),
                        body2: Default::default(),
                        params: JointParamsDesc::FixedJoint(FixedJointDesc {
                            local_anchor1_translation: Default::default(),
                            local_anchor1_rotation: Default::default(),
                            local_anchor2_translation: Default::default(),
                            local_anchor2_rotation: Default::default(),
                        }),
                    })))
                    .unwrap();
            }
        }
    }
}
