use crate::{command::Command, scene::commands::SceneContext};
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
