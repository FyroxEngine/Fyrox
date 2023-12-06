use crate::world::WorldViewerItemContextMenu;
use crate::{
    make_save_file_selector,
    menu::{create::CreateEntityMenu, create_menu_item, create_menu_item_shortcut},
    message::MessageSender,
    scene::{
        commands::{
            graph::{AddNodeCommand, ReplaceNodeCommand, SetGraphRootCommand},
            make_delete_selection_command, CommandGroup, RevertSceneNodePropertyCommand,
            SceneCommand,
        },
        GameScene, Selection,
    },
    settings::Settings,
    utils, Engine, Message, MessageDirection, PasteCommand,
};
use fyrox::asset::untyped::UntypedResource;
use fyrox::{
    core::{algebra::Vector2, pool::Handle, reflect::Reflect, scope_profile},
    gui::{
        file_browser::FileSelectorMessage,
        menu::{MenuItemBuilder, MenuItemContent, MenuItemMessage},
        message::UiMessage,
        popup::{Placement, PopupBuilder, PopupMessage},
        stack_panel::StackPanelBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        window::WindowMessage,
        BuildContext, RcUiNodeHandle, UiNode,
    },
};
use std::any::TypeId;
use std::path::PathBuf;

pub struct SceneNodeContextMenu {
    menu: RcUiNodeHandle,
    delete_selection: Handle<UiNode>,
    copy_selection: Handle<UiNode>,
    create_entity_menu: CreateEntityMenu,
    replace_with_menu: CreateEntityMenu,
    placement_target: Handle<UiNode>,
    save_as_prefab: Handle<UiNode>,
    save_as_prefab_dialog: Handle<UiNode>,
    paste: Handle<UiNode>,
    make_root: Handle<UiNode>,
    open_asset: Handle<UiNode>,
    reset_inheritable_properties: Handle<UiNode>,
}

impl WorldViewerItemContextMenu for SceneNodeContextMenu {
    fn menu(&self) -> RcUiNodeHandle {
        self.menu.clone()
    }
}

fn resource_path_of_first_selected_node(
    editor_selection: &Selection,
    game_scene: &GameScene,
    engine: &Engine,
) -> Option<PathBuf> {
    if let Selection::Graph(graph_selection) = editor_selection {
        if let Some(first) = graph_selection.nodes.first() {
            let scene = &engine.scenes[game_scene.scene];
            if let Some(resource) = scene.graph.try_get(*first).and_then(|n| n.resource()) {
                return resource.kind().into_path();
            }
        }
    }
    None
}

