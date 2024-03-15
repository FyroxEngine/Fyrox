use crate::fyrox::{
    core::pool::Handle,
    gui::{menu::MenuItemMessage, message::UiMessage, BuildContext, UiNode},
    scene::{base::BaseBuilder, dim2::rectangle::RectangleBuilder, node::Node},
};
use crate::menu::create_menu_item;

pub struct Dim2Menu {
    pub menu: Handle<UiNode>,
    create_sprite: Handle<UiNode>,
}

impl Dim2Menu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_sprite;

        let menu = create_menu_item(
            "2D",
            vec![{
                create_sprite = create_menu_item("Rectangle (2D Sprite)", vec![], ctx);
                create_sprite
            }],
            ctx,
        );

        Self {
            menu,

            create_sprite,
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage) -> Option<Node> {
        if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if message.destination() == self.create_sprite {
                let node =
                    RectangleBuilder::new(BaseBuilder::new().with_name("Sprite (2D)")).build_node();
                Some(node)
            } else {
                None
            }
        } else {
            None
        }
    }
}
