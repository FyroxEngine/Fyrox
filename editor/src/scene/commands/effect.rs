use crate::{define_universal_commands, scene::commands::SceneCommand, Command, SceneContext};
use fyrox::{
    core::{
        pool::{Handle, Ticket},
        reflect::{prelude::*, ResolvePath},
    },
    scene::sound::AudioBus,
};

define_universal_commands!(
    make_set_audio_bus_property_command,
    Command,
    SceneCommand,
    SceneContext,
    Handle<AudioBus>,
    ctx,
    handle,
    self,
    {
        ctx.scene
            .graph
            .sound_context
            .state()
            .bus_graph_mut()
            .try_get_bus_mut(self.handle)
            .unwrap()
    },
);

#[derive(Debug)]
pub struct AddAudioBusCommand {
    bus: Option<AudioBus>,
    handle: Handle<AudioBus>,
    ticket: Option<Ticket<AudioBus>>,
}

impl AddAudioBusCommand {
    pub fn new(bus: AudioBus) -> Self {
        Self {
            bus: Some(bus),
            handle: Default::default(),
            ticket: None,
        }
    }
}

impl Command for AddAudioBusCommand {
    fn name(&mut self, _: &SceneContext) -> String {
        "Add Effect".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let mut state = context.scene.graph.sound_context.state();
        let parent = state.bus_graph_ref().primary_bus_handle();
        self.handle = state
            .bus_graph_mut()
            .add_bus(self.bus.take().unwrap(), parent);
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let (ticket, effect) = context
            .scene
            .graph
            .sound_context
            .state()
            .bus_graph_mut()
            .try_take_reserve_bus(self.handle)
            .unwrap();
        self.bus = Some(effect);
        self.ticket = Some(ticket);
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        if let Some(ticket) = self.ticket.take() {
            context
                .scene
                .graph
                .sound_context
                .state()
                .bus_graph_mut()
                .forget_bus_ticket(ticket);
        }
    }
}

#[derive(Debug)]
pub struct RemoveAudioBusCommand {
    bus: Option<AudioBus>,
    handle: Handle<AudioBus>,
    ticket: Option<Ticket<AudioBus>>,
}

impl RemoveAudioBusCommand {
    pub fn new(handle: Handle<AudioBus>) -> Self {
        Self {
            bus: None,
            handle,
            ticket: None,
        }
    }
}

impl Command for RemoveAudioBusCommand {
    fn name(&mut self, _: &SceneContext) -> String {
        "Remove Effect".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let (ticket, effect) = context
            .scene
            .graph
            .sound_context
            .state()
            .bus_graph_mut()
            .try_take_reserve_bus(self.handle)
            .unwrap();
        self.bus = Some(effect);
        self.ticket = Some(ticket);
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let mut state = context.scene.graph.sound_context.state();
        let parent = state.bus_graph_ref().primary_bus_handle();
        self.handle = state
            .bus_graph_mut()
            .add_bus(self.bus.take().unwrap(), parent);
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        if let Some(ticket) = self.ticket.take() {
            context
                .scene
                .graph
                .sound_context
                .state()
                .bus_graph_mut()
                .forget_bus_ticket(ticket);
        }
    }
}
