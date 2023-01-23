use crate::{define_universal_commands, Command, SceneCommand, SceneContext};
use fyrox::core::reflect::prelude::*;

define_universal_commands!(
    make_set_scene_property_command,
    Command,
    SceneCommand,
    SceneContext,
    (),
    ctx,
    handle,
    self,
    { ctx.scene as &mut dyn Reflect },
);
