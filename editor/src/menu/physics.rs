use crate::fyrox::{
    core::pool::Handle,
    gui::{menu::MenuItemMessage, message::UiMessage, BuildContext, UiNode},
    scene::{
        base::BaseBuilder, collider::*, joint::*, node::Node, ragdoll::RagdollBuilder,
        rigidbody::RigidBodyBuilder,
    },
};
use crate::menu::create_menu_item;

pub struct PhysicsMenu {
    pub menu: Handle<UiNode>,
    create_rigid_body: Handle<UiNode>,
    create_revolute_joint: Handle<UiNode>,
    create_ball_joint: Handle<UiNode>,
    create_prismatic_joint: Handle<UiNode>,
    create_fixed_joint: Handle<UiNode>,
    create_collider: Handle<UiNode>,
    create_ragdoll: Handle<UiNode>,
}

impl PhysicsMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_rigid_body;
        let create_collider;
        let create_revolute_joint;
        let create_ball_joint;
        let create_prismatic_joint;
        let create_fixed_joint;
        let create_ragdoll;
        let menu = create_menu_item(
            "Physics",
            vec![
                {
                    create_rigid_body = create_menu_item("Rigid Body", vec![], ctx);
                    create_rigid_body
                },
                {
                    create_collider = create_menu_item("Collider", vec![], ctx);
                    create_collider
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
                {
                    create_ragdoll = create_menu_item("Ragdoll", vec![], ctx);
                    create_ragdoll
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
            create_collider,
            create_ragdoll,
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage) -> Option<Node> {
        if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if message.destination() == self.create_rigid_body {
                Some(RigidBodyBuilder::new(BaseBuilder::new().with_name("Rigid Body")).build_node())
            } else if message.destination() == self.create_revolute_joint {
                Some(
                    JointBuilder::new(BaseBuilder::new().with_name("Revolute Joint"))
                        .with_params(JointParams::RevoluteJoint(Default::default()))
                        .build_node(),
                )
            } else if message.destination() == self.create_ball_joint {
                Some(
                    JointBuilder::new(BaseBuilder::new().with_name("Ball Joint"))
                        .with_params(JointParams::BallJoint(Default::default()))
                        .build_node(),
                )
            } else if message.destination() == self.create_prismatic_joint {
                Some(
                    JointBuilder::new(BaseBuilder::new().with_name("Prismatic Joint"))
                        .with_params(JointParams::PrismaticJoint(Default::default()))
                        .build_node(),
                )
            } else if message.destination() == self.create_fixed_joint {
                Some(
                    JointBuilder::new(BaseBuilder::new().with_name("Fixed Joint"))
                        .with_params(JointParams::FixedJoint(Default::default()))
                        .build_node(),
                )
            } else if message.destination == self.create_collider {
                Some(
                    ColliderBuilder::new(BaseBuilder::new().with_name("Collider"))
                        .with_shape(ColliderShape::Cuboid(Default::default()))
                        .build_node(),
                )
            } else if message.destination == self.create_ragdoll {
                Some(RagdollBuilder::new(BaseBuilder::new().with_name("Ragdoll")).build_node())
            } else {
                None
            }
        } else {
            None
        }
    }
}
