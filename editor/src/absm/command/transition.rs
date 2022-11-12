use crate::{
    absm::command::fetch_machine,
    command::Command,
    define_universal_commands,
    scene::commands::{SceneCommand, SceneContext},
};
use fyrox::{
    animation::machine::transition::Transition,
    core::{pool::Handle, reflect::ResolvePath},
    scene::node::Node,
};

define_universal_commands!(
    make_set_transition_property_command,
    Command,
    SceneCommand,
    SceneContext,
    Handle<Transition>,
    ctx,
    handle,
    self,
    {
        let machine = fetch_machine(ctx, self.node_handle);
        &mut machine.transitions_mut()[self.handle]
    },
    node_handle: Handle<Node>
);
