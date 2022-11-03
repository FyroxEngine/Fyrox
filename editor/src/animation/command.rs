use crate::{define_command_stack, define_universal_commands};
use fyrox::{
    asset::ResourceDataRef,
    core::reflect::ResolvePath,
    resource::animation::{AnimationResourceError, AnimationResourceState},
};
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

#[derive(Debug)]
pub struct AnimationEditorContext<'a> {
    pub resource: ResourceDataRef<'a, AnimationResourceState, AnimationResourceError>,
}

define_command_stack!(
    AnimationCommandTrait,
    AnimationCommandStack,
    AnimationEditorContext
);

#[derive(Debug)]
pub struct AnimationCommand(pub Box<dyn AnimationCommandTrait>);

impl Deref for AnimationCommand {
    type Target = dyn AnimationCommandTrait;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for AnimationCommand {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

impl AnimationCommand {
    pub fn new<C: AnimationCommandTrait>(cmd: C) -> Self {
        Self(Box::new(cmd))
    }

    pub fn into_inner(self) -> Box<dyn AnimationCommandTrait> {
        self.0
    }
}

#[derive(Debug)]
pub struct CommandGroup {
    commands: Vec<AnimationCommand>,
}

impl From<Vec<AnimationCommand>> for CommandGroup {
    fn from(commands: Vec<AnimationCommand>) -> Self {
        Self { commands }
    }
}

impl CommandGroup {
    #[allow(dead_code)]
    pub fn push(&mut self, command: AnimationCommand) {
        self.commands.push(command)
    }
}

impl AnimationCommandTrait for CommandGroup {
    fn name(&mut self, context: &AnimationEditorContext) -> String {
        let mut name = String::from("Command group: ");
        for cmd in self.commands.iter_mut() {
            name.push_str(&cmd.name(context));
            name.push_str(", ");
        }
        name
    }

    fn execute(&mut self, context: &mut AnimationEditorContext) {
        for cmd in self.commands.iter_mut() {
            cmd.execute(context);
        }
    }

    fn revert(&mut self, context: &mut AnimationEditorContext) {
        // revert must be done in reverse order.
        for cmd in self.commands.iter_mut().rev() {
            cmd.revert(context);
        }
    }

    fn finalize(&mut self, context: &mut AnimationEditorContext) {
        for mut cmd in self.commands.drain(..) {
            cmd.finalize(context);
        }
    }
}

define_universal_commands!(
    make_set_animation_property_command,
    AnimationCommandTrait,
    AnimationCommand,
    AnimationEditorContext,
    (),
    ctx,
    handle,
    self,
    { &mut ctx.resource.animation_definition }
);
