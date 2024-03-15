use crate::fyrox::core::pool::ErasedHandle;
use crate::fyrox::core::variable::InheritableVariable;
use crate::fyrox::graph::{BaseSceneGraph, SceneGraphNode};
use crate::fyrox::{
    core::{
        algebra::Vector2,
        pool::{Handle, Ticket},
    },
    generic_animation::machine::{
        layer::MachineLayer, mask::LayerMask, Machine, PoseNode, State, Transition,
    },
};
use crate::{
    command::{CommandContext, CommandTrait},
    scene::commands::GameSceneContext,
    ui_scene::commands::UiSceneContext,
};
use std::fmt::Debug;

pub mod blend;

macro_rules! define_spawn_command {
    ($name:ident, $ent_type:ty, $container:ident) => {
        #[derive(Debug)]
        pub enum $name<N: Debug + 'static> {
            Unknown,
            NonExecuted {
                node_handle: Handle<N>,
                layer_index: usize,
                state: $ent_type,
            },
            Executed {
                node_handle: Handle<N>,
                layer_index: usize,
                handle: Handle<$ent_type>,
            },
            Reverted {
                node_handle: Handle<N>,
                layer_index: usize,
                ticket: Ticket<$ent_type>,
                state: $ent_type,
            },
        }

        impl<N: Debug + 'static> $name<N> {
            pub fn new(node_handle: Handle<N>, layer_index: usize, state: $ent_type) -> Self {
                Self::NonExecuted {
                    node_handle,
                    layer_index,
                    state,
                }
            }
        }

        impl<N: Debug + 'static> CommandTrait for $name<N> {
            fn name(&mut self, _context: &dyn CommandContext) -> String {
                "Add State".to_string()
            }

            fn execute(&mut self, context: &mut dyn CommandContext) {
                match std::mem::replace(self, $name::Unknown) {
                    $name::NonExecuted {
                        node_handle,
                        layer_index,
                        state,
                    } => {
                        let machine = fetch_machine(context, node_handle);
                        *self = $name::Executed {
                            node_handle,
                            layer_index,
                            handle: machine.layers_mut()[layer_index].$container().spawn(state),
                        };
                    }
                    $name::Reverted {
                        node_handle,
                        layer_index,
                        ticket,
                        state,
                    } => {
                        let machine = fetch_machine(context, node_handle);
                        *self = $name::Executed {
                            node_handle,
                            layer_index,
                            handle: machine.layers_mut()[layer_index]
                                .$container()
                                .put_back(ticket, state),
                        }
                    }
                    _ => unreachable!(),
                }
            }

            fn revert(&mut self, context: &mut dyn CommandContext) {
                match std::mem::replace(self, $name::Unknown) {
                    $name::Executed {
                        node_handle,
                        layer_index,
                        handle,
                    } => {
                        let machine = fetch_machine(context, node_handle);
                        let (ticket, state) = machine.layers_mut()[layer_index]
                            .$container()
                            .take_reserve(handle);
                        *self = $name::Reverted {
                            node_handle,
                            layer_index,
                            ticket,
                            state,
                        }
                    }
                    _ => unreachable!(),
                }
            }

            fn finalize(&mut self, context: &mut dyn CommandContext) {
                if let $name::Reverted {
                    node_handle,
                    layer_index,
                    ticket,
                    ..
                } = std::mem::replace(self, $name::Unknown)
                {
                    let machine = fetch_machine(context, node_handle);
                    machine.layers_mut()[layer_index]
                        .$container()
                        .forget_ticket(ticket)
                }
            }
        }
    };
}

define_spawn_command!(AddTransitionCommand, Transition<Handle<N>>, transitions_mut);

#[derive(Debug)]
pub enum AddStateCommand<N: Debug + 'static> {
    Unknown,
    NonExecuted {
        node_handle: Handle<N>,
        layer_index: usize,
        state: State<Handle<N>>,
    },
    Executed {
        node_handle: Handle<N>,
        layer_index: usize,
        handle: Handle<State<Handle<N>>>,
        prev_entry_state: Handle<State<Handle<N>>>,
    },
    Reverted {
        node_handle: Handle<N>,
        layer_index: usize,
        ticket: Ticket<State<Handle<N>>>,
        state: State<Handle<N>>,
    },
}

