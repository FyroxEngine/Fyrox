use crate::{
    fyrox::{
        asset::{untyped::UntypedResource, Resource},
        core::{
            parking_lot::Mutex,
            pool::{ErasedHandle, Handle},
        },
        gui::{
            self,
            font::FontResource,
            inspector::editors::{
                bit::BitFieldPropertyEditorDefinition,
                collection::VecCollectionPropertyEditorDefinition,
                enumeration::EnumPropertyEditorDefinition,
                inherit::InheritablePropertyEditorDefinition,
                inspectable::InspectablePropertyEditorDefinition,
                PropertyEditorDefinitionContainer,
            },
            UiNode, UserInterface,
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
                CompressionOptions, MipFilter, TextureMagnificationFilter,
                TextureMinificationFilter, TextureResource, TextureWrapMode,
            },
        },
        scene::{
            self,
            base::{
                Base, LevelOfDetail, LodGroup, Mobility, Property, PropertyValue, ScriptRecord,
            },
            camera::{
                ColorGradingLut, Exposure, OrthographicProjection, PerspectiveProjection,
                Projection, SkyBox,
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
                surface::{BlendShape, Surface, SurfaceResource},
                BatchingMode, RenderPath,
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
                Attenuate, AudioBus, Biquad, DistanceModel, Effect, SoundBuffer,
                SoundBufferResource, Status,
            },
            terrain::{Chunk, Layer},
            tilemap::{tileset::TileSet, Tile},
            transform::Transform,
        },
    },
    inspector::editors::{
        animation::{
            AnimationContainerPropertyEditorDefinition, AnimationPropertyEditorDefinition,
            MachinePropertyEditorDefinition,
        },
        font::FontPropertyEditorDefinition,
        handle::{EntityKind, NodeHandlePropertyEditorDefinition},
        material::MaterialPropertyEditorDefinition,
        resource::ResourceFieldPropertyEditorDefinition,
        script::ScriptPropertyEditorDefinition,
        spritesheet::SpriteSheetFramesContainerEditorDefinition,
        surface::SurfaceDataPropertyEditorDefinition,
        texture::TexturePropertyEditorDefinition,
    },
    message::MessageSender,
};
use fyrox::scene::tilemap::brush::{TileMapBrush, TileMapBrushResource};
use fyrox::scene::tilemap::tileset::TileCollider;

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

fn register_absm_property_editors<T>(container: &PropertyEditorDefinitionContainer)
where
    T: 'static,
{
    use crate::fyrox::generic_animation::machine::{
        node::{
            blendspace::{BlendSpace, BlendSpacePoint},
            BasePoseNode,
        },
        state::{StateAction, StateActionWrapper},
        transition::{AndNode, LogicNode, NotNode, OrNode, XorNode},
        BlendAnimations, BlendAnimationsByIndex, BlendPose, IndexedBlendInput, Machine,
        PlayAnimation, PoseNode, PoseWeight, State,
    };

    container.insert(InspectablePropertyEditorDefinition::<BasePoseNode<Handle<T>>>::new());
    container.insert(InspectablePropertyEditorDefinition::<
        IndexedBlendInput<Handle<T>>,
    >::new());
    container.insert(VecCollectionPropertyEditorDefinition::<
        IndexedBlendInput<Handle<T>>,
    >::new());
    container.insert(InspectablePropertyEditorDefinition::<
        BlendSpacePoint<Handle<T>>,
    >::new());
    container.insert(VecCollectionPropertyEditorDefinition::<
        BlendSpacePoint<Handle<T>>,
    >::new());
    container.insert(InspectablePropertyEditorDefinition::<BlendPose<Handle<T>>>::new());
    container.insert(VecCollectionPropertyEditorDefinition::<BlendPose<Handle<T>>>::new());
    container.insert(EnumPropertyEditorDefinition::<PoseWeight>::new());
    container.insert(EnumPropertyEditorDefinition::<StateAction<Handle<T>>>::new());
    container.insert(InspectablePropertyEditorDefinition::<
        StateActionWrapper<Handle<T>>,
    >::new());
    container.insert(VecCollectionPropertyEditorDefinition::<
        StateActionWrapper<Handle<T>>,
    >::new());
    container.insert(InspectablePropertyEditorDefinition::<
        BlendAnimationsByIndex<Handle<T>>,
    >::new());
    container.insert(InspectablePropertyEditorDefinition::<
        BlendAnimations<Handle<T>>,
    >::new());
    container.insert(InspectablePropertyEditorDefinition::<BlendSpace<Handle<T>>>::new());
    container.insert(InspectablePropertyEditorDefinition::<
        PlayAnimation<Handle<T>>,
    >::new());

    container.insert(InspectablePropertyEditorDefinition::<
        Handle<PoseNode<Handle<T>>>,
    >::new());
    container.insert(InspectablePropertyEditorDefinition::<
        Handle<State<Handle<T>>>,
    >::new());

    container.insert(MachinePropertyEditorDefinition::<Handle<T>>::default());
    container.insert(InheritablePropertyEditorDefinition::<Machine<Handle<T>>>::new());

    container.insert(EnumPropertyEditorDefinition::<LogicNode<Handle<T>>>::new());
    container.insert(InspectablePropertyEditorDefinition::<AndNode<Handle<T>>>::new());
    container.insert(InspectablePropertyEditorDefinition::<OrNode<Handle<T>>>::new());
    container.insert(InspectablePropertyEditorDefinition::<XorNode<Handle<T>>>::new());
    container.insert(InspectablePropertyEditorDefinition::<NotNode<Handle<T>>>::new());
}

