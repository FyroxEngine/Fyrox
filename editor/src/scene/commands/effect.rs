use crate::{define_universal_commands, define_vec_add_remove_commands, Command, SceneContext};
use fyrox::{
    core::reflect::Reflect,
    core::{
        pool::{Handle, Ticket},
        reflect::ResolvePath,
    },
    scene::sound::effect::{Effect, EffectInput},
};

use crate::scene::commands::SceneCommand;

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

#[macro_export]
macro_rules! define_effect_command {
    ($($name:ident($human_readable_name:expr, $value_type:ty) where fn swap($self:ident, $effect:ident) $apply_method:block )*) => {
        $(
            #[derive(Debug)]
            pub struct $name {
                handle: Handle<Effect>,
                value: $value_type,
            }

            impl $name {
                pub fn new(handle: Handle<Effect>, value: $value_type) -> Self {
                    Self { handle, value }
                }

                fn swap(&mut $self, context: &mut SoundContext) {
                    let $effect = &mut context.effect_mut($self.handle);
                    $apply_method
                }
            }

            impl Command for $name {
                fn name(&mut self, _context: &SceneContext) -> String {
                    $human_readable_name.to_owned()
                }

                fn execute(&mut self, context: &mut SceneContext) {
                    self.swap(&mut context.scene.graph.sound_context);
                }

                fn revert(&mut self, context: &mut SceneContext) {
                    self.swap(&mut context.scene.graph.sound_context);
                }
            }
        )*
    };
}

define_vec_add_remove_commands!(struct AddInputCommand, RemoveInputCommand<Effect, EffectInput> 
(self, context) { context.scene.graph.sound_context.effect_mut(self.handle).inputs_mut() });

#[macro_export]
macro_rules! define_effect_input_command {
    ($($name:ident($human_readable_name:expr, $value_type:ty) where fn swap($self:ident, $input:ident) $apply_method:block )*) => {
        $(
            #[derive(Debug)]
            pub struct $name {
                handle: Handle<Effect>,
                index: usize,
                value: $value_type,
            }

            impl $name {
                pub fn new(handle: Handle<Effect>, index: usize, value: $value_type) -> Self {
                    Self { handle, index, value }
                }

                fn swap(&mut $self, context: &mut SoundContext) {
                    let $input = &mut context.effect_mut($self.handle).inputs_mut()[$self.index];
                    $apply_method
                }
            }

            impl Command for $name {
                fn name(&mut self, _context: &SceneContext) -> String {
                    $human_readable_name.to_owned()
                }

                fn execute(&mut self, context: &mut SceneContext) {
                    self.swap(&mut context.scene.graph.sound_context);
                }

                fn revert(&mut self, context: &mut SceneContext) {
                    self.swap(&mut context.scene.graph.sound_context);
                }
            }
        )*
    };
}
