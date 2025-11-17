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
        menu::MenuItemMessage, message::UiMessage, widget::WidgetMessage, BuildContext, UiNode,
        UserInterface,
    },
};
use crate::scene::controller::SceneController;
use crate::{
    menu::{create_menu_item_shortcut, create_root_menu_item},
    message::MessageSender,
    scene::{commands::PasteCommand, GameScene, Selection},
    Engine, Message, Mode,
};
use fyrox::core::{uuid, Uuid};
use fyrox::gui::menu;

pub struct EditMenu {
    pub menu: Handle<UiNode>,
    pub undo: Handle<UiNode>,
    pub redo: Handle<UiNode>,
    pub copy: Handle<UiNode>,
    pub paste: Handle<UiNode>,
}

impl EditMenu {
    pub const EDIT: Uuid = uuid!("947547a7-d705-405f-81a8-7c498a22dbcc");
    pub const UNDO: Uuid = uuid!("8d25d35f-bcbe-4647-8d43-5eb0fbacc9ca");
    pub const REDO: Uuid = uuid!("ee825148-feab-435f-9db5-c3f2c89a989a");
    pub const COPY: Uuid = uuid!("7d3ad96c-06f0-43ee-b9d1-6f86a0b783d1");
    pub const PASTE: Uuid = uuid!("ae45f10b-7833-4c01-b426-37d1a29c4a8a");

    pub fn new(ctx: &mut BuildContext) -> Self {
        let redo;
        let undo;
        let copy;
        let paste;
        let menu = create_root_menu_item(
            "Edit",
            Self::EDIT,
            vec![
                {
                    undo = create_menu_item_shortcut("Undo", Self::UNDO, "Ctrl+Z", vec![], ctx);
                    undo
                },
                {
                    redo = create_menu_item_shortcut("Redo", Self::REDO, "Ctrl+Y", vec![], ctx);
                    redo
                },
                menu::make_menu_splitter(ctx),
                {
                    copy = create_menu_item_shortcut("Copy", Self::COPY, "Ctrl+C", vec![], ctx);
                    copy
                },
                {
                    paste = create_menu_item_shortcut("Paste", Self::PASTE, "Ctrl+V", vec![], ctx);
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
        ui.send(self.menu, WidgetMessage::Enabled(mode.is_edit()));
    }
}
