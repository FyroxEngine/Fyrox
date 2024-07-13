use crate::{
    absm::animation_container_ref,
    fyrox::{
        asset::manager::ResourceManager,
        core::{
            color::Color,
            log::{Log, MessageKind},
            pool::ErasedHandle,
            pool::Handle,
            reflect::prelude::*,
        },
        engine::SerializationContext,
        graph::BaseSceneGraph,
        gui::{
            button::ButtonMessage,
            grid::{Column, GridBuilder, Row},
            inspector::{
                editors::PropertyEditorDefinitionContainer, InspectorBuilder, InspectorContext,
                InspectorEnvironment, InspectorError, InspectorMessage,
            },
            message::{MessageDirection, UiMessage},
            scroll_viewer::ScrollViewerBuilder,
            text::{TextBuilder, TextMessage},
            widget::WidgetBuilder,
            window::{WindowBuilder, WindowTitle},
            BuildContext, Thickness, UiNode, UserInterface,
        },
        scene::SceneContainer,
    },
    gui::make_image_button_with_tooltip,
    inspector::editors::make_property_editors_container,
    load_image,
    message::MessageSender,
    scene::{controller::SceneController, GameScene, Selection},
    send_sync_message,
    ui_scene::UiScene,
    utils::window_content,
    Brush, Engine, Message, Mode, WidgetMessage, WrapMode, MSG_SYNC_FLAG,
};
use std::{any::Any, sync::Arc};

pub mod editors;
pub mod handlers;

#[derive(Clone, Debug)]
pub struct AnimationDefinition {
    name: String,
    handle: ErasedHandle,
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
    pub fn try_get_from(environment: &Option<Arc<dyn InspectorEnvironment>>) -> Option<&Self> {
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
    pub property_editors: Arc<PropertyEditorDefinitionContainer>,
    pub(crate) window: Handle<UiNode>,
    inspector: Handle<UiNode>,
    warning_text: Handle<UiNode>,
    type_name_text: Handle<UiNode>,
    docs_button: Handle<UiNode>,
}

fn fetch_available_animations(
    selection: &Selection,
    controller: &dyn SceneController,
    scenes: &SceneContainer,
) -> Vec<AnimationDefinition> {
    if let Some(ui_scene) = controller.downcast_ref::<UiScene>() {
        // TODO: Remove duplicated code.
        if let Some(absm_selection) = selection.as_absm::<UiNode>() {
            if let Some((_, animation_player)) =
                animation_container_ref(&ui_scene.ui, absm_selection.absm_node_handle)
            {
                return animation_player
                    .pair_iter()
                    .map(|(handle, anim)| AnimationDefinition {
                        name: anim.name().to_string(),
                        handle: handle.into(),
                    })
                    .collect();
            }
        }
    }

    if let Some(game_scene) = controller.downcast_ref::<GameScene>() {
        if let Some(absm_selection) = selection.as_absm() {
            if let Some((_, animation_player)) = animation_container_ref(
                &scenes[game_scene.scene].graph,
                absm_selection.absm_node_handle,
            ) {
                return animation_player
                    .pair_iter()
                    .map(|(handle, anim)| AnimationDefinition {
                        name: anim.name().to_string(),
                        handle: handle.into(),
                    })
                    .collect();
            }
        }
    }
    Default::default()
}

fn print_errors(sync_errors: &[InspectorError]) {
    for error in sync_errors {
        Log::writeln(
            MessageKind::Error,
            format!("Failed to sync property. Reason: {:?}", error),
        )
    }
}

fn is_out_of_sync(sync_errors: &[InspectorError]) -> bool {
    sync_errors
        .iter()
        .any(|err| matches!(err, &InspectorError::OutOfSync))
}

impl Inspector {
    pub fn new(ctx: &mut BuildContext, sender: MessageSender) -> Self {
        let property_editors = Arc::new(make_property_editors_container(sender));

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
                                        .with_wrap(WrapMode::Letter)
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
                                            Some(0),
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
            warning_text,
            type_name_text,
            docs_button,
        }
    }

    fn sync_to(
        &mut self,
        obj: &dyn Reflect,
        ui: &mut UserInterface,
    ) -> Result<(), Vec<InspectorError>> {
        let ctx = ui
            .node(self.inspector)
            .cast::<fyrox::gui::inspector::Inspector>()
            .unwrap()
            .context()
            .clone();

        ctx.sync(obj, ui, 0, true, Default::default())
    }

    pub fn sync_to_model(
        &mut self,
        editor_selection: &Selection,
        controller: &dyn SceneController,
        engine: &mut Engine,
        sender: &MessageSender,
    ) {
        let mut need_clear = true;

        let ui = engine.user_interfaces.first_mut();

        ui.send_message(WidgetMessage::visibility(
            self.warning_text,
            MessageDirection::ToWidget,
            editor_selection.len() > 1,
        ));

        controller.first_selected_entity(editor_selection, &engine.scenes, &mut |entity| {
            if let Err(errors) = self.sync_to(entity, ui) {
                if is_out_of_sync(&errors) {
                    let available_animations =
                        fetch_available_animations(editor_selection, controller, &engine.scenes);

                    self.change_context(
                        entity,
                        ui,
                        engine.resource_manager.clone(),
                        engine.serialization_context.clone(),
                        &available_animations,
                        sender,
                    );

                    need_clear = false;
                } else {
                    print_errors(&errors);
                }
            } else {
                need_clear = false;
            }
        });

        if need_clear {
            self.clear(ui);
        }
    }

    fn change_context(
        &mut self,
        obj: &dyn Reflect,
        ui: &mut UserInterface,
        resource_manager: ResourceManager,
        serialization_context: Arc<SerializationContext>,
        available_animations: &[AnimationDefinition],
        sender: &MessageSender,
    ) {
        let environment = Arc::new(EditorEnvironment {
            resource_manager,
            serialization_context,
            available_animations: available_animations.to_vec(),
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
            150.0,
        );

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
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        sender: &MessageSender,
    ) {
        if message.destination() == self.inspector
            && message.direction() == MessageDirection::FromWidget
        {
            if let Some(InspectorMessage::PropertyChanged(args)) =
                message.data::<InspectorMessage>()
            {
                controller.on_property_changed(args, editor_selection, engine);
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.docs_button {
                if let Some(doc) = controller.provide_docs(editor_selection, engine) {
                    sender.send(Message::ShowDocumentation(doc));
                }
            }
        }
    }
}
