use crate::{
    inspector::editors::make_property_editors_container,
    scene::settings::command::make_set_scene_property_command, EditorScene, Message,
    MessageDirection, MSG_SYNC_FLAG,
};
use fyrox::{
    core::pool::Handle,
    engine::Engine,
    gui::{
        inspector::{
            editors::{
                inspectable::InspectablePropertyEditorDefinition, PropertyEditorDefinitionContainer,
            },
            InspectorBuilder, InspectorContext, InspectorMessage,
        },
        message::UiMessage,
        scroll_viewer::ScrollViewerBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
    scene::{
        dim2,
        graph::{
            physics::{IntegrationParameters, PhysicsWorld},
            Graph,
        },
    },
};
use std::{rc::Rc, sync::mpsc::Sender};

mod command;

pub struct SceneSettingsWindow {
    pub window: Handle<UiNode>,
    inspector: Handle<UiNode>,
    property_definitions: Rc<PropertyEditorDefinitionContainer>,
}

impl SceneSettingsWindow {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
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

        container.insert(InspectablePropertyEditorDefinition::<Graph>::new());
        container.insert(InspectablePropertyEditorDefinition::<IntegrationParameters>::new());
        container.insert(InspectablePropertyEditorDefinition::<PhysicsWorld>::new());
        container.insert(InspectablePropertyEditorDefinition::<
            dim2::physics::PhysicsWorld,
        >::new());

        Self {
            window,
            inspector,
            property_definitions: Rc::new(container),
        }
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open_modal(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));
    }

    pub fn sync_to_model(&self, editor_scene: &EditorScene, engine: &mut Engine) {
        let ui = &mut engine.user_interface;
        let scene = &engine.scenes[editor_scene.scene];

        let context = InspectorContext::from_object(
            scene,
            &mut ui.build_ctx(),
            self.property_definitions.clone(),
            None,
            MSG_SYNC_FLAG,
            0,
            false,
        );

        ui.send_message(InspectorMessage::context(
            self.inspector,
            MessageDirection::ToWidget,
            context,
        ));
    }

    pub fn handle_ui_message(&self, message: &UiMessage, sender: &Sender<Message>) {
        if let Some(InspectorMessage::PropertyChanged(property_changed)) = message.data() {
            if message.destination() == self.inspector {
                if let Some(command) = make_set_scene_property_command((), property_changed) {
                    sender.send(Message::DoSceneCommand(command)).unwrap();
                }
            }
        }
    }
}
