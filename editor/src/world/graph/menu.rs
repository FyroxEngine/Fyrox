use crate::menu::create::CreateEntityMenu;
use crate::{
    scene::{commands::make_delete_selection_command, EditorScene, Selection},
    GameEngine, Message,
};
use fyrox::{
    core::{algebra::Vector2, pool::Handle, scope_profile},
    gui::{
        menu::{MenuItemBuilder, MenuItemContent, MenuItemMessage},
        message::UiMessage,
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
    create_entity_menu: CreateEntityMenu,
}

impl ItemContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let delete_selection;
        let copy_selection;

        let (create_entity_menu, create_entity_menu_root_items) = CreateEntityMenu::new(ctx);

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
                                arrow: true,
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
                                arrow: true,
                            })
                            .build(ctx);
                            copy_selection
                        })
                        .with_child(
                            MenuItemBuilder::new(
                                WidgetBuilder::new().with_min_size(Vector2::new(120.0, 20.0)),
                            )
                            .with_content(MenuItemContent::text("Create Child"))
                            .with_items(create_entity_menu_root_items)
                            .build(ctx),
                        ),
                )
                .build(ctx),
            )
            .build(ctx);

        Self {
            create_entity_menu,
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

        if let Selection::Graph(graph_selection) = &editor_scene.selection {
            if let Some(first) = graph_selection.nodes().first() {
                self.create_entity_menu
                    .handle_ui_message(message, sender, *first);
            }
        }

        if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
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
                        engine,
                    );
                }
            }
        }
    }
}
