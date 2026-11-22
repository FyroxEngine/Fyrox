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
    Message,
    fyrox::{
        core::{
            pool::Handle,
            some_or_return,
            uuid::{Uuid, uuid},
        },
        gui::{
            BuildContext, UserInterface,
            message::UiMessage,
            messagebox::{
                MessageBox, MessageBoxBuilder, MessageBoxButtons, MessageBoxMessage,
                MessageBoxResult,
            },
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowTitle},
        },
    },
    game_scene::container::SceneContainer,
    message::MessageSender,
};

pub struct SaveSceneConfirmationDialog {
    save_message_box: Handle<MessageBox>,
    scene_id: Uuid,
}

impl SaveSceneConfirmationDialog {
    pub const DIALOG_ID: Uuid = uuid!("9828ff5f-c2e2-4e72-9b17-0d5192155a77");

    pub fn new(ctx: &mut BuildContext) -> Self {
        let save_message_box = MessageBoxBuilder::new(
            WindowBuilder::new(
                WidgetBuilder::new()
                    .with_id(Self::DIALOG_ID)
                    .with_width(300.0)
                    .with_height(100.0),
            )
            .with_remove_on_close(true)
            .can_close(false)
            .can_maximize(false)
            .can_minimize(false)
            .open(false)
            .with_title(WindowTitle::text("Unsaved changes")),
        )
        .with_buttons(MessageBoxButtons::YesNoCancel)
        .build(ctx);

        Self {
            save_message_box,
            scene_id: Default::default(),
        }
    }

    pub fn open(&mut self, ui: &UserInterface, id: Uuid, scenes: &SceneContainer) {
        self.scene_id = id;
        let entry = some_or_return!(scenes.entry_by_scene_id(self.scene_id));
        let text = format!(
            "There are unsaved changes in the {} scene. Do you wish to save them before continue?",
            entry.name(),
        );
        ui.send(
            self.save_message_box,
            MessageBoxMessage::Open {
                title: None,
                text: Some(text),
            },
        );
    }

    pub fn force_destroy(self, ui: &UserInterface) {
        ui.send(self.save_message_box, WidgetMessage::Remove);
    }

    fn close_scene_and_destroy(self, sender: &MessageSender) -> Option<Self> {
        sender.send(Message::CloseScene(self.scene_id));
        None
    }

    fn on_yes_clicked(self, sender: &MessageSender, scenes: &SceneContainer) -> Option<Self> {
        let entry = some_or_return!(scenes.entry_by_scene_id(self.scene_id), Some(self));
        if let Some(path) = entry.path.clone() {
            // If the scene was already saved into some file - save it
            // immediately and perform the requested action.
            sender.send(Message::SaveScene {
                id: self.scene_id,
                path,
            });
            self.close_scene_and_destroy(sender)
        } else {
            // Otherwise, open save scene dialog and do the action after the
            // scene was saved.
            sender.send(Message::OpenSaveSceneDialog {
                default_file_info: entry.default_file_info(),
            });
            Some(self)
        }
    }

    pub fn handle_ui_message(
        self,
        message: &UiMessage,
        sender: &MessageSender,
        scenes: &SceneContainer,
    ) -> Option<Self> {
        if let Some(MessageBoxMessage::Close(result)) = message.data_from(self.save_message_box) {
            return match result {
                MessageBoxResult::No => self.close_scene_and_destroy(sender),
                MessageBoxResult::Yes => self.on_yes_clicked(sender, scenes),
                _ => Some(self),
            };
        }
        Some(self)
    }

    pub fn handle_message(self, message: &Message, sender: &MessageSender) -> Option<Self> {
        if let Message::SaveScene { id, .. } = message
            && *id == self.scene_id
        {
            self.close_scene_and_destroy(sender)
        } else {
            Some(self)
        }
    }
}
