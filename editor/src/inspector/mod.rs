use crate::{
    absm::{
        command::{
            pose::make_set_pose_property_command, state::make_set_state_property_command,
            transition::make_set_transition_property_command,
        },
        selection::SelectedEntity,
    },
    animation::{self, command::signal::make_animation_signal_property_command},
    inspector::{
        editors::make_property_editors_container,
        handlers::{
            node::SceneNodePropertyChangedHandler,
            sound_context::handle_sound_context_property_changed,
        },
    },
    scene::{commands::effect::make_set_effect_property_command, EditorScene, Selection},
    utils::window_content,
    Brush, CommandGroup, GameEngine, Message, Mode, WidgetMessage, WrapMode, MSG_SYNC_FLAG,
};
use fyrox::{
    animation::Animation,
    core::{color::Color, pool::Handle, reflect::prelude::*},
    engine::{resource_manager::ResourceManager, SerializationContext},
    gui::{
        grid::{Column, GridBuilder, Row},
        inspector::{
            editors::PropertyEditorDefinitionContainer, InspectorBuilder, InspectorContext,
            InspectorEnvironment, InspectorMessage,
        },
        message::{MessageDirection, UiMessage},
        scroll_viewer::ScrollViewerBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        BuildContext, Thickness, UiNode, UserInterface,
    },
    scene::{
        animation::{absm::AnimationBlendingStateMachine, AnimationPlayer},
        graph::Graph,
    },
    utils::log::{Log, MessageKind},
};
use std::{
    any::Any,
    rc::Rc,
    sync::{mpsc::Sender, Arc},
};

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
    pub sender: Sender<Message>,
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
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let property_editors = Rc::new(make_property_editors_container(sender));

        let warning_text_str =
            "Multiple objects are selected, showing properties of the first object only!\
            Only common properties will be editable!";

        let warning_text;
        let inspector;
        let window = WindowBuilder::new(WidgetBuilder::new())
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
                            ScrollViewerBuilder::new(WidgetBuilder::new().on_row(1))
                                .with_content({
                                    inspector =
                                        InspectorBuilder::new(WidgetBuilder::new()).build(ctx);
                                    inspector
                                })
                                .build(ctx),
                        ),
                )
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
        }
    }

    fn sync_to(&mut self, obj: &dyn Reflect, ui: &mut UserInterface) {
        let ctx = ui
            .node(self.inspector)
            .cast::<fyrox::gui::inspector::Inspector>()
            .unwrap()
            .context()
            .clone();

        if let Err(sync_errors) = ctx.sync(obj, ui, 0, true) {
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
                let obj: Option<&dyn Reflect> = match &editor_scene.selection {
                    Selection::Graph(selection) => scene
                        .graph
                        .try_get(selection.nodes()[0])
                        .map(|n| n.as_reflect()),
                    Selection::SoundContext => Some(&scene.graph.sound_context as &dyn Reflect),
                    Selection::Effect(selection) => scene
                        .graph
                        .sound_context
                        .try_get_effect(selection.effects[0])
                        .map(|e| e as &dyn Reflect),
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
                                    Some(signal as &dyn Reflect)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
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
                                            SelectedEntity::Transition(transition) => Some(
                                                &layer.transitions()[*transition] as &dyn Reflect,
                                            ),
                                            SelectedEntity::State(state) => {
                                                Some(&layer.states()[*state] as &dyn Reflect)
                                            }
                                            SelectedEntity::PoseNode(pose) => {
                                                Some(&layer.nodes()[*pose] as &dyn Reflect)
                                            }
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
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
        obj: &dyn Reflect,
        ui: &mut UserInterface,
        resource_manager: ResourceManager,
        serialization_context: Arc<SerializationContext>,
        graph: &Graph,
        selection: &Selection,
        sender: &Sender<Message>,
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
        sender: &Sender<Message>,
    ) {
        if let Message::SelectionChanged { .. } = message {
            let scene = &engine.scenes[editor_scene.scene];

            engine
                .user_interface
                .send_message(WidgetMessage::visibility(
                    self.warning_text,
                    MessageDirection::ToWidget,
                    editor_scene.selection.len() > 1,
                ));

            if !editor_scene.selection.is_empty() {
                let obj: Option<&dyn Reflect> = match &editor_scene.selection {
                    Selection::Graph(selection) => scene
                        .graph
                        .try_get(selection.nodes()[0])
                        .map(|n| n.as_reflect()),
                    Selection::SoundContext => Some(&scene.graph.sound_context as &dyn Reflect),
                    Selection::Effect(selection) => scene
                        .graph
                        .sound_context
                        .try_get_effect(selection.effects[0])
                        .map(|e| e as &dyn Reflect),
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
                                    Some(signal as &dyn Reflect)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
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
                                            SelectedEntity::Transition(transition) => Some(
                                                &layer.transitions()[*transition] as &dyn Reflect,
                                            ),
                                            SelectedEntity::State(state) => {
                                                Some(&layer.states()[*state] as &dyn Reflect)
                                            }
                                            SelectedEntity::PoseNode(pose) => {
                                                Some(&layer.nodes()[*pose] as &dyn Reflect)
                                            }
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(obj) = obj {
                    self.change_context(
                        obj,
                        &mut engine.user_interface,
                        engine.resource_manager.clone(),
                        engine.serialization_context.clone(),
                        &scene.graph,
                        &editor_scene.selection,
                        sender,
                    )
                }
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
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
        sender: &Sender<Message>,
    ) {
        let scene = &mut engine.scenes[editor_scene.scene];

        if message.destination() == self.inspector
            && message.direction() == MessageDirection::FromWidget
        {
            if let Some(InspectorMessage::PropertyChanged(args)) =
                message.data::<InspectorMessage>()
            {
                let group = match &editor_scene.selection {
                    Selection::Graph(selection) => selection
                        .nodes
                        .iter()
                        .filter_map(|&node_handle| {
                            if scene.graph.is_valid_handle(node_handle) {
                                Some(self.node_property_changed_handler.handle(
                                    args,
                                    node_handle,
                                    &mut scene.graph[node_handle],
                                ))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>(),
                    Selection::SoundContext => handle_sound_context_property_changed(args)
                        .map(|c| vec![c])
                        .unwrap_or_default(),
                    Selection::Effect(selection) => selection
                        .effects
                        .iter()
                        .filter_map(|&handle| make_set_effect_property_command(handle, args))
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
                    Log::err(format!("Failed to handle a property {}", args.path()))
                } else if group.len() == 1 {
                    sender
                        .send(Message::DoSceneCommand(group.into_iter().next().unwrap()))
                        .unwrap()
                } else {
                    sender
                        .send(Message::do_scene_command(CommandGroup::from(group)))
                        .unwrap();
                }
            }
        }
    }
}
