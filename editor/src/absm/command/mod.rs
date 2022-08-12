use crate::{absm::SelectedEntity, define_command_stack};
use fyrox::{
    animation::machine::{
        node::PoseNodeDefinition, parameter::ParameterDefinition, state::StateDefinition,
        transition::TransitionDefinition, MachineDefinition, MachineInstantiationError, Parameter,
    },
    asset::ResourceDataRef,
    core::{
        algebra::Vector2,
        pool::{Handle, Ticket},
    },
    resource::absm::AbsmResourceState,
};
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

pub mod pose;
pub mod state;
pub mod transition;

#[derive(Debug)]
pub struct AbsmEditorContext<'a> {
    pub selection: &'a mut Vec<SelectedEntity>,
    pub resource: ResourceDataRef<'a, AbsmResourceState, MachineInstantiationError>,
}

pub mod blend;

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

macro_rules! define_spawn_command {
    ($name:ident, $ent_type:ty, $container:ident) => {
        #[derive(Debug)]
        pub enum $name {
            Unknown,
            NonExecuted {
                state: $ent_type,
            },
            Executed {
                handle: Handle<$ent_type>,
            },
            Reverted {
                ticket: Ticket<$ent_type>,
                state: $ent_type,
            },
        }

        impl $name {
            pub fn new(state: $ent_type) -> Self {
                Self::NonExecuted { state }
            }
        }

        impl AbsmCommandTrait for $name {
            fn name(&mut self, _context: &AbsmEditorContext) -> String {
                "Add State".to_string()
            }

            fn execute(&mut self, context: &mut AbsmEditorContext) {
                match std::mem::replace(self, $name::Unknown) {
                    $name::NonExecuted { state } => {
                        *self = $name::Executed {
                            handle: context.resource.absm_definition.$container.spawn(state),
                        };
                    }
                    $name::Reverted { ticket, state } => {
                        *self = $name::Executed {
                            handle: context
                                .resource
                                .absm_definition
                                .$container
                                .put_back(ticket, state),
                        }
                    }
                    _ => unreachable!(),
                }
            }

            fn revert(&mut self, context: &mut AbsmEditorContext) {
                match std::mem::replace(self, $name::Unknown) {
                    $name::Executed { handle } => {
                        let (ticket, state) = context
                            .resource
                            .absm_definition
                            .$container
                            .take_reserve(handle);
                        *self = $name::Reverted { ticket, state }
                    }
                    _ => unreachable!(),
                }
            }

            fn finalize(&mut self, context: &mut AbsmEditorContext) {
                if let $name::Reverted { ticket, .. } = std::mem::replace(self, $name::Unknown) {
                    context
                        .resource
                        .absm_definition
                        .$container
                        .forget_ticket(ticket)
                }
            }
        }
    };
}

define_spawn_command!(AddTransitionCommand, TransitionDefinition, transitions);

#[derive(Debug)]
pub enum AddStateCommand {
    Unknown,
    NonExecuted {
        state: StateDefinition,
    },
    Executed {
        handle: Handle<StateDefinition>,
        prev_entry_state: Handle<StateDefinition>,
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
                let handle = context.resource.absm_definition.states.spawn(state);

                let prev_entry_state = context.resource.absm_definition.entry_state;

                // Set entry state if it wasn't set yet.
                if context.resource.absm_definition.entry_state.is_none() {
                    context.resource.absm_definition.entry_state = handle;
                }

                *self = AddStateCommand::Executed {
                    handle,
                    prev_entry_state,
                };
            }
            AddStateCommand::Reverted { ticket, state } => {
                let handle = context
                    .resource
                    .absm_definition
                    .states
                    .put_back(ticket, state);

                let prev_entry_state = context.resource.absm_definition.entry_state;

                // Set entry state if it wasn't set yet.
                if context.resource.absm_definition.entry_state.is_none() {
                    context.resource.absm_definition.entry_state = handle;
                }

                *self = AddStateCommand::Executed {
                    handle,
                    prev_entry_state,
                }
            }
            _ => unreachable!(),
        }
    }

    fn revert(&mut self, context: &mut AbsmEditorContext) {
        match std::mem::replace(self, AddStateCommand::Unknown) {
            AddStateCommand::Executed {
                handle,
                prev_entry_state,
            } => {
                context.resource.absm_definition.entry_state = prev_entry_state;

                let (ticket, state) = context.resource.absm_definition.states.take_reserve(handle);
                *self = AddStateCommand::Reverted { ticket, state }
            }
            _ => unreachable!(),
        }
    }

    fn finalize(&mut self, context: &mut AbsmEditorContext) {
        if let AddStateCommand::Reverted { ticket, .. } =
            std::mem::replace(self, AddStateCommand::Unknown)
        {
            context
                .resource
                .absm_definition
                .states
                .forget_ticket(ticket)
        }
    }
}

