use crate::{define_universal_commands, GameSceneCommand, GameSceneCommandTrait, GameSceneContext};
use fyrox::core::reflect::prelude::*;

define_universal_commands!(
    make_set_scene_property_command,
    GameSceneCommandTrait,
    GameSceneCommand,
    GameSceneContext,
    (),
    ctx,
    handle,
    self,
    { ctx.scene as &mut dyn Reflect },
);
