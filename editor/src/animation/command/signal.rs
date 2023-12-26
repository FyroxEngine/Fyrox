use crate::{
    animation::command::fetch_animation_player,
    command::GameSceneCommandTrait,
    define_universal_commands,
    scene::commands::{GameSceneCommand, GameSceneContext},
};
use fyrox::{
    animation::Animation,
    core::{pool::Handle, reflect::prelude::*, uuid::Uuid},
    scene::node::Node,
};

define_universal_commands!(
    make_animation_signal_property_command,
    GameSceneCommandTrait,
    GameSceneCommand,
    GameSceneContext,
    Uuid,
    ctx,
    handle,
    self,
    {
        fetch_animation_player(self.node_handle, ctx).animations_mut()[self.animation_handle]
            .signals_mut()
            .iter_mut()
            .find(|s| s.id == self.handle)
            .unwrap()
    },
    node_handle: Handle<Node>,
    animation_handle: Handle<Animation<Handle<Node>>>
);