#[derive(Debug)]
pub enum AddPoseNodeCommand {
    Unknown,
    NonExecuted {
        node: PoseNodeDefinition,
    },
    Executed {
        handle: Handle<PoseNodeDefinition>,
        prev_root_node: Handle<PoseNodeDefinition>,
    },
    Reverted {
        ticket: Ticket<PoseNodeDefinition>,
        node: PoseNodeDefinition,
    },
}

impl AddPoseNodeCommand {
    pub fn new(node: PoseNodeDefinition) -> Self {
        Self::NonExecuted { node }
    }
}

impl AbsmCommandTrait for AddPoseNodeCommand {
    fn name(&mut self, _context: &AbsmEditorContext) -> String {
        "Add Pose Node".to_string()
    }

    fn execute(&mut self, context: &mut AbsmEditorContext) {
        match std::mem::replace(self, AddPoseNodeCommand::Unknown) {
            AddPoseNodeCommand::NonExecuted { node } => {
                let parent_state = node.parent_state;

                let handle = context.resource.absm_definition.nodes.spawn(node);

                let parent_state_ref = &mut context.resource.absm_definition.states[parent_state];
                let prev_root_node = parent_state_ref.root;
                if parent_state_ref.root.is_none() {
                    parent_state_ref.root = handle;
                }

                *self = AddPoseNodeCommand::Executed {
                    handle,
                    prev_root_node,
                };
            }
            AddPoseNodeCommand::Reverted { ticket, node } => {
                let parent_state = node.parent_state;

                let handle = context
                    .resource
                    .absm_definition
                    .nodes
                    .put_back(ticket, node);

                let parent_state_ref = &mut context.resource.absm_definition.states[parent_state];
                let prev_root_node = parent_state_ref.root;
                if parent_state_ref.root.is_none() {
                    parent_state_ref.root = handle;
                }

                *self = AddPoseNodeCommand::Executed {
                    handle,
                    prev_root_node,
                }
            }
            _ => unreachable!(),
        }
    }

    fn revert(&mut self, context: &mut AbsmEditorContext) {
        match std::mem::replace(self, AddPoseNodeCommand::Unknown) {
            AddPoseNodeCommand::Executed {
                handle,
                prev_root_node,
            } => {
                let (ticket, node) = context.resource.absm_definition.nodes.take_reserve(handle);

                context.resource.absm_definition.states[node.parent_state].root = prev_root_node;

                *self = AddPoseNodeCommand::Reverted { ticket, node }
            }
            _ => unreachable!(),
        }
    }

    fn finalize(&mut self, context: &mut AbsmEditorContext) {
        if let AddPoseNodeCommand::Reverted { ticket, .. } =
            std::mem::replace(self, AddPoseNodeCommand::Unknown)
        {
            context.resource.absm_definition.nodes.forget_ticket(ticket)
        }
    }
}

macro_rules! define_move_command {
    ($name:ident, $ent_type:ty, $container:ident) => {
        #[derive(Debug)]
        pub struct $name {
            node: Handle<$ent_type>,
            old_position: Vector2<f32>,
            new_position: Vector2<f32>,
        }

        impl $name {
            pub fn new(
                node: Handle<$ent_type>,
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
                definition.$container[self.node].position = position;
            }
        }

        impl AbsmCommandTrait for $name {
            fn name(&mut self, _context: &AbsmEditorContext) -> String {
                "Move Node".to_owned()
            }

            fn execute(&mut self, context: &mut AbsmEditorContext) {
                let position = self.swap();
                self.set_position(&mut context.resource.absm_definition, position);
            }

            fn revert(&mut self, context: &mut AbsmEditorContext) {
                let position = self.swap();
                self.set_position(&mut context.resource.absm_definition, position);
            }
        }
    };
}

define_move_command!(MoveStateNodeCommand, StateDefinition, states);
define_move_command!(MovePoseNodeCommand, PoseNodeDefinition, nodes);

#[derive(Debug)]
pub struct ChangeSelectionCommand {
    pub selection: Vec<SelectedEntity>,
}