pub fn make_property_editors_container(sender: MessageSender) -> PropertyEditorDefinitionContainer {
    let container = PropertyEditorDefinitionContainer::with_default_editors();

    container.insert(TexturePropertyEditorDefinition { untyped: false });
    container.insert(TexturePropertyEditorDefinition { untyped: true });
    container.insert(FontPropertyEditorDefinition);
    container.insert(InheritablePropertyEditorDefinition::<FontResource>::new());
    container.insert(InheritablePropertyEditorDefinition::<Option<TextureResource>>::new());
    container.insert(InheritablePropertyEditorDefinition::<Option<UntypedResource>>::new());
    container.register_inheritable_vec_collection::<Option<TextureResource>>();
    container.register_inheritable_vec_collection::<Option<UntypedResource>>();

    container.insert(MaterialPropertyEditorDefinition {
        sender: Mutex::new(sender.clone()),
    });
    container.insert(InheritablePropertyEditorDefinition::<MaterialResource>::new());

    container.insert(InheritablePropertyEditorDefinition::<Handle<Node>>::new());
    container.register_inheritable_vec_collection::<Handle<Node>>();
    container.insert(NodeHandlePropertyEditorDefinition::new(
        sender.clone(),
        EntityKind::SceneNode,
    ));

    container.insert(InheritablePropertyEditorDefinition::<Handle<UiNode>>::new());
    container.register_inheritable_vec_collection::<Handle<UiNode>>();
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

    container.register_inheritable_vec_collection::<dim2::collider::GeometrySource>();
    container.register_inheritable_inspectable::<dim2::collider::GeometrySource>();

    container.insert(make_status_enum_editor_definition());

    container.insert(EnumPropertyEditorDefinition::<LodGroup>::new_optional());
    container.insert(InheritablePropertyEditorDefinition::<Option<LodGroup>>::new());

    {
        use crate::fyrox::scene::animation::spritesheet::prelude::*;
        container.register_inheritable_enum::<Status, _>();
        container.register_inheritable_inspectable::<LodGroup>();
        container.register_inheritable_inspectable::<SpriteSheetAnimation>();
        container.register_inheritable_vec_collection::<SpriteSheetAnimation>();
        container.register_inheritable_inspectable::<Signal>();
        container.register_inheritable_vec_collection::<Signal>();
    }

    container.insert(ResourceFieldPropertyEditorDefinition::<Model>::new(
        sender.clone(),
    ));
    container.insert(InheritablePropertyEditorDefinition::<Option<ModelResource>>::new());
    container.register_inheritable_vec_collection::<Option<ModelResource>>();

    container.insert(ResourceFieldPropertyEditorDefinition::<SoundBuffer>::new(
        sender.clone(),
    ));
    container.insert(InheritablePropertyEditorDefinition::<
        Option<SoundBufferResource>,
    >::new());
    container.register_inheritable_vec_collection::<Option<SoundBufferResource>>();

    container
        .insert(ResourceFieldPropertyEditorDefinition::<CurveResourceState>::new(sender.clone()));
    container.insert(InheritablePropertyEditorDefinition::<Option<CurveResource>>::new());
    container.register_inheritable_vec_collection::<Option<CurveResource>>();

    container.insert(ResourceFieldPropertyEditorDefinition::<UserInterface>::new(
        sender.clone(),
    ));
    container.insert(InheritablePropertyEditorDefinition::<
        Option<Resource<UserInterface>>,
    >::new());
    container.register_inheritable_vec_collection::<Option<UserInterface>>();

    container.insert(ResourceFieldPropertyEditorDefinition::<TileSet>::new(
        sender.clone(),
    ));
    container.insert(InheritablePropertyEditorDefinition::<
        Option<Resource<TileSet>>,
    >::new());
    container.register_inheritable_vec_collection::<Option<TileSet>>();

    container.insert(ResourceFieldPropertyEditorDefinition::<Shader>::new(
        sender.clone(),
    ));
    container.insert(InheritablePropertyEditorDefinition::<Option<ShaderResource>>::new());
    container.register_inheritable_vec_collection::<Option<ShaderResource>>();

    container.insert(ResourceFieldPropertyEditorDefinition::<TileMapBrush>::new(
        sender.clone(),
    ));
    container.insert(InheritablePropertyEditorDefinition::<
        Option<TileMapBrushResource>,
    >::new());
    container.register_inheritable_vec_collection::<Option<TileMapBrushResource>>();
    container.register_inheritable_inspectable::<TileMapBrush>();

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

    container.insert(EnumPropertyEditorDefinition::<Vec<ScriptRecord>>::new_optional());
    container.insert(VecCollectionPropertyEditorDefinition::<ScriptRecord>::new());
    container.insert(InspectablePropertyEditorDefinition::<ScriptRecord>::new());
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
    container.register_inheritable_inspectable::<dim2::collider::TileMapShape>();
    container.register_inheritable_inspectable::<ConvexPolyhedronShape>();
    container.insert(SpriteSheetFramesContainerEditorDefinition);

    container.insert(SurfaceDataPropertyEditorDefinition {
        sender: sender.clone(),
    });
    container.insert(InheritablePropertyEditorDefinition::<Option<SurfaceResource>>::new());
    container.register_inheritable_vec_collection::<Option<SurfaceResource>>();
    container.insert(InheritablePropertyEditorDefinition::<SurfaceResource>::new());
    container.insert(InheritablePropertyEditorDefinition::<Status>::new());

    register_absm_property_editors::<Node>(&container);
    register_absm_property_editors::<UiNode>(&container);

    container.insert(VecCollectionPropertyEditorDefinition::<
        Handle<scene::animation::Animation>,
    >::new());
    container.insert(AnimationPropertyEditorDefinition::<
        scene::animation::Animation,
    >::default());

    container.insert(VecCollectionPropertyEditorDefinition::<
        Handle<gui::animation::Animation>,
    >::new());
    container.insert(AnimationPropertyEditorDefinition::<gui::animation::Animation>::default());

    container.insert(AnimationContainerPropertyEditorDefinition::<
        scene::animation::AnimationContainer,
    >::default());
    container.insert(AnimationContainerPropertyEditorDefinition::<
        gui::animation::AnimationContainer,
    >::default());
    container.insert(InheritablePropertyEditorDefinition::<
        scene::animation::AnimationContainer,
    >::new());
    container.insert(InheritablePropertyEditorDefinition::<
        gui::animation::AnimationContainer,
    >::new());

    container.insert(InspectablePropertyEditorDefinition::<ParticleSystemRng>::new());
    container.insert(EnumPropertyEditorDefinition::<PolygonFillMode>::new());

    container.insert(EnumPropertyEditorDefinition::<MipFilter>::new());

    container.register_inheritable_inspectable::<Limb>();
    container.insert(VecCollectionPropertyEditorDefinition::<Limb>::new());

    container.register_inheritable_enum::<BatchingMode, _>();

    container.register_inheritable_inspectable::<Tile>();
    container.register_inheritable_vec_collection::<Tile>();

    container.register_inheritable_enum::<TileCollider, _>();

    container
}
