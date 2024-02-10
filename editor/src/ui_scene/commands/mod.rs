pub mod graph;
pub mod widget;

use crate::{
    command::{CommandContext, CommandTrait},
    message::MessageSender,
    scene::Selection,
    ui_scene::clipboard::Clipboard,
    Message,
};
use fyrox::gui::UserInterface;
use std::fmt::Debug;

pub struct UiSceneContext<'a> {
    pub ui: &'a mut UserInterface,
    pub selection: &'a mut Selection,
    pub message_sender: &'a MessageSender,
    pub clipboard: &'a mut Clipboard,
}

impl<'a> UiSceneContext<'a> {
    pub fn exec<F>(
        ui: &'a mut UserInterface,
        selection: &'a mut Selection,
        message_sender: &'a MessageSender,
        clipboard: &'a mut Clipboard,
        func: F,
    ) where
        F: FnOnce(&mut UiSceneContext<'static>),
    {
        // SAFETY: Temporarily extend lifetime to 'static and execute external closure with it.
        // The closure accepts this extended context by reference, so there's no way it escapes to
        // outer world. The initial lifetime is still preserved by this function call.
        func(unsafe {
            &mut std::mem::transmute::<UiSceneContext<'a>, UiSceneContext<'static>>(Self {
                ui,
                selection,
                message_sender,
                clipboard,
            })
        });
    }
}

impl CommandContext for UiSceneContext<'static> {}

#[derive(Debug)]
pub struct ChangeUiSelectionCommand {
    new_selection: Selection,
    old_selection: Selection,
}

impl ChangeUiSelectionCommand {
    pub fn new(new_selection: Selection, old_selection: Selection) -> Self {
        Self {
            new_selection,
            old_selection,
        }
    }

    fn swap(&mut self) -> Selection {
        let selection = self.new_selection.clone();
        std::mem::swap(&mut self.new_selection, &mut self.old_selection);
        selection
    }

    fn exec(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<UiSceneContext>();
        let old_selection = self.old_selection.clone();
        let new_selection = self.swap();
        if &new_selection != context.selection {
            *context.selection = new_selection;
            context
                .message_sender
                .send(Message::SelectionChanged { old_selection });
        }
    }
}

impl CommandTrait for ChangeUiSelectionCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Change Selection".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.exec(context);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.exec(context);
    }
}
