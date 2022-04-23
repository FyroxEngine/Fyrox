use crate::absm::command::{AbsmCommand, AbsmCommandTrait};
use std::{path::PathBuf, sync::mpsc::Sender};

pub enum AbsmMessage {
    DoCommand(AbsmCommand),
    Undo,
    Redo,
    ClearCommandStack,
    CreateNewAbsm,
    LoadAbsm,
    SaveCurrentAbsm,
    Sync,
    SetPreviewModel(PathBuf),
}

pub struct MessageSender {
    sender: Sender<AbsmMessage>,
}

impl MessageSender {
    pub fn new(sender: Sender<AbsmMessage>) -> Self {
        Self { sender }
    }

    fn send(&self, message: AbsmMessage) {
        self.sender.send(message).expect("Receiver must exist!")
    }

    pub fn do_command<T: AbsmCommandTrait>(&self, command: T) {
        self.send(AbsmMessage::DoCommand(AbsmCommand::new(command)))
    }

    pub fn do_command_value(&self, command: AbsmCommand) {
        self.send(AbsmMessage::DoCommand(command))
    }

    pub fn undo(&self) {
        self.send(AbsmMessage::Undo)
    }

    pub fn redo(&self) {
        self.send(AbsmMessage::Redo)
    }

    pub fn clear_command_stack(&self) {
        self.send(AbsmMessage::ClearCommandStack)
    }

    pub fn create_new_absm(&self) {
        self.send(AbsmMessage::CreateNewAbsm)
    }

    pub fn load_absm(&self) {
        self.send(AbsmMessage::LoadAbsm)
    }

    pub fn save_current_absm(&self) {
        self.send(AbsmMessage::SaveCurrentAbsm)
    }

    pub fn sync(&self) {
        self.send(AbsmMessage::Sync)
    }

    pub fn set_preview_model(&self, path: PathBuf) {
        self.send(AbsmMessage::SetPreviewModel(path))
    }
}
