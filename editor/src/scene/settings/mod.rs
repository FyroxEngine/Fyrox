use crate::{
    inspector::editors::make_property_editors_container, message::MessageSender,
    scene::settings::command::make_set_scene_property_command, GameScene, Message,
    MessageDirection, MSG_SYNC_FLAG,
};
use fyrox::{
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
            Graph, LowLevelGraph,
        },
        SceneRenderingOptions,
    },
    utils::lightmap::Lightmap,
};

use std::sync::Arc;

mod command;

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
        ));
    }

    pub fn sync_to_model(&self, game_scene: &GameScene, engine: &mut Engine) {
        let ui = &mut engine.user_interface;
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

                property.downcast_ref::<LowLevelGraph>(&mut |v| {
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
                if let Some(command) = make_set_scene_property_command((), property_changed) {
                    sender.send(Message::DoGameSceneCommand(command));
                }
            }
        }
    }
}