impl<N: Debug + 'static> AddStateCommand<N> {
    pub fn new(node_handle: Handle<N>, layer_index: usize, state: State<Handle<N>>) -> Self {
        Self::NonExecuted {
            node_handle,
            layer_index,
            state,
        }
    }
}

pub fn fetch_machine<N: Debug + 'static>(
    context: &mut dyn CommandContext,
    node_handle: Handle<N>,
) -> &mut Machine<Handle<N>> {
    // SAFETY: Borrow checker cannot resolve lifetime properly in the following `if` chain.
    // This is safe to do, because there's only one mutable reference anyway. Should be fixed
    // with Polonius.
    let context2 = unsafe { &mut *(context as *mut dyn CommandContext) };

    if let Some(game_scene) = context.component_mut::<GameSceneContext>() {
        game_scene
            .scene
            .graph
            .node_mut(ErasedHandle::from(node_handle).into())
            .component_mut::<InheritableVariable<Machine<Handle<N>>>>()
            .unwrap()
    } else if let Some(ui) = context2.component_mut::<UiSceneContext>() {
        ui.ui
            .node_mut(ErasedHandle::from(node_handle).into())
            .component_mut::<InheritableVariable<Machine<Handle<N>>>>()
            .unwrap()
    } else {
        panic!("Unsupported container!")
    }
}

impl<N: Debug + 'static> CommandTrait for AddStateCommand<N> {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Add State".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        match std::mem::replace(self, AddStateCommand::Unknown) {
            AddStateCommand::NonExecuted {
                node_handle,
                layer_index,
                state,
            } => {
                let machine = fetch_machine(context, node_handle);
                let layer = &mut machine.layers_mut()[layer_index];

                let handle = layer.add_state(state);

                let prev_entry_state = layer.entry_state();

                // Set entry state if it wasn't set yet.
                if layer.entry_state().is_none() {
                    layer.set_entry_state(handle);
                }

                *self = AddStateCommand::Executed {
                    node_handle,
                    layer_index,
                    handle,
                    prev_entry_state,
                };
            }
            AddStateCommand::Reverted {
                node_handle,
                layer_index,
                ticket,
                state,
            } => {
                let machine = fetch_machine(context, node_handle);
                let layer = &mut machine.layers_mut()[layer_index];

                let handle = layer.states_mut().put_back(ticket, state);

                let prev_entry_state = layer.entry_state();

                // Set entry state if it wasn't set yet.
                if layer.entry_state().is_none() {
                    layer.set_entry_state(handle);
                }

                *self = AddStateCommand::Executed {
                    node_handle,
                    layer_index,
                    handle,
                    prev_entry_state,
                }
            }
            _ => unreachable!(),
        }
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        match std::mem::replace(self, AddStateCommand::Unknown) {
            AddStateCommand::Executed {
                node_handle,
                layer_index,
                handle,
                prev_entry_state,
            } => {
                let machine = fetch_machine(context, node_handle);

                let layer = &mut machine.layers_mut()[layer_index];

                layer.set_entry_state(prev_entry_state);

                let (ticket, state) = layer.states_mut().take_reserve(handle);

                *self = AddStateCommand::Reverted {
                    node_handle,
                    layer_index,
                    ticket,
                    state,
                }
            }
            _ => unreachable!(),
        }
    }

    fn finalize(&mut self, context: &mut dyn CommandContext) {
        if let AddStateCommand::Reverted {
            node_handle,
            layer_index,
            ticket,
            ..
        } = std::mem::replace(self, AddStateCommand::Unknown)
        {
            let machine = fetch_machine(context, node_handle);
            machine.layers_mut()[layer_index]
                .states_mut()
                .forget_ticket(ticket)
        }
    }
}

