use crate::{
    command::Command,
    inspector::{
        editors::{
            material::MaterialPropertyEditorDefinition, texture::TexturePropertyEditorDefinition,
        },
        handlers::{
            base::handle_base_property_changed, camera::handle_camera_property_changed,
            terrain::handle_terrain_property_changed, transform::handle_transform_property_changed,
        },
    },
    scene::{EditorScene, Selection},
    GameEngine, Message, MSG_SYNC_FLAG,
};
use rg3d::{
    core::pool::Handle,
    engine::resource_manager::ResourceManager,
    gui::{
        inspector::{
            editors::{
                collection::VecCollectionPropertyEditorDefinition,
                enumeration::EnumPropertyEditorDefinition, PropertyEditorDefinitionContainer,
            },
            InspectorBuilder, InspectorContext, InspectorEnvironment,
        },
        message::{InspectorMessage, MessageDirection, UiMessage, UiMessageData},
        scroll_viewer::ScrollViewerBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        BuildContext, UiNode,
    },
    scene::{
        base::{Base, Mobility, PhysicsBinding},
        camera::{Camera, Exposure},
        decal::Decal,
        light::{point::PointLight, spot::SpotLight, BaseLight},
        mesh::{surface::Surface, RenderPath},
        particle_system::ParticleSystem,
        sprite::Sprite,
        terrain::{Layer, Terrain},
        transform::Transform,
    },
    utils::log::{Log, MessageKind},
};
use std::{
    any::{Any, TypeId},
    sync::{mpsc::Sender, Arc, Mutex},
};

pub mod editors;
pub mod handlers;

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
    property_editors: Arc<PropertyEditorDefinitionContainer>,
    // Hack. This flag tells whether the inspector should sync with model or not.
    // There is only one situation when it has to be `false` - when inspector has
    // got new context - in this case we don't need to sync with model, because
    // inspector is already in correct state.
    needs_sync: bool,
}

pub struct SenderHelper {
    sender: Sender<Message>,
}

impl SenderHelper {
    pub fn do_scene_command<C: Command>(&self, command: C) {
        self.sender
            .send(Message::do_scene_command(command))
            .unwrap();
    }
}

fn make_physics_binding_enum_editor_definition() -> EnumPropertyEditorDefinition<PhysicsBinding> {
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => PhysicsBinding::NodeWithBody,
            1 => PhysicsBinding::BodyWithNode,
            _ => unreachable!(),
        },
        index_generator: |v| *v as usize,
        names_generator: || vec!["Node With Body".to_string(), "Body With Node".to_string()],
    }
}

fn make_mobility_enum_editor_definition() -> EnumPropertyEditorDefinition<Mobility> {
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => Mobility::Static,
            1 => Mobility::Stationary,
            2 => Mobility::Dynamic,
            _ => unreachable!(),
        },
        index_generator: |v| *v as usize,
        names_generator: || {
            vec![
                "Static".to_string(),
                "Stationary".to_string(),
                "Dynamic".to_string(),
            ]
        },
    }
}

fn make_exposure_enum_editor_definition() -> EnumPropertyEditorDefinition<Exposure> {
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => Exposure::default(),
            1 => Exposure::Manual(1.0),
            _ => unreachable!(),
        },
        index_generator: |v| match v {
            Exposure::Auto { .. } => 0,
            Exposure::Manual(_) => 1,
        },
        names_generator: || vec!["Auto".to_string(), "Manual".to_string()],
    }
}

fn make_render_path_enum_editor_definition() -> EnumPropertyEditorDefinition<RenderPath> {
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => RenderPath::Deferred,
            1 => RenderPath::Forward,
            _ => unreachable!(),
        },
        index_generator: |v| *v as usize,
        names_generator: || vec!["Deferred".to_string(), "Forward".to_string()],
    }
}

fn make_property_editors_container(
    sender: Sender<Message>,
) -> Arc<PropertyEditorDefinitionContainer> {
    let mut container = PropertyEditorDefinitionContainer::new();

    container.insert(Arc::new(TexturePropertyEditorDefinition));
    container.insert(Arc::new(MaterialPropertyEditorDefinition {
        sender: Mutex::new(sender),
    }));
    container.insert(Arc::new(
        VecCollectionPropertyEditorDefinition::<Surface>::new(),
    ));
    container.insert(Arc::new(
        VecCollectionPropertyEditorDefinition::<Layer>::new(),
    ));
    container.insert(Arc::new(make_physics_binding_enum_editor_definition()));
    container.insert(Arc::new(make_mobility_enum_editor_definition()));
    container.insert(Arc::new(make_exposure_enum_editor_definition()));
    container.insert(Arc::new(make_render_path_enum_editor_definition()));

    Arc::new(container)
}

impl Inspector {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let property_editors = make_property_editors_container(sender);

        let inspector;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::text("Inspector"))
            .with_content(
                ScrollViewerBuilder::new(WidgetBuilder::new())
                    .with_content({
                        inspector = InspectorBuilder::new(WidgetBuilder::new()).build(ctx);
                        inspector
                    })
                    .build(ctx),
            )
            .build(ctx);

        Self {
            window,
            inspector,
            property_editors,
            needs_sync: true,
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let scene = &engine.scenes[editor_scene.scene];
        let ui = &mut engine.user_interface;

        if self.needs_sync {
            if let Selection::Graph(selection) = &editor_scene.selection {
                if selection.is_single_selection() {
                    let node_handle = selection.nodes()[0];
                    if scene.graph.is_valid_handle(node_handle) {
                        let node = &scene.graph[node_handle];

                        let ctx = ui
                            .node(self.inspector)
                            .cast::<rg3d::gui::inspector::Inspector>()
                            .unwrap()
                            .context()
                            .clone();

                        if let Err(sync_errors) = ctx.sync(node, ui) {
                            for error in sync_errors {
                                Log::writeln(
                                    MessageKind::Error,
                                    format!("Failed to sync property. Reason: {:?}", error),
                                )
                            }
                        }
                    }
                }
            }
        } else {
            self.needs_sync = true;
        }
    }

    pub fn handle_message(
        &mut self,
        message: &Message,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
    ) {
        if let Message::SelectionChanged = message {
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
                            self.property_editors.clone(),
                            Some(environment),
                            MSG_SYNC_FLAG,
                        );

                        self.needs_sync = false;

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
                                handle_base_property_changed(args, node_handle, &helper);
                            } else if args.owner_type_id == TypeId::of::<Transform>() {
                                handle_transform_property_changed(args, node_handle, node, &helper);
                            } else if args.owner_type_id == TypeId::of::<Camera>() {
                                handle_camera_property_changed(args, node_handle, node, &helper);
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
                            } else if args.owner_type_id == TypeId::of::<Terrain>() {
                                handle_terrain_property_changed(
                                    args,
                                    node_handle,
                                    &helper,
                                    &scene.graph,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
