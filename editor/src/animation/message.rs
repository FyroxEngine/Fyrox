use crate::animation::command::AnimationCommand;
use std::path::PathBuf;

pub enum Message {
    NewAnimation,
    DoCommand(AnimationCommand),
    Undo,
    Redo,
    ClearCommandStack,
    Exit,
    Save(PathBuf),
}
