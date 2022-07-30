use crate::{handle_properties, scene::commands::effect::*, SceneCommand};
use fyrox::{
    core::pool::Handle,
    gui::inspector::{CollectionChanged, FieldKind, PropertyChanged},
    scene::sound::{
        effect::{BaseEffect, Effect, EffectInput, ReverbEffect},
        Biquad,
    },
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
                CollectionChanged::Add(_) => Some(SceneCommand::new(AddInputCommand {
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
                    ..
                } => handle_effect_input_property_changed(property, handle, index),
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

fn handle_effect_input_property_changed(
    args: &PropertyChanged,
    handle: Handle<Effect>,
    index: usize,
) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            EffectInput::SOUND => Some(SceneCommand::new(SetEffectInputSound::new(
                handle,
                index,
                value.cast_clone()?,
            ))),
            EffectInput::FILTER => Some(SceneCommand::new(SetEffectInputFilter::new(
                handle,
                index,
                value.cast_clone()?,
            ))),
            _ => None,
        },
        FieldKind::Inspectable(ref value) => handle_effect_input_filter(&**value, handle, index),
        _ => None,
    }
}

fn handle_effect_input_filter(
    args: &PropertyChanged,
    handle: Handle<Effect>,
    index: usize,
) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            Biquad::B_0 => Some(SceneCommand::new(SetEffectInputFilterB0::new(
                handle,
                index,
                value.cast_clone()?,
            ))),
            Biquad::B_1 => Some(SceneCommand::new(SetEffectInputFilterB1::new(
                handle,
                index,
                value.cast_clone()?,
            ))),
            Biquad::B_2 => Some(SceneCommand::new(SetEffectInputFilterB2::new(
                handle,
                index,
                value.cast_clone()?,
            ))),
            Biquad::A_1 => Some(SceneCommand::new(SetEffectInputFilterA1::new(
                handle,
                index,
                value.cast_clone()?,
            ))),
            Biquad::A_2 => Some(SceneCommand::new(SetEffectInputFilterA2::new(
                handle,
                index,
                value.cast_clone()?,
            ))),
            _ => None,
        },
        _ => None,
    }
}