#[derive(Debug)]
pub enum AddPoseNodeCommand<N: Debug + 'static> {
    Unknown,
    NonExecuted {
        node_handle: Handle<N>,
        layer_index: usize,
        node: PoseNode<Handle<N>>,
    },
    Executed {
        node_handle: Handle<N>,
        layer_index: usize,
        handle: Handle<PoseNode<Handle<N>>>,
        prev_root_node: Handle<PoseNode<Handle<N>>>,
    },
    Reverted {
        node_handle: Handle<N>,
        layer_index: usize,
        ticket: Ticket<PoseNode<Handle<N>>>,
        node: PoseNode<Handle<N>>,
    },
}

impl<N: Debug + 'static> AddPoseNodeCommand<N> {
    pub fn new(node_handle: Handle<N>, layer_index: usize, node: PoseNode<Handle<N>>) -> Self {
        Self::NonExecuted {
            node_handle,
            layer_index,
            node,
        }
    }
}

impl<N: Debug + 'static> CommandTrait for AddPoseNodeCommand<N> {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Add Pose Node".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        match std::mem::replace(self, AddPoseNodeCommand::Unknown) {
            AddPoseNodeCommand::NonExecuted {
                node_handle,
                layer_index,
                node,
            } => {
                let machine = fetch_machine(context, node_handle);
                let layer = &mut machine.layers_mut()[layer_index];

                let parent_state = node.parent_state;

                let handle = layer.add_node(node);

                let parent_state_ref = &mut layer.states_mut()[parent_state];
                let prev_root_node = parent_state_ref.root;
                if parent_state_ref.root.is_none() {
                    parent_state_ref.root = handle;
                }

                *self = AddPoseNodeCommand::Executed {
                    node_handle,
                    layer_index,
                    handle,
                    prev_root_node,
                };
            }
            AddPoseNodeCommand::Reverted {
                node_handle,
                layer_index,
                ticket,
                node,
            } => {
                let machine = fetch_machine(context, node_handle);
                let layer = &mut machine.layers_mut()[layer_index];
                let parent_state = node.parent_state;

                let handle = layer.nodes_mut().put_back(ticket, node);

                let parent_state_ref = &mut layer.states_mut()[parent_state];
                let prev_root_node = parent_state_ref.root;
                if parent_state_ref.root.is_none() {
                    parent_state_ref.root = handle;
                }

                *self = AddPoseNodeCommand::Executed {
                    node_handle,
                    layer_index,
                    handle,
                    prev_root_node,
                }
            }
            _ => unreachable!(),
        }
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        match std::mem::replace(self, AddPoseNodeCommand::Unknown) {
            AddPoseNodeCommand::Executed {
                node_handle,
                layer_index,
                handle,
                prev_root_node,
            } => {
                let machine = fetch_machine(context, node_handle);
                let layer = &mut machine.layers_mut()[layer_index];
                let (ticket, node) = layer.nodes_mut().take_reserve(handle);

                layer.states_mut()[node.parent_state].root = prev_root_node;

                *self = AddPoseNodeCommand::Reverted {
                    node_handle,
                    layer_index,
                    ticket,
                    node,
                }
            }
            _ => unreachable!(),
        }
    }

    fn finalize(&mut self, context: &mut dyn CommandContext) {
        if let AddPoseNodeCommand::Reverted {
            node_handle,
            layer_index,
            ticket,
            ..
        } = std::mem::replace(self, AddPoseNodeCommand::Unknown)
        {
            let machine = fetch_machine(context, node_handle);
            let layer = &mut machine.layers_mut()[layer_index];
            layer.nodes_mut().forget_ticket(ticket)
        }
    }
}

