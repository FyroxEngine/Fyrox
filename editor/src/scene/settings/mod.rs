use crate::fyrox::{
    core::{color::Color, pool::Handle},
    engine::Engine,
    gui::{
        inspector::{
            editors::{
                enumeration::EnumPropertyEditorDefinition, PropertyEditorDefinitionContainer,
            },
            InspectorBuilder, InspectorContext, InspectorMessage, PropertyFilter,
        },
        message::UiMessage,
        scroll_viewer::ScrollViewerBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
    resource::texture::TextureResource,
    scene::{
        dim2,
        graph::{
            physics::{IntegrationParameters, PhysicsWorld},
            Graph, NodePool,
        },
        SceneRenderingOptions,
    },
    utils::lightmap::Lightmap,
};
use crate::{
    inspector::editors::make_property_editors_container, message::MessageSender, GameScene,
    Message, MessageDirection, MSG_SYNC_FLAG,
};

use crate::command::make_command;
use crate::fyrox::core::reflect::Reflect;
use crate::scene::commands::GameSceneContext;
use std::sync::Arc;

pub struct SceneSettingsWindow {
    pub window: Handle<UiNode>,
    inspector: Handle<UiNode>,
    property_definitions: Arc<PropertyEditorDefinitionContainer>,
}

impl SceneSettingsWindow {
    pub fn new(ctx: &mut BuildContext, sender: MessageSender) -> Self {
        let inspector;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(400.0).with_height(500.0))
            .with_content(
                ScrollViewerBuilder::new(WidgetBuilder::new())
                    .with_content({
                        inspector = InspectorBuilder::new(WidgetBuilder::new()).build(ctx);
                        inspector
                    })
                    .build(ctx),
            )
            .open(false)
            .can_minimize(false)
            .with_title(WindowTitle::text("Scene Settings"))
            .build(ctx);

        let container = make_property_editors_container(sender);

        container.register_inheritable_inspectable::<Graph>();
        container.register_inheritable_inspectable::<IntegrationParameters>();
        container.register_inheritable_inspectable::<PhysicsWorld>();
        container.register_inheritable_inspectable::<dim2::physics::PhysicsWorld>();
        container.register_inheritable_inspectable::<SceneRenderingOptions>();
        container.insert(EnumPropertyEditorDefinition::<Color>::new_optional());

        Self {
            window,
            inspector,
            property_definitions: Arc::new(container),
        }
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open_modal(
            self.window,
            MessageDirection::ToWidget,
            true,
            true,
        ));
    }

    pub fn sync_to_model(&self, game_scene: &GameScene, engine: &mut Engine) {
        let ui = &mut engine.user_interfaces.first_mut();
        let scene = &engine.scenes[game_scene.scene];

        let context = InspectorContext::from_object(
            scene,
            &mut ui.build_ctx(),
            self.property_definitions.clone(),
            None,
            MSG_SYNC_FLAG,
            0,
            false,
            PropertyFilter::new(|property| {
                let mut pass = true;

                property.downcast_ref::<NodePool>(&mut |v| {
                    if v.is_some() {
                        pass = false;
                    }
                });

                property.downcast_ref::<Option<TextureResource>>(&mut |v| {
                    if v.is_some() {
                        pass = false;
                    }
                });

                property.downcast_ref::<Option<Lightmap>>(&mut |v| {
                    if v.is_some() {
                        pass = false;
                    }
                });

                pass
            }),
            150.0,
        );

        ui.send_message(InspectorMessage::context(
            self.inspector,
            MessageDirection::ToWidget,
            context,
        ));
    }

    pub fn handle_ui_message(&self, message: &UiMessage, sender: &MessageSender) {
        if let Some(InspectorMessage::PropertyChanged(property_changed)) = message.data() {
            if message.destination() == self.inspector {
                if let Some(command) = make_command(property_changed, |ctx| {
                    ctx.get_mut::<GameSceneContext>().scene as &mut dyn Reflect
                }) {
                    sender.send(Message::DoCommand(command));
                }
            }
        }
    }
}
