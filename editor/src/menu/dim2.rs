use crate::{menu::create_menu_item, scene::commands::graph::AddNodeCommand, Message};
use rg3d::{
    core::pool::Handle,
    gui::{menu::MenuItemMessage, message::UiMessage, BuildContext, UiNode},
    scene::{
        base::BaseBuilder,
        dim2::{
            camera::CameraBuilder,
            light::{point::PointLightBuilder, spot::SpotLightBuilder, BaseLightBuilder},
            sprite::SpriteBuilder,
        },
        node::Node,
    },
};
use std::sync::mpsc::Sender;

pub struct Dim2Menu {
    pub menu: Handle<UiNode>,
    create_camera: Handle<UiNode>,
    create_sprite: Handle<UiNode>,
    create_point_light: Handle<UiNode>,
    create_spot_light: Handle<UiNode>,
}

impl Dim2Menu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_camera;
        let create_sprite;
        let create_point_light;
        let create_spot_light;
        let menu = create_menu_item(
            "2D",
            vec![
                {
                    create_sprite = create_menu_item("Sprite", vec![], ctx);
                    create_sprite
                },
                {
                    create_camera = create_menu_item("Camera", vec![], ctx);
                    create_camera
                },
                {
                    create_point_light = create_menu_item("Point Light", vec![], ctx);
                    create_point_light
                },
                {
                    create_spot_light = create_menu_item("Spot Light", vec![], ctx);
                    create_spot_light
                },
            ],
            ctx,
        );

        Self {
            menu,
            create_camera,
            create_sprite,
            create_point_light,
            create_spot_light,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        sender: &Sender<Message>,
        parent: Handle<Node>,
    ) {
        if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if message.destination() == self.create_point_light {
                let node = PointLightBuilder::new(BaseLightBuilder::new(
                    BaseBuilder::new().with_name("Point Light 2D"),
                ))
                .build_node();
                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(node, parent)))
                    .unwrap();
            } else if message.destination() == self.create_spot_light {
                let node = SpotLightBuilder::new(BaseLightBuilder::new(
                    BaseBuilder::new().with_name("Spot Light 2D"),
                ))
                .build_node();
                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(node, parent)))
                    .unwrap();
            } else if message.destination() == self.create_camera {
                let node =
                    CameraBuilder::new(BaseBuilder::new().with_name("Camera 2D")).build_node();
                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(node, parent)))
                    .unwrap();
            } else if message.destination() == self.create_sprite {
                let node =
                    SpriteBuilder::new(BaseBuilder::new().with_name("Sprite 2D")).build_node();
                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(node, parent)))
                    .unwrap();
            }
        }
    }
}
