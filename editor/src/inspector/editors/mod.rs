use crate::inspector::editors::handle::HandlePropertyEditorDefinition;
use crate::{
    inspector::editors::{
        material::MaterialPropertyEditorDefinition,
        resource::{
            ModelResourcePropertyEditorDefinition, SoundBufferResourcePropertyEditorDefinition,
        },
        texture::TexturePropertyEditorDefinition,
    },
    Message,
};
use rg3d::scene::base::LodControlledObject;
use rg3d::{
    core::{inspect::Inspect, parking_lot::Mutex, pool::ErasedHandle, pool::Handle},
    gui::inspector::editors::{
        array::ArrayPropertyEditorDefinition, collection::VecCollectionPropertyEditorDefinition,
        enumeration::EnumPropertyEditorDefinition,
        inspectable::InspectablePropertyEditorDefinition, PropertyEditorDefinitionContainer,
    },
    physics3d::{
        self,
        desc::{ColliderShapeDesc, InteractionGroupsDesc, JointParamsDesc},
    },
    scene::{
        self,
        base::{Base, LevelOfDetail, LodGroup, Mobility, PhysicsBinding, Property, PropertyValue},
        camera::{ColorGradingLut, Exposure, SkyBox},
        light::{
            directional::{CsmOptions, FrustumSplitOptions},
            BaseLight,
        },
        mesh::{surface::Surface, RenderPath},
        node::Node,
        particle_system::emitter::{base::BaseEmitter, Emitter},
        terrain::Layer,
    },
    scene2d,
    sound::source::{generic::GenericSource, Status},
};
use std::{fmt::Debug, rc::Rc, sync::mpsc::Sender};

pub mod handle;
pub mod material;
pub mod resource;
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

pub fn make_frustum_split_options_enum_editor_definition(
) -> EnumPropertyEditorDefinition<FrustumSplitOptions> {
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => FrustumSplitOptions::default(),
            1 => FrustumSplitOptions::Relative {
                fractions: [0.33, 0.66, 1.0],
            },
            _ => unreachable!(),
        },
        index_generator: |v| match v {
            FrustumSplitOptions::Absolute { .. } => 0,
            FrustumSplitOptions::Relative { .. } => 1,
        },
        names_generator: || vec!["Absolute".to_string(), "Relative".to_string()],
    }
}

pub fn make_property_enum_editor_definition() -> EnumPropertyEditorDefinition<PropertyValue> {
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => PropertyValue::NodeHandle(Default::default()),
            1 => PropertyValue::Handle(Default::default()),
            2 => PropertyValue::String("".to_owned()),
            3 => PropertyValue::I64(0),
            4 => PropertyValue::U64(0),
            5 => PropertyValue::I32(0),
            6 => PropertyValue::U32(0),
            7 => PropertyValue::I16(0),
            8 => PropertyValue::U16(0),
            9 => PropertyValue::I8(0),
            10 => PropertyValue::U8(0),
            11 => PropertyValue::F32(0.0),
            12 => PropertyValue::F64(0.0),
            _ => unreachable!(),
        },
        index_generator: |v| match v {
            PropertyValue::NodeHandle(_) => 0,
            PropertyValue::Handle(_) => 1,
            PropertyValue::String(_) => 2,
            PropertyValue::I64(_) => 3,
            PropertyValue::U64(_) => 4,
            PropertyValue::I32(_) => 5,
            PropertyValue::U32(_) => 6,
            PropertyValue::I16(_) => 7,
            PropertyValue::U16(_) => 8,
            PropertyValue::I8(_) => 9,
            PropertyValue::U8(_) => 10,
            PropertyValue::F32(_) => 11,
            PropertyValue::F64(_) => 12,
        },
        names_generator: || {
            vec![
                "Node Handle".to_string(),
                "Handle".to_string(),
                "String".to_string(),
                "I64".to_string(),
                "U64".to_string(),
                "I32".to_string(),
                "U32".to_string(),
                "I16".to_string(),
                "U16".to_string(),
                "I8".to_string(),
                "U8".to_string(),
                "F32".to_string(),
                "F64".to_string(),
            ]
        },
    }
}

pub fn make_property_editors_container(
    sender: Sender<Message>,
) -> Rc<PropertyEditorDefinitionContainer> {
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
    container.insert(make_physics_binding_enum_editor_definition());
    container.insert(make_mobility_enum_editor_definition());
    container.insert(make_exposure_enum_editor_definition());
    container.insert(make_render_path_enum_editor_definition());
    container.insert(make_status_enum_editor_definition());
    container.insert(make_rigid_body_type_editor_definition());
    container.insert(make_option_editor_definition::<f32>());
    container.insert(make_option_editor_definition::<LodGroup>());
    container.insert(make_property_enum_editor_definition());
    container.insert(ModelResourcePropertyEditorDefinition);
    container.insert(SoundBufferResourcePropertyEditorDefinition);
    container.insert(InspectablePropertyEditorDefinition::<InteractionGroupsDesc>::new());
    container.insert(InspectablePropertyEditorDefinition::<ColliderShapeDesc>::new());
    container.insert(InspectablePropertyEditorDefinition::<JointParamsDesc>::new());
    container.insert(InspectablePropertyEditorDefinition::<Base>::new());
    container.insert(InspectablePropertyEditorDefinition::<scene2d::base::Base>::new());
    container.insert(InspectablePropertyEditorDefinition::<BaseLight>::new());
    container.insert(InspectablePropertyEditorDefinition::<
        scene2d::light::BaseLight,
    >::new());
    container.insert(InspectablePropertyEditorDefinition::<BaseEmitter>::new());
    container.insert(InspectablePropertyEditorDefinition::<
        scene::transform::Transform,
    >::new());
    container.insert(InspectablePropertyEditorDefinition::<
        scene2d::transform::Transform,
    >::new());
    container.insert(InspectablePropertyEditorDefinition::<GenericSource>::new());
    container.insert(InspectablePropertyEditorDefinition::<CsmOptions>::new());
    container.insert(make_frustum_split_options_enum_editor_definition());
    container.insert(ArrayPropertyEditorDefinition::<f32, 3>::new());
    container.insert(make_option_editor_definition::<ColorGradingLut>());
    container.insert(make_option_editor_definition::<Box<SkyBox>>());
    container.insert(HandlePropertyEditorDefinition::<Node>::new(sender));

    Rc::new(container)
}
