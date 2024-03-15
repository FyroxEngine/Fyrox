use crate::fyrox::{
    core::pool::Handle,
    gui::{
        menu::MenuItemMessage,
        message::{MessageDirection, UiMessage},
        widget::WidgetMessage,
        BuildContext, UiNode, UserInterface,
    },
};
use crate::scene::controller::SceneController;
use crate::{
    menu::{create_menu_item_shortcut, create_root_menu_item},
    message::MessageSender,
    scene::{commands::PasteCommand, GameScene, Selection},
    Engine, Message, Mode,
};

pub struct EditMenu {
    pub menu: Handle<UiNode>,
    undo: Handle<UiNode>,
    redo: Handle<UiNode>,
    copy: Handle<UiNode>,
    paste: Handle<UiNode>,
}

impl EditMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let redo;
        let undo;
        let copy;
        let paste;
        let menu = create_root_menu_item(
            "Edit",
            vec![
                {
                    undo = create_menu_item_shortcut("Undo", "Ctrl+Z", vec![], ctx);
                    undo
                },
                {
                    redo = create_menu_item_shortcut("Redo", "Ctrl+Y", vec![], ctx);
                    redo
                },
                {
                    copy = create_menu_item_shortcut("Copy", "Ctrl+C", vec![], ctx);
                    copy
                },
                {
                    paste = create_menu_item_shortcut("Paste", "Ctrl+V", vec![], ctx);
                    paste
                },
            ],
            ctx,
        );

        Self {
            menu,
            undo,
            redo,
            copy,
            paste,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        sender: &MessageSender,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
    ) {
        if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if message.destination() == self.copy {
                if let Some(selection) = editor_selection.as_graph() {
                    if let Some(game_scene) = controller.downcast_mut::<GameScene>() {
                        game_scene.clipboard.fill_from_selection(
                            selection,
                            game_scene.scene,
                            engine,
                        );
                    }
                }
            } else if message.destination() == self.paste {
                if let Some(game_scene) = controller.downcast_mut::<GameScene>() {
                    if !game_scene.clipboard.is_empty() {
                        sender.do_command(PasteCommand::new(game_scene.scene_content_root));
                    }
                }
            } else if message.destination() == self.undo {
                sender.send(Message::UndoCurrentSceneCommand);
            } else if message.destination() == self.redo {
                sender.send(Message::RedoCurrentSceneCommand);
            }
        }
    }

    pub fn on_mode_changed(&mut self, ui: &UserInterface, mode: &Mode) {
        ui.send_message(WidgetMessage::enabled(
            self.menu,
            MessageDirection::ToWidget,
            mode.is_edit(),
        ));
    }
}
