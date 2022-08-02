use crate::{define_universal_commands, scene::commands::SceneCommand, Command, SceneContext};
use fyrox::{
    core::reflect::Reflect,
    core::{
        pool::{Handle, Ticket},
        reflect::ResolvePath,
    },
    scene::sound::effect::Effect,
};

define_universal_commands!(
    make_set_effect_property_command,
    Command,
    SceneCommand,
    SceneContext,
    Handle<Effect>,
    ctx,
    handle,
    self,
    {
        ctx.scene
            .graph
            .sound_context
            .effect_mut(self.handle)
            .as_reflect_mut()
    }
);

#[derive(Debug)]
pub struct AddEffectCommand {
    effect: Option<Effect>,
    handle: Handle<Effect>,
    ticket: Option<Ticket<Effect>>,
}

impl AddEffectCommand {
    pub fn new(effect: Effect) -> Self {
        Self {
            effect: Some(effect),
            handle: Default::default(),
            ticket: None,
        }
    }
}

impl Command for AddEffectCommand {
    fn name(&mut self, _: &SceneContext) -> String {
        "Add Effect".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        self.handle = context
            .scene
            .graph
            .sound_context
            .add_effect(self.effect.take().unwrap());
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let (ticket, effect) = context
            .scene
            .graph
            .sound_context
            .take_reserve_effect(self.handle);
        self.effect = Some(effect);
        self.ticket = Some(ticket);
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        if let Some(ticket) = self.ticket.take() {
            context
                .scene
                .graph
                .sound_context
                .forget_effect_ticket(ticket);
        }
    }
}

#[derive(Debug)]
pub struct RemoveEffectCommand {
    effect: Option<Effect>,
    handle: Handle<Effect>,
    ticket: Option<Ticket<Effect>>,
}

impl Command for RemoveEffectCommand {
    fn name(&mut self, _: &SceneContext) -> String {
        "Remove Effect".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let (ticket, effect) = context
            .scene
            .graph
            .sound_context
            .take_reserve_effect(self.handle);
        self.effect = Some(effect);
        self.ticket = Some(ticket);
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.handle = context
            .scene
            .graph
            .sound_context
            .add_effect(self.effect.take().unwrap());
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        if let Some(ticket) = self.ticket.take() {
            context
                .scene
                .graph
                .sound_context
                .forget_effect_ticket(ticket);
        }
    }
}
