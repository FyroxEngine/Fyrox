use crate::{make_command, scene::commands::sound::*, SceneCommand};
use rg3d::{
    core::pool::Handle,
    gui::inspector::{FieldKind, PropertyChanged},
    sound::source::{generic::GenericSource, spatial::SpatialSource, SoundSource},
};

pub fn handle_generic_source_property_changed(
    args: &PropertyChanged,
    handle: Handle<SoundSource>,
) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Object(ref value) => {
            match args.name.as_ref() {
                GenericSource::NAME => {
                    make_command!(SetSoundSourceNameCommand, handle, value)
                }
                GenericSource::GAIN => {
                    make_command!(SetSoundSourceGainCommand, handle, value)
                }
                GenericSource::BUFFER => {
                    make_command!(SetSoundSourceBufferCommand, handle, value)
                }
                GenericSource::PANNING => {
                    make_command!(SetSoundSourcePanningCommand, handle, value)
                }
                GenericSource::PITCH => {
                    make_command!(SetSoundSourcePitchCommand, handle, value)
                }
                GenericSource::LOOPING => {
                    make_command!(SetSoundSourceLoopingCommand, handle, value)
                }
                GenericSource::STATUS => {
                    // TODO
                    None
                }
                GenericSource::PLAY_ONCE => {
                    make_command!(SetSoundSourcePlayOnceCommand, handle, value)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

pub fn handle_spatial_source_property_changed(
    args: &PropertyChanged,
    handle: Handle<SoundSource>,
    source: &SoundSource,
) -> Option<SceneCommand> {
    if let SoundSource::Spatial(_) = source {
        match args.value {
            FieldKind::Object(ref value) => match args.name.as_ref() {
                SpatialSource::RADIUS => {
                    make_command!(SetSpatialSoundSourceRadiusCommand, handle, value)
                }
                SpatialSource::POSITION => {
                    make_command!(SetSpatialSoundSourcePositionCommand, handle, value)
                }
                SpatialSource::MAX_DISTANCE => {
                    make_command!(SetMaxDistanceCommand, handle, value)
                }
                SpatialSource::ROLLOFF_FACTOR => {
                    make_command!(SetRolloffFactorCommand, handle, value)
                }
                _ => None,
            },
            FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
                SpatialSource::GENERIC => handle_generic_source_property_changed(inner, handle),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}
