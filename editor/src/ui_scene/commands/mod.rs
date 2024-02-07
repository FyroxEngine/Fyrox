pub mod graph;
pub mod widget;

use crate::ui_scene::clipboard::Clipboard;
use crate::{
    define_command_stack, define_universal_commands, message::MessageSender, scene::Selection,
    Message,
};
use fyrox::{
    core::pool::Handle,
    core::reflect::prelude::*,
    core::reflect::SetFieldByPathError,
    gui::{UiNode, UserInterface},
};
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

define_command_stack!(UiCommand, UiCommandStack, UiSceneContext);

pub struct UiSceneContext<'a> {
    pub ui: &'a mut UserInterface,
    pub selection: &'a mut Selection,
    pub message_sender: &'a MessageSender,
    pub clipboard: &'a mut Clipboard,
}

#[derive(Debug)]
pub struct UiSceneCommand(pub Box<dyn UiCommand>);

impl Deref for UiSceneCommand {
    type Target = dyn UiCommand;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for UiSceneCommand {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

impl UiSceneCommand {
    pub fn new<C: UiCommand>(cmd: C) -> Self {
        Self(Box::new(cmd))
    }

    pub fn into_inner(self) -> Box<dyn UiCommand> {
        self.0
    }
}

#[derive(Debug, Default)]
pub struct UiCommandGroup {
    commands: Vec<UiSceneCommand>,
    custom_name: String,
}

impl From<Vec<UiSceneCommand>> for UiCommandGroup {
    fn from(commands: Vec<UiSceneCommand>) -> Self {
        Self {
            commands,
            custom_name: Default::default(),
        }
    }
}

impl UiCommandGroup {
    pub fn push<C: UiCommand>(&mut self, command: C) {
        self.commands.push(UiSceneCommand::new(command))
    }

    pub fn with_custom_name<S: AsRef<str>>(mut self, name: S) -> Self {
        self.custom_name = name.as_ref().to_string();
        self
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }
}

impl UiCommand for UiCommandGroup {
    fn name(&mut self, context: &UiSceneContext) -> String {
        if self.custom_name.is_empty() {
            let mut name = String::from("Command group: ");
            for cmd in self.commands.iter_mut() {
                name.push_str(&cmd.name(context));
                name.push_str(", ");
            }
            name
        } else {
            self.custom_name.clone()
        }
    }

    fn execute(&mut self, context: &mut UiSceneContext) {
        for cmd in self.commands.iter_mut() {
            cmd.execute(context);
        }
    }

    fn revert(&mut self, context: &mut UiSceneContext) {
        // revert must be done in reverse order.
        for cmd in self.commands.iter_mut().rev() {
            cmd.revert(context);
        }
    }

    fn finalize(&mut self, context: &mut UiSceneContext) {
        for mut cmd in self.commands.drain(..) {
            cmd.finalize(context);
        }
    }
}

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

    fn exec(&mut self, context: &mut UiSceneContext) {
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

impl UiCommand for ChangeUiSelectionCommand {
    fn name(&mut self, _context: &UiSceneContext) -> String {
        "Change Selection".to_string()
    }

    fn execute(&mut self, context: &mut UiSceneContext) {
        self.exec(context);
    }

    fn revert(&mut self, context: &mut UiSceneContext) {
        self.exec(context);
    }
}

define_universal_commands!(
    make_set_widget_property_command,
    UiCommand,
    UiSceneCommand,
    UiSceneContext,
    Handle<UiNode>,
    ctx,
    handle,
    self,
    { &mut *ctx.ui.node_mut(self.handle) as &mut dyn Reflect },
);
