use crate::{
    absm::command::{AbsmCommand, AbsmCommandTrait, AbsmEditorContext},
    define_universal_commands,
};
use fyrox::{
    animation::machine::state::StateDefinition,
    core::{pool::Handle, reflect::ResolvePath},
};

define_universal_commands!(
    make_set_state_property_command,
    AbsmCommandTrait,
    AbsmCommand,
    AbsmEditorContext,
    Handle<StateDefinition>,
    ctx,
    handle,
    self,
    { &mut ctx.resource.absm_definition.states[self.handle] }
);
