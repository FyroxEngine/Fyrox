use crate::{
    menu::{create_menu_item_shortcut, create_root_menu_item},
    scene::{commands::PasteCommand, EditorScene, Selection},
    GameEngine, Message,
};
use rg3d::{
    core::pool::Handle,
    gui::{
        message::{MenuItemMessage, UiMessage, UiMessageData},
        BuildContext, UiNode,
    },
};
use std::sync::mpsc::Sender;

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
        sender: &Sender<Message>,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
    ) {
        if let UiMessageData::MenuItem(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.copy {
                if let Selection::Graph(selection) = &editor_scene.selection {
                    editor_scene.clipboard.fill_from_selection(
                        selection,
                        editor_scene.scene,
                        &editor_scene.physics,
                        engine,
                    );
                }
            } else if message.destination() == self.paste {
                if !editor_scene.clipboard.is_empty() {
                    sender
                        .send(Message::do_scene_command(PasteCommand::new()))
                        .unwrap();
                }
            } else if message.destination() == self.undo {
                sender.send(Message::UndoSceneCommand).unwrap();
            } else if message.destination() == self.redo {
                sender.send(Message::RedoSceneCommand).unwrap();
            }
        }
    }
}
