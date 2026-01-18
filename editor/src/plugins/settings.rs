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

use crate::menu::create_menu_item_shortcut;
use crate::{
    fyrox::{
        core::{
            log::Log, parking_lot::lock_api::Mutex, pool::Handle, reflect::Reflect, some_or_return,
        },
        engine::Engine,
        graph::SceneGraph,
        gui::{
            button::{ButtonBuilder, ButtonMessage},
            dock::DockingManagerMessage,
            grid::{Column, GridBuilder, Row},
            inspector::{
                editors::PropertyEditorDefinitionContainer, Inspector, InspectorBuilder,
                InspectorContext, InspectorContextArgs, InspectorMessage, PropertyAction,
            },
            menu::MenuItemMessage,
            message::UiMessage,
            scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
            searchbar::{SearchBarBuilder, SearchBarMessage},
            stack_panel::StackPanelBuilder,
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
        },
    },
    load_image,
    plugin::EditorPlugin,
    settings::Settings,
    Editor,
};
use fyrox::core::{ok_or_return, uuid, Uuid};
use fyrox::engine::GraphicsContext;
use fyrox::gui::button::Button;
use fyrox::gui::text_box::EmptyTextPlaceholder;
use fyrox::gui::window::{Window, WindowAlignment};
use rust_fuzzy_search::fuzzy_compare;
use std::sync::Arc;

#[derive(Clone, PartialEq)]
struct GroupName(String);

pub struct SettingsWindow {
    pub window: Handle<Window>,
    ok: Handle<Button>,
    default: Handle<Button>,
    inspector: Handle<UiNode>,
    groups: Handle<UiNode>,
    scroll_viewer: Handle<UiNode>,
    search_bar: Handle<UiNode>,
    clipboard: Option<Box<dyn Reflect>>,
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
        .with_empty_text_placeholder(EmptyTextPlaceholder::Text("Search for a setting"))
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

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(700.0).with_height(800.0))
            .open(false)
            .with_title(WindowTitle::text("Settings"))
            .with_tab_label("Settings")
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
            clipboard: None,
        }
    }

    pub fn open(
        &self,
        ui: &mut UserInterface,
        settings: &Settings,
        property_editors: Arc<PropertyEditorDefinitionContainer>,
    ) {
        ui.send(
            self.window,
            WindowMessage::Open {
                alignment: WindowAlignment::Center,
                modal: false,
                focus_content: true,
            },
        );

        self.sync_to_model(ui, settings, property_editors);
    }

    fn sync_to_model(
        &self,
        ui: &mut UserInterface,
        settings: &Settings,
        property_editors: Arc<PropertyEditorDefinitionContainer>,
    ) {
        let ctx = &mut ui.build_ctx();
        let context = InspectorContext::from_object(InspectorContextArgs {
            object: &**settings,
            ctx,
            definition_container: property_editors,
            environment: None,
            layer_index: 0,
            generate_property_string_values: true,
            filter: Default::default(),
            name_column_width: 250.0,
            base_path: Default::default(),
            has_parent_object: false,
        });
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
                    .to_base()
                })
                .collect::<Vec<_>>();
        ui.send(self.groups, WidgetMessage::ReplaceChildren(groups));
        ui.send(self.inspector, InspectorMessage::Context(context));
    }

    fn apply_filter(&self, filter_text: &str, ui: &UserInterface) {
        fn apply_recursive(
            filter_text: &str,
            inspector: Handle<UiNode>,
            ui: &UserInterface,
        ) -> bool {
            let inspector = ok_or_return!(ui.try_get_of_type::<Inspector>(inspector), false);

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

                ui.send(
                    entry.property_container,
                    WidgetMessage::Visibility(inner_match),
                );

                is_any_match |= inner_match;
            }

            is_any_match
        }

        apply_recursive(filter_text, self.inspector, ui);
    }

    pub fn handle_ui_message(
        mut self,
        message: &UiMessage,
        engine: &mut Engine,
        settings: &mut Settings,
        docking_manager: Handle<UiNode>,
        property_editors: Arc<PropertyEditorDefinitionContainer>,
    ) -> Option<Self> {
        let ui = engine.user_interfaces.first_mut();

        if message.data::<InspectorMessage>().is_some() {
            // This is tricky - since Settings has DerefMut impl, it causes infinite syncing loop
            // if called on each message.
            let settings_data = &mut **settings;
            Inspector::handle_context_menu_message(
                self.inspector,
                message,
                ui,
                settings_data,
                &mut self.clipboard,
            );
        }

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.ok {
                ui.send(self.window, WindowMessage::Close);
            } else if message.destination() == self.default {
                **settings = Default::default();

                self.sync_to_model(ui, settings, property_editors);
            }

            if let Ok(node) = ui.try_get_node(message.destination()) {
                if let Some(user_data) = node.user_data_cloned::<GroupName>() {
                    let inspector = ui.try_get_of_type::<Inspector>(self.inspector).unwrap();

                    if let Some(entry) = inspector.context.find_property_editor_by_tag(&user_data.0)
                    {
                        ui.send(
                            self.scroll_viewer,
                            ScrollViewerMessage::BringIntoView(entry.property_container),
                        );
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
                ui.send(self.window, WidgetMessage::Remove);
                ui.send(
                    docking_manager,
                    DockingManagerMessage::RemoveFloatingWindow(self.window),
                );
                return None;
            }
        } else if let Some(SearchBarMessage::Text(search_text)) = message.data_from(self.search_bar)
        {
            let filter = search_text.to_lowercase();
            self.apply_filter(&filter, ui);
        }

        if let GraphicsContext::Initialized(ref mut graphics_context) = engine.graphics_context {
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
    pub const SETTINGS: Uuid = uuid!("7c7799e9-d15e-44be-a70a-8e280d55ff18");

    fn on_open_settings_clicked(&mut self, editor: &mut Editor) {
        let window = self
            .window
            .get_or_insert_with(|| SettingsWindow::new(&mut editor.engine));
        let ui = editor.engine.user_interfaces.first_mut();
        window.open(ui, &editor.settings, editor.property_editors.clone());
        ui.send(
            editor.docking_manager,
            DockingManagerMessage::AddFloatingWindow(window.window),
        );
    }
}

impl EditorPlugin for SettingsPlugin {
    fn on_start(&mut self, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();
        let ctx = &mut ui.build_ctx();
        self.open_settings = create_menu_item_shortcut(
            "Editor Settings...",
            load_image!("../../resources/settings.png"),
            Self::SETTINGS,
            "",
            vec![],
            ctx,
        );
        ui.send(
            editor.menu.file_menu.menu,
            MenuItemMessage::AddItem(self.open_settings),
        );
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
            editor.docking_manager,
            editor.property_editors.clone(),
        );
    }
}
