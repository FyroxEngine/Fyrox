use crate::{make_command, scene::commands::sound::*, SceneCommand};
use fyrox::{
    core::pool::Handle,
    gui::inspector::{FieldKind, PropertyChanged},
    scene::{node::Node, sound::Sound},
};

pub fn handle_sound_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Object(ref value) => {
            match args.name.as_ref() {
                Sound::GAIN => {
                    make_command!(SetSoundSourceGainCommand, handle, value)
                }
                Sound::BUFFER => {
                    make_command!(SetSoundSourceBufferCommand, handle, value)
                }
                Sound::PANNING => {
                    make_command!(SetSoundSourcePanningCommand, handle, value)
                }
                Sound::PITCH => {
                    make_command!(SetSoundSourcePitchCommand, handle, value)
                }
                Sound::LOOPING => {
                    make_command!(SetSoundSourceLoopingCommand, handle, value)
                }
                Sound::STATUS => {
                    // TODO
                    None
                }
                Sound::PLAY_ONCE => {
                    make_command!(SetSoundSourcePlayOnceCommand, handle, value)
                }
                Sound::RADIUS => {
                    make_command!(SetSpatialSoundSourceRadiusCommand, handle, value)
                }
                Sound::MAX_DISTANCE => {
                    make_command!(SetMaxDistanceCommand, handle, value)
                }
                Sound::ROLLOFF_FACTOR => {
                    make_command!(SetRolloffFactorCommand, handle, value)
                }
                _ => None,
            }
        }
        _ => None,
    }
}
