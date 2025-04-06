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

//! Settings window plugin.

use crate::{
    fyrox::{
        core::{log::Log, parking_lot::lock_api::Mutex, pool::Handle, some_or_return},
        engine::Engine,
        graph::{BaseSceneGraph, SceneGraph},
        gui::{
            button::{ButtonBuilder, ButtonMessage},
            dock::DockingManagerMessage,
            grid::{Column, GridBuilder, Row},
            inspector::{
                editors::{
                    collection::VecCollectionPropertyEditorDefinition,
                    enumeration::EnumPropertyEditorDefinition,
                    inspectable::InspectablePropertyEditorDefinition,
                    key::HotKeyPropertyEditorDefinition, PropertyEditorDefinitionContainer,
                },
                Inspector, InspectorBuilder, InspectorContext, InspectorMessage, PropertyAction,
            },
            menu::MenuItemMessage,
            message::{MessageDirection, UiMessage},
            scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
            searchbar::{SearchBarBuilder, SearchBarMessage},
            stack_panel::StackPanelBuilder,
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
        },
        renderer::{CsmSettings, QualitySettings, ShadowMapPrecision},
    },
    menu::create_menu_item,
    message::MessageSender,
    plugin::EditorPlugin,
    settings::{
        build::BuildSettings,
        camera::CameraSettings,
        debugging::DebuggingSettings,
        general::{EditorStyle, GeneralSettings, ScriptEditor},
        graphics::GraphicsSettings,
        keys::{KeyBindings, TerrainKeyBindings},
        model::ModelSettings,
        move_mode::MoveInteractionModeSettings,
        navmesh::NavmeshSettings,
        rotate_mode::RotateInteractionModeSettings,
        selection::SelectionSettings,
        Settings,
    },
    Editor, MSG_SYNC_FLAG,
};
use fyrox::asset::manager::ResourceManager;
use fyrox_build_tools::{BuildProfile, CommandDescriptor, EnvironmentVariable};
use rust_fuzzy_search::fuzzy_compare;
use std::sync::Arc;

fn make_property_editors_container(
    sender: MessageSender,
    resource_manager: ResourceManager,
) -> Arc<PropertyEditorDefinitionContainer> {
    let container = crate::plugins::inspector::editors::make_property_editors_container(
        sender,
        resource_manager,
    );

    container.insert(InspectablePropertyEditorDefinition::<GeneralSettings>::new());
    container.insert(InspectablePropertyEditorDefinition::<GraphicsSettings>::new());
    container.insert(InspectablePropertyEditorDefinition::<SelectionSettings>::new());
    container.insert(EnumPropertyEditorDefinition::<ShadowMapPrecision>::new());
    container.insert(EnumPropertyEditorDefinition::<ScriptEditor>::new());
    container.insert(EnumPropertyEditorDefinition::<EditorStyle>::new());
    container.insert(InspectablePropertyEditorDefinition::<DebuggingSettings>::new());
    container.insert(InspectablePropertyEditorDefinition::<CsmSettings>::new());
    container.insert(InspectablePropertyEditorDefinition::<QualitySettings>::new());
    container.insert(InspectablePropertyEditorDefinition::<CameraSettings>::new());
    container.insert(InspectablePropertyEditorDefinition::<
        MoveInteractionModeSettings,
    >::new());
    container.insert(InspectablePropertyEditorDefinition::<
        RotateInteractionModeSettings,
    >::new());
    container.insert(InspectablePropertyEditorDefinition::<ModelSettings>::new());
    container.insert(InspectablePropertyEditorDefinition::<NavmeshSettings>::new());
    container.insert(InspectablePropertyEditorDefinition::<KeyBindings>::new());
    container.insert(InspectablePropertyEditorDefinition::<TerrainKeyBindings>::new());
    container.insert(InspectablePropertyEditorDefinition::<BuildSettings>::new());
    container.insert(VecCollectionPropertyEditorDefinition::<EnvironmentVariable>::new());
    container.insert(InspectablePropertyEditorDefinition::<EnvironmentVariable>::new());
    container.insert(VecCollectionPropertyEditorDefinition::<BuildProfile>::new());
    container.insert(InspectablePropertyEditorDefinition::<BuildProfile>::new());
    container.insert(VecCollectionPropertyEditorDefinition::<CommandDescriptor>::new());
    container.insert(InspectablePropertyEditorDefinition::<CommandDescriptor>::new());
    container.insert(HotKeyPropertyEditorDefinition);
    Arc::new(container)
}