macro_rules! define_move_command {
    ($name:ident, $ent_type:ty, $container:ident) => {
        #[derive(Debug)]
        pub struct $name<N: Debug + 'static> {
            absm_node_handle: Handle<N>,
            layer_index: usize,
            node: Handle<$ent_type>,
            old_position: Vector2<f32>,
            new_position: Vector2<f32>,
        }

        impl<N: Debug + 'static> $name<N> {
            pub fn new(
                absm_node_handle: Handle<N>,
                node: Handle<$ent_type>,
                layer_index: usize,
                old_position: Vector2<f32>,
                new_position: Vector2<f32>,
            ) -> Self {
                Self {
                    absm_node_handle,
                    layer_index,
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

            fn set_position(&self, context: &mut dyn CommandContext, position: Vector2<f32>) {
                let machine = fetch_machine(context, self.absm_node_handle);
                machine.layers_mut()[self.layer_index].$container()[self.node].position = position;
            }
        }

        impl<N: Debug + 'static> CommandTrait for $name<N> {
            fn name(&mut self, _context: &dyn CommandContext) -> String {
                "Move Entity".to_owned()
            }

            fn execute(&mut self, context: &mut dyn CommandContext) {
                let position = self.swap();
                self.set_position(context, position);
            }

            fn revert(&mut self, context: &mut dyn CommandContext) {
                let position = self.swap();
                self.set_position(context, position);
            }
        }
    };
}

define_move_command!(MoveStateNodeCommand, State<Handle<N>>, states_mut);
define_move_command!(MovePoseNodeCommand, PoseNode<Handle<N>>, nodes_mut);

macro_rules! define_free_command {
    ($name:ident, $ent_type:ty, $container:ident) => {
        #[derive(Debug)]
        pub enum $name<N: Debug + 'static> {
            Unknown,
            NonExecuted {
                node_handle: Handle<N>,
                layer_index: usize,
                entity_handle: Handle<$ent_type>,
            },
            Executed {
                node_handle: Handle<N>,
                layer_index: usize,
                entity: $ent_type,
                ticket: Ticket<$ent_type>,
            },
            Reverted {
                node_handle: Handle<N>,
                layer_index: usize,
                entity_handle: Handle<$ent_type>,
            },
        }

        impl<N: Debug + 'static> $name<N> {
            pub fn new(
                node_handle: Handle<N>,
                layer_index: usize,
                entity_handle: Handle<$ent_type>,
            ) -> Self {
                Self::NonExecuted {
                    node_handle,
                    layer_index,
                    entity_handle,
                }
            }
        }

        impl<N: Debug + 'static> CommandTrait for $name<N> {
            fn name(&mut self, _context: &dyn CommandContext) -> String {
                "Free Entity".to_owned()
            }

            fn execute(&mut self, context: &mut dyn CommandContext) {
                match std::mem::replace(self, Self::Unknown) {
                    Self::NonExecuted {
                        node_handle,
                        layer_index,
                        entity_handle,
                    }
                    | Self::Reverted {
                        node_handle,
                        layer_index,
                        entity_handle,
                    } => {
                        let machine = fetch_machine(context, node_handle);
                        let (ticket, entity) = machine.layers_mut()[layer_index]
                            .$container()
                            .take_reserve(entity_handle);
                        *self = Self::Executed {
                            node_handle,
                            layer_index,
                            entity,
                            ticket,
                        }
                    }
                    _ => unreachable!(),
                }
            }

            fn revert(&mut self, context: &mut dyn CommandContext) {
                match std::mem::replace(self, Self::Unknown) {
                    Self::Executed {
                        node_handle,
                        layer_index,
                        entity,
                        ticket,
                    } => {
                        let machine = fetch_machine(context, node_handle);

                        *self = Self::Reverted {
                            node_handle,
                            layer_index,
                            entity_handle: machine.layers_mut()[layer_index]
                                .$container()
                                .put_back(ticket, entity),
                        };
                    }
                    _ => unreachable!(),
                }
            }

            fn finalize(&mut self, context: &mut dyn CommandContext) {
                match std::mem::replace(self, Self::Unknown) {
                    Self::Executed {
                        node_handle,
                        layer_index,
                        ticket,
                        ..
                    } => {
                        let machine = fetch_machine(context, node_handle);
                        machine.layers_mut()[layer_index]
                            .$container()
                            .forget_ticket(ticket);
                    }
                    _ => (),
                }
            }
        }
    };
}

