use crate::{
    scene::{commands::make_delete_selection_command, EditorScene, Selection},
    GameEngine, Message,
};
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
}

impl ItemContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let delete_selection;
        let copy_selection;

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
                        }),
                )
                .build(ctx),
            )
            .build(ctx);

        Self {
            menu,
            delete_selection,
            copy_selection,
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

        if let UiMessageData::MenuItem(MenuItemMessage::Click) = message.data() {
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
            }
        }
    }
}
