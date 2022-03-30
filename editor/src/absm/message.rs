use crate::absm::command::AbsmCommand;

pub enum AbsmMessage {
    DoCommand(AbsmCommand),
    Undo,
    Redo,
    ClearCommandStack,
    CreateNewAbsm,
    LoadAbsm,
    SaveCurrentAbsm,
}
