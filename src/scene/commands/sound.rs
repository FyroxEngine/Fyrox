use crate::get_set_swap;
use crate::{command::Command, scene::commands::SceneContext};
use rg3d::core::algebra::Vector3;
use rg3d::sound::buffer::SoundBufferResource;
use rg3d::sound::context::SoundContext;
use rg3d::{
    core::pool::{Handle, Ticket},
    sound::source::SoundSource,
};

#[derive(Debug)]
pub struct AddSoundSourceCommand {
    ticket: Option<Ticket<SoundSource>>,
    handle: Handle<SoundSource>,
    source: Option<SoundSource>,
    cached_name: String,
}

impl AddSoundSourceCommand {
    pub fn new(source: SoundSource) -> Self {
        Self {
            ticket: None,
            handle: Default::default(),
            cached_name: format!("Add Node {}", source.name()),
            source: Some(source),
        }
    }
}

impl<'a> Command<'a> for AddSoundSourceCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        self.cached_name.clone()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        match self.ticket.take() {
            None => {
                self.handle = context
                    .scene
                    .sound_context
                    .state()
                    .add_source(self.source.take().unwrap());
            }
            Some(ticket) => {
                let handle = context
                    .scene
                    .sound_context
                    .state()
                    .put_back(ticket, self.source.take().unwrap());
                assert_eq!(handle, self.handle);
            }
        }
    }

    fn revert(&mut self, context: &mut Self::Context) {
        let (ticket, source) = context
            .scene
            .sound_context
            .state()
            .take_reserve(self.handle);
        self.ticket = Some(ticket);
        self.source = Some(source);
    }

    fn finalize(&mut self, context: &mut Self::Context) {
        if let Some(ticket) = self.ticket.take() {
            context.scene.sound_context.state().forget_ticket(ticket)
        }
    }
}

#[derive(Debug)]
pub struct DeleteSoundSourceCommand {
    handle: Handle<SoundSource>,
    ticket: Option<Ticket<SoundSource>>,
    source: Option<SoundSource>,
}

impl DeleteSoundSourceCommand {
    pub fn new(handle: Handle<SoundSource>) -> Self {
        Self {
            handle,
            ticket: None,
            source: None,
        }
    }
}

impl<'a> Command<'a> for DeleteSoundSourceCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Delete Sound Source".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        let (ticket, source) = context
            .scene
            .sound_context
            .state()
            .take_reserve(self.handle);
        self.source = Some(source);
        self.ticket = Some(ticket);
    }

    fn revert(&mut self, context: &mut Self::Context) {
        self.handle = context
            .scene
            .sound_context
            .state()
            .put_back(self.ticket.take().unwrap(), self.source.take().unwrap());
    }

    fn finalize(&mut self, context: &mut Self::Context) {
        if let Some(ticket) = self.ticket.take() {
            context.scene.sound_context.state().forget_ticket(ticket)
        }
    }
}

#[derive(Debug)]
pub struct MoveSpatialSoundSourceCommand {
    source: Handle<SoundSource>,
    old_position: Vector3<f32>,
    new_position: Vector3<f32>,
}

impl MoveSpatialSoundSourceCommand {
    pub fn new(
        node: Handle<SoundSource>,
        old_position: Vector3<f32>,
        new_position: Vector3<f32>,
    ) -> Self {
        Self {
            source: node,
            old_position,
            new_position,
        }
    }

    fn swap(&mut self) -> Vector3<f32> {
        let position = self.new_position;
        std::mem::swap(&mut self.new_position, &mut self.old_position);
        position
    }

    fn set_position(&self, sound_context: &SoundContext, position: Vector3<f32>) {
        let mut state = sound_context.state();
        if let SoundSource::Spatial(spatial) = state.source_mut(self.source) {
            spatial.set_position(position);
        }
    }
}

impl<'a> Command<'a> for MoveSpatialSoundSourceCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Move Spatial Sound Source".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        let position = self.swap();
        self.set_position(&context.scene.sound_context, position);
    }

    fn revert(&mut self, context: &mut Self::Context) {
        let position = self.swap();
        self.set_position(&context.scene.sound_context, position);
    }
}

