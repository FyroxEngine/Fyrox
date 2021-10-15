use crate::{
    inspector::editors::{
        material::MaterialPropertyEditorDefinition, texture::TexturePropertyEditorDefinition,
    },
    Message,
};
use rg3d::core::pool::ErasedHandle;
use rg3d::gui::inspector::editors::inspectable::InspectablePropertyEditorDefinition;
use rg3d::physics3d::desc::InteractionGroupsDesc;
use rg3d::scene::base::{LevelOfDetail, LodGroup};
use rg3d::sound::source::Status;
use rg3d::{
    core::inspect::Inspect,
    gui::inspector::editors::{
        collection::VecCollectionPropertyEditorDefinition,
        enumeration::EnumPropertyEditorDefinition, PropertyEditorDefinitionContainer,
    },
    physics3d,
    scene::{
        base::{Mobility, PhysicsBinding},
        camera::Exposure,
        mesh::surface::Surface,
        mesh::RenderPath,
        particle_system::emitter::Emitter,
        terrain::Layer,
    },
};
use std::{
    fmt::Debug,
    sync::{mpsc::Sender, Arc, Mutex},
};

pub mod material;
pub mod texture;

pub fn make_physics_binding_enum_editor_definition() -> EnumPropertyEditorDefinition<PhysicsBinding>
{
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => PhysicsBinding::NodeWithBody,
            1 => PhysicsBinding::BodyWithNode,
            _ => unreachable!(),
        },
        index_generator: |v| *v as usize,
        names_generator: || vec!["Node With Body".to_string(), "Body With Node".to_string()],
    }
}

pub fn make_mobility_enum_editor_definition() -> EnumPropertyEditorDefinition<Mobility> {
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => Mobility::Static,
            1 => Mobility::Stationary,
            2 => Mobility::Dynamic,
            _ => unreachable!(),
        },
        index_generator: |v| *v as usize,
        names_generator: || {
            vec![
                "Static".to_string(),
                "Stationary".to_string(),
                "Dynamic".to_string(),
            ]
        },
    }
}

pub fn make_exposure_enum_editor_definition() -> EnumPropertyEditorDefinition<Exposure> {
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => Exposure::default(),
            1 => Exposure::Manual(1.0),
            _ => unreachable!(),
        },
        index_generator: |v| match v {
            Exposure::Auto { .. } => 0,
            Exposure::Manual(_) => 1,
        },
        names_generator: || vec!["Auto".to_string(), "Manual".to_string()],
    }
}

pub fn make_render_path_enum_editor_definition() -> EnumPropertyEditorDefinition<RenderPath> {
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => RenderPath::Deferred,
            1 => RenderPath::Forward,
            _ => unreachable!(),
        },
        index_generator: |v| *v as usize,
        names_generator: || vec!["Deferred".to_string(), "Forward".to_string()],
    }
}

pub fn make_status_enum_editor_definition() -> EnumPropertyEditorDefinition<Status> {
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => Status::Stopped,
            1 => Status::Playing,
            2 => Status::Paused,
            _ => unreachable!(),
        },
        index_generator: |v| *v as usize,
        names_generator: || {
            vec![
                "Stopped".to_string(),
                "Playing".to_string(),
                "Paused".to_string(),
            ]
        },
    }
}

pub fn make_rigid_body_type_editor_definition(
) -> EnumPropertyEditorDefinition<physics3d::desc::RigidBodyTypeDesc> {
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => physics3d::desc::RigidBodyTypeDesc::Dynamic,
            1 => physics3d::desc::RigidBodyTypeDesc::Static,
            2 => physics3d::desc::RigidBodyTypeDesc::KinematicPositionBased,
            3 => physics3d::desc::RigidBodyTypeDesc::KinematicVelocityBased,
            _ => unreachable!(),
        },
        index_generator: |v| *v as usize,
        names_generator: || {
            vec![
                "Dynamic".to_string(),
                "Static".to_string(),
                "Kinematic (Position Based)".to_string(),
                "Kinematic (Velocity Based)".to_string(),
            ]
        },
    }
}

pub fn make_option_editor_definition<T>() -> EnumPropertyEditorDefinition<Option<T>>
where
    T: Inspect + Default + Debug + Send + Sync + 'static,
{
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => None,
            1 => Some(Default::default()),
            _ => unreachable!(),
        },
        index_generator: |v| match v {
            None => 0,
            Some(_) => 1,
        },
        names_generator: || vec!["None".to_string(), "Some".to_string()],
    }
}

pub fn make_property_editors_container(
    sender: Sender<Message>,
) -> Arc<PropertyEditorDefinitionContainer> {
    let mut container = PropertyEditorDefinitionContainer::new();

    container.insert(Arc::new(TexturePropertyEditorDefinition));
    container.insert(Arc::new(MaterialPropertyEditorDefinition {
        sender: Mutex::new(sender),
    }));
    container.insert(Arc::new(
        VecCollectionPropertyEditorDefinition::<Surface>::new(),
    ));
    container.insert(Arc::new(
        VecCollectionPropertyEditorDefinition::<Layer>::new(),
    ));
    container.insert(Arc::new(
        VecCollectionPropertyEditorDefinition::<Emitter>::new(),
    ));
    container.insert(Arc::new(VecCollectionPropertyEditorDefinition::<
        LevelOfDetail,
    >::new()));
    container.insert(Arc::new(VecCollectionPropertyEditorDefinition::<
        ErasedHandle,
    >::new()));
    container.insert(Arc::new(make_physics_binding_enum_editor_definition()));
    container.insert(Arc::new(make_mobility_enum_editor_definition()));
    container.insert(Arc::new(make_exposure_enum_editor_definition()));
    container.insert(Arc::new(make_render_path_enum_editor_definition()));
    container.insert(Arc::new(make_status_enum_editor_definition()));
    container.insert(Arc::new(make_rigid_body_type_editor_definition()));
    container.insert(Arc::new(make_option_editor_definition::<f32>()));
    container.insert(Arc::new(make_option_editor_definition::<LodGroup>()));
    container.insert(Arc::new(InspectablePropertyEditorDefinition::<
        InteractionGroupsDesc,
    >::new()));

    Arc::new(container)
}
