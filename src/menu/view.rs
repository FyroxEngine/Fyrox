use crate::menu::{create_menu_item, create_root_menu_item, Panels};
use rg3d::{
    core::pool::Handle,
    gui::{
        message::{MenuItemMessage, MessageDirection, UiMessage, UiMessageData, WindowMessage},
        BuildContext, UiNode, UserInterface,
    },
};

pub struct ViewMenu {
    pub menu: Handle<UiNode>,
    sidebar: Handle<UiNode>,
    world_outliner: Handle<UiNode>,
    asset_browser: Handle<UiNode>,
    light_panel: Handle<UiNode>,
    log_panel: Handle<UiNode>,
}

fn switch_window_state(window: Handle<UiNode>, ui: &UserInterface, center: bool) {
    let current_state = ui.node(window).visibility();
    ui.send_message(if current_state {
        WindowMessage::close(window, MessageDirection::ToWidget)
    } else {
        WindowMessage::open(window, MessageDirection::ToWidget, center)
    })
}

impl ViewMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let sidebar;
        let asset_browser;
        let world_outliner;

        let light_panel;
        let log_panel;

        let menu = create_root_menu_item(
            "View",
            vec![
                {
                    sidebar = create_menu_item("Sidebar", vec![], ctx);
                    sidebar
                },
                {
                    asset_browser = create_menu_item("Asset Browser", vec![], ctx);
                    asset_browser
                },
                {
                    world_outliner = create_menu_item("World Outliner", vec![], ctx);
                    world_outliner
                },
                {
                    light_panel = create_menu_item("Light Panel", vec![], ctx);
                    light_panel
                },
                {
                    log_panel = create_menu_item("Log Panel", vec![], ctx);
                    log_panel
                },
            ],
            ctx,
        );

        Self {
            menu,
            sidebar,
            world_outliner,
            asset_browser,
            light_panel,
            log_panel,
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &UserInterface, panels: &Panels) {
        if let UiMessageData::MenuItem(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.asset_browser {
                switch_window_state(panels.asset_window, ui, false);
            } else if message.destination() == self.light_panel {
                switch_window_state(panels.light_panel, ui, true);
            } else if message.destination() == self.world_outliner {
                switch_window_state(panels.world_outliner_window, ui, false);
            } else if message.destination() == self.sidebar {
                switch_window_state(panels.sidebar_window, ui, false);
            } else if message.destination() == self.log_panel {
                switch_window_state(panels.log_panel, ui, false);
            }
        }
    }
}
