use crate::{
    absm::command::{AbsmCommand, AbsmCommandTrait, AbsmEditorContext},
    define_universal_commands,
};
use fyrox::core::reflect::ResolvePath;

define_universal_commands!(
    make_set_parameters_property_command,
    AbsmCommandTrait,
    AbsmCommand,
    AbsmEditorContext,
    (),
    ctx,
    handle,
    self,
    { &mut ctx.resource.absm_definition.parameters }
);