macro_rules! define_sound_source_command {
    ($name:ident($human_readable_name:expr, $value_type:ty) where fn swap($self:ident, $source:ident) $apply_method:block ) => {
        #[derive(Debug)]
        pub struct $name {
            handle: Handle<SoundSource>,
            value: $value_type,
        }

        impl $name {
            pub fn new(handle: Handle<SoundSource>, value: $value_type) -> Self {
                Self { handle, value }
            }

            fn swap(&mut $self, sound_context: &SoundContext) {
                let mut state = sound_context.state();
                let $source = state.source_mut($self.handle);
                $apply_method
            }
        }

        impl<'a> Command<'a> for $name {
            type Context = SceneContext<'a>;

            fn name(&mut self, _context: &Self::Context) -> String {
                $human_readable_name.to_owned()
            }

            fn execute(&mut self, context: &mut Self::Context) {
                self.swap(&context.scene.sound_context);
            }

            fn revert(&mut self, context: &mut Self::Context) {
                self.swap(&context.scene.sound_context);
            }
        }
    };
}

macro_rules! define_spatial_sound_source_command {
    ($name:ident($human_readable_name:expr, $value_type:ty) where fn swap($self:ident, $source:ident) $apply_method:block ) => {
        #[derive(Debug)]
        pub struct $name {
            handle: Handle<SoundSource>,
            value: $value_type,
        }

        impl $name {
            pub fn new(handle: Handle<SoundSource>, value: $value_type) -> Self {
                Self { handle, value }
            }

            fn swap(&mut $self, sound_context: &SoundContext) {
                let mut state = sound_context.state();
                if let SoundSource::Spatial($source) = state.source_mut($self.handle) {
                    $apply_method
                } else {
                    unreachable!();
                }
            }
        }

        impl<'a> Command<'a> for $name {
            type Context = SceneContext<'a>;

            fn name(&mut self, _context: &Self::Context) -> String {
                $human_readable_name.to_owned()
            }

            fn execute(&mut self, context: &mut Self::Context) {
                self.swap(&context.scene.sound_context);
            }

            fn revert(&mut self, context: &mut Self::Context) {
                self.swap(&context.scene.sound_context);
            }
        }
    };
}

define_sound_source_command!(SetSoundSourceGainCommand("Set Sound Source Gain", f32) where fn swap(self, source) {
    get_set_swap!(self, source, gain, set_gain);
});

define_sound_source_command!(SetSoundSourceBufferCommand("Set Sound Source Buffer", Option<SoundBufferResource>) where fn swap(self, source) {
    get_set_swap!(self, source, buffer, set_buffer);
});

define_sound_source_command!(SetSoundSourceNameCommand("Set Sound Source Name", String) where fn swap(self, source) {
    get_set_swap!(self, source, name_owned, set_name);
});

define_sound_source_command!(SetSoundSourcePitchCommand("Set Sound Source Pitch", f64) where fn swap(self, source) {
    get_set_swap!(self, source, pitch, set_pitch);
});

define_sound_source_command!(SetSoundSourceLoopingCommand("Set Sound Source Looping", bool) where fn swap(self, source) {
    get_set_swap!(self, source, is_looping, set_looping);
});

define_sound_source_command!(SetSoundSourcePlayOnceCommand("Set Sound Source Play Once", bool) where fn swap(self, source) {
    get_set_swap!(self, source, is_play_once, set_play_once);
});

define_spatial_sound_source_command!(SetSpatialSoundSourcePositionCommand("Set Spatial Sound Source Position", Vector3<f32>) where fn swap(self, source) {
    get_set_swap!(self, source, position, set_position);
});

define_spatial_sound_source_command!(SetSpatialSoundSourceRadiusCommand("Set Spatial Sound Source Radius", f32) where fn swap(self, source) {
    get_set_swap!(self, source, radius, set_radius);
});

define_spatial_sound_source_command!(SetSpatialSoundSourceRolloffFactorCommand("Set Spatial Sound Source Rolloff Factor", f32) where fn swap(self, source) {
    get_set_swap!(self, source, rolloff_factor, set_rolloff_factor);
});

define_spatial_sound_source_command!(SetSpatialSoundSourceMaxDistanceCommand("Set Spatial Sound Source Max Distance", f32) where fn swap(self, source) {
    get_set_swap!(self, source, max_distance, set_max_distance);
});
