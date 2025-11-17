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

use crate::{
    fyrox::{
        core::pool::Handle,
        graph::BaseSceneGraph,
        gui::{
            menu::{self, MenuItemMessage},
            message::UiMessage,
            window::WindowMessage,
            BuildContext, UiNode, UserInterface,
        },
    },
    menu::{create_menu_item, create_root_menu_item, Panels},
    message::MessageSender,
    Message,
};
use fyrox::core::{uuid, Uuid};

pub struct ViewMenu {
    pub menu: Handle<UiNode>,
    pub inspector: Handle<UiNode>,
    pub world_viewer: Handle<UiNode>,
    pub asset_browser: Handle<UiNode>,
    pub light_panel: Handle<UiNode>,
    pub log_panel: Handle<UiNode>,
    pub nav_mesh: Handle<UiNode>,
    pub audio: Handle<UiNode>,
    pub command_stack: Handle<UiNode>,
    pub save_layout: Handle<UiNode>,
    pub load_layout: Handle<UiNode>,
    pub reset_layout: Handle<UiNode>,
}

fn switch_window_state(window: Handle<UiNode>, ui: &UserInterface, center: bool) {
    let current_state = ui.node(window).visibility();
    if current_state {
        ui.send(window, WindowMessage::Close);
    } else {
        ui.send(
            window,
            WindowMessage::Open {
                center,
                focus_content: true,
            },
        )
    }
}

impl ViewMenu {
    pub const VIEW: Uuid = uuid!("f6a9a297-6efc-4b62-83b6-3955c0c43a00");
    pub const INSPECTOR: Uuid = uuid!("58ad3e9f-1e4f-43a5-b203-ef9d92982c0e");
    pub const ASSET_BROWSER: Uuid = uuid!("a2b8931e-2979-435b-9d72-00bf60a2d1b9");
    pub const WORLD_VIEWER: Uuid = uuid!("7b2dce51-9de4-4f35-9d5b-c8606607480f");
    pub const LIGHT_PANEL: Uuid = uuid!("9363ce91-6034-4e99-a6fc-2b3edac27182");
    pub const LOG_PANEL: Uuid = uuid!("105847a5-bd6d-4d12-8aaa-ba19c53a9c38");
    pub const NAV_MESH: Uuid = uuid!("aa387bc2-fecc-474f-be40-ec1d6ec1ac25");
    pub const AUDIO: Uuid = uuid!("8b3eb8c5-eb9e-4d53-8fac-2b6e75b2e624");
    pub const COMMAND_STACK: Uuid = uuid!("a1542ff2-b5d6-4807-9fd7-f1aa970611ec");
    pub const SAVE_LAYOUT: Uuid = uuid!("ae126347-550a-4013-aaa5-423c609b0cfe");
    pub const LOAD_LAYOUT: Uuid = uuid!("6e3dbc5e-9012-4e8f-8026-77bd03a55a48");
    pub const RESET_LAYOUT: Uuid = uuid!("5416d790-65cb-481e-ab99-d0b9fe1f23c6");

    pub fn new(ctx: &mut BuildContext) -> Self {
        let inspector;
        let asset_browser;
        let world_viewer;
        let light_panel;
        let log_panel;
        let nav_mesh;
        let audio;
        let command_stack;
        let save_layout;
        let load_layout;
        let reset_layout;
        let menu = create_root_menu_item(
            "View",
            Self::VIEW,
            vec![
                {
                    inspector = create_menu_item("Inspector", Self::INSPECTOR, vec![], ctx);
                    inspector
                },
                {
                    asset_browser =
                        create_menu_item("Asset Browser", Self::ASSET_BROWSER, vec![], ctx);
                    asset_browser
                },
                {
                    world_viewer =
                        create_menu_item("World Viewer", Self::WORLD_VIEWER, vec![], ctx);
                    world_viewer
                },
                {
                    light_panel = create_menu_item("Light Panel", Self::LIGHT_PANEL, vec![], ctx);
                    light_panel
                },
                {
                    log_panel = create_menu_item("Log Panel", Self::LOG_PANEL, vec![], ctx);
                    log_panel
                },
                {
                    nav_mesh = create_menu_item("Navmesh Panel", Self::NAV_MESH, vec![], ctx);
                    nav_mesh
                },
                {
                    audio = create_menu_item("Audio Panel", Self::AUDIO, vec![], ctx);
                    audio
                },
                {
                    command_stack =
                        create_menu_item("Command Stack Panel", Self::COMMAND_STACK, vec![], ctx);
                    command_stack
                },
                menu::make_menu_splitter(ctx),
                {
                    save_layout = create_menu_item("Save Layout", Self::SAVE_LAYOUT, vec![], ctx);
                    save_layout
                },
                {
                    load_layout = create_menu_item("Load Layout", Self::LOAD_LAYOUT, vec![], ctx);
                    load_layout
                },
                {
                    reset_layout =
                        create_menu_item("Reset Layout", Self::RESET_LAYOUT, vec![], ctx);
                    reset_layout
                },
            ],
            ctx,
        );

        Self {
            menu,
            inspector,
            world_viewer,
            asset_browser,
            light_panel,
            log_panel,
            nav_mesh,
            audio,
            command_stack,
            save_layout,
            load_layout,
            reset_layout,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        ui: &UserInterface,
        panels: &Panels,
        sender: &MessageSender,
    ) {
        if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if message.destination() == self.asset_browser {
                switch_window_state(panels.asset_window, ui, false);
            } else if message.destination() == self.light_panel {
                switch_window_state(panels.light_panel, ui, true);
            } else if message.destination() == self.world_viewer {
                switch_window_state(panels.world_outliner_window, ui, false);
            } else if message.destination() == self.inspector {
                switch_window_state(panels.inspector_window, ui, false);
            } else if message.destination() == self.log_panel {
                switch_window_state(panels.log_panel, ui, false);
            } else if message.destination() == self.nav_mesh {
                switch_window_state(panels.navmesh_panel, ui, false);
            } else if message.destination() == self.audio {
                switch_window_state(panels.audio_panel, ui, false);
            } else if message.destination() == self.command_stack {
                switch_window_state(panels.command_stack_panel, ui, false);
            } else if message.destination() == self.save_layout {
                sender.send(Message::SaveLayout);
            } else if message.destination() == self.load_layout {
                sender.send(Message::LoadLayout);
            } else if message.destination() == self.reset_layout {
                sender.send(Message::ResetLayout);
            }
        }
    }
}
