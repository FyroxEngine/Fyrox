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

use crate::fyrox::{
    core::pool::Handle,
    gui::{menu::MenuItemMessage, message::UiMessage, BuildContext, UiNode},
    scene::{
        base::BaseBuilder,
        dim2::{collider::*, joint::*, rigidbody::RigidBodyBuilder},
        node::Node,
    },
};
use crate::menu::create_menu_item;

pub struct Physics2dMenu {
    pub menu: Handle<UiNode>,
    create_rigid_body: Handle<UiNode>,
    create_ball_joint: Handle<UiNode>,
    create_prismatic_joint: Handle<UiNode>,
    create_fixed_joint: Handle<UiNode>,
    create_collider: Handle<UiNode>,
}

impl Physics2dMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_rigid_body;
        let create_collider;
        let create_ball_joint;
        let create_prismatic_joint;
        let create_fixed_joint;
        let menu = create_menu_item(
            "Physics 2D",
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
            create_ball_joint,
            create_prismatic_joint,
            create_fixed_joint,
            create_collider,
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage) -> Option<Node> {
        if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if message.destination() == self.create_rigid_body {
                Some(
                    RigidBodyBuilder::new(BaseBuilder::new().with_name("Rigid Body 2D"))
                        .build_node(),
                )
            } else if message.destination() == self.create_ball_joint {
                Some(
                    JointBuilder::new(BaseBuilder::new().with_name("Ball Joint 2D"))
                        .with_params(JointParams::BallJoint(Default::default()))
                        .build_node(),
                )
            } else if message.destination() == self.create_prismatic_joint {
                Some(
                    JointBuilder::new(BaseBuilder::new().with_name("Prismatic Joint 2D"))
                        .with_params(JointParams::PrismaticJoint(Default::default()))
                        .build_node(),
                )
            } else if message.destination() == self.create_fixed_joint {
                Some(
                    JointBuilder::new(BaseBuilder::new().with_name("Fixed Joint 2D"))
                        .with_params(JointParams::FixedJoint(Default::default()))
                        .build_node(),
                )
            } else if message.destination == self.create_collider {
                Some(
                    ColliderBuilder::new(BaseBuilder::new().with_name("Collider 2D"))
                        .with_shape(ColliderShape::Cuboid(Default::default()))
                        .build_node(),
                )
            } else {
                None
            }
        } else {
            None
        }
    }
}