impl ChangeSelectionCommand {
    fn swap(&mut self, context: &mut AbsmEditorContext) {
        std::mem::swap(&mut self.selection, context.selection);
    }
}

impl AbsmCommandTrait for ChangeSelectionCommand {
    fn name(&mut self, _: &AbsmEditorContext) -> String {
        "Change Selection".to_string()
    }

    fn execute(&mut self, context: &mut AbsmEditorContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut AbsmEditorContext) {
        self.swap(context)
    }
}

macro_rules! define_free_command {
    ($name:ident, $ent_type:ty, $container:ident) => {
        #[derive(Debug)]
        pub enum $name {
            Unknown,
            NonExecuted(Handle<$ent_type>),
            Executed {
                state: $ent_type,
                ticket: Ticket<$ent_type>,
            },
            Reverted(Handle<$ent_type>),
        }

        impl $name {
            pub fn new(state: Handle<$ent_type>) -> Self {
                Self::NonExecuted(state)
            }
        }

        impl AbsmCommandTrait for $name {
            fn name(&mut self, _context: &AbsmEditorContext) -> String {
                "Delete State".to_owned()
            }

            fn execute(&mut self, context: &mut AbsmEditorContext) {
                match std::mem::replace(self, Self::Unknown) {
                    Self::NonExecuted(state) | Self::Reverted(state) => {
                        let (ticket, state) = context
                            .resource
                            .absm_definition
                            .$container
                            .take_reserve(state);
                        *self = Self::Executed { state, ticket }
                    }
                    _ => unreachable!(),
                }
            }

            fn revert(&mut self, context: &mut AbsmEditorContext) {
                match std::mem::replace(self, Self::Unknown) {
                    Self::Executed { state, ticket } => {
                        *self = Self::Reverted(
                            context
                                .resource
                                .absm_definition
                                .$container
                                .put_back(ticket, state),
                        );
                    }
                    _ => unreachable!(),
                }
            }

            fn finalize(&mut self, context: &mut AbsmEditorContext) {
                match std::mem::replace(self, Self::Unknown) {
                    Self::Executed { ticket, .. } => {
                        context
                            .resource
                            .absm_definition
                            .$container
                            .forget_ticket(ticket);
                    }
                    _ => (),
                }
            }
        }
    };
}

define_free_command!(DeleteStateCommand, StateDefinition, states);
define_free_command!(DeletePoseNodeCommand, PoseNodeDefinition, nodes);
define_free_command!(DeleteTransitionCommand, TransitionDefinition, transitions);

#[macro_export]
macro_rules! define_push_element_to_collection_command {
    ($name:ident<$model_handle:ty, $value_type:ty>($self:ident, $context:ident) $get_collection:block) => {
        #[derive(Debug)]
        pub struct $name {
            pub handle: $model_handle,
            pub value: Option<$value_type>,
        }

        impl $name {
            pub fn new(handle: $model_handle, value: $value_type) -> Self {
                Self {
                    handle,
                    value: Some(value)
                }
            }
        }

        impl AbsmCommandTrait for $name {
            fn name(&mut self, _context: &AbsmEditorContext) -> String {
                "Push Element To Collection".to_string()
            }

            fn execute(&mut $self, $context: &mut AbsmEditorContext) {
                let collection = $get_collection;
                collection.push($self.value.take().unwrap());
            }

            fn revert(&mut $self, $context: &mut AbsmEditorContext) {
                let collection = $get_collection;
                $self.value = Some(collection.pop().unwrap());
            }
        }
    };
}

#[macro_export]
macro_rules! define_remove_collection_element_command {
    ($name:ident<$model_handle:ty, $value_type:ty>($self:ident, $context:ident) $get_collection:block) => {
        #[derive(Debug)]
        #[allow(dead_code)]
        pub struct $name {
            handle: $model_handle,
            index: usize,
            value: Option<$value_type>,
        }

        impl $name {
            pub fn new(handle: $model_handle, index: usize) -> Self {
                Self {
                    handle,
                    value: None,
                    index
                }
            }
        }

        impl AbsmCommandTrait for $name {
            fn name(&mut self, _context: &AbsmEditorContext) -> String {
                "Remove Collection Element".to_string()
            }

            fn execute(&mut $self, $context: &mut AbsmEditorContext) {
                let collection = $get_collection;
                $self.value = Some(collection.remove($self.index));
            }

            fn revert(&mut $self, $context: &mut AbsmEditorContext) {
                let collection = $get_collection;
                collection.insert($self.index, $self.value.take().unwrap())
            }
        }
    };
}

