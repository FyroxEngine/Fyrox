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
    fyrox::graph::{SceneGraph, SceneGraphNode},
    fyrox::{
        core::pool::Handle,
        gui::{
            menu::MenuItemMessage,
            message::UiMessage,
            popup::{Placement, PopupBuilder, PopupMessage},
            stack_panel::StackPanelBuilder,
            widget::{WidgetBuilder, WidgetMessage},
            BuildContext, RcUiNodeHandle, UiNode,
        },
    },
    menu::{create_menu_item, create_menu_item_shortcut, ui::UiMenu},
    message::MessageSender,
    scene::{controller::SceneController, Selection},
    ui_scene::{
        commands::graph::{PasteWidgetCommand, SetUiRootCommand},
        UiScene,
    },
    utils,
    world::WorldViewerItemContextMenu,
    Engine, Message,
};
use fyrox::asset::manager::ResourceManager;
use fyrox::core::{uuid, Uuid};
use fyrox::gui::constructor::WidgetConstructorContainer;
use fyrox::gui::menu::{ContextMenuBuilder, MenuItem};
use std::path::PathBuf;

pub struct WidgetContextMenu {
    menu: RcUiNodeHandle,
    delete_selection: Handle<MenuItem>,
    copy_selection: Handle<MenuItem>,
    widgets_menu: UiMenu,
    placement_target: Handle<UiNode>,
    paste: Handle<MenuItem>,
    make_root: Handle<MenuItem>,
    open_asset: Handle<MenuItem>,
}

impl WorldViewerItemContextMenu for WidgetContextMenu {
    fn menu(&self) -> RcUiNodeHandle {
        self.menu.clone()
    }
}

fn resource_path_of_first_selected_node(
    editor_selection: &Selection,
    ui_scene: &UiScene,
    resource_manager: &ResourceManager,
) -> Option<PathBuf> {
    if let Some(ui_selection) = editor_selection.as_ui() {
        if let Some(first) = ui_selection.widgets.first() {
            if let Some(resource) = ui_scene
                .ui
                .try_get_node(*first)
                .ok()
                .and_then(|n| n.resource())
            {
                return resource_manager.resource_path(resource.as_ref());
            }
        }
    }
    None
}

impl WidgetContextMenu {
    pub const DELETE_SELECTION: Uuid = uuid!("30eef2a7-9f12-4e64-9142-b604f25e9e06");
    pub const COPY_SELECTION: Uuid = uuid!("0a2d10bc-de1e-4196-aba6-0a51c54eb238");
    pub const PASTE_AS_CHILD: Uuid = uuid!("d3b86c8c-1efd-4917-9543-b0f5f32f8cbd");
    pub const MAKE_ROOT: Uuid = uuid!("968318f6-21c7-430f-a13d-36aefb61cde2");
    pub const OPEN_ASSET: Uuid = uuid!("f3f7d0fa-e905-4371-8973-dfc1eb758e5a");

    pub fn new(
        widget_constructors_container: &WidgetConstructorContainer,
        ctx: &mut BuildContext,
    ) -> Self {
        let delete_selection;
        let copy_selection;
        let paste;
        let make_root;
        let open_asset;

        let widgets_menu = UiMenu::new(widget_constructors_container, "Create Child Widget", ctx);

        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
                .with_content(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_child({
                                delete_selection = create_menu_item_shortcut(
                                    "Delete Selection",
                                    None,
                                    Self::DELETE_SELECTION,
                                    "Del",
                                    vec![],
                                    ctx,
                                );
                                delete_selection
                            })
                            .with_child({
                                copy_selection = create_menu_item_shortcut(
                                    "Copy Selection",
                                    None,
                                    Self::COPY_SELECTION,
                                    "Ctrl+C",
                                    vec![],
                                    ctx,
                                );
                                copy_selection
                            })
                            .with_child({
                                paste = create_menu_item(
                                    "Paste As Child",
                                    Self::PASTE_AS_CHILD,
                                    vec![],
                                    ctx,
                                );
                                paste
                            })
                            .with_child({
                                make_root =
                                    create_menu_item("Make Root", Self::MAKE_ROOT, vec![], ctx);
                                make_root
                            })
                            .with_child({
                                open_asset =
                                    create_menu_item("Open Asset", Self::OPEN_ASSET, vec![], ctx);
                                open_asset
                            })
                            .with_child(widgets_menu.menu),
                    )
                    .build(ctx),
                )
                .with_restrict_picking(false),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            widgets_menu,
            menu,
            delete_selection,
            copy_selection,
            placement_target: Default::default(),
            paste,
            make_root,
            open_asset,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        sender: &MessageSender,
    ) {
        if let Some(ui_scene) = controller.downcast_mut::<UiScene>() {
            self.widgets_menu
                .handle_ui_message(sender, message, ui_scene, editor_selection);

            if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
                if message.destination() == self.delete_selection {
                    if let Some(ui_selection) = editor_selection.as_ui() {
                        sender.send(Message::DoCommand(
                            ui_selection.make_deletion_command(&ui_scene.ui),
                        ));
                    }
                } else if message.destination() == self.copy_selection {
                    if let Some(ui_selection) = editor_selection.as_ui() {
                        ui_scene
                            .clipboard
                            .fill_from_selection(ui_selection, &ui_scene.ui);
                    }
                } else if message.destination() == self.paste {
                    if let Some(ui_selection) = editor_selection.as_ui() {
                        if let Some(first) = ui_selection.widgets.first() {
                            if !ui_scene.clipboard.is_empty() {
                                sender.do_command(PasteWidgetCommand::new(*first));
                            }
                        }
                    }
                } else if message.destination() == self.make_root {
                    if let Some(selection) = editor_selection.as_ui() {
                        if let Some(first) = selection.widgets.first() {
                            sender.do_command(SetUiRootCommand {
                                root: *first,
                                link_scheme: Default::default(),
                            });
                        }
                    }
                } else if message.destination() == self.open_asset {
                    if let Some(path) = resource_path_of_first_selected_node(
                        editor_selection,
                        ui_scene,
                        &engine.resource_manager,
                    ) {
                        if utils::is_native_scene(&path) {
                            sender.send(Message::LoadScene(path));
                        }
                    }
                }
            } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data()
            {
                if message.destination() == self.menu.handle() {
                    self.placement_target = *target;

                    // Check if there's something to paste and deactivate "Paste" if nothing.
                    engine.user_interfaces.first_mut().send(
                        self.paste,
                        WidgetMessage::Enabled(!ui_scene.clipboard.is_empty()),
                    );
                }
            }
        }
    }
}
