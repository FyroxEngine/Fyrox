use crate::{scene::commands::sound_context::*, SceneCommand};
use fyrox::{
    gui::inspector::{FieldKind, PropertyChanged},
    scene::sound::context::SoundContext,
};

pub fn handle_sound_context_property_changed(args: &PropertyChanged) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            SoundContext::PAUSED => Some(SceneCommand::new(SetPausedCommand::new(
                value.cast_clone()?,
            ))),
            SoundContext::MASTER_GAIN => Some(SceneCommand::new(SetMasterGainCommand::new(
                value.cast_clone()?,
            ))),
            SoundContext::DISTANCE_MODEL => Some(SceneCommand::new(SetDistanceModelCommand::new(
                value.cast_clone()?,
            ))),
            SoundContext::RENDERER => Some(SceneCommand::new(SetRendererCommand::new(
                value.cast_clone()?,
            ))),
            _ => None,
        },
        _ => None,
    }
}
