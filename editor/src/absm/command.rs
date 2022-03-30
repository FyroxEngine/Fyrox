use crate::define_command_stack;
use fyrox::{
    animation::machine::{
        state::StateDefinition, transition::TransitionDefinition, MachineDefinition,
    },
    core::{
        algebra::Vector2,
        pool::{Handle, Ticket},
    },
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
pub struct CommandGroup {
    commands: Vec<AbsmCommand>,
}

impl From<Vec<AbsmCommand>> for CommandGroup {
    fn from(commands: Vec<AbsmCommand>) -> Self {
        Self { commands }
    }
}

impl CommandGroup {
    #[allow(dead_code)]
    pub fn push(&mut self, command: AbsmCommand) {
        self.commands.push(command)
    }
}

impl AbsmCommandTrait for CommandGroup {
    fn name(&mut self, context: &AbsmEditorContext) -> String {
        let mut name = String::from("Command group: ");
        for cmd in self.commands.iter_mut() {
            name.push_str(&cmd.name(context));
            name.push_str(", ");
        }
        name
    }

    fn execute(&mut self, context: &mut AbsmEditorContext) {
        for cmd in self.commands.iter_mut() {
            cmd.execute(context);
        }
    }

    fn revert(&mut self, context: &mut AbsmEditorContext) {
        // revert must be done in reverse order.
        for cmd in self.commands.iter_mut().rev() {
            cmd.revert(context);
        }
    }

    fn finalize(&mut self, context: &mut AbsmEditorContext) {
        for mut cmd in self.commands.drain(..) {
            cmd.finalize(context);
        }
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
        if let AddStateCommand::Reverted { ticket, .. } =
            std::mem::replace(self, AddStateCommand::Unknown)
        {
            context.definition.states.forget_ticket(ticket)
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
        if let AddTransitionCommand::Reverted { ticket, .. } =
            std::mem::replace(self, AddTransitionCommand::Unknown)
        {
            context.definition.transitions.forget_ticket(ticket)
        }
    }
}

#[derive(Debug)]
pub struct MoveStateNodeCommand {
    node: Handle<StateDefinition>,
    old_position: Vector2<f32>,
    new_position: Vector2<f32>,
}

impl MoveStateNodeCommand {
    pub fn new(
        node: Handle<StateDefinition>,
        old_position: Vector2<f32>,
        new_position: Vector2<f32>,
    ) -> Self {
        Self {
            node,
            old_position,
            new_position,
        }
    }

    fn swap(&mut self) -> Vector2<f32> {
        let position = self.new_position;
        std::mem::swap(&mut self.new_position, &mut self.old_position);
        position
    }

    fn set_position(&self, definition: &mut MachineDefinition, position: Vector2<f32>) {
        definition.states[self.node].position = position;
    }
}

impl AbsmCommandTrait for MoveStateNodeCommand {
    fn name(&mut self, _context: &AbsmEditorContext) -> String {
        "Move State Node".to_owned()
    }

    fn execute(&mut self, context: &mut AbsmEditorContext) {
        let position = self.swap();
        self.set_position(context.definition, position);
    }

    fn revert(&mut self, context: &mut AbsmEditorContext) {
        let position = self.swap();
        self.set_position(context.definition, position);
    }
}
