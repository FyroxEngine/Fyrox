use crate::menu::create_menu_item;
use fyrox::{
    core::pool::Handle,
    gui::{menu::MenuItemMessage, message::UiMessage, BuildContext, UiNode},
    scene::{
        animation::{absm::AnimationBlendingStateMachineBuilder, AnimationPlayerBuilder},
        base::BaseBuilder,
        node::Node,
    },
};

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
                let node = AnimationBlendingStateMachineBuilder::new(
                    BaseBuilder::new().with_name("Animation Blending State Machine"),
                )
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
