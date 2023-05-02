use crate::{
    make_save_file_selector,
    menu::{create::CreateEntityMenu, create_menu_item, create_menu_item_shortcut},
    scene::{
        commands::{
            graph::{AddNodeCommand, ReplaceNodeCommand},
            make_delete_selection_command,
        },
        EditorScene, Selection,
    },
    world::graph::item::SceneItem,
    Engine, Message, MessageDirection, PasteCommand,
};
use fyrox::gui::RcUiNodeHandle;
use fyrox::{
    core::{algebra::Vector2, pool::Handle, scope_profile},
    gui::{
        file_browser::FileSelectorMessage,
        menu::{MenuItemBuilder, MenuItemContent, MenuItemMessage},
        message::UiMessage,
        popup::{Placement, PopupBuilder, PopupMessage},
        stack_panel::StackPanelBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        window::WindowMessage,
        BuildContext, UiNode,
    },
    scene::node::Node,
};
use std::sync::mpsc::Sender;

pub struct ItemContextMenu {
    pub menu: RcUiNodeHandle,
    delete_selection: Handle<UiNode>,
    copy_selection: Handle<UiNode>,
    create_entity_menu: CreateEntityMenu,
    replace_with_menu: CreateEntityMenu,
    placement_target: Handle<UiNode>,
    // TODO: Ideally this should belong to node-specific context menu only.
    preview_camera: Handle<UiNode>,
    save_as_prefab: Handle<UiNode>,
    save_as_prefab_dialog: Handle<UiNode>,
    paste: Handle<UiNode>,
}

impl ItemContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let delete_selection;
        let copy_selection;
        let save_as_prefab;
        let paste;

        let (create_entity_menu, create_entity_menu_root_items) = CreateEntityMenu::new(ctx);
        let (replace_with_menu, replace_with_menu_root_items) = CreateEntityMenu::new(ctx);

        let preview_camera;
        let menu = PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
            .with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            delete_selection =
                                create_menu_item_shortcut("Delete Selection", "Del", vec![], ctx);
                            delete_selection
                        })
                        .with_child({
                            copy_selection =
                                create_menu_item_shortcut("Copy Selection", "Ctrl+C", vec![], ctx);
                            copy_selection
                        })
                        .with_child({
                            paste = create_menu_item("Paste As Child", vec![], ctx);
                            paste
                        })
                        .with_child({
                            save_as_prefab = create_menu_item("Save As Prefab...", vec![], ctx);
                            save_as_prefab
                        })
                        .with_child(
                            MenuItemBuilder::new(
                                WidgetBuilder::new().with_min_size(Vector2::new(120.0, 22.0)),
                            )
                            .with_content(MenuItemContent::text("Create Child"))
                            .with_items(create_entity_menu_root_items)
                            .build(ctx),
                        )
                        .with_child({
                            preview_camera = MenuItemBuilder::new(
                                WidgetBuilder::new()
                                    .with_enabled(false)
                                    .with_min_size(Vector2::new(120.0, 22.0)),
                            )
                            .with_content(MenuItemContent::text_no_arrow("Preview"))
                            .build(ctx);
                            preview_camera
                        })
                        .with_child(
                            MenuItemBuilder::new(
                                WidgetBuilder::new().with_min_size(Vector2::new(120.0, 22.0)),
                            )
                            .with_content(MenuItemContent::text("Replace With"))
                            .with_items(replace_with_menu_root_items)
                            .build(ctx),
                        ),
                )
                .build(ctx),
            )
            .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        // TODO: Not sure if this is the right place for this dialog.
        let save_as_prefab_dialog = make_save_file_selector(ctx);

        Self {
            create_entity_menu,
            menu,
            delete_selection,
            copy_selection,
            placement_target: Default::default(),
            preview_camera,
            save_as_prefab,
            save_as_prefab_dialog,
            replace_with_menu,
            paste,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &mut EditorScene,
        engine: &Engine,
        sender: &Sender<Message>,
    ) {
        scope_profile!();

        if let Selection::Graph(graph_selection) = &editor_scene.selection {
            if let Some(first) = graph_selection.nodes().first() {
                if let Some(node) = self.create_entity_menu.handle_ui_message(message) {
                    sender
                        .send(Message::do_scene_command(AddNodeCommand::new(node, *first)))
                        .unwrap();
                } else if let Some(replacement) = self.replace_with_menu.handle_ui_message(message)
                {
                    sender
                        .send(Message::do_scene_command(ReplaceNodeCommand {
                            handle: *first,
                            node: replacement,
                        }))
                        .unwrap();
                }
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
            } else if message.destination() == self.paste {
                if let Selection::Graph(graph_selection) = &editor_scene.selection {
                    if let Some(first) = graph_selection.nodes.first() {
                        if !editor_scene.clipboard.is_empty() {
                            sender
                                .send(Message::do_scene_command(PasteCommand::new(*first)))
                                .unwrap();
                        }
                    }
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
            } else if message.destination() == self.save_as_prefab {
                engine
                    .user_interface
                    .send_message(WindowMessage::open_modal(
                        self.save_as_prefab_dialog,
                        MessageDirection::ToWidget,
                        true,
                    ));
                engine
                    .user_interface
                    .send_message(FileSelectorMessage::root(
                        self.save_as_prefab_dialog,
                        MessageDirection::ToWidget,
                        Some(std::env::current_dir().unwrap()),
                    ));
            }
        } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == *self.menu {
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

                // Check if there's something to paste and deactivate "Paste" if nothing.
                engine.user_interface.send_message(WidgetMessage::enabled(
                    self.paste,
                    MessageDirection::ToWidget,
                    !editor_scene.clipboard.is_empty(),
                ))
            }
        } else if let Some(FileSelectorMessage::Commit(path)) = message.data() {
            if message.destination() == self.save_as_prefab_dialog {
                sender
                    .send(Message::SaveSelectionAsPrefab(path.clone()))
                    .unwrap();
            }
        }
    }
}
