use crate::{
    absm::command::{AbsmCommand, AbsmCommandTrait, AbsmEditorContext},
    define_universal_commands,
};
use fyrox::{
    animation::machine::transition::TransitionDefinition,
    core::{pool::Handle, reflect::ResolvePath},
};

define_universal_commands!(
    make_set_transition_property_command,
    AbsmCommandTrait,
    AbsmCommand,
    AbsmEditorContext,
    Handle<TransitionDefinition>,
    ctx,
    handle,
    self,
    { &mut ctx.resource.absm_definition.transitions[self.handle] }
);
