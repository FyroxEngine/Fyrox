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
    fyrox::{
        asset::manager::ResourceManager,
        core::{
            log::{Log, MessageKind},
            pool::{ErasedHandle, Handle},
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
    load_image,
    message::MessageSender,
    plugin::EditorPlugin,
    plugins::{absm::animation_container_ref, inspector::editors::make_property_editors_container},
    scene::{controller::SceneController, GameScene, Selection},
    send_sync_message,
    ui_scene::UiScene,
    utils::window_content,
    Editor, Message, WidgetMessage, WrapMode, MSG_SYNC_FLAG,
};
use fyrox::gui::style::resource::StyleResource;
use fyrox::{
    core::type_traits::prelude::*,
    gui::{
        border::BorderBuilder,
        inspector::InspectorContextArgs,
        stack_panel::StackPanelBuilder,
        style::{resource::StyleResourceExt, Style},
        utils::make_image_button_with_tooltip,
    },
};
use std::{any::Any, sync::mpsc::Sender, sync::Arc};

pub mod editors;
pub mod handlers;

#[derive(Clone, Debug)]
pub struct AnimationDefinition {
    name: String,
    handle: ErasedHandle,
}

#[derive(ComponentProvider)]
pub struct EditorEnvironment {
    pub resource_manager: ResourceManager,
    pub serialization_context: Arc<SerializationContext>,
    /// List of animations definitions (name + handle). It is filled only if current selection
    /// is `AnimationBlendingStateMachine`. The list is filled using ABSM's animation player.
    pub available_animations: Vec<AnimationDefinition>,
    pub sender: MessageSender,
    pub icon_request_sender: Sender<IconRequest>,
    #[component(include)]
    pub style: Option<StyleResource>,
}

impl EditorEnvironment {
    pub fn try_get_from(
        environment: &Option<Arc<dyn InspectorEnvironment>>,
    ) -> Result<&Self, InspectorError> {
        let environment = &**environment.as_ref().ok_or(InspectorError::Custom(
            "Missing InspectorEnvironment".into(),
        ))?;
        environment
            .as_any()
            .downcast_ref::<Self>()
            .ok_or(InspectorError::Custom(format!(
                "Expected InspectorEnvironment to be EditorEnvironment, found: {}",
                environment.name(),
            )))
    }
}

impl InspectorEnvironment for EditorEnvironment {
    fn name(&self) -> String {
        format!("EditorEnvironment:{:?}", self.type_id())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct InspectorPlugin {
    /// Allows you to register your property editors for custom types.
    pub property_editors: Arc<PropertyEditorDefinitionContainer>,
    pub(crate) window: Handle<UiNode>,
    pub inspector: Handle<UiNode>,
    pub head: Handle<UiNode>,
    pub footer: Handle<UiNode>,
    warning_text: Handle<UiNode>,
    type_name_text: Handle<UiNode>,
    docs_button: Handle<UiNode>,
    clipboard: Option<Box<dyn Reflect>>,
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

fn current_widget_style(
    selection: &Selection,
    controller: &dyn SceneController,
) -> Option<StyleResource> {
    if let Some(ui_scene) = controller.downcast_ref::<UiScene>() {
        if let Some(ui_selection) = selection.as_ui() {
            return ui_scene
                .ui
                .try_get_node(ui_selection.widgets[0])
                .and_then(|n| n.style.clone());
        }
    }
    None
}

fn print_errors(sync_errors: &[InspectorError]) {
    for error in sync_errors {
        Log::writeln(
            MessageKind::Error,
            format!("Failed to sync property. Reason: {error:?}"),
        )
    }
}

fn is_out_of_sync(sync_errors: &[InspectorError]) -> bool {
    sync_errors
        .iter()
        .any(|err| matches!(err, &InspectorError::OutOfSync))
}

impl InspectorPlugin {
    pub fn new(
        ctx: &mut BuildContext,
        sender: MessageSender,
        resource_manager: ResourceManager,
    ) -> Self {
        let property_editors = Arc::new(make_property_editors_container(sender, resource_manager));

        let warning_text_str =
            "Multiple objects are selected, showing properties of the first object only!\
            Only common properties will be editable!";

        let head = StackPanelBuilder::new(WidgetBuilder::new()).build(ctx);
        let footer = BorderBuilder::new(WidgetBuilder::new().on_row(3)).build(ctx);
        let inspector = InspectorBuilder::new(WidgetBuilder::new()).build(ctx);
        let content =
            StackPanelBuilder::new(WidgetBuilder::new().with_child(head).with_child(inspector))
                .build(ctx);

        let warning_text;
        let type_name_text;
        let docs_button;
        let window = WindowBuilder::new(WidgetBuilder::new().with_name("Inspector"))
            .with_title(WindowTitle::text("Inspector"))
            .with_tab_label("Inspector")
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            warning_text = TextBuilder::new(
                                WidgetBuilder::new()
                                    .with_visibility(false)
                                    .with_margin(Thickness::left(4.0))
                                    .with_foreground(ctx.style.property(Style::BRUSH_ERROR))
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
                                            load_image!("../../../resources/doc.png"),
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
                                .with_content(content)
                                .build(ctx),
                        )
                        .with_child(footer),
                )
                .add_row(Row::auto())
                .add_row(Row::auto())
                .add_row(Row::stretch())
                .add_row(Row::auto())
                .add_column(Column::stretch())
                .build(ctx),
            )
            .build(ctx);

        Self {
            window,
            inspector,
            head,
            property_editors,
            warning_text,
            type_name_text,
            docs_button,
            clipboard: None,
            footer,
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

        ctx.sync(obj, ui, 0, true, Default::default(), Default::default())
    }

    fn change_context(
        &mut self,
        obj: &dyn Reflect,
        ui: &mut UserInterface,
        resource_manager: ResourceManager,
        serialization_context: Arc<SerializationContext>,
        available_animations: &[AnimationDefinition],
        sender: &MessageSender,
        icon_request_sender: Sender<IconRequest>,
        has_parent_object: bool,
        style: Option<StyleResource>,
    ) {
        let environment = Arc::new(EditorEnvironment {
            resource_manager,
            serialization_context,
            available_animations: available_animations.to_vec(),
            sender: sender.clone(),
            icon_request_sender,
            style,
        });

        let context = InspectorContext::from_object(InspectorContextArgs {
            object: obj,
            ctx: &mut ui.build_ctx(),
            definition_container: self.property_editors.clone(),
            environment: Some(environment),
            sync_flag: MSG_SYNC_FLAG,
            layer_index: 0,
            generate_property_string_values: true,
            filter: Default::default(),
            name_column_width: 150.0,
            base_path: Default::default(),
            has_parent_object,
        });

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

    fn clear(&self, ui: &UserInterface) {
        ui.send_message(InspectorMessage::context(
            self.inspector,
            MessageDirection::ToWidget,
            Default::default(),
        ));
    }
}

impl EditorPlugin for InspectorPlugin {
    fn on_sync_to_model(&mut self, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();

        let Some(entry) = editor.scenes.current_scene_entry_mut() else {
            self.clear(ui);
            return;
        };

        let mut need_clear = true;

        ui.send_message(WidgetMessage::visibility(
            self.warning_text,
            MessageDirection::ToWidget,
            entry.selection.len() > 1,
        ));

        entry.selection.first_selected_entity(
            &*entry.controller,
            &editor.engine.scenes,
            &mut |entity, has_parent_object| {
                if let Err(errors) = self.sync_to(entity, ui) {
                    if is_out_of_sync(&errors) {
                        let available_animations = fetch_available_animations(
                            &entry.selection,
                            &*entry.controller,
                            &editor.engine.scenes,
                        );

                        let style = current_widget_style(&entry.selection, &*entry.controller);

                        self.change_context(
                            entity,
                            ui,
                            editor.engine.resource_manager.clone(),
                            editor.engine.serialization_context.clone(),
                            &available_animations,
                            &editor.message_sender,
                            editor.asset_browser.preview_sender.clone(),
                            has_parent_object,
                            style,
                        );

                        need_clear = false;
                    } else {
                        print_errors(&errors);
                    }
                } else {
                    need_clear = false;
                }
            },
        );

        if need_clear {
            self.clear(ui);
        }
    }

    fn on_mode_changed(&mut self, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first();

        ui.send_message(WidgetMessage::enabled(
            window_content(self.window, ui),
            MessageDirection::ToWidget,
            editor.mode.is_edit(),
        ));
    }

    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        let Some(entry) = editor.scenes.current_scene_entry_mut() else {
            return;
        };

        if (message.destination() == self.inspector
            || editor
                .engine
                .user_interfaces
                .first()
                .is_node_child_of(message.destination(), self.inspector))
            && message.direction() == MessageDirection::FromWidget
        {
            if let Some(msg) = message.data::<InspectorMessage>() {
                match msg {
                    InspectorMessage::CopyValue { path } => {
                        entry.selection.first_selected_entity(
                            &*entry.controller,
                            &editor.engine.scenes,
                            &mut |entity, _| {
                                entity.resolve_path(path, &mut |result| {
                                    if let Ok(result) = result {
                                        self.clipboard = result.try_clone_box();
                                    }
                                });
                            },
                        );
                    }
                    InspectorMessage::PasteValue { dest } => {
                        if let Some(value) = self.clipboard.as_ref() {
                            entry
                                .selection
                                .paste_property(dest, &**value, &editor.message_sender);
                        }
                    }
                    InspectorMessage::PropertyContextMenuOpened { path } => {
                        let mut can_paste = false;
                        let mut can_copy = false;

                        // TODO: This could work incorrectly in case of multiselection of objects
                        // of different types.
                        entry.selection.first_selected_entity(
                            &*entry.controller,
                            &editor.engine.scenes,
                            &mut |entity, _| {
                                entity.resolve_path(path, &mut |result| {
                                    if let Ok(property) = result {
                                        can_copy = property.try_clone_box().is_some();

                                        if let Some(value) = self.clipboard.as_ref() {
                                            value.as_any(&mut |value| {
                                                property.as_any(&mut |property| {
                                                    can_paste =
                                                        property.type_id() == value.type_id();
                                                })
                                            })
                                        }
                                    }
                                });
                            },
                        );

                        editor.engine.user_interfaces.first().send_message(
                            InspectorMessage::property_context_menu_status(
                                message.destination(),
                                MessageDirection::ToWidget,
                                can_copy,
                                can_paste,
                            ),
                        )
                    }
                    _ => (),
                }
            }
        }

        if let Some(InspectorMessage::PropertyChanged(args)) =
            message.data_from::<InspectorMessage>(self.inspector)
        {
            entry.selection.on_property_changed(
                &mut *entry.controller,
                args,
                &mut editor.engine,
                &editor.message_sender,
            );
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.docs_button {
                if let Some(doc) = entry
                    .selection
                    .provide_docs(&*entry.controller, &editor.engine)
                {
                    editor.message_sender.send(Message::ShowDocumentation(doc));
                }
            }
        }
    }
}
