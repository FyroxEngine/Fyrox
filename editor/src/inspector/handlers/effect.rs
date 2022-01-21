use crate::{handle_properties, scene::commands::effect::*, SceneCommand};
use fyrox::{
    core::pool::Handle,
    gui::inspector::{CollectionChanged, FieldKind, PropertyChanged},
    scene::sound::effect::{BaseEffect, Effect, ReverbEffect},
};

pub fn handle_base_effect_property_changed(
    args: &PropertyChanged,
    handle: Handle<Effect>,
) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Object(ref value) => {
            handle_properties!(args.name.as_ref(), handle, value,
                BaseEffect::NAME => SetNameCommand,
                BaseEffect::GAIN => SetGainCommand
            )
        }
        FieldKind::Collection(ref collection_changed) => match args.name.as_ref() {
            BaseEffect::INPUTS => match **collection_changed {
                CollectionChanged::Add => Some(SceneCommand::new(AddInputCommand {
                    handle,
                    value: Default::default(),
                })),
                CollectionChanged::Remove(i) => Some(SceneCommand::new(RemoveInputCommand {
                    handle,
                    index: i,
                    value: None,
                })),
                CollectionChanged::ItemChanged {
                    index,
                    ref property,
                } => match property.value {
                    FieldKind::Inspectable(ref property_changed) => {
                        match property_changed.name.as_ref() {
                            "0" => {
                                if let FieldKind::Object(ref value) = property_changed.value {
                                    None
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                },
            },
            _ => None,
        },
        _ => None,
    }
}

pub fn handle_reverb_effect_property_changed(
    args: &PropertyChanged,
    handle: Handle<Effect>,
) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Object(ref value) => {
            handle_properties!(args.name.as_ref(), handle, value,
                ReverbEffect::DRY => SetReverbDryCommand,
                ReverbEffect::WET => SetReverbWetCommand,
                ReverbEffect::FC => SetReverbFcCommand,
                ReverbEffect::DECAY_TIME => SetReverbDecayTimeCommand
            )
        }
        FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
            ReverbEffect::BASE => handle_base_effect_property_changed(inner, handle),
            _ => None,
        },
        _ => None,
    }
}
