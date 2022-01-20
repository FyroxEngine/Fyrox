use crate::inspector::handlers::sound_context::handle_sound_context_property_changed;
use crate::{
    inspector::{
        editors::make_property_editors_container,
        handlers::node::{particle_system::ParticleSystemHandler, SceneNodePropertyChangedHandler},
    },
    scene::{EditorScene, Selection},
    Brush, CommandGroup, GameEngine, Message, WidgetMessage, WrapMode, MSG_SYNC_FLAG,
};
use fyrox::{
    core::{color::Color, inspect::Inspect, pool::Handle},
    engine::resource_manager::ResourceManager,
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
    utils::log::{Log, MessageKind},
};
use std::{any::Any, rc::Rc, sync::mpsc::Sender};

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
    warning_text: Handle<UiNode>,
}

#[macro_export]
macro_rules! make_command {
    ($cmd:ty, $handle:expr, $value:expr) => {
        Some(crate::scene::commands::SceneCommand::new(<$cmd>::new(
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
                crate::make_command!($cmd, $handle, $value)
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
                        crate::make_command!($cmd, $handle, value)
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
        let property_editors = make_property_editors_container(sender);

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
            node_property_changed_handler: SceneNodePropertyChangedHandler {
                particle_system_handler: ParticleSystemHandler::new(ctx),
            },
            warning_text,
        }
    }

    fn sync_to(&mut self, obj: &dyn Inspect, ui: &mut UserInterface) {
        let ctx = ui
            .node(self.inspector)
            .cast::<fyrox::gui::inspector::Inspector>()
            .unwrap()
            .context()
            .clone();

        if let Err(sync_errors) = ctx.sync(obj, ui, 0) {
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
                let obj: Option<&dyn Inspect> = match &editor_scene.selection {
                    Selection::Graph(selection) => scene
                        .graph
                        .try_get(selection.nodes()[0])
                        .map(|n| n as &dyn Inspect),
                    Selection::SoundContext => Some(&scene.graph.sound_context as &dyn Inspect),
                    Selection::Effect(selection) => scene
                        .graph
                        .sound_context
                        .try_get_effect(selection.effects[0])
                        .map(|e| e as &dyn Inspect),
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
            0,
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

            engine
                .user_interface
                .send_message(WidgetMessage::visibility(
                    self.warning_text,
                    MessageDirection::ToWidget,
                    editor_scene.selection.len() > 1,
                ));

            if !editor_scene.selection.is_empty() {
                let obj: Option<&dyn Inspect> = match &editor_scene.selection {
                    Selection::Graph(selection) => scene
                        .graph
                        .try_get(selection.nodes()[0])
                        .map(|n| n as &dyn Inspect),
                    Selection::SoundContext => Some(&scene.graph.sound_context as &dyn Inspect),
                    Selection::Effect(selection) => scene
                        .graph
                        .sound_context
                        .try_get_effect(selection.effects[0])
                        .map(|e| e as &dyn Inspect),
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
        let scene = &engine.scenes[editor_scene.scene];

        // Special case for particle systems.
        if let Selection::Graph(selection) = &editor_scene.selection {
            if let Some(group) = self
                .node_property_changed_handler
                .particle_system_handler
                .handle_ui_message(message, selection, &engine.user_interface)
            {
                sender
                    .send(Message::do_scene_command(CommandGroup::from(group)))
                    .unwrap();
            }
        }

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
                                self.node_property_changed_handler.handle(
                                    args,
                                    node_handle,
                                    &scene.graph[node_handle],
                                    &engine.user_interface,
                                    scene,
                                )
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>(),
                    Selection::SoundContext => handle_sound_context_property_changed(args)
                        .map(|c| vec![c])
                        .unwrap_or_default(),
                    _ => vec![],
                };

                if group.is_empty() {
                    sender
                        .send(Message::Log(format!(
                            "Failed to handle a property {}",
                            args.path()
                        )))
                        .unwrap();
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
