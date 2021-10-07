use crate::{inspector::SenderHelper, scene::commands::sound::SetSoundSourceGainCommand};
use rg3d::{
    core::pool::Handle,
    gui::message::{FieldKind, PropertyChanged},
    sound::source::{generic::GenericSource, SoundSource},
};

pub fn handle_generic_source_property_changed(
    args: &PropertyChanged,
    source_handle: Handle<SoundSource>,
    helper: &SenderHelper,
) {
    if let FieldKind::Object(ref value) = args.value {
        match args.name.as_ref() {
            GenericSource::NAME => {
                // TODO
            }
            GenericSource::GAIN => {
                helper.do_scene_command(SetSoundSourceGainCommand::new(
                    source_handle,
                    *value.cast_value().unwrap(),
                ));
            }
            GenericSource::BUFFER => {
                // TODO
            }
            GenericSource::PANNING => {
                // TODO
            }
            GenericSource::PITCH => {
                // TODO
            }
            GenericSource::LOOPING => {
                // TODO
            }
            GenericSource::STATUS => {
                // TODO
            }
            GenericSource::PLAY_ONCE => {
                // TODO
            }
            _ => println!("Unhandled property of Transform: {:?}", args),
        }
    }
}
