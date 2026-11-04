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
    Engine, Message,
    fyrox::{
        asset::manager::ResourceManager,
        core::{
            ok_or_return,
            pool::Handle,
            some_or_return,
            uuid::{Uuid, uuid},
        },
        graph::{NodeWrapper, SceneGraph},
        gui::{
            BuildContext, RcUiNodeHandle, UiNode,
            menu::{ContextMenuBuilder, MenuItem, MenuItemMessage, make_menu_splitter},
            message::UiMessage,
            popup::{Placement, PopupBuilder, PopupMessage},
            stack_panel::StackPanelBuilder,
            widget::{WidgetBuilder, WidgetMessage},
        },
    },
    menu::{create_menu_item, create_menu_item_shortcut},
    message::MessageSender,
    scene::{Selection, controller::SceneController},
    ui_scene::{
        UiScene,
        commands::graph::{PasteWidgetCommand, SetUiRootCommand},
    },
    utils,
    world::{WorldViewerItemContextMenu, create::EntityCreatorMode},
};
use std::path::PathBuf;

pub struct WidgetContextMenu {
    menu: RcUiNodeHandle,
    delete_selection: Handle<MenuItem>,
    copy_selection: Handle<MenuItem>,
    create_child: Handle<MenuItem>,
    placement_target: Handle<UiNode>,
    paste: Handle<MenuItem>,
    make_root: Handle<MenuItem>,
    open_asset: Handle<MenuItem>,
    show_asset: Handle<MenuItem>,
    duplicate_selection: Handle<MenuItem>,
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
    if let Some(ui_selection) = editor_selection.as_ui()
        && let Some(first) = ui_selection.widgets.first()
        && let Some(resource) = ui_scene
            .ui
            .try_get_node(*first)
            .ok()
            .and_then(|n| n.resource())
    {
        return resource_manager.resource_path(resource.as_ref());
    }
    None
}

impl WidgetContextMenu {
    pub const DELETE_SELECTION: Uuid = uuid!("30eef2a7-9f12-4e64-9142-b604f25e9e06");
    pub const COPY_SELECTION: Uuid = uuid!("0a2d10bc-de1e-4196-aba6-0a51c54eb238");
    pub const DUPLICATE_SELECTION: Uuid = uuid!("d39d5a97-d515-45d0-8c83-2081872e3ede");
    pub const PASTE_AS_CHILD: Uuid = uuid!("d3b86c8c-1efd-4917-9543-b0f5f32f8cbd");
    pub const CREATE_CHILD_WIDGET: Uuid = uuid!("26b2ed1f-c4b8-4ec4-8112-9f29fc600101");
    pub const MAKE_ROOT: Uuid = uuid!("968318f6-21c7-430f-a13d-36aefb61cde2");
    pub const OPEN_ASSET: Uuid = uuid!("f3f7d0fa-e905-4371-8973-dfc1eb758e5a");
    pub const SHOW_ASSET: Uuid = uuid!("0e8ef1bf-bc59-421d-80f4-c234f8469c01");

