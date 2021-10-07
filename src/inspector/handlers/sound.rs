use crate::{do_command, inspector::SenderHelper, scene::commands::sound::*};
use rg3d::{
    core::pool::Handle,
    gui::message::{FieldKind, PropertyChanged},
    sound::source::{generic::GenericSource, spatial::SpatialSource, SoundSource},
};

pub fn handle_generic_source_property_changed(
    args: &PropertyChanged,
    handle: Handle<SoundSource>,
    helper: &SenderHelper,
) -> Option<()> {
    if let FieldKind::Object(ref value) = args.value {
        match args.name.as_ref() {
            GenericSource::NAME => {
                do_command!(helper, SetSoundSourceNameCommand, handle, value)
            }
            GenericSource::GAIN => {
                do_command!(helper, SetSoundSourceGainCommand, handle, value)
            }
            GenericSource::BUFFER => {
                do_command!(helper, SetSoundSourceBufferCommand, handle, value)
            }
            GenericSource::PANNING => {
                do_command!(helper, SetSoundSourcePanningCommand, handle, value)
            }
            GenericSource::PITCH => {
                do_command!(helper, SetSoundSourcePitchCommand, handle, value)
            }
            GenericSource::LOOPING => {
                do_command!(helper, SetSoundSourceLoopingCommand, handle, value)
            }
            GenericSource::STATUS => {
                // TODO
            }
            GenericSource::PLAY_ONCE => {
                do_command!(helper, SetSoundSourcePlayOnceCommand, handle, value)
            }
            _ => println!("Unhandled property of GenericSource: {:?}", args),
        }
    }
    Some(())
}

pub fn handle_spatial_source_property_changed(
    args: &PropertyChanged,
    handle: Handle<SoundSource>,
    helper: &SenderHelper,
) -> Option<()> {
    if let FieldKind::Object(ref value) = args.value {
        match args.name.as_ref() {
            SpatialSource::RADIUS => {
                do_command!(helper, SetSpatialSoundSourceRadiusCommand, handle, value)
            }
            SpatialSource::POSITION => {
                do_command!(helper, SetSpatialSoundSourcePositionCommand, handle, value)
            }
            SpatialSource::MAX_DISTANCE => {
                do_command!(helper, SetMaxDistanceCommand, handle, value)
            }
            SpatialSource::ROLLOFF_FACTOR => {
                do_command!(helper, SetRolloffFactorCommand, handle, value)
            }
            _ => println!("Unhandled property of SpatialSource: {:?}", args),
        }
    }
    Some(())
}
