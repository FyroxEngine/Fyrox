use crate::{
    absm::command::fetch_machine,
    command::Command,
    define_universal_commands,
    scene::commands::{SceneCommand, SceneContext},
};
use fyrox::{
    core::{pool::Handle, reflect::prelude::*},
    scene::node::Node,
};

define_universal_commands!(
    make_set_parameters_property_command,
    Command,
    SceneCommand,
    SceneContext,
    (),
    ctx,
    handle,
    self,
    { fetch_machine(ctx, self.node_handle).parameters_mut() },
    node_handle: Handle<Node>
);
