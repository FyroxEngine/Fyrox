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

use crate::fyrox::graph::BaseSceneGraph;
use crate::fyrox::{
    core::pool::Handle,
    gui::{
        menu::MenuItemMessage,
        message::{MessageDirection, UiMessage},
        window::WindowMessage,
        BuildContext, UiNode, UserInterface,
    },
};
use crate::{
    menu::{create_menu_item, create_root_menu_item, Panels},
    message::MessageSender,
    Message,
};

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
}

fn switch_window_state(window: Handle<UiNode>, ui: &UserInterface, center: bool) {
    let current_state = ui.node(window).visibility();
    ui.send_message(if current_state {
        WindowMessage::close(window, MessageDirection::ToWidget)
    } else {
        WindowMessage::open(window, MessageDirection::ToWidget, center, true)
    })
}

impl ViewMenu {
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
        let menu = create_root_menu_item(
            "View",
            vec![
                {
                    inspector = create_menu_item("Inspector", vec![], ctx);
                    inspector
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
                {
                    save_layout = create_menu_item("Save Layout", vec![], ctx);
                    save_layout
                },
                {
                    load_layout = create_menu_item("Load Layout", vec![], ctx);
                    load_layout
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
            }
        }
    }
}
