use crate::define_command_stack;
use fyrox::animation::machine::transition::TransitionDefinition;
use fyrox::{
    animation::machine::{state::StateDefinition, MachineDefinition},
    core::pool::{Handle, Ticket},
};
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

#[derive(Debug)]
pub struct AbsmEditorContext<'a> {
    pub definition: &'a mut MachineDefinition,
}

define_command_stack!(AbsmCommandTrait, AbsmCommandStack, AbsmEditorContext);

#[derive(Debug)]
pub struct AbsmCommand(pub Box<dyn AbsmCommandTrait>);

impl Deref for AbsmCommand {
    type Target = dyn AbsmCommandTrait;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for AbsmCommand {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

impl AbsmCommand {
    pub fn new<C: AbsmCommandTrait>(cmd: C) -> Self {
        Self(Box::new(cmd))
    }

    pub fn into_inner(self) -> Box<dyn AbsmCommandTrait> {
        self.0
    }
}

#[derive(Debug)]
pub enum AddStateCommand {
    Unknown,
    NonExecuted {
        state: StateDefinition,
    },
    Executed {
        handle: Handle<StateDefinition>,
    },
    Reverted {
        ticket: Ticket<StateDefinition>,
        state: StateDefinition,
    },
}

impl AddStateCommand {
    pub fn new(state: StateDefinition) -> Self {
        Self::NonExecuted { state }
    }
}

impl AbsmCommandTrait for AddStateCommand {
    fn name(&mut self, _context: &AbsmEditorContext) -> String {
        "Add State".to_string()
    }

    fn execute(&mut self, context: &mut AbsmEditorContext) {
        match std::mem::replace(self, AddStateCommand::Unknown) {
            AddStateCommand::NonExecuted { state } => {
                *self = AddStateCommand::Executed {
                    handle: context.definition.states.spawn(state),
                };
            }
            AddStateCommand::Reverted { ticket, state } => {
                *self = AddStateCommand::Executed {
                    handle: context.definition.states.put_back(ticket, state),
                }
            }
            _ => unreachable!(),
        }
    }

    fn revert(&mut self, context: &mut AbsmEditorContext) {
        match std::mem::replace(self, AddStateCommand::Unknown) {
            AddStateCommand::Executed { handle } => {
                let (ticket, state) = context.definition.states.take_reserve(handle);
                *self = AddStateCommand::Reverted { ticket, state }
            }
            _ => unreachable!(),
        }
    }

    fn finalize(&mut self, context: &mut AbsmEditorContext) {
        match std::mem::replace(self, AddStateCommand::Unknown) {
            AddStateCommand::Reverted { ticket, .. } => {
                context.definition.states.forget_ticket(ticket)
            }
            _ => (),
        }
    }
}

#[derive(Debug)]
pub enum AddTransitionCommand {
    Unknown,
    NonExecuted {
        state: TransitionDefinition,
    },
    Executed {
        handle: Handle<TransitionDefinition>,
    },
    Reverted {
        ticket: Ticket<TransitionDefinition>,
        state: TransitionDefinition,
    },
}

impl AddTransitionCommand {
    pub fn new(state: TransitionDefinition) -> Self {
        Self::NonExecuted { state }
    }
}

impl AbsmCommandTrait for AddTransitionCommand {
    fn name(&mut self, _context: &AbsmEditorContext) -> String {
        "Add State".to_string()
    }

    fn execute(&mut self, context: &mut AbsmEditorContext) {
        match std::mem::replace(self, AddTransitionCommand::Unknown) {
            AddTransitionCommand::NonExecuted { state } => {
                *self = AddTransitionCommand::Executed {
                    handle: context.definition.transitions.spawn(state),
                };
            }
            AddTransitionCommand::Reverted { ticket, state } => {
                *self = AddTransitionCommand::Executed {
                    handle: context.definition.transitions.put_back(ticket, state),
                }
            }
            _ => unreachable!(),
        }
    }

    fn revert(&mut self, context: &mut AbsmEditorContext) {
        match std::mem::replace(self, AddTransitionCommand::Unknown) {
            AddTransitionCommand::Executed { handle } => {
                let (ticket, state) = context.definition.transitions.take_reserve(handle);
                *self = AddTransitionCommand::Reverted { ticket, state }
            }
            _ => unreachable!(),
        }
    }

    fn finalize(&mut self, context: &mut AbsmEditorContext) {
        match std::mem::replace(self, AddTransitionCommand::Unknown) {
            AddTransitionCommand::Reverted { ticket, .. } => {
                context.definition.transitions.forget_ticket(ticket)
            }
            _ => (),
        }
    }
}