#[macro_export]
macro_rules! define_set_collection_element_command {
    ($name:ident<$model_handle:ty, $value_type:ty>($self:ident, $context:ident) $get_value:block) => {
        #[derive(Debug)]
        pub struct $name {
            pub handle: $model_handle,
            pub index: usize,
            pub value: $value_type,
        }

        impl $name {
            pub fn swap(&mut $self, $context: &mut AbsmEditorContext) {
                let value = $get_value;
                std::mem::swap(value, &mut $self.value);
            }
        }

        impl AbsmCommandTrait for $name {
            fn name(&mut self,
                #[allow(unused_variables)]
                $context: &AbsmEditorContext
            ) -> String {
                "Set Collection Element".to_owned()
            }

            fn execute(&mut self, $context: &mut AbsmEditorContext) {
                self.swap($context);
            }

            fn revert(&mut self, $context: &mut AbsmEditorContext) {
                self.swap($context);
            }
        }
    };
}

#[derive(Debug)]
pub struct SetMachineEntryStateCommand {
    pub entry: Handle<StateDefinition>,
}

impl SetMachineEntryStateCommand {
    fn swap(&mut self, context: &mut AbsmEditorContext) {
        std::mem::swap(
            &mut context.resource.absm_definition.entry_state,
            &mut self.entry,
        );
    }
}

impl AbsmCommandTrait for SetMachineEntryStateCommand {
    fn name(&mut self, _context: &AbsmEditorContext) -> String {
        "Set Entry State".to_string()
    }

    fn execute(&mut self, context: &mut AbsmEditorContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut AbsmEditorContext) {
        self.swap(context)
    }
}

#[macro_export]
macro_rules! define_absm_swap_command {
    ($name:ident<$model_type:ty, $value_type:ty>[$($field_name:ident:$field_type:ty),*]($self:ident, $context:ident) $get_field:block) => {
        #[derive(Debug)]
        pub struct $name {
            pub handle: $model_type,
            pub value: $value_type,
            $(
                pub $field_name: $field_type,
            )*
        }

        impl $name {
            fn swap(&mut $self, $context: &mut AbsmEditorContext) {
                let field = $get_field;

                std::mem::swap(field, &mut $self.value);
            }
        }

        impl AbsmCommandTrait for $name {
            fn name(&mut self, _context: &AbsmEditorContext) -> String {
                stringify!($name).to_string()
            }

            fn execute(&mut self, context: &mut AbsmEditorContext) {
                self.swap(context)
            }

            fn revert(&mut self, context: &mut AbsmEditorContext) {
                self.swap(context)
            }
        }
    };
}

define_absm_swap_command!(SetStateRootPoseCommand<Handle<StateDefinition>, Handle<PoseNodeDefinition>>[](self, context) {
    &mut context.resource.absm_definition.states[self.handle].root
});

define_push_element_to_collection_command!(AddParameterCommand<(), ParameterDefinition>(self, context) {
   &mut context.resource.absm_definition.parameters.container
});

define_remove_collection_element_command!(RemoveParameterCommand<(), ParameterDefinition>(self, context) {
    &mut context.resource.absm_definition.parameters.container
});

define_absm_swap_command!(SetParameterNameCommand<usize, String>[](self, context) {
    &mut context.resource.absm_definition.parameters.container[self.handle].name
});

define_absm_swap_command!(SetParameterValueCommand<usize, Parameter>[](self, context) {
    &mut context.resource.absm_definition.parameters.container[self.handle].value
});

define_absm_swap_command!(SetParameterWeightValueCommand<usize, f32>[](self, context) {
    if let Parameter::Weight(ref mut weight) = context.resource.absm_definition.parameters.container[self.handle].value {
        weight
    } else {
        unreachable!()
    }
});

define_absm_swap_command!(SetParameterRuleValueCommand<usize, bool>[](self, context) {
    if let Parameter::Rule(ref mut rule) = context.resource.absm_definition.parameters.container[self.handle].value {
        rule
    } else {
        unreachable!()
    }
});

define_absm_swap_command!(SetParameterIndexValueCommand<usize, u32>[](self, context) {
    if let Parameter::Index(ref mut index) = context.resource.absm_definition.parameters.container[self.handle].value {
        index
    } else {
        unreachable!()
    }
});
