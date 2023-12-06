use crate::message::MessageSender;
use crate::{
    absm::{
        command::{
            pose::make_set_pose_property_command, state::make_set_state_property_command,
            transition::make_set_transition_property_command,
        },
        selection::SelectedEntity,
    },
    animation::{self, command::signal::make_animation_signal_property_command},
    gui::make_image_button_with_tooltip,
    inspector::{
        editors::make_property_editors_container, handlers::node::SceneNodePropertyChangedHandler,
    },
    load_image,
    scene::{commands::effect::make_set_audio_bus_property_command, GameScene, Selection},
    send_sync_message,
    utils::window_content,
    Brush, CommandGroup, Engine, Message, Mode, WidgetMessage, WrapMode, MSG_SYNC_FLAG,
};
use fyrox::{
    animation::Animation,
    asset::manager::ResourceManager,
    core::{
        color::Color,
        log::{Log, MessageKind},
        pool::Handle,
        reflect::prelude::*,
    },
    engine::SerializationContext,
    gui::{
        button::ButtonMessage,
        grid::{Column, GridBuilder, Row},
        inspector::{
            editors::PropertyEditorDefinitionContainer, InspectorBuilder, InspectorContext,
            InspectorEnvironment, InspectorMessage,
        },
        message::{MessageDirection, UiMessage},
        scroll_viewer::ScrollViewerBuilder,
        text::{TextBuilder, TextMessage},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        BuildContext, Thickness, UiNode, UserInterface,
    },
    scene::{
        animation::{absm::AnimationBlendingStateMachine, AnimationPlayer},
        graph::Graph,
    },
};
use std::{any::Any, rc::Rc, sync::Arc};

pub mod editors;
pub mod handlers;

pub struct AnimationDefinition {
    name: String,
    handle: Handle<Animation>,
}

pub struct EditorEnvironment {
    pub resource_manager: ResourceManager,
    pub serialization_context: Arc<SerializationContext>,
    /// List of animations definitions (name + handle). It is filled only if current selection
    /// is `AnimationBlendingStateMachine`. The list is filled using ABSM's animation player.
    pub available_animations: Vec<AnimationDefinition>,
    pub sender: MessageSender,
}

impl EditorEnvironment {
    pub fn try_get_from(environment: &Option<Rc<dyn InspectorEnvironment>>) -> Option<&Self> {
        environment
            .as_ref()
            .and_then(|e| e.as_any().downcast_ref::<Self>())
    }
}

