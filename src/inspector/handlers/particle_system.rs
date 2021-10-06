use crate::{
    inspector::SenderHelper,
    scene::commands::particle_system::{
        SetParticleSystemAccelerationCommand, SetParticleSystemEnabledCommand,
        SetParticleSystemTextureCommand, SetSoftBoundarySharpnessFactorCommand,
    },
};
use rg3d::{
    core::pool::Handle,
    gui::message::{CollectionChanged, FieldKind, PropertyChanged},
    resource::texture::Texture,
    scene::{node::Node, particle_system::ParticleSystem},
};

pub fn handle_particle_system_property_changed(
    args: &PropertyChanged,
    node_handle: Handle<Node>,
    helper: &SenderHelper,
) {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            ParticleSystem::TEXTURE => {
                helper.do_scene_command(SetParticleSystemTextureCommand::new(
                    node_handle,
                    value.cast_value::<Option<Texture>>().unwrap().clone(),
                ));
            }
            ParticleSystem::ACCELERATION => {
                helper.do_scene_command(SetParticleSystemAccelerationCommand::new(
                    node_handle,
                    *value.cast_value().unwrap(),
                ))
            }
            ParticleSystem::ENABLED => helper.do_scene_command(
                SetParticleSystemEnabledCommand::new(node_handle, *value.cast_value().unwrap()),
            ),
            ParticleSystem::SOFT_BOUNDARY_SHARPNESS_FACTOR => {
                helper.do_scene_command(SetSoftBoundarySharpnessFactorCommand::new(
                    node_handle,
                    *value.cast_value().unwrap(),
                ))
            }
            _ => println!("Unhandled property of Transform: {:?}", args),
        },
        FieldKind::Collection(ref collection_changed) => match args.name.as_ref() {
            ParticleSystem::EMITTERS => match &**collection_changed {
                CollectionChanged::Add => {}
                CollectionChanged::Remove(_) => {}
                CollectionChanged::ItemChanged { .. } => {}
            },
            _ => (),
        },
        _ => {}
    }
}