#[derive(Clone, PartialEq)]
struct GroupName(String);

pub struct SettingsWindow {
    pub window: Handle<UiNode>,
    ok: Handle<UiNode>,
    default: Handle<UiNode>,
    inspector: Handle<UiNode>,
    groups: Handle<UiNode>,
    scroll_viewer: Handle<UiNode>,
    search_bar: Handle<UiNode>,
}

impl SettingsWindow {
    pub fn new(engine: &mut Engine) -> Self {
        let ok;
        let default;

        let ctx = &mut engine.user_interfaces.first_mut().build_ctx();

        let search_bar = SearchBarBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .on_column(0)
                .with_uniform_margin(2.0),
        )
        .build(ctx);

        let inspector = InspectorBuilder::new(WidgetBuilder::new()).build(ctx);

        let groups = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .on_column(0)
                .with_uniform_margin(2.0),
        )
        .with_orientation(Orientation::Vertical)
        .build(ctx);

        let scroll_viewer = ScrollViewerBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .on_row(0)
                .on_column(1),
        )
        .with_content(inspector)
        .build(ctx);

        let inner_content = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .on_column(0)
                .with_child(groups)
                .with_child(scroll_viewer),
        )
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_row(Row::stretch())
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(500.0).with_height(600.0))
            .open(false)
            .with_title(WindowTitle::text("Settings"))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(search_bar)
                        .with_child(inner_content)
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(2)
                                    .with_horizontal_alignment(HorizontalAlignment::Right)
                                    .with_child({
                                        default = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_width(80.0)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("Default")
                                        .build(ctx);
                                        default
                                    })
                                    .with_child({
                                        ok = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_width(80.0)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("OK")
                                        .build(ctx);
                                        ok
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        ),
                )
                .add_row(Row::strict(25.0))
                .add_row(Row::stretch())
                .add_row(Row::strict(25.0))
                .add_column(Column::stretch())
                .build(ctx),
            )
            .build(ctx);

        Self {
            window,
            ok,
            default,
            inspector,
            groups,
            scroll_viewer,
            search_bar,
        }
    }

    pub fn open(
        &self,
        ui: &mut UserInterface,
        settings: &Settings,
        sender: &MessageSender,
        resource_manager: ResourceManager,
    ) {
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
            true,
        ));

        self.sync_to_model(ui, settings, sender, resource_manager);
    }

    fn sync_to_model(
        &self,
        ui: &mut UserInterface,
        settings: &Settings,
        sender: &MessageSender,
        resource_manager: ResourceManager,
    ) {
        let ctx = &mut ui.build_ctx();
        let context = InspectorContext::from_object(
            &**settings,
            ctx,
            make_property_editors_container(sender.clone(), resource_manager),
            None,
            MSG_SYNC_FLAG,
            0,
            true,
            Default::default(),
            150.0,
        );
        let groups =
            context
                .entries
                .iter()
                .map(|entry| {
                    ButtonBuilder::new(WidgetBuilder::new().with_user_data(Arc::new(Mutex::new(
                        GroupName(entry.property_tag.clone()),
                    ))))
                    .with_text(&entry.property_display_name)
                    .build(ctx)
                })
                .collect::<Vec<_>>();
        ui.send_message(WidgetMessage::replace_children(
            self.groups,
            MessageDirection::ToWidget,
            groups,
        ));
        ui.send_message(InspectorMessage::context(
            self.inspector,
            MessageDirection::ToWidget,
            context,
        ));
    }

    fn apply_filter(&self, filter_text: &str, ui: &UserInterface) {
        fn apply_recursive(
            filter_text: &str,
            inspector: Handle<UiNode>,
            ui: &UserInterface,
        ) -> bool {
            let inspector = some_or_return!(ui.try_get_of_type::<Inspector>(inspector), false);

            let mut is_any_match = false;
            for entry in inspector.context.entries.iter() {
                // First look at any inner inspectors, because they could also contain properties
                // matching search criteria.
                let mut inner_match = false;
                let sub_inspector = ui.find_handle(entry.property_editor, &mut |node| {
                    node.has_component::<Inspector>()
                });
                if sub_inspector.is_some() {
                    inner_match |= apply_recursive(filter_text, sub_inspector, ui);
                }

                let display_name = entry.property_display_name.to_lowercase();
                inner_match |= display_name.contains(filter_text)
                    || fuzzy_compare(filter_text, display_name.as_str()) >= 0.5;

                ui.send_message(WidgetMessage::visibility(
                    entry.property_container,
                    MessageDirection::ToWidget,
                    inner_match,
                ));

                is_any_match |= inner_match;
            }

            is_any_match
        }

        apply_recursive(filter_text, self.inspector, ui);
    }

    pub fn handle_ui_message(
        self,
        message: &UiMessage,
        engine: &mut Engine,
        settings: &mut Settings,
        sender: &MessageSender,
        docking_manager: Handle<UiNode>,
    ) -> Option<Self> {
        let ui = engine.user_interfaces.first_mut();

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.ok {
                ui.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));
            } else if message.destination() == self.default {
                **settings = Default::default();

                self.sync_to_model(ui, settings, sender, engine.resource_manager.clone());
            }

            if let Some(node) = ui.try_get(message.destination()) {
                if let Some(user_data) = node.user_data_cloned::<GroupName>() {
                    let inspector = ui.try_get_of_type::<Inspector>(self.inspector).unwrap();

                    if let Some(entry) = inspector.context.find_property_editor_by_tag(&user_data.0)
                    {
                        ui.send_message(ScrollViewerMessage::bring_into_view(
                            self.scroll_viewer,
                            MessageDirection::ToWidget,
                            entry.property_container,
                        ));
                    }
                }
            }
        } else if let Some(InspectorMessage::PropertyChanged(property_changed)) = message.data() {
            if message.destination() == self.inspector {
                PropertyAction::from_field_kind(&property_changed.value).apply(
                    &property_changed.path(),
                    &mut **settings,
                    &mut Log::verify,
                );
            }
        } else if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.window {
                ui.send_message(WidgetMessage::remove(
                    self.window,
                    MessageDirection::ToWidget,
                ));
                ui.send_message(DockingManagerMessage::remove_floating_window(
                    docking_manager,
                    MessageDirection::ToWidget,
                    self.window,
                ));
                return None;
            }
        } else if let Some(SearchBarMessage::Text(search_text)) = message.data() {
            if message.destination() == self.search_bar
                && message.direction() == MessageDirection::FromWidget
            {
                let filter = search_text.to_lowercase();
                self.apply_filter(&filter, ui);
            }
        }

        let graphics_context = engine.graphics_context.as_initialized_mut();

        if settings.graphics.quality != graphics_context.renderer.get_quality_settings() {
            if let Err(e) = graphics_context
                .renderer
                .set_quality_settings(&settings.graphics.quality)
            {
                Log::err(format!(
                    "An error occurred at attempt to set new graphics settings: {e:?}"
                ));
            } else {
                Log::info("New graphics quality settings were successfully set!");
            }
        }

        Some(self)
    }
}

