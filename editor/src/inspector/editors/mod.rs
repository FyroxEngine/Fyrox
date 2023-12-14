use crate::inspector::editors::handle::EntityKind;
use crate::{
    inspector::editors::{
        animation::{
            AnimationContainerPropertyEditorDefinition, AnimationPropertyEditorDefinition,
            MachinePropertyEditorDefinition,
        },
        font::FontPropertyEditorDefinition,
        handle::NodeHandlePropertyEditorDefinition,
        material::MaterialPropertyEditorDefinition,
        resource::ResourceFieldPropertyEditorDefinition,
        script::ScriptPropertyEditorDefinition,
        spritesheet::SpriteSheetFramesContainerEditorDefinition,
        surface::SurfaceDataPropertyEditorDefinition,
        texture::TexturePropertyEditorDefinition,
    },
    message::MessageSender,
};
use fyrox::{
    animation::{
        machine::{
            node::{
                blendspace::{BlendSpace, BlendSpacePoint},
                BasePoseNode,
            },
            state::{StateAction, StateActionWrapper},
            transition::{AndNode, LogicNode, NotNode, OrNode, XorNode},
            BlendAnimations, BlendAnimationsByIndex, BlendPose, IndexedBlendInput, Machine,
            PlayAnimation, PoseNode, PoseWeight, State,
        },
        Animation, AnimationContainer,
    },
    core::{
        futures::executor::block_on,
        parking_lot::Mutex,
        pool::{ErasedHandle, Handle},
    },
    gui::inspector::editors::{
        bit::BitFieldPropertyEditorDefinition, collection::VecCollectionPropertyEditorDefinition,
        enumeration::EnumPropertyEditorDefinition, inherit::InheritablePropertyEditorDefinition,
        inspectable::InspectablePropertyEditorDefinition, path::PathPropertyEditorDefinition,
        PropertyEditorDefinitionContainer,
    },
    material::{
        shader::{Shader, ShaderResource},
        MaterialResource,
    },
    renderer::framework::state::PolygonFillMode,
    resource::{
        curve::{CurveResource, CurveResourceState},
        model::{MaterialSearchOptions, Model, ModelResource},
        texture::{
            CompressionOptions, MipFilter, TextureMagnificationFilter, TextureMinificationFilter,
            TextureResource, TextureWrapMode,
        },
    },
    scene::{
        base::{Base, LevelOfDetail, LodGroup, Mobility, Property, PropertyValue},
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
        mesh::{
            surface::{BlendShape, Surface, SurfaceSharedData},
            RenderPath,
        },
        node::Node,
        particle_system::{
            emitter::{
                base::BaseEmitter, cuboid::CuboidEmitter, cylinder::CylinderEmitter,
                sphere::SphereEmitter, Emitter,
            },
            ParticleSystemRng,
        },
        ragdoll::Limb,
        rigidbody::RigidBodyType,
        sound::{
            self,
            filter::{
                AllPassFilterEffect, BandPassFilterEffect, HighPassFilterEffect,
                HighShelfFilterEffect, LowPassFilterEffect, LowShelfFilterEffect,
            },
            reverb::Reverb,
            Attenuate, AudioBus, Biquad, DistanceModel, Effect, SoundBuffer, SoundBufferResource,
            Status,
        },
        terrain::{Chunk, Layer},
        transform::Transform,
    },
};
use std::rc::Rc;

pub mod animation;
pub mod font;
pub mod handle;
pub mod material;
pub mod resource;
pub mod script;
pub mod spritesheet;
pub mod surface;
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

