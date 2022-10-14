use crate::{
    inspector::editors::{
        handle::NodeHandlePropertyEditorDefinition, material::MaterialPropertyEditorDefinition,
        resource::ResourceFieldPropertyEditorDefinition, script::ScriptPropertyEditorDefinition,
        spritesheet::SpriteSheetFramesContainerEditorDefinition,
        texture::TexturePropertyEditorDefinition,
    },
    Message,
};
use fyrox::{
    animation::{self, machine::MachineInstantiationError, spritesheet::FrameBounds},
    core::{
        futures::executor::block_on,
        parking_lot::Mutex,
        pool::{ErasedHandle, Handle},
    },
    gui::inspector::editors::{
        bit::BitFieldPropertyEditorDefinition, enumeration::EnumPropertyEditorDefinition,
        inherit::InheritablePropertyEditorDefinition, PropertyEditorDefinitionContainer,
    },
    material::shader::{Shader, ShaderError, ShaderState},
    resource::{
        absm::{AbsmResource, AbsmResourceState},
        curve::{CurveResource, CurveResourceError, CurveResourceState},
        model::{MaterialSearchOptions, Model, ModelData, ModelLoadError},
        texture::{
            CompressionOptions, Texture, TextureMagnificationFilter, TextureMinificationFilter,
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
        collider::{
            BallShape, BitMask, CapsuleShape, ColliderShape, ConeShape, ConvexPolyhedronShape,
            CuboidShape, CylinderShape, GeometrySource, HeightfieldShape, InteractionGroups,
            SegmentShape, TriangleShape, TrimeshShape,
        },
        dim2,
        graph::physics::CoefficientCombineRule,
        joint::*,
        light::{
            directional::{CsmOptions, FrustumSplitOptions},
            BaseLight,
        },
        mesh::{surface::Surface, RenderPath},
        node::{Node, NodeHandle},
        particle_system::{
            emitter::{
                base::BaseEmitter, cuboid::CuboidEmitter, cylinder::CylinderEmitter,
                sphere::SphereEmitter, Emitter,
            },
            EmitterWrapper,
        },
        rigidbody::RigidBodyType,
        sound::{
            self,
            effect::{BaseEffect, Effect, EffectInput, ReverbEffect},
            Biquad, DistanceModel, SoundBufferResource, SoundBufferResourceLoadError,
            SoundBufferState, Status,
        },
        terrain::Layer,
        transform::Transform,
    },
};
use std::{rc::Rc, sync::mpsc::Sender};

pub mod handle;
pub mod material;
pub mod resource;
pub mod script;
pub mod spritesheet;
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
    let container = PropertyEditorDefinitionContainer::new();

    container.insert(TexturePropertyEditorDefinition);
    container.insert(InheritablePropertyEditorDefinition::<Option<Texture>>::new());
    container.insert(InheritablePropertyEditorDefinition::<Handle<Node>>::new());

    container.insert(MaterialPropertyEditorDefinition {
        sender: Mutex::new(sender.clone()),
    });

    container.register_inheritable_vec_collection::<Handle<Node>>();
    container.insert(NodeHandlePropertyEditorDefinition::new(sender));
    container.register_inheritable_inspectable::<NodeHandle>();
    container.register_inheritable_vec_collection::<NodeHandle>();

    container.register_inheritable_vec_collection::<Surface>();
    container.register_inheritable_vec_collection::<Layer>();
    container.register_inheritable_vec_collection::<EmitterWrapper>();
    container.register_inheritable_vec_collection::<LevelOfDetail>();
    container.register_inheritable_vec_collection::<ErasedHandle>();
    container.register_inheritable_vec_collection::<Property>();
    container.register_inheritable_vec_collection::<LodControlledObject>();
    container.register_inheritable_vec_collection::<GeometrySource>();
    container.register_inheritable_vec_collection::<EffectInput>();

    container.insert(make_status_enum_editor_definition());

    container.insert(EnumPropertyEditorDefinition::<LodGroup>::new_optional());
    container.insert(InheritablePropertyEditorDefinition::<Option<LodGroup>>::new());

    container.register_inheritable_enum::<animation::spritesheet::Status, _>();

    container.register_inheritable_inspectable::<LodGroup>();

    container.register_inheritable_inspectable::<animation::spritesheet::SpriteSheetAnimation>();
    container.register_inheritable_vec_collection::<animation::spritesheet::SpriteSheetAnimation>();

    container.register_inheritable_inspectable::<FrameBounds>();
    container.register_inheritable_vec_collection::<FrameBounds>();

    container.register_inheritable_inspectable::<animation::spritesheet::signal::Signal>();
    container.register_inheritable_vec_collection::<animation::spritesheet::signal::Signal>();

    container.insert(ResourceFieldPropertyEditorDefinition::<
        Model,
        ModelData,
        ModelLoadError,
    >::new(Rc::new(|resource_manager, path| {
        block_on(resource_manager.request_model(path))
    })));
    container.insert(InheritablePropertyEditorDefinition::<Option<Model>>::new());

    container.insert(ResourceFieldPropertyEditorDefinition::<
        SoundBufferResource,
        SoundBufferState,
        SoundBufferResourceLoadError,
    >::new(Rc::new(|resource_manager, path| {
        block_on(resource_manager.request_sound_buffer(path))
    })));
    container.insert(InheritablePropertyEditorDefinition::<
        Option<SoundBufferResource>,
    >::new());

    container.insert(ResourceFieldPropertyEditorDefinition::<
        AbsmResource,
        AbsmResourceState,
        MachineInstantiationError,
    >::new(Rc::new(|resource_manager, path| {
        block_on(resource_manager.request_absm(path))
    })));
    container.insert(InheritablePropertyEditorDefinition::<Option<AbsmResource>>::new());

    container.insert(ResourceFieldPropertyEditorDefinition::<
        CurveResource,
        CurveResourceState,
        CurveResourceError,
    >::new(Rc::new(|resource_manager, path| {
        block_on(resource_manager.request_curve(path))
    })));
    container.insert(InheritablePropertyEditorDefinition::<Option<CurveResource>>::new());

    container.insert(ResourceFieldPropertyEditorDefinition::<
        Shader,
        ShaderState,
        ShaderError,
    >::new(Rc::new(|resource_manager, path| {
        block_on(resource_manager.request_shader(path))
    })));
    container.insert(InheritablePropertyEditorDefinition::<Option<Shader>>::new());

    container.register_inheritable_inspectable::<ColorGradingLut>();
    container.register_inheritable_inspectable::<InteractionGroups>();
    container.register_inheritable_inspectable::<GeometrySource>();

    container.register_inheritable_enum::<JointParams, _>();
    container.register_inheritable_enum::<dim2::joint::JointParams, _>();

    container.register_inheritable_inspectable::<BallJoint>();
    container.register_inheritable_inspectable::<dim2::joint::BallJoint>();
    container.register_inheritable_inspectable::<FixedJoint>();
    container.register_inheritable_inspectable::<dim2::joint::FixedJoint>();
    container.register_inheritable_inspectable::<RevoluteJoint>();
    container.register_inheritable_inspectable::<PrismaticJoint>();
    container.register_inheritable_inspectable::<dim2::joint::PrismaticJoint>();

    container.register_inheritable_inspectable::<Base>();
    container.register_inheritable_inspectable::<BaseEffect>();
    container.register_inheritable_inspectable::<BaseLight>();

    container.register_inheritable_enum::<Effect, _>();
    container.register_inheritable_enum::<Emitter, _>();

    container.register_inheritable_inspectable::<ReverbEffect>();
    container.register_inheritable_inspectable::<Biquad>();
    container.register_inheritable_inspectable::<BaseEmitter>();
    container.register_inheritable_inspectable::<SphereEmitter>();
    container.register_inheritable_inspectable::<CylinderEmitter>();
    container.register_inheritable_inspectable::<CuboidEmitter>();
    container.register_inheritable_inspectable::<PerspectiveProjection>();
    container.register_inheritable_inspectable::<OrthographicProjection>();
    container.register_inheritable_inspectable::<Transform>();
    container.register_inheritable_inspectable::<CsmOptions>();

    container.register_inheritable_option::<ColorGradingLut>();
    container.register_inheritable_option::<Biquad>();
    container.register_inheritable_option::<SkyBox>();

    container.register_inheritable_inspectable::<SkyBox>();

    container.register_inheritable_enum::<dim2::collider::ColliderShape, _>();
    container.register_inheritable_enum::<CoefficientCombineRule, _>();
    container.register_inheritable_enum::<CompressionOptions, _>();
    container.register_inheritable_enum::<TextureWrapMode, _>();
    container.register_inheritable_enum::<TextureMagnificationFilter, _>();
    container.register_inheritable_enum::<TextureMinificationFilter, _>();
    container.register_inheritable_enum::<Projection, _>();
    container.register_inheritable_enum::<ColliderShape, _>();
    container.register_inheritable_enum::<PropertyValue, _>();
    container.register_inheritable_enum::<Mobility, _>();
    container.register_inheritable_enum::<RigidBodyType, _>();
    container.register_inheritable_enum::<Exposure, _>();
    container.register_inheritable_enum::<FrustumSplitOptions, _>();
    container.register_inheritable_enum::<MaterialSearchOptions, _>();
    container.register_inheritable_enum::<DistanceModel, _>();
    container.register_inheritable_enum::<sound::Renderer, _>();
    container.register_inheritable_enum::<RenderPath, _>();

    container.insert(ScriptPropertyEditorDefinition {});
    container.insert(BitFieldPropertyEditorDefinition::<BitMask>::new());

    container.register_inheritable_inspectable::<BallShape>();
    container.register_inheritable_inspectable::<dim2::collider::BallShape>();
    container.register_inheritable_inspectable::<CylinderShape>();
    container.register_inheritable_inspectable::<ConeShape>();
    container.register_inheritable_inspectable::<CuboidShape>();
    container.register_inheritable_inspectable::<dim2::collider::CuboidShape>();
    container.register_inheritable_inspectable::<CapsuleShape>();
    container.register_inheritable_inspectable::<dim2::collider::CapsuleShape>();
    container.register_inheritable_inspectable::<SegmentShape>();
    container.register_inheritable_inspectable::<dim2::collider::SegmentShape>();
    container.register_inheritable_inspectable::<TriangleShape>();
    container.register_inheritable_inspectable::<dim2::collider::TriangleShape>();
    container.register_inheritable_inspectable::<TrimeshShape>();
    container.register_inheritable_inspectable::<dim2::collider::TrimeshShape>();
    container.register_inheritable_inspectable::<HeightfieldShape>();
    container.register_inheritable_inspectable::<dim2::collider::HeightfieldShape>();
    container.register_inheritable_inspectable::<ConvexPolyhedronShape>();
    container.insert(SpriteSheetFramesContainerEditorDefinition);

    container
}
