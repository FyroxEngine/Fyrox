use crate::{
    menu::{
        create::CreateEntityMenu, edit::EditMenu, file::FileMenu, utils::UtilsMenu, view::ViewMenu,
    },
    scene::EditorScene,
    send_sync_message,
    settings::Settings,
    GameEngine, Message,
};
use rg3d::{
    core::{algebra::Vector2, pool::Handle, scope_profile},
    gui::{
        menu::{MenuBuilder, MenuItemBuilder, MenuItemContent},
        message::{MessageDirection, UiMessage, WidgetMessage},
        widget::WidgetBuilder,
        BuildContext, Thickness, UiNode, UserInterface,
    },
};
use std::sync::mpsc::Sender;

pub mod create;
pub mod edit;
pub mod file;
pub mod physics;
pub mod utils;
pub mod view;

pub struct Menu {
    pub menu: Handle<UiNode>,
    create_entity_menu: CreateEntityMenu,
    edit_menu: EditMenu,
    pub file_menu: FileMenu,
    view_menu: ViewMenu,
    message_sender: Sender<Message>,
    utils_menu: UtilsMenu,
}

pub struct Panels {
    pub light_panel: Handle<UiNode>,
    pub log_panel: Handle<UiNode>,
    pub inspector_window: Handle<UiNode>,
    pub world_outliner_window: Handle<UiNode>,
    pub asset_window: Handle<UiNode>,
    pub configurator_window: Handle<UiNode>,
    pub path_fixer: Handle<UiNode>,
}

pub struct MenuContext<'a, 'b> {
    pub engine: &'a mut GameEngine,
    pub editor_scene: Option<&'b mut EditorScene>,
    pub panels: Panels,
    pub settings: &'b mut Settings,
}

pub fn create_root_menu_item(
    text: &str,
    items: Vec<Handle<UiNode>>,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    MenuItemBuilder::new(WidgetBuilder::new().with_margin(Thickness::right(10.0)))
        .with_content(MenuItemContent::text(text))
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
    pub fn new(
        engine: &mut GameEngine,
        message_sender: Sender<Message>,
        settings: &Settings,
    ) -> Self {
        let file_menu = FileMenu::new(engine, &message_sender, settings);
        let ctx = &mut engine.user_interface.build_ctx();
        let create_entity_menu = CreateEntityMenu::new(ctx);
        let edit_menu = EditMenu::new(ctx);
        let view_menu = ViewMenu::new(ctx);
        let utils_menu = UtilsMenu::new(ctx);

        let menu = MenuBuilder::new(WidgetBuilder::new().on_row(0))
            .with_items(vec![
                file_menu.menu,
                edit_menu.menu,
                create_entity_menu.menu,
                view_menu.menu,
                utils_menu.menu,
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
        }
    }

    pub fn open_load_file_selector(&self, ui: &mut UserInterface) {
        self.file_menu.open_load_file_selector(ui)
    }

    pub fn sync_to_model(&mut self, editor_scene: Option<&EditorScene>, ui: &mut UserInterface) {
        scope_profile!();

        for &widget in [
            self.file_menu.close_scene,
            self.file_menu.save,
            self.file_menu.save_as,
            self.create_entity_menu.menu,
            self.edit_menu.menu,
        ]
        .iter()
        {
            send_sync_message(
                ui,
                WidgetMessage::enabled(widget, MessageDirection::ToWidget, editor_scene.is_some()),
            );
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, mut ctx: MenuContext) {
        scope_profile!();

        if let Some(scene) = ctx.editor_scene.as_mut() {
            self.edit_menu.handle_ui_message(
                message,
                &self.message_sender,
                &mut **scene,
                ctx.engine,
            );
        }

        self.create_entity_menu
            .handle_ui_message(message, &self.message_sender);
        self.utils_menu
            .handle_ui_message(message, &ctx.panels, &ctx.engine.user_interface);
        self.file_menu.handle_ui_message(
            message,
            &self.message_sender,
            &ctx.editor_scene,
            ctx.engine,
            ctx.settings,
            ctx.panels.configurator_window,
        );
        self.view_menu
            .handle_ui_message(message, &ctx.engine.user_interface, &ctx.panels);
    }
}