define_free_command!(DeleteStateCommand, State<Handle<N>>, states_mut);
define_free_command!(DeletePoseNodeCommand, PoseNode<Handle<N>>, nodes_mut);
define_free_command!(
    DeleteTransitionCommand,
    Transition<Handle<N>>,
    transitions_mut
);

#[macro_export]
macro_rules! define_push_element_to_collection_command {
    ($name:ident<$model_handle:ty, $value_type:ty>($self:ident, $context:ident) $get_collection:block) => {
        #[derive(Debug)]
        pub struct $name<N: Debug + 'static> {
            pub node_handle: Handle<N>,
            pub handle: $model_handle,
            pub layer_index: usize,
            pub value: Option<$value_type>,
        }

        impl<N: Debug + 'static> $name<N> {
            pub fn new(node_handle: Handle<N>, handle: $model_handle, layer_index: usize, value: $value_type) -> Self {
                Self {
                    node_handle,
                    handle,
                    layer_index,
                    value: Some(value)
                }
            }
        }

        impl<N: Debug + 'static> CommandTrait for $name<N> {
            fn name(&mut self, _context: &dyn CommandContext) -> String {
                "Push Element To Collection".to_string()
            }

            fn execute(&mut $self, $context: &mut dyn CommandContext) {
                ($get_collection).push($self.value.take().unwrap());
            }

            fn revert(&mut $self, $context: &mut dyn CommandContext) {
                $self.value = Some(($get_collection).pop().unwrap());
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

        impl CommandTrait for $name {
            fn name(&mut self, _context: &dyn CommandContext) -> String {
                "Remove Collection Element".to_string()
            }

            fn execute(&mut $self, $context: &mut dyn CommandContext) {
                let collection = $get_collection;
                $self.value = Some(collection.remove($self.index));
            }

            fn revert(&mut $self, $context: &mut dyn CommandContext) {
                let collection = $get_collection;
                collection.insert($self.index, $self.value.take().unwrap())
            }
        }
    };
}

#[macro_export]
macro_rules! define_set_collection_element_command {
    ($name:ident<$model_handle:ty, $value_type:ty>($self:ident, $context:ident) $swap_value:block) => {
        #[derive(Debug)]
        pub struct $name<N: Debug + 'static> {
            pub node_handle: Handle<N>,
            pub handle: $model_handle,
            pub layer_index: usize,
            pub index: usize,
            pub value: $value_type,
        }

        impl<N: Debug + 'static> $name<N> {
            pub fn swap(&mut $self, $context: &mut dyn CommandContext) {
                 $swap_value
            }
        }

        impl<N: Debug + 'static> CommandTrait for $name<N> {
            fn name(&mut self,
                #[allow(unused_variables)]
                $context: &dyn CommandContext
            ) -> String {
                "Set Collection Element".to_owned()
            }

            fn execute(&mut self, $context: &mut dyn CommandContext) {
                self.swap($context);
            }

            fn revert(&mut self, $context: &mut dyn CommandContext) {
                self.swap($context);
            }
        }
    };
}

#[derive(Debug)]
pub struct SetMachineEntryStateCommand<N: Debug + 'static> {
    pub node_handle: Handle<N>,
    pub layer: usize,
    pub entry: Handle<State<Handle<N>>>,
}

impl<N: Debug + 'static> SetMachineEntryStateCommand<N> {
    fn swap(&mut self, context: &mut dyn CommandContext) {
        let machine = fetch_machine(context, self.node_handle);
        let layer = &mut machine.layers_mut()[self.layer];

        let prev = layer.entry_state();
        layer.set_entry_state(self.entry);
        self.entry = prev;
    }
}

impl<N: Debug + 'static> CommandTrait for SetMachineEntryStateCommand<N> {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Entry State".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }
}

