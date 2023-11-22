use fyrox::{
    asset::manager::ResourceManager,
    core::{
        algebra::{UnitQuaternion, Vector3},
        futures::executor::block_on,
        log::Log,
        pool::Handle,
        sstorage::ImmutableString,
        uuid::Uuid,
        TypeUuidProvider,
    },
    fxhash::FxHashMap,
    material::{Material, MaterialResource, PropertyValue},
    resource::{
        model::{Model, ModelResourceExtension},
        texture::Texture,
    },
    scene::{
        base::BaseBuilder,
        mesh::{
            surface::{SurfaceBuilder, SurfaceData, SurfaceSharedData},
            MeshBuilder,
        },
        node::Node,
        sound::{SoundBuffer, SoundBuilder, Status},
        Scene,
    },
};
use std::path::Path;

#[derive(Default)]
pub struct AssetPreviewGeneratorsCollection {
    pub map: FxHashMap<Uuid, Box<dyn AssetPreview>>,
}

impl AssetPreviewGeneratorsCollection {
    pub fn new() -> Self {
        let mut this = Self::default();
        this.add(Texture::type_uuid(), TexturePreview);
        this.add(Model::type_uuid(), ModelPreview);
        this.add(SoundBuffer::type_uuid(), SoundPreview);
        this
    }

    pub fn add<T: AssetPreview>(
        &mut self,
        resource_type_uuid: Uuid,
        generator: T,
    ) -> Option<Box<dyn AssetPreview>> {
        self.map.insert(resource_type_uuid, Box::new(generator))
    }
}

pub trait AssetPreview: 'static {
    fn generate(
        &mut self,
        resource_path: &Path,
        resource_manager: &ResourceManager,
        scene: &mut Scene,
    ) -> Handle<Node>;
}

pub struct TexturePreview;

impl AssetPreview for TexturePreview {
    fn generate(
        &mut self,
        resource_path: &Path,
        resource_manager: &ResourceManager,
        scene: &mut Scene,
    ) -> Handle<Node> {
        let mut material = Material::standard_two_sides();
        Log::verify(material.set_property(
            &ImmutableString::new("diffuseTexture"),
            PropertyValue::Sampler {
                value: Some(resource_manager.request::<Texture>(resource_path)),
                fallback: Default::default(),
            },
        ));
        let material = MaterialResource::new_ok(material);

        MeshBuilder::new(BaseBuilder::new())
            .with_surfaces(vec![SurfaceBuilder::new(SurfaceSharedData::new(
                SurfaceData::make_quad(
                    &UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 180.0f32.to_radians())
                        .to_homogeneous(),
                ),
            ))
            .with_material(material)
            .build()])
            .build(&mut scene.graph)
    }
}

pub struct SoundPreview;

impl AssetPreview for SoundPreview {
    fn generate(
        &mut self,
        resource_path: &Path,
        resource_manager: &ResourceManager,
        scene: &mut Scene,
    ) -> Handle<Node> {
        if let Ok(buffer) = block_on(resource_manager.request::<SoundBuffer>(resource_path)) {
            SoundBuilder::new(BaseBuilder::new())
                .with_buffer(Some(buffer))
                .with_status(Status::Playing)
                .build(&mut scene.graph)
        } else {
            Handle::NONE
        }
    }
}

pub struct ModelPreview;

impl AssetPreview for ModelPreview {
    fn generate(
        &mut self,
        resource_path: &Path,
        resource_manager: &ResourceManager,
        scene: &mut Scene,
    ) -> Handle<Node> {
        if let Ok(model) = block_on(resource_manager.request::<Model>(resource_path)) {
            model.instantiate(scene)
        } else {
            Handle::NONE
        }
    }
}
