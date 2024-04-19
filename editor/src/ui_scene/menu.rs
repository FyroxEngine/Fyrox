use crate::fyrox::graph::{BaseSceneGraph, SceneGraphNode};
use crate::fyrox::{
    core::pool::Handle,
    gui::{
        menu::MenuItemMessage,
        message::UiMessage,
        popup::{Placement, PopupBuilder, PopupMessage},
        stack_panel::StackPanelBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        BuildContext, RcUiNodeHandle, UiNode,
    },
};
use crate::{
    menu::{create_menu_item, create_menu_item_shortcut, ui::UiMenu},
    message::MessageSender,
    scene::{controller::SceneController, Selection},
    ui_scene::{
        commands::graph::{PasteWidgetCommand, SetUiRootCommand},
        UiScene,
    },
    utils,
    world::WorldViewerItemContextMenu,
    Engine, Message, MessageDirection,
};
use fyrox::gui::menu::ContextMenuBuilder;
use std::path::PathBuf;

pub struct WidgetContextMenu {
    menu: RcUiNodeHandle,
    delete_selection: Handle<UiNode>,
    copy_selection: Handle<UiNode>,
    widgets_menu: UiMenu,
    placement_target: Handle<UiNode>,
    paste: Handle<UiNode>,
    make_root: Handle<UiNode>,
    open_asset: Handle<UiNode>,
}

impl WorldViewerItemContextMenu for WidgetContextMenu {
    fn menu(&self) -> RcUiNodeHandle {
        self.menu.clone()
    }
}

fn resource_path_of_first_selected_node(
    editor_selection: &Selection,
    ui_scene: &UiScene,
) -> Option<PathBuf> {
    if let Some(ui_selection) = editor_selection.as_ui() {
        if let Some(first) = ui_selection.widgets.first() {
            if let Some(resource) = ui_scene.ui.try_get(*first).and_then(|n| n.resource()) {
                return resource.kind().into_path();
            }
        }
    }
    None
}

impl WidgetContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let delete_selection;
        let copy_selection;
        let paste;
        let make_root;
        let open_asset;

        let widgets_menu = UiMenu::new(UiMenu::default_entries(), "Create Child Widget", ctx);

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
                            make_root = create_menu_item("Make Root", vec![], ctx);
                            make_root
                        })
                        .with_child({
                            open_asset = create_menu_item("Open Asset", vec![], ctx);
                            open_asset
                        })
                        .with_child(widgets_menu.menu),
                )
                .build(ctx),
            ),
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
                    if let Some(path) =
                        resource_path_of_first_selected_node(editor_selection, ui_scene)
                    {
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
                    engine
                        .user_interfaces
                        .first_mut()
                        .send_message(WidgetMessage::enabled(
                            self.paste,
                            MessageDirection::ToWidget,
                            !ui_scene.clipboard.is_empty(),
                        ));
                }
            }
        }
    }
}
