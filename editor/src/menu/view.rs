use crate::menu::{create_menu_item, create_root_menu_item, Panels};
use fyrox::{
    core::pool::Handle,
    gui::{
        menu::MenuItemMessage,
        message::{MessageDirection, UiMessage},
        window::WindowMessage,
        BuildContext, UiNode, UserInterface,
    },
};

pub struct ViewMenu {
    pub menu: Handle<UiNode>,
    sidebar: Handle<UiNode>,
    world_viewer: Handle<UiNode>,
    asset_browser: Handle<UiNode>,
    light_panel: Handle<UiNode>,
    log_panel: Handle<UiNode>,
    nav_mesh: Handle<UiNode>,
    audio: Handle<UiNode>,
    command_stack: Handle<UiNode>,
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
        let world_viewer;
        let light_panel;
        let log_panel;
        let nav_mesh;
        let audio;
        let command_stack;
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
                    world_viewer = create_menu_item("World Viewer", vec![], ctx);
                    world_viewer
                },
                {
                    light_panel = create_menu_item("Light Panel", vec![], ctx);
                    light_panel
                },
                {
                    log_panel = create_menu_item("Log Panel", vec![], ctx);
                    log_panel
                },
                {
                    nav_mesh = create_menu_item("Navmesh Panel", vec![], ctx);
                    nav_mesh
                },
                {
                    audio = create_menu_item("Audio Panel", vec![], ctx);
                    audio
                },
                {
                    command_stack = create_menu_item("Command Stack Panel", vec![], ctx);
                    command_stack
                },
            ],
            ctx,
        );

        Self {
            menu,
            sidebar,
            world_viewer,
            asset_browser,
            light_panel,
            log_panel,
            nav_mesh,
            audio,
            command_stack,
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &UserInterface, panels: &Panels) {
        if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if message.destination() == self.asset_browser {
                switch_window_state(panels.asset_window, ui, false);
            } else if message.destination() == self.light_panel {
                switch_window_state(panels.light_panel, ui, true);
            } else if message.destination() == self.world_viewer {
                switch_window_state(panels.world_outliner_window, ui, false);
            } else if message.destination() == self.sidebar {
                switch_window_state(panels.inspector_window, ui, false);
            } else if message.destination() == self.log_panel {
                switch_window_state(panels.log_panel, ui, false);
            } else if message.destination() == self.nav_mesh {
                switch_window_state(panels.navmesh_panel, ui, false);
            } else if message.destination() == self.audio {
                switch_window_state(panels.audio_panel, ui, false);
            } else if message.destination() == self.command_stack {
                switch_window_state(panels.command_stack_panel, ui, false);
            }
        }
    }
}
