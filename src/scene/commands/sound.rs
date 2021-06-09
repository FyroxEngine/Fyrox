use crate::command::Command;
use crate::scene::commands::SceneContext;
use rg3d::core::pool::{Handle, Ticket};
use rg3d::sound::source::SoundSource;

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
        let (ticket, node) = context
            .scene
            .sound_context
            .state()
            .take_reserve(self.handle);
        self.ticket = Some(ticket);
        self.source = Some(node);
    }

    fn finalize(&mut self, context: &mut Self::Context) {
        if let Some(ticket) = self.ticket.take() {
            context.scene.sound_context.state().forget_ticket(ticket)
        }
    }
}
