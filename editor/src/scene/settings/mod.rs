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

use crate::asset::preview::cache::IconRequest;
use crate::{
    command::make_command,
    fyrox::{
        core::{color::Color, pool::Handle, reflect::Reflect},
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
            BuildContext, UiNode,
        },
        scene::{
            dim2,
            graph::{
                physics::{IntegrationParameters, PhysicsWorld},
                Graph, NodePool,
            },
            SceneRenderingOptions,
        },
        utils::lightmap::Lightmap,
    },
    message::MessageSender,
    plugins::inspector::{editors::make_property_editors_container, EditorEnvironment},
    scene::commands::GameSceneContext,
    GameScene, Message, MessageDirection, MSG_SYNC_FLAG,
};
use fyrox::{
    asset::manager::ResourceManager,
    graph::SceneGraph,
    gui::{inspector::InspectorContextArgs, window::Window},
};
use std::sync::mpsc::Sender;
use std::sync::Arc;

pub struct SceneSettingsWindow {
    pub window: Handle<UiNode>,
    inspector: Handle<UiNode>,
    property_definitions: Arc<PropertyEditorDefinitionContainer>,
}

impl SceneSettingsWindow {
    pub fn new(
        ctx: &mut BuildContext,
        sender: MessageSender,
        resource_manager: ResourceManager,
    ) -> Self {
        let inspector;
        let window = WindowBuilder::new(
            WidgetBuilder::new()
                .with_width(400.0)
                .with_height(500.0)
                .with_name("SceneSettingsWindow"),
        )
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

        let container = make_property_editors_container(sender, resource_manager);

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

    pub fn open(
        &self,
        game_scene: &GameScene,
        engine: &mut Engine,
        sender: MessageSender,
        icon_request_sender: Sender<IconRequest>,
    ) {
        let ui = engine.user_interfaces.first();
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
            true,
        ));
        self.sync_to_model(true, game_scene, engine, sender, icon_request_sender);
    }

    pub fn sync_to_model(
        &self,
        force: bool,
        game_scene: &GameScene,
        engine: &mut Engine,
        sender: MessageSender,
        icon_request_sender: Sender<IconRequest>,
    ) {
        let ui = engine.user_interfaces.first_mut();
        if !force
            && !ui
                .try_get_of_type::<Window>(self.window)
                .unwrap()
                .is_globally_visible()
        {
            return;
        }

        let scene = &engine.scenes[game_scene.scene];

        let environment = Arc::new(EditorEnvironment {
            resource_manager: engine.resource_manager.clone(),
            serialization_context: engine.serialization_context.clone(),
            available_animations: Default::default(),
            sender,
            icon_request_sender,
        });

        let context = InspectorContext::from_object(InspectorContextArgs {
            object: scene,
            ctx: &mut ui.build_ctx(),
            definition_container: self.property_definitions.clone(),
            environment: Some(environment),
            sync_flag: MSG_SYNC_FLAG,
            layer_index: 0,
            generate_property_string_values: false,
            filter: PropertyFilter::new(|property| {
                let mut pass = true;

                property.downcast_ref::<NodePool>(&mut |v| {
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
            name_column_width: 150.0,
            base_path: Default::default(),
        });

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
