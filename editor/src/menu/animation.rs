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
        animation::{absm::prelude::*, prelude::*},
        base::BaseBuilder,
        node::Node,
    },
};
use crate::menu::create_menu_item;

pub struct AnimationMenu {
    pub menu: Handle<UiNode>,
    create_animation_player: Handle<UiNode>,
    create_absm: Handle<UiNode>,
}

impl AnimationMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_animation_player;
        let create_absm;

        let menu = create_menu_item(
            "Animation",
            vec![
                {
                    create_animation_player = create_menu_item("Animation Player", vec![], ctx);
                    create_animation_player
                },
                {
                    create_absm = create_menu_item("Animation Blending State Machine", vec![], ctx);
                    create_absm
                },
            ],
            ctx,
        );

        Self {
            menu,
            create_animation_player,
            create_absm,
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage) -> Option<Node> {
        if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if message.destination() == self.create_animation_player {
                let node =
                    AnimationPlayerBuilder::new(BaseBuilder::new().with_name("Animation Player"))
                        .build_node();
                Some(node)
            } else if message.destination() == self.create_absm {
                let mut machine = Machine::default();

                let mut layer = MachineLayer::new();
                layer.set_name("Base Layer");

                machine.add_layer(layer);

                let node = AnimationBlendingStateMachineBuilder::new(
                    BaseBuilder::new().with_name("Animation Blending State Machine"),
                )
                .with_machine(machine)
                .build_node();
                Some(node)
            } else {
                None
            }
        } else {
            None
        }
    }
}
