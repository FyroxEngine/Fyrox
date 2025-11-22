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
    command::{Command, CommandGroup},
    fyrox::{
        asset::untyped::UntypedResource,
        core::{algebra::Vector2, algebra::Vector3, pool::Handle, reflect::Reflect},
        engine::SerializationContext,
        graph::BaseSceneGraph,
        gui::{
            constructor::WidgetConstructorContainer,
            file_browser::FileSelectorMessage,
            menu::{
                self, ContextMenuBuilder, MenuItemBuilder, MenuItemContent, MenuItemMessage,
                SortingPredicate,
            },
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
    Engine, Message, PasteCommand,
};
use fyrox::core::{uuid, Uuid};
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
            if let Some(resource) = scene.graph.try_get_node(*first).and_then(|n| n.resource()) {
                return engine.resource_manager.resource_path(resource.as_ref());
            }
        }
    }
    None
}

impl SceneNodeContextMenu {
    pub const CREATE_CHILD: Uuid = uuid!("a5720895-4206-43aa-9dea-dcdccfcdd73a");
    pub const DELETE: Uuid = uuid!("fc272ecf-1c4f-44d0-bbf1-2e21c8d9ecd2");
    pub const COPY: Uuid = uuid!("50626c7b-b50f-4547-873c-a11b4125cdd5");
    pub const PASTE_AS_CHILD: Uuid = uuid!("bb57ff9b-ba80-4f66-b4ad-860b98379e11");
    pub const CREATE_PARENT: Uuid = uuid!("c58e6407-3819-4254-84c2-5a58eeb4afd9");
    pub const REPLACE_WITH: Uuid = uuid!("92163075-4d90-4b39-ba9d-af0ebaf62348");
    pub const SET_AS_ROOT: Uuid = uuid!("689adb98-7603-4576-a0ab-3f48c2d57d22");
    pub const OPEN_PARENT_PREFAB: Uuid = uuid!("99903c9e-d7ac-49bd-9ee3-3bbd8536f595");
    pub const SAVE_AS_PREFAB: Uuid = uuid!("82a70cff-b536-46fb-83c0-4a886585d871");
    pub const RESET_INHERITABLE: Uuid = uuid!("95c6437f-dc23-4ec1-9490-d35f0864f027");
    pub const SAVE_AS_PREFAB_FILE_SELECTOR: Uuid = uuid!("5d438037-a4be-4d70-a830-138185e1a049");

