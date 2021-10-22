use crate::inspector::handlers::joint::handle_joint_property_changed;
use crate::physics::{Collider, Joint};
use crate::{
    command::Command,
    inspector::{
        editors::make_property_editors_container,
        handlers::{
            collider::handle_collider_property_changed,
            node::{particle_system::ParticleSystemHandler, SceneNodePropertyChangedHandler},
            rigid_body::handle_rigid_body_property_changed,
            sound::*,
        },
    },
    physics::RigidBody,
    scene::{EditorScene, Selection},
    GameEngine, Message, MSG_SYNC_FLAG,
};
use rg3d::{
    core::{inspect::Inspect, pool::Handle},
    engine::resource_manager::ResourceManager,
    gui::{
        inspector::{
            editors::PropertyEditorDefinitionContainer, InspectorBuilder, InspectorContext,
            InspectorEnvironment,
        },
        message::{InspectorMessage, MessageDirection, UiMessage, UiMessageData},
        scroll_viewer::ScrollViewerBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
    sound::source::{generic::GenericSource, spatial::SpatialSource},
    utils::log::{Log, MessageKind},
};
use std::{
    any::{Any, TypeId},
    rc::Rc,
    sync::mpsc::Sender,
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
    property_editors: Rc<PropertyEditorDefinitionContainer>,
    // Hack. This flag tells whether the inspector should sync with model or not.
    // There is only one situation when it has to be `false` - when inspector has
    // got new context - in this case we don't need to sync with model, because
    // inspector is already in correct state.
    needs_sync: bool,
    node_property_changed_handler: SceneNodePropertyChangedHandler,
}

pub struct SenderHelper {
    sender: Sender<Message>,
}

impl SenderHelper {
    pub fn do_scene_command<C: Command>(&self, command: C) -> Option<()> {
        self.sender
            .send(Message::do_scene_command(command))
            .unwrap();
        Some(())
    }
}

#[macro_export]
macro_rules! do_command {
    ($helper:expr, $cmd:ty, $handle:expr, $value:expr) => {
        $helper.do_scene_command(<$cmd>::new($handle, $value.cast_value().cloned()?))
    };
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
            node_property_changed_handler: SceneNodePropertyChangedHandler {
                particle_system_handler: ParticleSystemHandler::new(ctx),
            },
        }
    }

    fn sync_to(&mut self, obj: &dyn Inspect, ui: &mut UserInterface) {
        let ctx = ui
            .node(self.inspector)
            .cast::<rg3d::gui::inspector::Inspector>()
            .unwrap()
            .context()
            .clone();

        if let Err(sync_errors) = ctx.sync(obj, ui) {
            for error in sync_errors {
                Log::writeln(
                    MessageKind::Error,
                    format!("Failed to sync property. Reason: {:?}", error),
                )
            }
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let scene = &engine.scenes[editor_scene.scene];

        if self.needs_sync {
            if editor_scene.selection.is_single_selection() {
                let ctx = scene.sound_context.state();
                let obj: Option<&dyn Inspect> = match &editor_scene.selection {
                    Selection::Graph(selection) => scene
                        .graph
                        .try_get(selection.nodes()[0])
                        .map(|n| n as &dyn Inspect),
                    Selection::Sound(selection) => ctx
                        .sources()
                        .try_borrow(selection.sources()[0])
                        .map(|s| s as &dyn Inspect),
                    Selection::RigidBody(selection) => editor_scene
                        .physics
                        .bodies
                        .try_borrow(selection.bodies()[0])
                        .map(|s| s as &dyn Inspect),
                    Selection::Joint(selection) => editor_scene
                        .physics
                        .joints
                        .try_borrow(selection.joints()[0])
                        .map(|s| s as &dyn Inspect),
                    _ => None,
                };

                if let Some(obj) = obj {
                    self.sync_to(obj, &mut engine.user_interface);
                }
            }
        } else {
            self.needs_sync = true;
        }
    }

    fn change_context(
        &mut self,
        obj: &dyn Inspect,
        ui: &mut UserInterface,
        resource_manager: ResourceManager,
    ) {
        let environment = Rc::new(EditorEnvironment { resource_manager });

        let context = InspectorContext::from_object(
            obj,
            &mut ui.build_ctx(),
            self.property_editors.clone(),
            Some(environment),
            MSG_SYNC_FLAG,
        );

        self.needs_sync = false;

        ui.send_message(InspectorMessage::context(
            self.inspector,
            MessageDirection::ToWidget,
            context,
        ));
    }

    pub fn handle_message(
        &mut self,
        message: &Message,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
    ) {
        if let Message::SelectionChanged = message {
            let scene = &engine.scenes[editor_scene.scene];

            if editor_scene.selection.is_single_selection() {
                let ctx = scene.sound_context.state();
                let obj: Option<&dyn Inspect> = match &editor_scene.selection {
                    Selection::Graph(selection) => scene
                        .graph
                        .try_get(selection.nodes()[0])
                        .map(|n| n as &dyn Inspect),
                    Selection::Sound(selection) => ctx
                        .sources()
                        .try_borrow(selection.sources()[0])
                        .map(|s| s as &dyn Inspect),
                    Selection::RigidBody(selection) => editor_scene
                        .physics
                        .bodies
                        .try_borrow(selection.bodies()[0])
                        .map(|s| s as &dyn Inspect),
                    Selection::Joint(selection) => editor_scene
                        .physics
                        .joints
                        .try_borrow(selection.joints()[0])
                        .map(|s| s as &dyn Inspect),
                    Selection::Collider(selection) => editor_scene
                        .physics
                        .colliders
                        .try_borrow(selection.colliders()[0])
                        .map(|s| s as &dyn Inspect),
                    _ => None,
                };

                if let Some(obj) = obj {
                    self.change_context(
                        obj,
                        &mut engine.user_interface,
                        engine.resource_manager.clone(),
                    )
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

        let scene = &engine.scenes[editor_scene.scene];

        let mut success = Some(());

        // Special case for particle systems.
        if let Selection::Graph(selection) = &editor_scene.selection {
            if let Some(first) = selection.nodes().first() {
                self.node_property_changed_handler
                    .particle_system_handler
                    .handle_ui_message(message, *first, &helper, &engine.user_interface);
            }
        }

        if editor_scene.selection.is_single_selection()
            && message.destination() == self.inspector
            && message.direction() == MessageDirection::FromWidget
        {
            if let UiMessageData::Inspector(InspectorMessage::PropertyChanged(args)) =
                message.data()
            {
                match &editor_scene.selection {
                    Selection::Graph(selection) => {
                        let node_handle = selection.nodes()[0];
                        if scene.graph.is_valid_handle(node_handle) {
                            success = self.node_property_changed_handler.handle(
                                args,
                                node_handle,
                                &scene.graph[node_handle],
                                &helper,
                                &engine.user_interface,
                                scene,
                            );
                        }
                    }
                    Selection::Sound(selection) => {
                        let source_handle = selection.sources()[0];
                        success = if args.owner_type_id == TypeId::of::<GenericSource>() {
                            handle_generic_source_property_changed(args, source_handle, &helper)
                        } else if args.owner_type_id == TypeId::of::<SpatialSource>() {
                            handle_spatial_source_property_changed(args, source_handle, &helper)
                        } else {
                            Some(())
                        }
                    }
                    Selection::RigidBody(selection) => {
                        let rigid_body_handle = selection.bodies()[0];
                        success = if args.owner_type_id == TypeId::of::<RigidBody>() {
                            handle_rigid_body_property_changed(args, rigid_body_handle, &helper)
                        } else {
                            Some(())
                        }
                    }
                    Selection::Collider(selection) => {
                        let collider_handle = selection.colliders()[0];
                        let collider = &editor_scene.physics.colliders[collider_handle];
                        success = if args.owner_type_id == TypeId::of::<Collider>() {
                            handle_collider_property_changed(
                                args,
                                collider_handle,
                                collider,
                                &helper,
                            )
                        } else {
                            Some(())
                        }
                    }
                    Selection::Joint(selection) => {
                        let joint_handle = selection.joints()[0];
                        let joint = &editor_scene.physics.joints[joint_handle];
                        success = if args.owner_type_id == TypeId::of::<Joint>() {
                            handle_joint_property_changed(args, joint_handle, joint, &helper)
                        } else {
                            Some(())
                        }
                    }
                    _ => {}
                }
            }
        }

        if let UiMessageData::Inspector(InspectorMessage::PropertyChanged(args)) = message.data() {
            if success.is_none() {
                sender
                    .send(Message::Log(format!(
                        "Failed to handle property {}",
                        args.path()
                    )))
                    .unwrap();
            }
        }
    }
}
