use crate::animation::command::AnimationCommand;

pub enum Message {
    NewAnimation,
    DoCommand(AnimationCommand),
    Undo,
    Redo,
    ClearCommandStack,
    Exit,
}