pub fn make_property_editors_container(sender: MessageSender) -> PropertyEditorDefinitionContainer {
    let container = PropertyEditorDefinitionContainer::new();

    container.insert(TexturePropertyEditorDefinition { untyped: false });
    container.insert(TexturePropertyEditorDefinition { untyped: true });
    container.insert(FontPropertyEditorDefinition);
    container.insert(InheritablePropertyEditorDefinition::<Option<TextureResource>>::new());
    container.register_inheritable_vec_collection::<Option<TextureResource>>();

    container.insert(InheritablePropertyEditorDefinition::<Handle<Node>>::new());

    container.insert(MaterialPropertyEditorDefinition {
        sender: Mutex::new(sender.clone()),
    });
    container.insert(InheritablePropertyEditorDefinition::<MaterialResource>::new());

    container.register_inheritable_vec_collection::<Handle<Node>>();
    container.insert(NodeHandlePropertyEditorDefinition::new(
        sender.clone(),
        EntityKind::SceneNode,
    ));
    container.insert(NodeHandlePropertyEditorDefinition::new(
        sender.clone(),
        EntityKind::UiNode,
    ));

    container.register_inheritable_vec_collection::<Surface>();
    container.register_inheritable_inspectable::<Surface>();

    container.register_inheritable_vec_collection::<Layer>();
    container.register_inheritable_inspectable::<Layer>();

    container.register_inheritable_vec_collection::<Emitter>();

    container.register_inheritable_vec_collection::<LevelOfDetail>();
    container.register_inheritable_inspectable::<LevelOfDetail>();

    container.register_inheritable_vec_collection::<ErasedHandle>();
    container.register_inheritable_inspectable::<ErasedHandle>();

    container.register_inheritable_vec_collection::<Property>();
    container.register_inheritable_inspectable::<Property>();

    container.register_inheritable_vec_collection::<GeometrySource>();
    container.register_inheritable_inspectable::<GeometrySource>();

    container.insert(make_status_enum_editor_definition());

    container.insert(EnumPropertyEditorDefinition::<LodGroup>::new_optional());
    container.insert(InheritablePropertyEditorDefinition::<Option<LodGroup>>::new());

    container.register_inheritable_enum::<fyrox::animation::spritesheet::Status, _>();

    container.register_inheritable_inspectable::<LodGroup>();

    container
        .register_inheritable_inspectable::<fyrox::animation::spritesheet::SpriteSheetAnimation>();
    container
        .register_inheritable_vec_collection::<fyrox::animation::spritesheet::SpriteSheetAnimation>(
        );

    container.register_inheritable_inspectable::<fyrox::animation::spritesheet::signal::Signal>();
    container
        .register_inheritable_vec_collection::<fyrox::animation::spritesheet::signal::Signal>();

    container.insert(ResourceFieldPropertyEditorDefinition::<Model>::new(
        Rc::new(|resource_manager, path| resource_manager.try_request::<Model>(path).map(block_on)),
        sender.clone(),
    ));
    container.insert(InheritablePropertyEditorDefinition::<Option<ModelResource>>::new());
    container.register_inheritable_vec_collection::<Option<ModelResource>>();

    container.insert(ResourceFieldPropertyEditorDefinition::<SoundBuffer>::new(
        Rc::new(|resource_manager, path| {
            resource_manager
                .try_request::<SoundBuffer>(path)
                .map(block_on)
        }),
        sender.clone(),
    ));
    container.insert(InheritablePropertyEditorDefinition::<
        Option<SoundBufferResource>,
    >::new());
    container.register_inheritable_vec_collection::<Option<SoundBufferResource>>();

    container.insert(
        ResourceFieldPropertyEditorDefinition::<CurveResourceState>::new(
            Rc::new(|resource_manager, path| {
                resource_manager
                    .try_request::<CurveResourceState>(path)
                    .map(block_on)
            }),
            sender.clone(),
        ),
    );
    container.insert(InheritablePropertyEditorDefinition::<Option<CurveResource>>::new());
    container.register_inheritable_vec_collection::<Option<CurveResource>>();

    container.insert(ResourceFieldPropertyEditorDefinition::<Shader>::new(
        Rc::new(|resource_manager, path| {
            resource_manager.try_request::<Shader>(path).map(block_on)
        }),
        sender,
    ));
    container.insert(InheritablePropertyEditorDefinition::<Option<ShaderResource>>::new());
    container.register_inheritable_vec_collection::<Option<ShaderResource>>();

    container.register_inheritable_inspectable::<ColorGradingLut>();
    container.register_inheritable_inspectable::<InteractionGroups>();

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
    container.register_inheritable_inspectable::<BaseLight>();

    container.insert(EnumPropertyEditorDefinition::<Effect>::new());
    container.insert(VecCollectionPropertyEditorDefinition::<Effect>::new());

    container.insert(InspectablePropertyEditorDefinition::<Attenuate>::new());
    container.insert(InspectablePropertyEditorDefinition::<LowPassFilterEffect>::new());
    container.insert(InspectablePropertyEditorDefinition::<HighPassFilterEffect>::new());
    container.insert(InspectablePropertyEditorDefinition::<AllPassFilterEffect>::new());
    container.insert(InspectablePropertyEditorDefinition::<BandPassFilterEffect>::new());
    container.insert(InspectablePropertyEditorDefinition::<LowShelfFilterEffect>::new());
    container.insert(InspectablePropertyEditorDefinition::<HighShelfFilterEffect>::new());
    container.insert(InspectablePropertyEditorDefinition::<Reverb>::new());

    container.register_inheritable_enum::<Emitter, _>();

    container.register_inheritable_inspectable::<Biquad>();
    container.register_inheritable_inspectable::<AudioBus>();
    container.register_inheritable_inspectable::<BaseEmitter>();
    container.register_inheritable_inspectable::<SphereEmitter>();
    container.register_inheritable_inspectable::<CylinderEmitter>();
    container.register_inheritable_inspectable::<CuboidEmitter>();
    container.register_inheritable_inspectable::<PerspectiveProjection>();
    container.register_inheritable_inspectable::<OrthographicProjection>();
    container.register_inheritable_inspectable::<Transform>();
    container.register_inheritable_inspectable::<CsmOptions>();

    container.register_inheritable_inspectable::<Chunk>();
    container.register_inheritable_vec_collection::<Chunk>();

    container.register_inheritable_vec_collection::<BlendShape>();
    container.register_inheritable_inspectable::<BlendShape>();

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

    container.insert(SurfaceDataPropertyEditorDefinition);
    container.insert(InheritablePropertyEditorDefinition::<SurfaceSharedData>::new());
    container.insert(InheritablePropertyEditorDefinition::<Status>::new());

    container.insert(InspectablePropertyEditorDefinition::<BasePoseNode>::new());
    container.insert(InspectablePropertyEditorDefinition::<IndexedBlendInput>::new());
    container.insert(VecCollectionPropertyEditorDefinition::<IndexedBlendInput>::new());
    container.insert(InspectablePropertyEditorDefinition::<BlendSpacePoint>::new());
    container.insert(VecCollectionPropertyEditorDefinition::<BlendSpacePoint>::new());
    container.insert(InspectablePropertyEditorDefinition::<BlendPose>::new());
    container.insert(VecCollectionPropertyEditorDefinition::<BlendPose>::new());
    container.insert(EnumPropertyEditorDefinition::<PoseWeight>::new());
    container.insert(EnumPropertyEditorDefinition::<StateAction>::new());
    container.insert(InspectablePropertyEditorDefinition::<StateActionWrapper>::new());
    container.insert(VecCollectionPropertyEditorDefinition::<StateActionWrapper>::new());
    container.insert(InspectablePropertyEditorDefinition::<BlendAnimationsByIndex>::new());
    container.insert(InspectablePropertyEditorDefinition::<BlendAnimations>::new());
    container.insert(InspectablePropertyEditorDefinition::<BlendSpace>::new());
    container.insert(InspectablePropertyEditorDefinition::<PlayAnimation>::new());

    container.insert(InspectablePropertyEditorDefinition::<Handle<PoseNode>>::new());
    container.insert(InspectablePropertyEditorDefinition::<Handle<State>>::new());

    container.insert(VecCollectionPropertyEditorDefinition::<Handle<Animation>>::new());
    container.insert(AnimationPropertyEditorDefinition);

    container.insert(AnimationContainerPropertyEditorDefinition);
    container.insert(InheritablePropertyEditorDefinition::<AnimationContainer>::new());

    container.insert(MachinePropertyEditorDefinition);
    container.insert(InheritablePropertyEditorDefinition::<Machine>::new());

    container.insert(EnumPropertyEditorDefinition::<LogicNode>::new());
    container.insert(InspectablePropertyEditorDefinition::<AndNode>::new());
    container.insert(InspectablePropertyEditorDefinition::<OrNode>::new());
    container.insert(InspectablePropertyEditorDefinition::<XorNode>::new());
    container.insert(InspectablePropertyEditorDefinition::<NotNode>::new());

    container.insert(InspectablePropertyEditorDefinition::<ParticleSystemRng>::new());
    container.insert(EnumPropertyEditorDefinition::<PolygonFillMode>::new());

    container.insert(EnumPropertyEditorDefinition::<MipFilter>::new());

    container.register_inheritable_inspectable::<Limb>();
    container.insert(VecCollectionPropertyEditorDefinition::<Limb>::new());

    container.insert(PathPropertyEditorDefinition);

    container
}