#[macro_export]
macro_rules! define_absm_swap_command {
    ($name:ident<$model_type:ty, $value_type:ty>[$($field_name:ident:$field_type:ty),*]($self:ident, $context:ident) $get_field:block) => {
        #[derive(Debug)]
        pub struct $name<N: Debug + 'static> {
            pub node_handle: Handle<N>,
            pub handle: $model_type,
            pub value: $value_type,
            $(
                pub $field_name: $field_type,
            )*
        }

        impl<N: Debug + 'static> $name<N> {
            fn swap(&mut $self, $context: &mut dyn CommandContext) {
                let field = $get_field;

                std::mem::swap(field, &mut $self.value);
            }
        }

        impl<N: Debug + 'static> CommandTrait for $name<N> {
            fn name(&mut self, _context: &dyn CommandContext) -> String {
                stringify!($name).to_string()
            }

            fn execute(&mut self, context: &mut dyn CommandContext) {
                self.swap(context)
            }

            fn revert(&mut self, context: &mut dyn CommandContext) {
                self.swap(context)
            }
        }
    };
}

define_absm_swap_command!(SetStateRootPoseCommand<Handle<State<Handle<N>>>, Handle<PoseNode<Handle<N>>>>[layer_index: usize](self, context) {
    let machine = fetch_machine(context, self.node_handle);
    &mut machine.layers_mut()[self.layer_index].states_mut()[self.handle].root
});

#[derive(Debug)]
pub struct SetLayerNameCommand<N: Debug + 'static> {
    pub absm_node_handle: Handle<N>,
    pub layer_index: usize,
    pub name: String,
}

impl<N: Debug + 'static> SetLayerNameCommand<N> {
    fn swap(&mut self, context: &mut dyn CommandContext) {
        let layer =
            &mut fetch_machine(context, self.absm_node_handle).layers_mut()[self.layer_index];
        let prev = layer.name().to_string();
        layer.set_name(self.name.clone());
        self.name = prev;
    }
}

impl<N: Debug + 'static> CommandTrait for SetLayerNameCommand<N> {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Layer Name".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }
}

#[derive(Debug)]
pub struct AddLayerCommand<N: Debug + 'static> {
    pub absm_node_handle: Handle<N>,
    pub layer: Option<MachineLayer<Handle<N>>>,
}

impl<N: Debug + 'static> CommandTrait for AddLayerCommand<N> {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Add Layer".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        fetch_machine(context, self.absm_node_handle).add_layer(self.layer.take().unwrap());
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.layer = fetch_machine(context, self.absm_node_handle).pop_layer();
    }
}

#[derive(Debug)]
pub struct RemoveLayerCommand<N: Debug + 'static> {
    pub absm_node_handle: Handle<N>,
    pub layer_index: usize,
    pub layer: Option<MachineLayer<Handle<N>>>,
}

impl<N: Debug + 'static> RemoveLayerCommand<N> {
    pub fn new(absm_node_handle: Handle<N>, layer_index: usize) -> Self {
        Self {
            absm_node_handle,
            layer_index,
            layer: None,
        }
    }
}

impl<N: Debug + 'static> CommandTrait for RemoveLayerCommand<N> {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        format!("Remove {} Layer", self.layer_index)
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.layer =
            Some(fetch_machine(context, self.absm_node_handle).remove_layer(self.layer_index));
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        fetch_machine(context, self.absm_node_handle)
            .insert_layer(self.layer_index, self.layer.take().unwrap());
    }
}

#[derive(Debug)]
pub struct SetLayerMaskCommand<N: Debug + 'static> {
    pub absm_node_handle: Handle<N>,
    pub layer_index: usize,
    pub mask: LayerMask<Handle<N>>,
}

impl<N: Debug + 'static> SetLayerMaskCommand<N> {
    fn swap(&mut self, context: &mut dyn CommandContext) {
        let layer =
            &mut fetch_machine(context, self.absm_node_handle).layers_mut()[self.layer_index];
        let old = layer.mask().clone();
        layer.set_mask(std::mem::replace(&mut self.mask, old));
    }
}

impl<N: Debug + 'static> CommandTrait for SetLayerMaskCommand<N> {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Layer Mask".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }
}
