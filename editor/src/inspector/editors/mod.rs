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
use rg3d::{
    core::{inspect::Inspect, parking_lot::Mutex, pool::ErasedHandle, pool::Handle},
    gui::inspector::editors::{
        array::ArrayPropertyEditorDefinition, collection::VecCollectionPropertyEditorDefinition,
        enumeration::EnumPropertyEditorDefinition,
        inspectable::InspectablePropertyEditorDefinition, PropertyEditorDefinitionContainer,
    },
    scene::{
        self,
        base::{
            Base, LevelOfDetail, LodControlledObject, LodGroup, Mobility, Property, PropertyValue,
        },
        camera::{ColorGradingLut, Exposure, SkyBox},
        collider::{ColliderShapeDesc, InteractionGroupsDesc},
        joint::*,
        light::{
            directional::{CsmOptions, FrustumSplitOptions},
            BaseLight,
        },
        mesh::{surface::Surface, RenderPath},
        node::Node,
        particle_system::emitter::{base::BaseEmitter, Emitter},
        rigidbody::RigidBodyTypeDesc,
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

pub fn make_rigid_body_type_editor_definition() -> EnumPropertyEditorDefinition<RigidBodyTypeDesc> {
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => RigidBodyTypeDesc::Dynamic,
            1 => RigidBodyTypeDesc::Static,
            2 => RigidBodyTypeDesc::KinematicPositionBased,
            3 => RigidBodyTypeDesc::KinematicVelocityBased,
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
    T: Inspect + Default + Debug + 'static,
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

pub fn make_shape_property_editor_definition() -> EnumPropertyEditorDefinition<ColliderShapeDesc> {
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => ColliderShapeDesc::Ball(Default::default()),
            1 => ColliderShapeDesc::Cylinder(Default::default()),
            2 => ColliderShapeDesc::RoundCylinder(Default::default()),
            3 => ColliderShapeDesc::Cone(Default::default()),
            4 => ColliderShapeDesc::Cuboid(Default::default()),
            5 => ColliderShapeDesc::Capsule(Default::default()),
            6 => ColliderShapeDesc::Segment(Default::default()),
            7 => ColliderShapeDesc::Triangle(Default::default()),
            8 => ColliderShapeDesc::Trimesh(Default::default()),
            9 => ColliderShapeDesc::Heightfield(Default::default()),
            _ => unreachable!(),
        },
        index_generator: |v| match v {
            ColliderShapeDesc::Ball(_) => 0,
            ColliderShapeDesc::Cylinder(_) => 1,
            ColliderShapeDesc::RoundCylinder(_) => 2,
            ColliderShapeDesc::Cone(_) => 3,
            ColliderShapeDesc::Cuboid(_) => 4,
            ColliderShapeDesc::Capsule(_) => 5,
            ColliderShapeDesc::Segment(_) => 6,
            ColliderShapeDesc::Triangle(_) => 7,
            ColliderShapeDesc::Trimesh(_) => 8,
            ColliderShapeDesc::Heightfield(_) => 9,
        },
        names_generator: || {
            vec![
                "Ball".to_string(),
                "Cylinder".to_string(),
                "RoundCylinder".to_string(),
                "Cone".to_string(),
                "Cuboid".to_string(),
                "Capsule".to_string(),
                "Segment".to_string(),
                "Triangle".to_string(),
                "Trimesh".to_string(),
                "Heightfield".to_string(),
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
    container.insert(make_shape_property_editor_definition());

    Rc::new(container)
}
