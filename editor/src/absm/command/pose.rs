use crate::{
    absm::command::fetch_machine,
    command::GameSceneCommandTrait,
    define_universal_commands,
    scene::commands::{GameSceneCommand, GameSceneContext},
};
use fyrox::{
    animation::machine::node::PoseNode,
    core::{pool::Handle, reflect::prelude::*},
    scene::node::Node,
};

define_universal_commands! {
    make_set_pose_property_command,
    GameSceneCommandTrait,
    GameSceneCommand,
    GameSceneContext,
    Handle<PoseNode>,
    ctx,
    handle,
    self,
    {
        let machine = fetch_machine(ctx, self.node_handle);
        &mut machine.layers_mut()[self.layer_index].nodes_mut()[self.handle]
    },
    node_handle: Handle<Node>,
    layer_index: usize
}
