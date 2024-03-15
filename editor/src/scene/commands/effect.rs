use crate::fyrox::{
    core::pool::{Handle, Ticket},
    scene::sound::AudioBus,
};
use crate::{command::CommandContext, CommandTrait, GameSceneContext};

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

impl CommandTrait for AddAudioBusCommand {
    fn name(&mut self, _: &dyn CommandContext) -> String {
        "Add Effect".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let mut state = context.scene.graph.sound_context.state();
        let parent = state.bus_graph_ref().primary_bus_handle();
        self.handle = state
            .bus_graph_mut()
            .add_bus(self.bus.take().unwrap(), parent);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
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

    fn finalize(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
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

impl CommandTrait for RemoveAudioBusCommand {
    fn name(&mut self, _: &dyn CommandContext) -> String {
        "Remove Effect".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
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

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let mut state = context.scene.graph.sound_context.state();
        let parent = state.bus_graph_ref().primary_bus_handle();
        self.handle = state
            .bus_graph_mut()
            .add_bus(self.bus.take().unwrap(), parent);
    }

    fn finalize(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
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
pub struct LinkAudioBuses {
    pub child: Handle<AudioBus>,
    pub parent: Handle<AudioBus>,
}

impl LinkAudioBuses {
    fn swap(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let mut state = context.scene.graph.sound_context.state();
        let graph = state.bus_graph_mut();
        let old_parent = graph.try_get_bus_ref(self.child).unwrap().parent();
        graph.link_buses(self.child, self.parent);
        self.parent = old_parent;
    }
}

impl CommandTrait for LinkAudioBuses {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Link Audio Buses".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }
}
