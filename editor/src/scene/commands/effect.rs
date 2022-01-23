use crate::{define_vec_add_remove_commands, get_set_swap, Command, SceneContext};
use fyrox::{
    core::pool::{Handle, Ticket},
    scene::{
        node::Node,
        sound::{
            context::SoundContext,
            effect::{Effect, EffectInput},
            Biquad,
        },
    },
};

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

define_effect_command! {
    SetNameCommand("Set Effect Name", String) where fn swap(self, effect) {
        get_set_swap!(self, effect, name_owned, set_name);
    }

    SetGainCommand("Set Effect Gain", f32) where fn swap(self, effect) {
        get_set_swap!(self, effect, gain, set_gain);
    }

    SetReverbDryCommand("Set Reverb Dry", f32) where fn swap(self, effect) {
        get_set_swap!(self, effect.as_reverb_mut(), dry, set_dry);
    }

    SetReverbWetCommand("Set Reverb Wet", f32) where fn swap(self, effect) {
        get_set_swap!(self, effect.as_reverb_mut(), wet, set_wet);
    }

    SetReverbFcCommand("Set Reverb Fc", f32) where fn swap(self, effect) {
        get_set_swap!(self, effect.as_reverb_mut(), fc, set_fc);
    }
}

define_effect_command!(SetReverbDecayTimeCommand("Set Reverb Decay Time", f32) where fn swap(self, effect) {
    get_set_swap!(self, effect.as_reverb_mut(), decay_time, set_decay_time);
});

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

define_effect_input_command! {
    SetEffectInputSound("Set Effect Input Sound", Handle<Node>) where fn swap(self, input) {
        std::mem::swap(&mut input.sound, &mut self.value)
    }

    SetEffectInputFilter("Set Effect Input Filter", Option<Biquad>) where fn swap(self, input) {
        std::mem::swap(&mut input.filter, &mut self.value)
    }

    SetEffectInputFilterB0("Set Effect Input Filter B0", f32) where fn swap(self, input) {
        std::mem::swap(&mut input.filter.as_mut().unwrap().b0, &mut self.value)
    }

    SetEffectInputFilterB1("Set Effect Input Filter B1", f32) where fn swap(self, input) {
        std::mem::swap(&mut input.filter.as_mut().unwrap().b1, &mut self.value)
    }

    SetEffectInputFilterB2("Set Effect Input Filter B2", f32) where fn swap(self, input) {
        std::mem::swap(&mut input.filter.as_mut().unwrap().b2, &mut self.value)
    }

    SetEffectInputFilterA1("Set Effect Input Filter A1", f32) where fn swap(self, input) {
        std::mem::swap(&mut input.filter.as_mut().unwrap().a1, &mut self.value)
    }

    SetEffectInputFilterA2("Set Effect Input Filter A2", f32) where fn swap(self, input) {
        std::mem::swap(&mut input.filter.as_mut().unwrap().a2, &mut self.value)
    }
}
