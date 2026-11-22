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
    Message, Mode,
    fyrox::{
        core::{pool::Handle, reflect::prelude::*},
        engine::Engine,
        graph::SceneGraph,
        gui::{
            BuildContext, HorizontalAlignment, Orientation, Thickness, UserInterface,
            VerticalAlignment,
            button::{Button, ButtonBuilder, ButtonMessage},
            grid::{Column, GridBuilder},
            image::Image,
            message::UiMessage,
            stack_panel::StackPanelBuilder,
            widget::{WidgetBuilder, WidgetMessage},
            window::{Window, WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
        },
        scene::navmesh::NavigationalMesh,
    },
    game_scene::{GameScene, Selection},
    message::MessageSender,
    plugins::navmesh::{
        self, command::ConnectNavmeshEdgesCommand, interaction::NavmeshInteractionMode,
        selection::NavmeshEntity,
    },
    utils::window_content,
};

pub struct NavmeshPanel {
    pub window: Handle<Window>,
    connect_edges: Handle<Button>,
    edit: Handle<Button>,
    sender: MessageSender,
    scene_frame: Handle<Image>,
}

impl NavmeshPanel {
    pub fn new(scene_frame: Handle<Image>, ctx: &mut BuildContext, sender: MessageSender) -> Self {
        let edit = ButtonBuilder::new(
            WidgetBuilder::new()
                .with_width(120.0)
                .with_height(26.0)
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_text("Edit")
        .build(ctx);

        let connect_edges = ButtonBuilder::new(
            WidgetBuilder::new()
                .with_width(120.0)
                .with_height(26.0)
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_text("Connect Edges")
        .build(ctx);

        let window = WindowBuilder::new(
            WidgetBuilder::new()
                .with_width(250.0)
                .with_height(60.0)
                .with_name("NavmeshPanel"),
        )
        .open(false)
        .with_title(WindowTitle::text("Navmesh"))
        .with_tab_label("Navmesh")
        .with_content(
            GridBuilder::new(
                WidgetBuilder::new().with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_child(edit)
                            .with_child(connect_edges),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                ),
            )
            .add_column(Column::stretch())
            .build(ctx),
        )
        .build(ctx);

        Self {
            window,
            sender,
            connect_edges,
            scene_frame,
            edit,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        message_sender: &MessageSender,
        editor_selection: &Selection,
    ) {
        if let Some(ButtonMessage::Click) = message.data_from(self.connect_edges) {
            if let Some(selection) = navmesh::fetch_selection(editor_selection) {
                let vertices = selection
                    .entities()
                    .iter()
                    .filter_map(|entity| {
                        if let NavmeshEntity::Edge(v) = *entity {
                            Some(v)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                self.sender.do_command(ConnectNavmeshEdgesCommand::new(
                    selection.navmesh_node(),
                    [vertices[0], vertices[1]],
                ));
            }
        } else if let Some(ButtonMessage::Click) = message.data_from(self.edit) {
            message_sender.send(Message::SetInteractionMode(
                NavmeshInteractionMode::type_info().type_uuid,
            ))
        }
    }

    pub fn sync_to_model(
        &mut self,
        engine: &Engine,
        editor_selection: &Selection,
        game_scene: &GameScene,
    ) {
        let mut navmesh_selected = false;

        let graph = &engine.scenes[game_scene.scene].graph;
        if let Some(selection) = navmesh::fetch_selection(editor_selection) {
            navmesh_selected = graph
                .try_get_of_type::<NavigationalMesh>(selection.navmesh_node())
                .is_ok();
        }

        if navmesh_selected {
            engine.user_interfaces.first().send(
                self.window,
                WindowMessage::Open {
                    alignment: WindowAlignment::Relative {
                        relative_to: self.scene_frame.to_base(),
                        horizontal_alignment: HorizontalAlignment::Right,
                        vertical_alignment: VerticalAlignment::Top,
                        margin: Thickness::uniform(1.0),
                    },
                    modal: false,
                    focus_content: false,
                },
            );
        } else {
            engine
                .user_interfaces
                .first()
                .send(self.window, WindowMessage::Close);
        }
    }

    pub fn on_mode_changed(&mut self, ui: &UserInterface, mode: &Mode) {
        ui.send(
            window_content(self.window, ui),
            WidgetMessage::Enabled(mode.is_edit()),
        );
    }

    pub fn destroy(self, ui: &UserInterface) {
        ui.send(self.window, WidgetMessage::Remove);
    }
}