    pub fn new(ctx: &mut BuildContext) -> Self {
        let delete_selection = create_menu_item_shortcut(
            "Delete Selection",
            None,
            Self::DELETE_SELECTION,
            "Del",
            vec![],
            ctx,
        );
        let copy_selection = create_menu_item_shortcut(
            "Copy Selection",
            None,
            Self::COPY_SELECTION,
            "Ctrl+C",
            vec![],
            ctx,
        );
        let duplicate_selection = create_menu_item(
            "Duplicate Selection",
            Self::DUPLICATE_SELECTION,
            vec![],
            ctx,
        );
        let paste = create_menu_item("Paste As Child", Self::PASTE_AS_CHILD, vec![], ctx);
        let make_root = create_menu_item("Make Root", Self::MAKE_ROOT, vec![], ctx);
        let open_asset = create_menu_item("Open Parent Prefab", Self::OPEN_ASSET, vec![], ctx);
        let show_asset = create_menu_item(
            "Show Prefab In Asset Browser",
            Self::SHOW_ASSET,
            vec![],
            ctx,
        );
        let create_child = create_menu_item(
            "Create Child Widget...",
            Self::CREATE_CHILD_WIDGET,
            vec![],
            ctx,
        );
        let stack_panel;
        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
                .with_content({
                    stack_panel = StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_child(create_child)
                            .with_child(make_menu_splitter(ctx))
                            .with_child(delete_selection)
                            .with_child(copy_selection)
                            .with_child(duplicate_selection)
                            .with_child(paste)
                            .with_child(make_menu_splitter(ctx))
                            .with_child(make_root)
                            .with_child(open_asset)
                            .with_child(show_asset),
                    )
                    .build(ctx);
                    stack_panel
                })
                .with_restrict_picking(false),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            create_child,
            menu,
            delete_selection,
            copy_selection,
            placement_target: Default::default(),
            paste,
            make_root,
            open_asset,
            show_asset,
            duplicate_selection,
        }
    }

    fn on_duplicate_clicked(
        &self,
        editor_selection: &Selection,
        ui_scene: &mut UiScene,
        sender: &MessageSender,
    ) {
        let ui_selection = some_or_return!(editor_selection.as_ui());
        let root_widgets = ui_selection.root_widgets(&ui_scene.ui);
        let first_root = some_or_return!(root_widgets.first().cloned());
        let parent = ok_or_return!(ui_scene.ui.try_get(first_root).map(|r| r.parent()));
        ui_scene
            .clipboard
            .fill_from_selection(ui_selection, &ui_scene.ui);
        sender.do_command(PasteWidgetCommand::new(parent));
    }

    fn on_delete_selection_clicked(
        &self,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        sender: &MessageSender,
    ) {
        if let Some(command) = editor_selection.make_delete_selection_command(controller, engine) {
            sender.send(Message::DoCommand(command));
        }
    }

    fn on_copy_selection_clicked(&self, editor_selection: &Selection, ui_scene: &mut UiScene) {
        if let Some(ui_selection) = editor_selection.as_ui() {
            ui_scene
                .clipboard
                .fill_from_selection(ui_selection, &ui_scene.ui);
        }
    }

    fn on_paste_clicked(
        &self,
        editor_selection: &Selection,
        ui_scene: &mut UiScene,
        sender: &MessageSender,
    ) {
        if let Some(ui_selection) = editor_selection.as_ui()
            && let Some(first) = ui_selection.widgets.first()
            && !ui_scene.clipboard.is_empty()
        {
            sender.do_command(PasteWidgetCommand::new(*first));
        }
    }

    fn on_make_root_clicked(&self, editor_selection: &Selection, sender: &MessageSender) {
        if let Some(selection) = editor_selection.as_ui()
            && let Some(first) = selection.widgets.first()
        {
            sender.do_command(SetUiRootCommand {
                root: *first,
                link_scheme: Default::default(),
            });
        }
    }

    fn on_open_asset_clicked(
        &self,
        editor_selection: &Selection,
        ui_scene: &mut UiScene,
        engine: &mut Engine,
        sender: &MessageSender,
    ) {
        if let Some(path) = resource_path_of_first_selected_node(
            editor_selection,
            ui_scene,
            &engine.resource_manager,
        ) && utils::is_native_scene(&path)
        {
            sender.send(Message::LoadScene(path));
        }
    }

    fn on_show_asset_clicked(
        &self,
        editor_selection: &Selection,
        ui_scene: &mut UiScene,
        engine: &mut Engine,
        sender: &MessageSender,
    ) {
        if let Some(path) = resource_path_of_first_selected_node(
            editor_selection,
            ui_scene,
            &engine.resource_manager,
        ) {
            sender.send(Message::ShowInAssetBrowser(path));
        }
    }

    fn on_menu_placed(
        &mut self,
        target: Handle<UiNode>,
        engine: &mut Engine,
        ui_scene: &mut UiScene,
    ) {
        self.placement_target = target;

        // Check if there's something to paste and deactivate "Paste" if nothing.
        engine.user_interfaces.first_mut().send(
            self.paste,
            WidgetMessage::Enabled(!ui_scene.clipboard.is_empty()),
        );
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        sender: &MessageSender,
    ) {
        let ui_scene = some_or_return!(controller.downcast_mut::<UiScene>());
        if let Some(MenuItemMessage::Click) = message.data_from(self.delete_selection) {
            self.on_delete_selection_clicked(editor_selection, controller, engine, sender);
        } else if let Some(MenuItemMessage::Click) = message.data_from(self.copy_selection) {
            self.on_copy_selection_clicked(editor_selection, ui_scene);
        } else if let Some(MenuItemMessage::Click) = message.data_from(self.duplicate_selection) {
            self.on_duplicate_clicked(editor_selection, ui_scene, sender);
        } else if let Some(MenuItemMessage::Click) = message.data_from(self.paste) {
            self.on_paste_clicked(editor_selection, ui_scene, sender);
        } else if let Some(MenuItemMessage::Click) = message.data_from(self.make_root) {
            self.on_make_root_clicked(editor_selection, sender);
        } else if let Some(MenuItemMessage::Click) = message.data_from(self.open_asset) {
            self.on_open_asset_clicked(editor_selection, ui_scene, engine, sender);
        } else if let Some(MenuItemMessage::Click) = message.data_from(self.show_asset) {
            self.on_show_asset_clicked(editor_selection, ui_scene, engine, sender);
        } else if let Some(MenuItemMessage::Click) = message.data_from(self.create_child) {
            sender.send(Message::OpenEntityCreator(EntityCreatorMode::CreateChild));
        } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) =
            message.data_for(self.menu.handle())
        {
            self.on_menu_placed(*target, engine, ui_scene)
        }
    }
}