impl SceneNodeContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let delete_selection;
        let copy_selection;
        let save_as_prefab;
        let paste;
        let make_root;
        let open_asset;
        let reset_inheritable_properties;

        let (create_entity_menu, create_entity_menu_root_items) = CreateEntityMenu::new(ctx);
        let (replace_with_menu, replace_with_menu_root_items) = CreateEntityMenu::new(ctx);

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
                            make_root = create_menu_item("Make Root", vec![], ctx);
                            make_root
                        })
                        .with_child(
                            MenuItemBuilder::new(
                                WidgetBuilder::new().with_min_size(Vector2::new(120.0, 22.0)),
                            )
                            .with_content(MenuItemContent::text("Replace With"))
                            .with_items(replace_with_menu_root_items)
                            .build(ctx),
                        )
                        .with_child({
                            open_asset = create_menu_item("Open Asset", vec![], ctx);
                            open_asset
                        })
                        .with_child({
                            reset_inheritable_properties =
                                create_menu_item("Reset Inheritable Properties", vec![], ctx);
                            reset_inheritable_properties
                        }),
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
            save_as_prefab,
            save_as_prefab_dialog,
            replace_with_menu,
            paste,
            make_root,
            open_asset,
            reset_inheritable_properties,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_selection: &Selection,
        game_scene: &mut GameScene,
        engine: &Engine,
        sender: &MessageSender,
        settings: &Settings,
    ) {
        scope_profile!();

        if let Selection::Graph(graph_selection) = editor_selection {
            if let Some(first) = graph_selection.nodes().first() {
                if let Some(node) = self.create_entity_menu.handle_ui_message(message) {
                    sender.do_scene_command(AddNodeCommand::new(node, *first, true));
                } else if let Some(replacement) = self.replace_with_menu.handle_ui_message(message)
                {
                    sender.do_scene_command(ReplaceNodeCommand {
                        handle: *first,
                        node: replacement,
                    });
                }
            }
        }

        if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if message.destination() == self.delete_selection {
                if settings.general.show_node_removal_dialog
                    && game_scene.is_current_selection_has_external_refs(
                        editor_selection,
                        &engine.scenes[game_scene.scene].graph,
                    )
                {
                    sender.send(Message::OpenNodeRemovalDialog);
                } else {
                    sender.send(Message::DoSceneCommand(make_delete_selection_command(
                        editor_selection,
                        game_scene,
                        engine,
                    )));
                }
            } else if message.destination() == self.copy_selection {
                if let Selection::Graph(graph_selection) = editor_selection {
                    game_scene.clipboard.fill_from_selection(
                        graph_selection,
                        game_scene.scene,
                        engine,
                    );
                }
            } else if message.destination() == self.paste {
                if let Selection::Graph(graph_selection) = editor_selection {
                    if let Some(first) = graph_selection.nodes.first() {
                        if !game_scene.clipboard.is_empty() {
                            sender.do_scene_command(PasteCommand::new(*first));
                        }
                    }
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
            } else if message.destination() == self.make_root {
                if let Selection::Graph(graph_selection) = editor_selection {
                    if let Some(first) = graph_selection.nodes.first() {
                        sender.do_scene_command(SetGraphRootCommand {
                            root: *first,
                            revert_list: Default::default(),
                        });
                    }
                }
            } else if message.destination() == self.open_asset {
                if let Some(path) =
                    resource_path_of_first_selected_node(editor_selection, game_scene, engine)
                {
                    if utils::is_native_scene(&path) {
                        sender.send(Message::LoadScene(path));
                    }
                }
            } else if message.destination() == self.reset_inheritable_properties {
                if let Selection::Graph(graph_selection) = editor_selection {
                    let scene = &engine.scenes[game_scene.scene];
                    let mut commands = Vec::new();
                    for node_handle in graph_selection.nodes.iter() {
                        if let Some(node) = scene.graph.try_get(*node_handle) {
                            (node as &dyn Reflect).enumerate_fields_recursively(
                                &mut |path, _, val| {
                                    val.as_inheritable_variable(&mut |inheritable| {
                                        if inheritable.is_some() {
                                            commands.push(SceneCommand::new(
                                                RevertSceneNodePropertyCommand::new(
                                                    path.to_string(),
                                                    *node_handle,
                                                ),
                                            ));
                                        }
                                    });
                                },
                                &[TypeId::of::<UntypedResource>()],
                            )
                        }
                    }
                    sender.do_scene_command(CommandGroup::from(commands));
                }
            }
        } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == *self.menu {
                self.placement_target = *target;

                // Check if there's something to paste and deactivate "Paste" if nothing.
                engine.user_interface.send_message(WidgetMessage::enabled(
                    self.paste,
                    MessageDirection::ToWidget,
                    !game_scene.clipboard.is_empty(),
                ));

                engine.user_interface.send_message(WidgetMessage::enabled(
                    self.open_asset,
                    MessageDirection::ToWidget,
                    resource_path_of_first_selected_node(editor_selection, game_scene, engine)
                        .map_or(false, |p| utils::is_native_scene(&p)),
                ));
            }
        } else if let Some(FileSelectorMessage::Commit(path)) = message.data() {
            if message.destination() == self.save_as_prefab_dialog {
                sender.send(Message::SaveSelectionAsPrefab(path.clone()));
            }
        }
    }
}