    pub fn new(
        serialization_context: &SerializationContext,
        widget_constructors_container: &WidgetConstructorContainer,
        ctx: &mut BuildContext,
    ) -> Self {
        let delete_selection;
        let copy_selection;
        let save_as_prefab;
        let paste;
        let make_root;
        let open_asset;
        let reset_inheritable_properties;
        let create_parent;
        let create_child;
        let replace_with;

        let create_child_entity_menu =
            CreateEntityMenu::new(serialization_context, widget_constructors_container, ctx);
        let create_parent_entity_menu =
            CreateEntityMenu::new(serialization_context, widget_constructors_container, ctx);
        let replace_with_menu =
            CreateEntityMenu::new(serialization_context, widget_constructors_container, ctx);

        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
                .with_content(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_child({
                                create_child = MenuItemBuilder::new(
                                    WidgetBuilder::new().with_min_size(Vector2::new(120.0, 22.0)),
                                )
                                .with_content(MenuItemContent::text("Create Child Node"))
                                .with_items(create_child_entity_menu.root_items.clone())
                                .build(ctx);
                                create_child
                            })
                            .with_child({
                                delete_selection = create_menu_item_shortcut(
                                    "Delete Node(s)",
                                    Self::DELETE,
                                    "Del",
                                    vec![],
                                    ctx,
                                );
                                delete_selection
                            })
                            .with_child(menu::make_menu_splitter(ctx))
                            .with_child({
                                copy_selection = create_menu_item_shortcut(
                                    "Copy Node(s)",
                                    Self::COPY,
                                    "Ctrl+C",
                                    vec![],
                                    ctx,
                                );
                                copy_selection
                            })
                            .with_child({
                                paste = create_menu_item(
                                    "Paste As Child Node",
                                    Self::PASTE_AS_CHILD,
                                    vec![],
                                    ctx,
                                );
                                paste
                            })
                            .with_child(menu::make_menu_splitter(ctx))
                            .with_child({
                                create_parent = MenuItemBuilder::new(
                                    WidgetBuilder::new()
                                        .with_id(Self::CREATE_PARENT)
                                        .with_min_size(Vector2::new(120.0, 22.0)),
                                )
                                .with_content(MenuItemContent::text("Create Parent Node"))
                                .with_items(create_parent_entity_menu.root_items.clone())
                                .build(ctx);
                                create_parent
                            })
                            .with_child({
                                replace_with = MenuItemBuilder::new(
                                    WidgetBuilder::new()
                                        .with_id(Self::REPLACE_WITH)
                                        .with_min_size(Vector2::new(120.0, 22.0)),
                                )
                                .with_content(MenuItemContent::text("Replace With Node"))
                                .with_items(replace_with_menu.root_items.clone())
                                .build(ctx);
                                replace_with
                            })
                            .with_child({
                                make_root = create_menu_item(
                                    "Set As Root Node",
                                    Self::SET_AS_ROOT,
                                    vec![],
                                    ctx,
                                );
                                make_root
                            })
                            .with_child(menu::make_menu_splitter(ctx))
                            .with_child({
                                open_asset = create_menu_item(
                                    "Open Parent Prefab",
                                    Self::OPEN_PARENT_PREFAB,
                                    vec![],
                                    ctx,
                                );
                                open_asset
                            })
                            .with_child({
                                save_as_prefab = create_menu_item(
                                    "Save Node(s) As Prefab...",
                                    Self::SAVE_AS_PREFAB,
                                    vec![],
                                    ctx,
                                );
                                save_as_prefab
                            })
                            .with_child({
                                reset_inheritable_properties = create_menu_item(
                                    "Reset Inheritable Properties",
                                    Self::RESET_INHERITABLE,
                                    vec![],
                                    ctx,
                                );
                                reset_inheritable_properties
                            }),
                    )
                    .build(ctx),
                )
                .with_restrict_picking(false),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        for item in [create_child, create_parent, replace_with] {
            ctx.inner().send(
                item,
                MenuItemMessage::Sort(SortingPredicate::sort_by_text()),
            )
        }

        Self {
            create_child_entity_menu,
            menu,
            delete_selection,
            copy_selection,
            placement_target: Default::default(),
            save_as_prefab,
            save_as_prefab_dialog: Default::default(),
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
        if let Some(node) = self.create_child_entity_menu.handle_ui_message(
            message,
            sender,
            controller,
            editor_selection,
            engine,
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
            engine,
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
        } else if let Some(replacement) = self.replace_with_menu.handle_ui_message(
            message,
            sender,
            controller,
            editor_selection,
            engine,
        ) {
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
                    let ui = engine.user_interfaces.first_mut();

                    self.save_as_prefab_dialog = make_save_file_selector(
                        &mut ui.build_ctx(),
                        PathBuf::from("unnamed.rgs"),
                        Self::SAVE_AS_PREFAB_FILE_SELECTOR,
                    );

                    ui.send(
                        self.save_as_prefab_dialog,
                        WindowMessage::OpenModal {
                            center: true,
                            focus_content: true,
                        },
                    );
                    ui.send(
                        self.save_as_prefab_dialog,
                        FileSelectorMessage::Root(Some(std::env::current_dir().unwrap())),
                    );
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
                            if let Some(node) = scene.graph.try_get_node(*node_handle) {
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
                    engine.user_interfaces.first_mut().send(
                        self.paste,
                        WidgetMessage::Enabled(!game_scene.clipboard.is_empty()),
                    );

                    engine.user_interfaces.first().send(
                        self.open_asset,
                        WidgetMessage::Enabled(
                            resource_path_of_first_selected_node(
                                editor_selection,
                                game_scene,
                                engine,
                            )
                            .is_some_and(|p| utils::is_native_scene(&p)),
                        ),
                    );
                }
            } else if let Some(FileSelectorMessage::Commit(path)) = message.data() {
                if message.destination() == self.save_as_prefab_dialog {
                    sender.send(Message::SaveSelectionAsPrefab(path.clone()));
                }
            }
        }
    }
}
