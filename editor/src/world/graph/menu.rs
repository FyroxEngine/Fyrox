use crate::{
    command::{Command, CommandGroup},
    fyrox::{
        asset::untyped::UntypedResource,
        core::{algebra::Vector2, algebra::Vector3, pool::Handle, reflect::Reflect, scope_profile},
        graph::BaseSceneGraph,
        gui::{
            file_browser::FileSelectorMessage,
            menu::{ContextMenuBuilder, MenuItemBuilder, MenuItemContent, MenuItemMessage},
            message::UiMessage,
            popup::{Placement, PopupBuilder, PopupMessage},
            stack_panel::StackPanelBuilder,
            widget::{WidgetBuilder, WidgetMessage},
            window::WindowMessage,
            BuildContext, RcUiNodeHandle, UiNode,
        },
    },
    make_save_file_selector,
    menu::{create::CreateEntityMenu, create_menu_item, create_menu_item_shortcut},
    message::MessageSender,
    scene::{
        commands::{
            graph::{
                AddNodeCommand, LinkNodesCommand, MoveNodeCommand, ReplaceNodeCommand,
                SetGraphRootCommand, SetNodeTransformCommand,
            },
            make_delete_selection_command, RevertSceneNodePropertyCommand,
        },
        controller::SceneController,
        GameScene, Selection,
    },
    settings::Settings,
    utils,
    world::WorldViewerItemContextMenu,
    Engine, Message, MessageDirection, PasteCommand,
};
use std::{any::TypeId, path::PathBuf};

pub struct SceneNodeContextMenu {
    menu: RcUiNodeHandle,
    delete_selection: Handle<UiNode>,
    copy_selection: Handle<UiNode>,
    create_child_entity_menu: CreateEntityMenu,
    create_parent_entity_menu: CreateEntityMenu,
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
    if let Some(graph_selection) = editor_selection.as_graph() {
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

        let (create_child_entity_menu, create_child_entity_menu_root_items) =
            CreateEntityMenu::new(ctx);
        let (create_parent_entity_menu, create_parent_entity_menu_root_items) =
            CreateEntityMenu::new(ctx);
        let (replace_with_menu, replace_with_menu_root_items) = CreateEntityMenu::new(ctx);

        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_visibility(false)).with_content(
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
                            .with_content(MenuItemContent::text("Create Parent"))
                            .with_items(create_parent_entity_menu_root_items)
                            .build(ctx),
                        )
                        .with_child(
                            MenuItemBuilder::new(
                                WidgetBuilder::new().with_min_size(Vector2::new(120.0, 22.0)),
                            )
                            .with_content(MenuItemContent::text("Create Child"))
                            .with_items(create_child_entity_menu_root_items)
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
            ),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        // TODO: Not sure if this is the right place for this dialog.
        let save_as_prefab_dialog = make_save_file_selector(ctx, PathBuf::from("unnamed.rgs"));