impl InspectorEnvironment for EditorEnvironment {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct Inspector {
    /// Allows you to register your property editors for custom types.
    pub property_editors: Rc<PropertyEditorDefinitionContainer>,
    pub(crate) window: Handle<UiNode>,
    inspector: Handle<UiNode>,
    // Hack. This flag tells whether the inspector should sync with model or not.
    // There is only one situation when it has to be `false` - when inspector has
    // got new context - in this case we don't need to sync with model, because
    // inspector is already in correct state.
    needs_sync: bool,
    node_property_changed_handler: SceneNodePropertyChangedHandler,
    warning_text: Handle<UiNode>,
    type_name_text: Handle<UiNode>,
    docs_button: Handle<UiNode>,
}

#[macro_export]
macro_rules! make_command {
    ($cmd:ty, $handle:expr, $value:expr) => {
        Some($crate::scene::commands::SceneCommand::new(<$cmd>::new(
            $handle,
            $value.cast_value().cloned()?,
        )))
    };
}

#[macro_export]
macro_rules! handle_properties {
    ($name:expr, $handle:expr, $value:expr, $($prop:path => $cmd:ty),*) => {
        match $name {
            $($prop => {
                $crate::make_command!($cmd, $handle, $value)
            })*
            _ => None,
        }
    }
}

#[macro_export]
macro_rules! handle_property_changed {
    ($args:expr, $handle:expr, $($prop:path => $cmd:ty),*) => {
        match $args.value {
            FieldKind::Object(ref value) => {
                match $args.name.as_ref() {
                    $($prop => {
                        $crate::make_command!($cmd, $handle, value)
                    })*
                    _ => None,
                }
            }
            _ => None
        }
    }
}

impl Inspector {
    pub fn new(ctx: &mut BuildContext, sender: MessageSender) -> Self {
        let property_editors = Rc::new(make_property_editors_container(sender));

        let warning_text_str =
            "Multiple objects are selected, showing properties of the first object only!\
            Only common properties will be editable!";

        let warning_text;
        let type_name_text;
        let inspector;
        let docs_button;
        let window = WindowBuilder::new(WidgetBuilder::new().with_name("Inspector"))
            .with_title(WindowTitle::text("Inspector"))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            warning_text = TextBuilder::new(
                                WidgetBuilder::new()
                                    .with_visibility(false)
                                    .with_margin(Thickness::left(4.0))
                                    .with_foreground(Brush::Solid(Color::RED))
                                    .on_row(0),
                            )
                            .with_wrap(WrapMode::Word)
                            .with_text(warning_text_str)
                            .build(ctx);
                            warning_text
                        })
                        .with_child(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(1)
                                    .with_child({
                                        type_name_text = TextBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(4.0))
                                                .on_row(0)
                                                .on_column(0),
                                        )
                                        .with_wrap(WrapMode::Word)
                                        .build(ctx);
                                        type_name_text
                                    })
                                    .with_child({
                                        docs_button = make_image_button_with_tooltip(
                                            ctx,
                                            18.0,
                                            18.0,
                                            load_image(include_bytes!("../../resources/doc.png")),
                                            "Open Documentation",
                                        );
                                        ctx[docs_button].set_column(1);
                                        docs_button
                                    }),
                            )
                            .add_row(Row::strict(22.0))
                            .add_column(Column::stretch())
                            .add_column(Column::auto())
                            .build(ctx),
                        )
                        .with_child(
                            ScrollViewerBuilder::new(WidgetBuilder::new().on_row(2))
                                .with_content({
                                    inspector =
                                        InspectorBuilder::new(WidgetBuilder::new()).build(ctx);
                                    inspector
                                })
                                .build(ctx),
                        ),
                )
                .add_row(Row::auto())
                .add_row(Row::auto())
                .add_row(Row::stretch())
                .add_column(Column::stretch())
                .build(ctx),
            )
            .build(ctx);

        Self {
            window,
            inspector,
            property_editors,
            needs_sync: true,
            node_property_changed_handler: SceneNodePropertyChangedHandler,
            warning_text,
            type_name_text,
            docs_button,
        }
    }

    fn sync_to(&mut self, obj: &dyn Reflect, ui: &mut UserInterface) {
        let ctx = ui
            .node(self.inspector)
            .cast::<fyrox::gui::inspector::Inspector>()
            .unwrap()
            .context()
            .clone();

        if let Err(sync_errors) = ctx.sync(obj, ui, 0, true, Default::default()) {
            for error in sync_errors {
                Log::writeln(
                    MessageKind::Error,
                    format!("Failed to sync property. Reason: {:?}", error),
                )
            }
        }
    }

    pub fn sync_to_model(
        &mut self,
        editor_selection: &Selection,
        game_scene: &GameScene,
        engine: &mut Engine,
    ) {
        let scene = &engine.scenes[game_scene.scene];

        if self.needs_sync {
            if editor_selection.is_single_selection() {
                match editor_selection {
                    Selection::Graph(selection) => {
                        if let Some(node) = scene.graph.try_get(selection.nodes()[0]) {
                            node.as_reflect(&mut |node| {
                                self.sync_to(node, &mut engine.user_interface)
                            })
                        }
                    }

                    Selection::AudioBus(selection) => {
                        let state = scene.graph.sound_context.state();
                        if let Some(effect) =
                            state.bus_graph_ref().try_get_bus_ref(selection.buses[0])
                        {
                            self.sync_to(effect as &dyn Reflect, &mut engine.user_interface);
                        }
                    }
                    Selection::Animation(selection) => {
                        if let Some(animation) = scene
                            .graph
                            .try_get_of_type::<AnimationPlayer>(selection.animation_player)
                            .and_then(|player| player.animations().try_get(selection.animation))
                        {
                            if let Some(animation::selection::SelectedEntity::Signal(id)) =
                                selection.entities.first()
                            {
                                if let Some(signal) =
                                    animation.signals().iter().find(|s| s.id == *id)
                                {
                                    self.sync_to(
                                        signal as &dyn Reflect,
                                        &mut engine.user_interface,
                                    );
                                }
                            }
                        }
                    }
                    Selection::Absm(selection) => {
                        if let Some(node) = scene
                            .graph
                            .try_get_of_type::<AnimationBlendingStateMachine>(
                                selection.absm_node_handle,
                            )
                        {
                            if let Some(first) = selection.entities.first() {
                                let machine = node.machine();
                                if let Some(layer_index) = selection.layer {
                                    if let Some(layer) = machine.layers().get(layer_index) {
                                        match first {
                                            SelectedEntity::Transition(transition) => {
                                                self.sync_to(
                                                    &layer.transitions()[*transition]
                                                        as &dyn Reflect,
                                                    &mut engine.user_interface,
                                                );
                                            }
                                            SelectedEntity::State(state) => {
                                                self.sync_to(
                                                    &layer.states()[*state] as &dyn Reflect,
                                                    &mut engine.user_interface,
                                                );
                                            }
                                            SelectedEntity::PoseNode(pose) => {
                                                self.sync_to(
                                                    &layer.nodes()[*pose] as &dyn Reflect,
                                                    &mut engine.user_interface,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => (),
                };
            }
        } else {
            self.needs_sync = true;
        }
    }

    fn change_context(
        &mut self,
        obj: &dyn Reflect,
        ui: &mut UserInterface,
        resource_manager: ResourceManager,
        serialization_context: Arc<SerializationContext>,
        graph: &Graph,
        selection: &Selection,
        sender: &MessageSender,
    ) {
        let environment = Rc::new(EditorEnvironment {
            resource_manager,
            serialization_context,
            available_animations: if let Selection::Absm(absm_selection) = selection {
                if let Some(animation_player) = graph
                    .try_get(absm_selection.absm_node_handle)
                    .and_then(|n| n.query_component_ref::<AnimationBlendingStateMachine>())
                    .and_then(|absm| graph.try_get(absm.animation_player()))
                    .and_then(|n| n.query_component_ref::<AnimationPlayer>())
                {
                    animation_player
                        .animations()
                        .pair_iter()
                        .map(|(handle, anim)| AnimationDefinition {
                            name: anim.name().to_string(),
                            handle,
                        })
                        .collect()
                } else {
                    Default::default()
                }
            } else {
                Default::default()
            },
            sender: sender.clone(),
        });

        let context = InspectorContext::from_object(
            obj,
            &mut ui.build_ctx(),
            self.property_editors.clone(),
            Some(environment),
            MSG_SYNC_FLAG,
            0,
            true,
            Default::default(),
        );

        self.needs_sync = false;

        ui.send_message(InspectorMessage::context(
            self.inspector,
            MessageDirection::ToWidget,
            context,
        ));

        send_sync_message(
            ui,
            TextMessage::text(
                self.type_name_text,
                MessageDirection::ToWidget,
                format!("Type Name: {}", obj.type_name()),
            ),
        );
    }

    pub fn handle_message(
        &mut self,
        message: &Message,
        editor_selection: &Selection,
        game_scene: &GameScene,
        engine: &mut Engine,
        sender: &MessageSender,
    ) {
        if let Message::SelectionChanged { .. } = message {
            let scene = &engine.scenes[game_scene.scene];

            engine
                .user_interface
                .send_message(WidgetMessage::visibility(
                    self.warning_text,
                    MessageDirection::ToWidget,
                    editor_selection.len() > 1,
                ));

            if !editor_selection.is_empty() {
                match &editor_selection {
                    Selection::Graph(selection) => {
                        if let Some(node) = scene.graph.try_get(selection.nodes()[0]) {
                            node.as_reflect(&mut |node| {
                                self.change_context(
                                    node,
                                    &mut engine.user_interface,
                                    engine.resource_manager.clone(),
                                    engine.serialization_context.clone(),
                                    &scene.graph,
                                    editor_selection,
                                    sender,
                                )
                            })
                        }
                    }
                    Selection::AudioBus(selection) => {
                        let state = scene.graph.sound_context.state();
                        if let Some(effect) =
                            state.bus_graph_ref().try_get_bus_ref(selection.buses[0])
                        {
                            self.change_context(
                                effect as &dyn Reflect,
                                &mut engine.user_interface,
                                engine.resource_manager.clone(),
                                engine.serialization_context.clone(),
                                &scene.graph,
                                editor_selection,
                                sender,
                            )
                        }
                    }
                    Selection::Animation(selection) => {
                        if let Some(animation) = scene
                            .graph
                            .try_get_of_type::<AnimationPlayer>(selection.animation_player)
                            .and_then(|player| player.animations().try_get(selection.animation))
                        {
                            if let Some(animation::selection::SelectedEntity::Signal(id)) =
                                selection.entities.first()
                            {
                                if let Some(signal) =
                                    animation.signals().iter().find(|s| s.id == *id)
                                {
                                    self.change_context(
                                        signal as &dyn Reflect,
                                        &mut engine.user_interface,
                                        engine.resource_manager.clone(),
                                        engine.serialization_context.clone(),
                                        &scene.graph,
                                        editor_selection,
                                        sender,
                                    )
                                }
                            }
                        }
                    }
                    Selection::Absm(selection) => {
                        if let Some(node) = scene
                            .graph
                            .try_get(selection.absm_node_handle)
                            .and_then(|n| n.query_component_ref::<AnimationBlendingStateMachine>())
                        {
                            if let Some(first) = selection.entities.first() {
                                let machine = node.machine();
                                if let Some(layer_index) = selection.layer {
                                    if let Some(layer) = machine.layers().get(layer_index) {
                                        match first {
                                            SelectedEntity::Transition(transition) => self
                                                .change_context(
                                                    &layer.transitions()[*transition]
                                                        as &dyn Reflect,
                                                    &mut engine.user_interface,
                                                    engine.resource_manager.clone(),
                                                    engine.serialization_context.clone(),
                                                    &scene.graph,
                                                    editor_selection,
                                                    sender,
                                                ),
                                            SelectedEntity::State(state) => self.change_context(
                                                &layer.states()[*state] as &dyn Reflect,
                                                &mut engine.user_interface,
                                                engine.resource_manager.clone(),
                                                engine.serialization_context.clone(),
                                                &scene.graph,
                                                editor_selection,
                                                sender,
                                            ),
                                            SelectedEntity::PoseNode(pose) => self.change_context(
                                                &layer.nodes()[*pose] as &dyn Reflect,
                                                &mut engine.user_interface,
                                                engine.resource_manager.clone(),
                                                engine.serialization_context.clone(),
                                                &scene.graph,
                                                editor_selection,
                                                sender,
                                            ),
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => (),
                };
            } else {
                self.clear(&engine.user_interface);
            }
        }
    }

    pub fn clear(&self, ui: &UserInterface) {
        ui.send_message(InspectorMessage::context(
            self.inspector,
            MessageDirection::ToWidget,
            Default::default(),
        ));
    }

    pub fn on_mode_changed(&mut self, ui: &UserInterface, mode: &Mode) {
        ui.send_message(WidgetMessage::enabled(
            window_content(self.window, ui),
            MessageDirection::ToWidget,
            mode.is_edit(),
        ));
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_selection: &Selection,
        game_scene: &GameScene,
        engine: &mut Engine,
        sender: &MessageSender,
    ) {
        let scene = &mut engine.scenes[game_scene.scene];

        if message.destination() == self.inspector
            && message.direction() == MessageDirection::FromWidget
        {
            if let Some(InspectorMessage::PropertyChanged(args)) =
                message.data::<InspectorMessage>()
            {
                let group = match editor_selection {
                    Selection::Graph(selection) => selection
                        .nodes
                        .iter()
                        .filter_map(|&node_handle| {
                            if scene.graph.is_valid_handle(node_handle) {
                                self.node_property_changed_handler.handle(
                                    args,
                                    node_handle,
                                    &mut scene.graph[node_handle],
                                )
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>(),
                    Selection::AudioBus(selection) => selection
                        .buses
                        .iter()
                        .filter_map(|&handle| make_set_audio_bus_property_command(handle, args))
                        .collect::<Vec<_>>(),
                    Selection::Animation(selection) => {
                        if scene
                            .graph
                            .try_get_of_type::<AnimationPlayer>(selection.animation_player)
                            .and_then(|player| player.animations().try_get(selection.animation))
                            .is_some()
                        {
                            selection
                                .entities
                                .iter()
                                .filter_map(|e| {
                                    if let animation::selection::SelectedEntity::Signal(id) = e {
                                        make_animation_signal_property_command(
                                            *id,
                                            args,
                                            selection.animation_player,
                                            selection.animation,
                                        )
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        } else {
                            vec![]
                        }
                    }
                    Selection::Absm(selection) => {
                        if scene
                            .graph
                            .try_get(selection.absm_node_handle)
                            .and_then(|n| n.query_component_ref::<AnimationBlendingStateMachine>())
                            .is_some()
                        {
                            if let Some(layer_index) = selection.layer {
                                selection
                                    .entities
                                    .iter()
                                    .filter_map(|ent| match ent {
                                        SelectedEntity::Transition(transition) => {
                                            make_set_transition_property_command(
                                                *transition,
                                                args,
                                                selection.absm_node_handle,
                                                layer_index,
                                            )
                                        }
                                        SelectedEntity::State(state) => {
                                            make_set_state_property_command(
                                                *state,
                                                args,
                                                selection.absm_node_handle,
                                                layer_index,
                                            )
                                        }
                                        SelectedEntity::PoseNode(pose) => {
                                            make_set_pose_property_command(
                                                *pose,
                                                args,
                                                selection.absm_node_handle,
                                                layer_index,
                                            )
                                        }
                                    })
                                    .collect()
                            } else {
                                vec![]
                            }
                        } else {
                            vec![]
                        }
                    }
                    _ => vec![],
                };

                if group.is_empty() {
                    if !args.is_inheritable() {
                        Log::err(format!("Failed to handle a property {}", args.path()))
                    }
                } else if group.len() == 1 {
                    sender.send(Message::DoSceneCommand(group.into_iter().next().unwrap()))
                } else {
                    sender.do_scene_command(CommandGroup::from(group));
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.docs_button {
                let entity = match editor_selection {
                    Selection::None => None,
                    Selection::Graph(graph_selection) => graph_selection
                        .nodes
                        .first()
                        .map(|h| scene.graph[*h].doc().to_string()),
                    Selection::Navmesh(navmesh_selection) => Some(
                        scene.graph[navmesh_selection.navmesh_node()]
                            .doc()
                            .to_string(),
                    ),
                    Selection::AudioBus(audio_bus_selection) => {
                        audio_bus_selection.buses.first().and_then(|h| {
                            scene
                                .graph
                                .sound_context
                                .state()
                                .bus_graph_ref()
                                .try_get_bus_ref(*h)
                                .map(|bus| bus.doc().to_string())
                        })
                    }
                    Selection::Absm(absm_selection) => Some(
                        scene.graph[absm_selection.absm_node_handle]
                            .doc()
                            .to_string(),
                    ),
                    Selection::Animation(animation_selection) => Some(
                        scene.graph[animation_selection.animation_player]
                            .doc()
                            .to_string(),
                    ),
                };

                if let Some(doc) = entity {
                    sender.send(Message::ShowDocumentation(doc));
                }
            }
        }
    }
}
