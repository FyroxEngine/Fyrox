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

use crate::ui_scene::commands::UiSceneContext;
use crate::{
    asset::preview::cache::IconRequest,
    command::make_command,
    fyrox::{
        core::{color::Color, pool::Handle, reflect::Reflect},
        engine::Engine,
        gui::{
            inspector::{
                editors::{
                    enumeration::EnumPropertyEditorDefinition, PropertyEditorDefinitionContainer,
                },
                InspectorBuilder, InspectorContext, InspectorContextArgs, InspectorMessage,
                PropertyFilter,
            },
            message::UiMessage,
            scroll_viewer::ScrollViewerBuilder,
            widget::WidgetBuilder,
            window::{Window, WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, WidgetPool,
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
    plugins::inspector::EditorEnvironment,
    scene::{commands::GameSceneContext, controller::SceneController},
    ui_scene::UiScene,
    GameScene, Message,
};
use fyrox::gui::inspector::Inspector;
use std::sync::{mpsc::Sender, Arc};

pub struct SceneSettingsWindow {
    pub window: Handle<Window>,
    inspector: Handle<Inspector>,
    property_definitions: Arc<PropertyEditorDefinitionContainer>,
}

impl SceneSettingsWindow {
    pub fn new(
        ctx: &mut BuildContext,
        property_definitions: Arc<PropertyEditorDefinitionContainer>,
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

        property_definitions.register_inheritable_inspectable::<Graph>();
        property_definitions.register_inheritable_inspectable::<IntegrationParameters>();
        property_definitions.register_inheritable_inspectable::<PhysicsWorld>();
        property_definitions.register_inheritable_inspectable::<dim2::physics::PhysicsWorld>();
        property_definitions.register_inheritable_inspectable::<SceneRenderingOptions>();
        property_definitions.insert(EnumPropertyEditorDefinition::<Color>::new_optional());

        Self {
            window,
            inspector,
            property_definitions,
        }
    }

    pub fn open(
        &self,
        controller: &dyn SceneController,
        engine: &mut Engine,
        sender: MessageSender,
        icon_request_sender: Sender<IconRequest>,
    ) {
        let ui = engine.user_interfaces.first();
        ui.send(
            self.window,
            WindowMessage::Open {
                alignment: WindowAlignment::Center,
                modal: false,
                focus_content: true,
            },
        );
        self.sync_to_model(true, controller, engine, sender, icon_request_sender);
    }

    pub fn sync_to_model(
        &self,
        force: bool,
        controller: &dyn SceneController,
        engine: &mut Engine,
        sender: MessageSender,
        icon_request_sender: Sender<IconRequest>,
    ) {
        let ui = engine.user_interfaces.first_mut();
        if !force && !ui[self.window].is_globally_visible() {
            return;
        }

        let object = if let Some(game_scene) = controller.downcast_ref::<GameScene>() {
            &engine.scenes[game_scene.scene] as &dyn Reflect
        } else if let Some(ui_scene) = controller.downcast_ref::<UiScene>() {
            &ui_scene.ui as &dyn Reflect
        } else {
            return;
        };

        let environment = Arc::new(EditorEnvironment {
            resource_manager: engine.resource_manager.clone(),
            serialization_context: engine.serialization_context.clone(),
            dyn_type_constructors: engine.dyn_type_constructors.clone(),
            available_animations: Default::default(),
            sender,
            icon_request_sender,
            style: None,
        });

        let context = InspectorContext::from_object(InspectorContextArgs {
            object,
            ctx: &mut ui.build_ctx(),
            definition_container: self.property_definitions.clone(),
            environment: Some(environment),
            layer_index: 0,
            generate_property_string_values: false,
            filter: PropertyFilter::new(|property| {
                let mut pass = true;

                property.downcast_ref::<NodePool>(&mut |v| {
                    if v.is_some() {
                        pass = false;
                    }
                });

                property.downcast_ref::<WidgetPool>(&mut |v| {
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
            has_parent_object: false,
        });

        ui.send(self.inspector, InspectorMessage::Context(context));
    }

    pub fn handle_ui_message(
        &self,
        controller: &dyn SceneController,
        message: &UiMessage,
        sender: &MessageSender,
    ) {
        if let Some(InspectorMessage::PropertyChanged(property_changed)) = message.data() {
            if message.destination() == self.inspector {
                if controller.downcast_ref::<GameScene>().is_some() {
                    if let Some(command) = make_command(property_changed, |ctx| {
                        Some(ctx.get_mut::<GameSceneContext>().scene as &mut dyn Reflect)
                    }) {
                        sender.send(Message::DoCommand(command));
                    }
                } else if controller.downcast_ref::<UiScene>().is_some() {
                    if let Some(command) = make_command(property_changed, |ctx| {
                        Some(ctx.get_mut::<UiSceneContext>().ui as &mut dyn Reflect)
                    }) {
                        sender.send(Message::DoCommand(command));
                    }
                }
            }
        }
    }
}