        Self {
            create_child_entity_menu,
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
            create_parent_entity_menu,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        sender: &MessageSender,
        settings: &Settings,
    ) {
        scope_profile!();

        if let Some(node) = self.create_child_entity_menu.handle_ui_message(
            message,
            sender,
            controller,
            editor_selection,
        ) {
            if let Some(graph_selection) = editor_selection.as_graph() {
                if let Some(parent) = graph_selection.nodes().first() {
                    sender.do_command(AddNodeCommand::new(node, *parent, true));
                }
            }
        } else if let Some(node) = self.create_parent_entity_menu.handle_ui_message(
            message,
            sender,
            controller,
            editor_selection,
        ) {
            if let Some(graph_selection) = editor_selection.as_graph() {
                if let Some(first) = graph_selection.nodes().first() {
                    if let Some(game_scene) = controller.downcast_ref::<GameScene>() {
                        let scene = &engine.scenes[game_scene.scene];

                        let position = game_scene
                            .camera_controller
                            .placement_position(&scene.graph, *first);

                        let first_ref = &scene.graph[*first];
                        let parent = if first_ref.parent().is_some() {
                            first_ref.parent()
                        } else {
                            game_scene.scene_content_root
                        };

                        let new_parent_handle = scene.graph.generate_free_handles(1)[0];
                        let mut commands = CommandGroup::from(vec![
                            Command::new(AddNodeCommand::new(node, parent, true)),
                            Command::new(LinkNodesCommand::new(*first, new_parent_handle)),
                        ]);

                        if parent == game_scene.scene_content_root {
                            commands.push(MoveNodeCommand::new(
                                new_parent_handle,
                                Vector3::default(),
                                position,
                            ));
                        }

                        if *first == game_scene.scene_content_root {
                            commands.push(SetGraphRootCommand {
                                root: new_parent_handle,
                                link_scheme: Default::default(),
                            })
                        }
                        sender.do_command(commands);
                    }
                }
            }
        } else if let Some(replacement) =
            self.replace_with_menu
                .handle_ui_message(message, sender, controller, editor_selection)
        {
            if let Some(graph_selection) = editor_selection.as_graph() {
                if let Some(first) = graph_selection.nodes().first() {
                    sender.do_command(ReplaceNodeCommand {
                        handle: *first,
                        node: replacement,
                    });
                }
            }
        }

        if let Some(game_scene) = controller.downcast_mut::<GameScene>() {
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
                        sender.send(Message::DoCommand(make_delete_selection_command(
                            editor_selection,
                            game_scene,
                            engine,
                        )));
                    }
                } else if message.destination() == self.copy_selection {
                    if let Some(graph_selection) = editor_selection.as_graph() {
                        game_scene.clipboard.fill_from_selection(
                            graph_selection,
                            game_scene.scene,
                            engine,
                        );
                    }
                } else if message.destination() == self.paste {
                    if let Some(graph_selection) = editor_selection.as_graph() {
                        if let Some(first) = graph_selection.nodes.first() {
                            if !game_scene.clipboard.is_empty() {
                                sender.do_command(PasteCommand::new(*first));
                            }
                        }
                    }
                } else if message.destination() == self.save_as_prefab {
                    engine
                        .user_interfaces
                        .first_mut()
                        .send_message(WindowMessage::open_modal(
                            self.save_as_prefab_dialog,
                            MessageDirection::ToWidget,
                            true,
                            true,
                        ));
                    engine
                        .user_interfaces
                        .first_mut()
                        .send_message(FileSelectorMessage::root(
                            self.save_as_prefab_dialog,
                            MessageDirection::ToWidget,
                            Some(std::env::current_dir().unwrap()),
                        ));
                } else if message.destination() == self.make_root {
                    if let Some(graph_selection) = editor_selection.as_graph() {
                        if let Some(first) = graph_selection.nodes.first() {
                            let commands = CommandGroup::from(vec![
                                Command::new(SetNodeTransformCommand::new(
                                    *first,
                                    engine.scenes[game_scene.scene].graph[*first]
                                        .local_transform()
                                        .clone(),
                                    Default::default(),
                                )),
                                Command::new(SetGraphRootCommand {
                                    root: *first,
                                    link_scheme: Default::default(),
                                }),
                            ]);

                            sender.do_command(commands);
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
                    if let Some(graph_selection) = editor_selection.as_graph() {
                        let scene = &engine.scenes[game_scene.scene];
                        let mut commands = Vec::new();
                        for node_handle in graph_selection.nodes.iter() {
                            if let Some(node) = scene.graph.try_get(*node_handle) {
                                (node as &dyn Reflect).enumerate_fields_recursively(
                                    &mut |path, _, val| {
                                        val.as_inheritable_variable(&mut |inheritable| {
                                            if inheritable.is_some() {
                                                commands.push(Command::new(
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
                        sender.do_command(CommandGroup::from(commands));
                    }
                }
            } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data()
            {
                if message.destination() == self.menu.handle() {
                    self.placement_target = *target;

                    // Check if there's something to paste and deactivate "Paste" if nothing.
                    engine
                        .user_interfaces
                        .first_mut()
                        .send_message(WidgetMessage::enabled(
                            self.paste,
                            MessageDirection::ToWidget,
                            !game_scene.clipboard.is_empty(),
                        ));

                    engine
                        .user_interfaces
                        .first()
                        .send_message(WidgetMessage::enabled(
                            self.open_asset,
                            MessageDirection::ToWidget,
                            resource_path_of_first_selected_node(
                                editor_selection,
                                game_scene,
                                engine,
                            )
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
}
