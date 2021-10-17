use crate::scene::commands::physics::SetBodyCommand;
use crate::{
    scene::{commands::make_delete_selection_command, EditorScene, Selection},
    GameEngine, Message,
};
use rg3d::gui::message::{MessageDirection, PopupMessage, WidgetMessage};
use rg3d::{
    core::{algebra::Vector2, pool::Handle, scope_profile},
    gui::{
        menu::{MenuItemBuilder, MenuItemContent},
        message::{MenuItemMessage, UiMessage, UiMessageData},
        popup::PopupBuilder,
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        BuildContext, UiNode,
    },
};
use std::sync::mpsc::Sender;

pub struct ItemContextMenu {
    pub menu: Handle<UiNode>,
    delete_selection: Handle<UiNode>,
    copy_selection: Handle<UiNode>,
    add_rigid_body: Handle<UiNode>,
}

impl ItemContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let delete_selection;
        let copy_selection;
        let add_rigid_body;

        let menu = PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
            .with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            delete_selection = MenuItemBuilder::new(
                                WidgetBuilder::new().with_min_size(Vector2::new(120.0, 20.0)),
                            )
                            .with_content(MenuItemContent::Text {
                                text: "Delete Selection",
                                shortcut: "Del",
                                icon: Default::default(),
                            })
                            .build(ctx);
                            delete_selection
                        })
                        .with_child({
                            copy_selection = MenuItemBuilder::new(
                                WidgetBuilder::new().with_min_size(Vector2::new(120.0, 20.0)),
                            )
                            .with_content(MenuItemContent::Text {
                                text: "Copy Selection",
                                shortcut: "Ctrl+C",
                                icon: Default::default(),
                            })
                            .build(ctx);
                            copy_selection
                        })
                        .with_child({
                            add_rigid_body = MenuItemBuilder::new(
                                WidgetBuilder::new().with_min_size(Vector2::new(120.0, 20.0)),
                            )
                            .with_content(MenuItemContent::text("Add Rigid Body"))
                            .build(ctx);
                            add_rigid_body
                        }),
                )
                .build(ctx),
            )
            .build(ctx);

        Self {
            menu,
            delete_selection,
            copy_selection,
            add_rigid_body,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &mut EditorScene,
        engine: &GameEngine,
        sender: &Sender<Message>,
    ) {
        scope_profile!();

        match message.data() {
            UiMessageData::MenuItem(MenuItemMessage::Click) => {
                if message.destination() == self.delete_selection {
                    sender
                        .send(Message::DoSceneCommand(make_delete_selection_command(
                            editor_scene,
                            engine,
                        )))
                        .unwrap();
                } else if message.destination() == self.copy_selection {
                    if let Selection::Graph(graph_selection) = &editor_scene.selection {
                        editor_scene.clipboard.fill_from_selection(
                            graph_selection,
                            editor_scene.scene,
                            &editor_scene.physics,
                            engine,
                        );
                    }
                } else if message.destination() == self.add_rigid_body
                    && editor_scene.selection.is_single_selection()
                {
                    if let Selection::Graph(graph_selection) = &editor_scene.selection {
                        sender
                            .send(Message::do_scene_command(SetBodyCommand::new(
                                *graph_selection.nodes.first().unwrap(),
                                Default::default(),
                            )))
                            .unwrap();
                    }
                }
            }
            UiMessageData::Popup(PopupMessage::Open) => {
                if message.destination() == self.menu {
                    let enabled = if let Selection::Graph(graph_selection) = &editor_scene.selection
                    {
                        graph_selection.is_single_selection()
                            && !editor_scene
                                .physics
                                .binder
                                .contains_key(graph_selection.nodes.first().unwrap())
                    } else {
                        false
                    };
                    engine.user_interface.send_message(WidgetMessage::enabled(
                        self.add_rigid_body,
                        MessageDirection::ToWidget,
                        enabled,
                    ));
                }
            }
            _ => {}
        }
    }
}
