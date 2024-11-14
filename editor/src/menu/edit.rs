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
    pub undo: Handle<UiNode>,
    pub redo: Handle<UiNode>,
    pub copy: Handle<UiNode>,
    pub paste: Handle<UiNode>,
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
