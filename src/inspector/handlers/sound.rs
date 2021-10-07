use crate::{
    inspector::SenderHelper,
    scene::commands::sound::{
        SetSoundSourceBufferCommand, SetSoundSourceGainCommand, SetSoundSourceLoopingCommand,
        SetSoundSourceNameCommand, SetSoundSourcePanningCommand, SetSoundSourcePitchCommand,
        SetSoundSourcePlayOnceCommand, SetSpatialSoundSourceMaxDistanceCommand,
        SetSpatialSoundSourcePositionCommand, SetSpatialSoundSourceRadiusCommand,
        SetSpatialSoundSourceRolloffFactorCommand,
    },
};
use rg3d::{
    core::pool::Handle,
    gui::message::{FieldKind, PropertyChanged},
    sound::source::{generic::GenericSource, spatial::SpatialSource, SoundSource},
};

pub fn handle_generic_source_property_changed(
    args: &PropertyChanged,
    source_handle: Handle<SoundSource>,
    helper: &SenderHelper,
) -> Option<()> {
    if let FieldKind::Object(ref value) = args.value {
        match args.name.as_ref() {
            GenericSource::NAME => helper.do_scene_command(SetSoundSourceNameCommand::new(
                source_handle,
                value.cast_value().cloned()?,
            )),
            GenericSource::GAIN => {
                helper.do_scene_command(SetSoundSourceGainCommand::new(
                    source_handle,
                    *value.cast_value()?,
                ));
            }
            GenericSource::BUFFER => helper.do_scene_command(SetSoundSourceBufferCommand::new(
                source_handle,
                value.cast_value().cloned()?,
            )),
            GenericSource::PANNING => helper.do_scene_command(SetSoundSourcePanningCommand::new(
                source_handle,
                *value.cast_value()?,
            )),
            GenericSource::PITCH => helper.do_scene_command(SetSoundSourcePitchCommand::new(
                source_handle,
                *value.cast_value()?,
            )),
            GenericSource::LOOPING => helper.do_scene_command(SetSoundSourceLoopingCommand::new(
                source_handle,
                *value.cast_value()?,
            )),
            GenericSource::STATUS => {
                // TODO
            }
            GenericSource::PLAY_ONCE => helper.do_scene_command(
                SetSoundSourcePlayOnceCommand::new(source_handle, *value.cast_value()?),
            ),
            _ => println!("Unhandled property of GenericSource: {:?}", args),
        }
    }
    Some(())
}

pub fn handle_spatial_source_property_changed(
    args: &PropertyChanged,
    source_handle: Handle<SoundSource>,
    helper: &SenderHelper,
) -> Option<()> {
    if let FieldKind::Object(ref value) = args.value {
        match args.name.as_ref() {
            SpatialSource::RADIUS => helper.do_scene_command(
                SetSpatialSoundSourceRadiusCommand::new(source_handle, *value.cast_value()?),
            ),
            SpatialSource::POSITION => helper.do_scene_command(
                SetSpatialSoundSourcePositionCommand::new(source_handle, *value.cast_value()?),
            ),
            SpatialSource::MAX_DISTANCE => helper.do_scene_command(
                SetSpatialSoundSourceMaxDistanceCommand::new(source_handle, *value.cast_value()?),
            ),
            SpatialSource::ROLLOFF_FACTOR => helper.do_scene_command(
                SetSpatialSoundSourceRolloffFactorCommand::new(source_handle, *value.cast_value()?),
            ),
            _ => println!("Unhandled property of SpatialSource: {:?}", args),
        }
    }
    Some(())
}
