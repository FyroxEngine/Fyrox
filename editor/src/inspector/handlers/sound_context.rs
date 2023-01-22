use crate::{scene::commands::sound_context::*, SceneCommand};
use fyrox::scene::sound::{DistanceModel, Renderer};
use fyrox::{
    gui::inspector::{FieldKind, PropertyChanged},
    scene::sound::context::SoundContext,
};

pub fn handle_sound_context_property_changed(args: &PropertyChanged) -> Option<SceneCommand> {
    let mut command = None;

    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            SoundContext::PAUSED => value.cast_clone::<bool>(&mut |value| {
                command = Some(SceneCommand::new(SetPausedCommand::new(value.unwrap())))
            }),
            SoundContext::MASTER_GAIN => value.cast_clone::<f32>(&mut |value| {
                command = Some(SceneCommand::new(SetMasterGainCommand::new(value.unwrap())))
            }),
            SoundContext::DISTANCE_MODEL => value.cast_clone::<DistanceModel>(&mut |value| {
                command = Some(SceneCommand::new(SetDistanceModelCommand::new(
                    value.unwrap(),
                )))
            }),
            SoundContext::RENDERER => {
                value.cast_clone::<Renderer>(&mut |value| {
                    command = Some(SceneCommand::new(SetRendererCommand::new(value.unwrap())))
                });
            }
            _ => (),
        },
        _ => (),
    }

    command
}
