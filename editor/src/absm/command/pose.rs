use crate::{
    absm::command::{AbsmCommand, AbsmCommandTrait, AbsmEditorContext},
    define_universal_commands,
};
use fyrox::{
    animation::machine::node::PoseNodeDefinition,
    core::{pool::Handle, reflect::ResolvePath},
};

define_universal_commands!(
    make_set_pose_property_command,
    AbsmCommandTrait,
    AbsmCommand,
    AbsmEditorContext,
    Handle<PoseNodeDefinition>,
    ctx,
    handle,
    self,
    { &mut ctx.resource.absm_definition.nodes[self.handle] }
);