#[derive(Default)]
pub struct SettingsPlugin {
    window: Option<SettingsWindow>,
    open_settings: Handle<UiNode>,
}

impl SettingsPlugin {
    fn on_open_settings_clicked(&mut self, editor: &mut Editor) {
        let window = self
            .window
            .get_or_insert_with(|| SettingsWindow::new(&mut editor.engine));
        let ui = editor.engine.user_interfaces.first_mut();
        window.open(
            ui,
            &editor.settings,
            &editor.message_sender,
            editor.engine.resource_manager.clone(),
        );
        ui.send_message(DockingManagerMessage::add_floating_window(
            editor.docking_manager,
            MessageDirection::ToWidget,
            window.window,
        ));
    }
}

impl EditorPlugin for SettingsPlugin {
    fn on_start(&mut self, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();
        let ctx = &mut ui.build_ctx();
        self.open_settings = create_menu_item("Editor Settings...", vec![], ctx);
        ui.send_message(MenuItemMessage::add_item(
            editor.menu.file_menu.menu,
            MessageDirection::ToWidget,
            self.open_settings,
        ));
    }

    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.open_settings {
                self.on_open_settings_clicked(editor);
            }
        }

        let window = some_or_return!(self.window.take());
        self.window = window.handle_ui_message(
            message,
            &mut editor.engine,
            &mut editor.settings,
            &editor.message_sender,
            editor.docking_manager,
        );
    }
}
