use crate::{
    menu::create::CreateEntityMenu,
    scene::{commands::make_delete_selection_command, EditorScene, Selection},
    world::graph::item::SceneItem,
    GameEngine, Message, MessageDirection,
};
use fyrox::{
    core::{algebra::Vector2, pool::Handle, scope_profile},
    gui::{
        menu::{MenuItemBuilder, MenuItemContent, MenuItemMessage},
        message::UiMessage,
        popup::{Placement, PopupBuilder, PopupMessage},
        stack_panel::StackPanelBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        BuildContext, UiNode,
    },
    scene::node::Node,
};
use std::sync::mpsc::Sender;

pub struct ItemContextMenu {
    pub menu: Handle<UiNode>,
    delete_selection: Handle<UiNode>,
    copy_selection: Handle<UiNode>,
    create_entity_menu: CreateEntityMenu,
    placement_target: Handle<UiNode>,
    // TODO: Ideally this should belong to node-specific context menu only.
    preview_camera: Handle<UiNode>,
}

impl ItemContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let delete_selection;
        let copy_selection;

        let (create_entity_menu, create_entity_menu_root_items) = CreateEntityMenu::new(ctx);

        let preview_camera;
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
                        )
                        .with_child({
                            preview_camera = MenuItemBuilder::new(
                                WidgetBuilder::new()
                                    .with_enabled(false)
                                    .with_min_size(Vector2::new(120.0, 20.0)),
                            )
                            .with_content(MenuItemContent::text_no_arrow("Preview"))
                            .build(ctx);
                            preview_camera
                        }),
                )
                .build(ctx),
            )
            .build(ctx);

        Self {
            create_entity_menu,
            menu,
            delete_selection,
            copy_selection,
            placement_target: Default::default(),
            preview_camera,
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
            } else if message.destination() == self.preview_camera {
                let new_preview_camera = engine
                    .user_interface
                    .try_get_node(self.placement_target)
                    .and_then(|n| n.query_component::<SceneItem<Node>>())
                    .unwrap()
                    .entity_handle;
                if editor_scene.preview_camera == new_preview_camera {
                    editor_scene.preview_camera = Handle::NONE;
                } else {
                    editor_scene.preview_camera = new_preview_camera
                }
            }
        } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.menu {
                self.placement_target = *target;

                // Check if placement target is a Camera.
                let mut is_camera = false;
                if let Some(placement_target) = engine
                    .user_interface
                    .try_get_node(self.placement_target)
                    .and_then(|n| n.query_component::<SceneItem<Node>>())
                {
                    if let Some(node) = engine.scenes[editor_scene.scene]
                        .graph
                        .try_get(placement_target.entity_handle)
                    {
                        is_camera = node.is_camera();
                    }
                }

                engine.user_interface.send_message(WidgetMessage::enabled(
                    self.preview_camera,
                    MessageDirection::ToWidget,
                    is_camera,
                ));
            }
        }
    }
}
