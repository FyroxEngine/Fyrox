use crate::{
    gui::{BuildContext, EditorUiMessage, EditorUiNode, Ui, UiMessage, UiNode},
    inspector::editors::texture::TexturePropertyEditorDefinition,
    scene::{
        commands::{
            graph::{MoveNodeCommand, RotateNodeCommand},
            SceneCommand,
        },
        EditorScene, Selection,
    },
    GameEngine, Message,
};
use rg3d::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        pool::Handle,
    },
    engine::resource_manager::ResourceManager,
    gui::{
        inspector::{
            editors::PropertyEditorDefinitionContainer, InspectorBuilder, InspectorContext,
            InspectorEnvironment,
        },
        message::{InspectorMessage, MessageDirection, UiMessageData},
        scroll_viewer::ScrollViewerBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
    },
    scene::{
        base::Base, camera::Camera, decal::Decal, light::point::PointLight, light::spot::SpotLight,
        light::BaseLight, particle_system::ParticleSystem, sprite::Sprite, transform::Transform,
    },
};
use std::{any::Any, any::TypeId, sync::mpsc::Sender, sync::Arc};

pub mod editors;

pub struct EditorEnvironment {
    resource_manager: ResourceManager,
}

impl InspectorEnvironment for EditorEnvironment {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct Inspector {
    pub window: Handle<UiNode>,
    inspector: Handle<UiNode>,
    property_editors: Arc<PropertyEditorDefinitionContainer<EditorUiMessage, EditorUiNode>>,
}

struct SenderHelper {
    sender: Sender<Message>,
}

impl SenderHelper {
    pub fn do_scene_command(&self, command: SceneCommand) {
        self.sender.send(Message::DoSceneCommand(command)).unwrap();
    }
}

impl Inspector {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let mut container = PropertyEditorDefinitionContainer::new();
        container.insert(Arc::new(TexturePropertyEditorDefinition));
        let property_editors = Arc::new(container);

        let inspector;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::text("Inspector"))
            .with_content(
                ScrollViewerBuilder::new(WidgetBuilder::new())
                    .with_content({
                        inspector = InspectorBuilder::new(WidgetBuilder::new())
                            .with_property_editor_definitions(property_editors.clone())
                            .build(ctx);
                        inspector
                    })
                    .build(ctx),
            )
            .build(ctx);

        Self {
            window,
            inspector,
            property_editors,
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let scene = &engine.scenes[editor_scene.scene];

        if let Selection::Graph(selection) = &editor_scene.selection {
            if selection.is_single_selection() {
                let node_handle = selection.nodes()[0];
                if scene.graph.is_valid_handle(node_handle) {
                    let node = &scene.graph[node_handle];

                    let environment = Arc::new(EditorEnvironment {
                        resource_manager: engine.resource_manager.clone(),
                    });

                    let context = InspectorContext::from_object(
                        node,
                        &mut engine.user_interface.build_ctx(),
                        &*self.property_editors,
                        Some(environment),
                    );

                    engine
                        .user_interface
                        .send_message(InspectorMessage::context(
                            self.inspector,
                            MessageDirection::ToWidget,
                            context,
                        ));
                }
            }
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
        sender: &Sender<Message>,
    ) {
        let helper = SenderHelper {
            sender: sender.clone(),
        };

        if message.destination() == self.inspector
            && message.direction() == MessageDirection::FromWidget
        {
            if let UiMessageData::Inspector(InspectorMessage::PropertyChanged(args)) =
                message.data()
            {
                let scene = &engine.scenes[editor_scene.scene];

                if let Selection::Graph(selection) = &editor_scene.selection {
                    if selection.is_single_selection() {
                        let node_handle = selection.nodes()[0];

                        let node = &scene.graph[node_handle];

                        if scene.graph.is_valid_handle(node_handle) {
                            if args.owner_type_id == TypeId::of::<Base>() {
                            } else if args.owner_type_id == TypeId::of::<Transform>() {
                                match args.name.as_ref() {
                                    "local_position" => {
                                        helper.do_scene_command(SceneCommand::MoveNode(
                                            MoveNodeCommand::new(
                                                node_handle,
                                                **node.local_transform().position(),
                                                *args.cast_value::<Vector3<f32>>().unwrap(),
                                            ),
                                        ));
                                    }
                                    "local_rotation" => {
                                        helper.do_scene_command(SceneCommand::RotateNode(
                                            RotateNodeCommand::new(
                                                node_handle,
                                                **node.local_transform().rotation(),
                                                *args.cast_value::<UnitQuaternion<f32>>().unwrap(),
                                            ),
                                        ));
                                    }
                                    "local_scale" => {
                                        helper.do_scene_command(SceneCommand::RotateNode(
                                            RotateNodeCommand::new(
                                                node_handle,
                                                **node.local_transform().rotation(),
                                                *args.cast_value::<UnitQuaternion<f32>>().unwrap(),
                                            ),
                                        ));
                                    }
                                    _ => println!("Unhandled property of Transform: {:?}", args),
                                }
                            } else if args.owner_type_id == TypeId::of::<Camera>() {
                                // TODO
                            } else if args.owner_type_id == TypeId::of::<Sprite>() {
                                // TODO
                            } else if args.owner_type_id == TypeId::of::<BaseLight>() {
                                // TODO
                            } else if args.owner_type_id == TypeId::of::<PointLight>() {
                                // TODO
                            } else if args.owner_type_id == TypeId::of::<SpotLight>() {
                                // TODO
                            } else if args.owner_type_id == TypeId::of::<ParticleSystem>() {
                                // TODO
                            } else if args.owner_type_id == TypeId::of::<Decal>() {
                                // TODO
                            }
                        }
                    }
                }
            }
        }
    }
}
