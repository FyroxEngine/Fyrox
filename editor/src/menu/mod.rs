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
    export::ExportWindow,
    fyrox::{
        core::{algebra::Vector2, pool::Handle},
        gui::{
            menu::{MenuBuilder, MenuItemBuilder, MenuItemContent},
            message::{MessageDirection, UiMessage},
            widget::{WidgetBuilder, WidgetMessage},
            BuildContext, Thickness, UiNode, UserInterface,
        },
    },
    menu::{
        create::CreateEntityRootMenu, edit::EditMenu, file::FileMenu, help::HelpMenu,
        utils::UtilsMenu, view::ViewMenu,
    },
    message::MessageSender,
    scene::{container::EditorSceneEntry, controller::SceneController},
    send_sync_message,
    settings::Settings,
    stats::StatisticsWindow,
    Engine, Mode, SceneSettingsWindow,
};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

pub mod create;
pub mod edit;
pub mod file;
pub mod help;
pub mod ui;
pub mod utils;
pub mod view;

pub struct Menu {
    pub menu: Handle<UiNode>,
    pub create_entity_menu: CreateEntityRootMenu,
    pub edit_menu: EditMenu,
    pub file_menu: FileMenu,
    pub view_menu: ViewMenu,
    pub message_sender: MessageSender,
    pub utils_menu: UtilsMenu,
    pub help_menu: HelpMenu,
}

pub struct Panels<'b> {
    pub scene_frame: Handle<UiNode>,
    pub light_panel: Handle<UiNode>,
    pub log_panel: Handle<UiNode>,
    pub navmesh_panel: Handle<UiNode>,
    pub audio_panel: Handle<UiNode>,
    pub command_stack_panel: Handle<UiNode>,
    pub inspector_window: Handle<UiNode>,
    pub world_outliner_window: Handle<UiNode>,
    pub asset_window: Handle<UiNode>,
    pub configurator_window: Handle<UiNode>,
    pub scene_settings: &'b SceneSettingsWindow,
    pub export_window: &'b mut Option<ExportWindow>,
    pub statistics_window: &'b mut Option<StatisticsWindow>,
}

pub struct MenuContext<'a, 'b> {
    pub engine: &'a mut Engine,
    pub game_scene: Option<&'b mut EditorSceneEntry>,
    pub panels: Panels<'b>,
    pub settings: &'b mut Settings,
    pub icon_request_sender: Sender<IconRequest>,
}

pub fn create_root_menu_item(
    text: &str,
    items: Vec<Handle<UiNode>>,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    MenuItemBuilder::new(WidgetBuilder::new().with_margin(Thickness::right(10.0)))
        .with_content(MenuItemContent::text_centered(text))
        .with_items(items)
        .build(ctx)
}

pub fn create_menu_item(
    text: &str,
    items: Vec<Handle<UiNode>>,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    MenuItemBuilder::new(WidgetBuilder::new().with_min_size(Vector2::new(120.0, 22.0)))
        .with_content(MenuItemContent::text(text))
        .with_items(items)
        .build(ctx)
}

pub fn create_menu_item_shortcut(
    text: &str,
    shortcut: &str,
    items: Vec<Handle<UiNode>>,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    MenuItemBuilder::new(WidgetBuilder::new().with_min_size(Vector2::new(120.0, 22.0)))
        .with_content(MenuItemContent::text_with_shortcut(text, shortcut))
        .with_items(items)
        .build(ctx)
}

impl Menu {
    pub fn new(engine: &mut Engine, message_sender: MessageSender, settings: &Settings) -> Self {
        let file_menu = FileMenu::new(engine, settings);
        let ctx = &mut engine.user_interfaces.first_mut().build_ctx();
        let create_entity_menu = CreateEntityRootMenu::new(
            &engine.serialization_context,
            &engine.widget_constructors,
            ctx,
        );
        let edit_menu = EditMenu::new(ctx);
        let view_menu = ViewMenu::new(ctx);
        let utils_menu = UtilsMenu::new(ctx);
        let help_menu = HelpMenu::new(ctx);

        let menu = MenuBuilder::new(WidgetBuilder::new().on_row(0))
            .with_items(vec![
                file_menu.menu,
                edit_menu.menu,
                create_entity_menu.menu,
                view_menu.menu,
                utils_menu.menu,
                help_menu.menu,
            ])
            .build(ctx);

        Self {
            menu,
            create_entity_menu,
            edit_menu,
            message_sender,
            file_menu,
            view_menu,
            utils_menu,
            help_menu,
        }
    }

    pub fn open_load_file_selector(&self, ui: &mut UserInterface) {
        self.file_menu.open_load_file_selector(ui)
    }

    pub fn open_save_file_selector(&mut self, ui: &mut UserInterface, default_file_name: PathBuf) {
        self.file_menu
            .open_save_file_selector(ui, default_file_name)
    }

    pub fn sync_to_model(&mut self, has_active_scene: bool, ui: &mut UserInterface) {
        for &widget in [
            self.file_menu.close_scene,
            self.file_menu.save,
            self.file_menu.save_as,
            self.create_entity_menu.menu,
            self.edit_menu.menu,
            self.file_menu.open_scene_settings,
        ]
        .iter()
        {
            send_sync_message(
                ui,
                WidgetMessage::enabled(widget, MessageDirection::ToWidget, has_active_scene),
            );
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, mut ctx: MenuContext) {
        if let Some(entry) = ctx.game_scene.as_mut() {
            self.edit_menu.handle_ui_message(
                message,
                &self.message_sender,
                &entry.selection,
                &mut *entry.controller,
                ctx.engine,
            );

            self.create_entity_menu.handle_ui_message(
                message,
                &self.message_sender,
                &mut *entry.controller,
                &entry.selection,
                ctx.engine,
            );
        }

        self.utils_menu.handle_ui_message(
            message,
            &mut ctx.panels,
            ctx.engine.user_interfaces.first_mut(),
        );
        self.file_menu.handle_ui_message(
            message,
            &self.message_sender,
            ctx.game_scene,
            ctx.engine,
            ctx.settings,
            &mut ctx.panels,
            ctx.icon_request_sender.clone(),
        );
        self.view_menu.handle_ui_message(
            message,
            ctx.engine.user_interfaces.first(),
            &ctx.panels,
            &self.message_sender,
        );
        self.help_menu.handle_ui_message(message);
    }

    pub fn on_scene_changed(&self, controller: &dyn SceneController, ui: &UserInterface) {
        self.create_entity_menu.on_scene_changed(controller, ui);
    }

    pub fn on_mode_changed(&mut self, ui: &UserInterface, mode: &Mode) {
        self.create_entity_menu.on_mode_changed(ui, mode);
        self.edit_menu.on_mode_changed(ui, mode);
        self.file_menu.on_mode_changed(ui, mode);
    }
}
