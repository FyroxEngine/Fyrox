use crate::inspector::editors::script::ScriptPropertyEditorDefinition;
use crate::{
    inspector::editors::{
        handle::HandlePropertyEditorDefinition,
        material::MaterialPropertyEditorDefinition,
        resource::{
            ModelResourcePropertyEditorDefinition, SoundBufferResourcePropertyEditorDefinition,
        },
        texture::TexturePropertyEditorDefinition,
    },
    Message,
};
use fyrox::{
    core::{parking_lot::Mutex, pool::ErasedHandle, pool::Handle},
    gui::inspector::editors::{
        array::ArrayPropertyEditorDefinition, collection::VecCollectionPropertyEditorDefinition,
        enumeration::EnumPropertyEditorDefinition,
        inspectable::InspectablePropertyEditorDefinition, PropertyEditorDefinitionContainer,
    },
    resource::{
        model::MaterialSearchOptions,
        texture::{
            CompressionOptions, TextureMagnificationFilter, TextureMinificationFilter,
            TextureWrapMode,
        },
    },
    scene::{
        base::{
            Base, LevelOfDetail, LodControlledObject, LodGroup, Mobility, Property, PropertyValue,
        },
        camera::{
            ColorGradingLut, Exposure, OrthographicProjection, PerspectiveProjection, Projection,
            SkyBox,
        },
        collider::{ColliderShape, GeometrySource, InteractionGroups},
        dim2,
        graph::physics::CoefficientCombineRule,
        joint::*,
        light::{
            directional::{CsmOptions, FrustumSplitOptions},
            BaseLight,
        },
        mesh::{surface::Surface, RenderPath},
        node::Node,
        particle_system::emitter::{base::BaseEmitter, Emitter},
        rigidbody::RigidBodyType,
        sound::{
            self,
            effect::{BaseEffect, EffectInput},
            Biquad, DistanceModel, Status,
        },
        terrain::Layer,
        transform::Transform,
    },
};
use std::sync::mpsc::Sender;

pub mod handle;
pub mod material;
pub mod resource;
pub mod script;
pub mod texture;

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

pub fn make_property_editors_container(
    sender: Sender<Message>,
) -> PropertyEditorDefinitionContainer {
    let mut container = PropertyEditorDefinitionContainer::new();

    container.insert(TexturePropertyEditorDefinition);
    container.insert(MaterialPropertyEditorDefinition {
        sender: Mutex::new(sender.clone()),
    });
    container.insert(VecCollectionPropertyEditorDefinition::<Surface>::new());
    container.insert(VecCollectionPropertyEditorDefinition::<Layer>::new());
    container.insert(VecCollectionPropertyEditorDefinition::<Emitter>::new());
    container.insert(VecCollectionPropertyEditorDefinition::<LevelOfDetail>::new());
    container.insert(VecCollectionPropertyEditorDefinition::<ErasedHandle>::new());
    container.insert(VecCollectionPropertyEditorDefinition::<Handle<Node>>::new());
    container.insert(VecCollectionPropertyEditorDefinition::<Property>::new());
    container.insert(VecCollectionPropertyEditorDefinition::<LodControlledObject>::new());
    container.insert(VecCollectionPropertyEditorDefinition::<GeometrySource>::new());
    container.insert(VecCollectionPropertyEditorDefinition::<EffectInput>::new());
    container.insert(make_status_enum_editor_definition());
    container.insert(EnumPropertyEditorDefinition::<f32>::new_optional());
    container.insert(EnumPropertyEditorDefinition::<LodGroup>::new_optional());
    container.insert(ModelResourcePropertyEditorDefinition);
    container.insert(SoundBufferResourcePropertyEditorDefinition);
    container.insert(InspectablePropertyEditorDefinition::<InteractionGroups>::new());
    container.insert(InspectablePropertyEditorDefinition::<ColliderShape>::new());
    container.insert(InspectablePropertyEditorDefinition::<GeometrySource>::new());
    container.insert(InspectablePropertyEditorDefinition::<JointParams>::new());
    container.insert(InspectablePropertyEditorDefinition::<Base>::new());
    container.insert(InspectablePropertyEditorDefinition::<BaseEffect>::new());
    container.insert(InspectablePropertyEditorDefinition::<BaseLight>::new());
    container.insert(InspectablePropertyEditorDefinition::<BaseEmitter>::new());
    container.insert(InspectablePropertyEditorDefinition::<PerspectiveProjection>::new());
    container.insert(InspectablePropertyEditorDefinition::<OrthographicProjection>::new());
    container.insert(InspectablePropertyEditorDefinition::<Transform>::new());
    container.insert(InspectablePropertyEditorDefinition::<CsmOptions>::new());
    container.insert(ArrayPropertyEditorDefinition::<f32, 3>::new());
    container.insert(ArrayPropertyEditorDefinition::<f32, 2>::new());
    container.insert(EnumPropertyEditorDefinition::<ColorGradingLut>::new_optional());
    container.insert(EnumPropertyEditorDefinition::<Biquad>::new_optional());
    container.insert(EnumPropertyEditorDefinition::<Box<SkyBox>>::new_optional());
    container.insert(HandlePropertyEditorDefinition::<Node>::new(sender));
    container.insert(EnumPropertyEditorDefinition::<dim2::collider::ColliderShape>::new());
    container.insert(EnumPropertyEditorDefinition::<CoefficientCombineRule>::new());
    container.insert(EnumPropertyEditorDefinition::<CompressionOptions>::new());
    container.insert(EnumPropertyEditorDefinition::<TextureWrapMode>::new());
    container.insert(EnumPropertyEditorDefinition::<TextureMagnificationFilter>::new());
    container.insert(EnumPropertyEditorDefinition::<TextureMinificationFilter>::new());
    container.insert(EnumPropertyEditorDefinition::<Projection>::new());
    container.insert(EnumPropertyEditorDefinition::<ColliderShape>::new());
    container.insert(EnumPropertyEditorDefinition::<PropertyValue>::new());
    container.insert(EnumPropertyEditorDefinition::<Mobility>::new());
    container.insert(EnumPropertyEditorDefinition::<RigidBodyType>::new());
    container.insert(EnumPropertyEditorDefinition::<Exposure>::new());
    container.insert(EnumPropertyEditorDefinition::<RenderPath>::new());
    container.insert(EnumPropertyEditorDefinition::<FrustumSplitOptions>::new());
    container.insert(EnumPropertyEditorDefinition::<MaterialSearchOptions>::new());
    container.insert(EnumPropertyEditorDefinition::<DistanceModel>::new());
    container.insert(EnumPropertyEditorDefinition::<sound::Renderer>::new());
    container.insert(ScriptPropertyEditorDefinition {});

    container
}
