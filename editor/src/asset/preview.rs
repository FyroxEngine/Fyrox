use crate::load_image;
use fyrox::{
    asset::{manager::ResourceManager, untyped::UntypedResource},
    core::{
        algebra::{Matrix4, UnitQuaternion, Vector3},
        log::Log,
        pool::Handle,
        sstorage::ImmutableString,
        uuid::Uuid,
        TypeUuidProvider,
    },
    fxhash::FxHashMap,
    gui::draw::SharedTexture,
    material::{shader::Shader, Material, MaterialResource, PropertyValue},
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
    utils::into_gui_texture,
};

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
        this.add(Shader::type_uuid(), ShaderPreview);
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
        resource: &UntypedResource,
        resource_manager: &ResourceManager,
        scene: &mut Scene,
    ) -> Handle<Node>;

    fn icon(
        &self,
        resource: &UntypedResource,
        resource_manager: &ResourceManager,
    ) -> Option<SharedTexture>;
}

pub struct TexturePreview;

impl AssetPreview for TexturePreview {
    fn generate(
        &mut self,
        resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        scene: &mut Scene,
    ) -> Handle<Node> {
        if let Some(texture) = resource.try_cast::<Texture>() {
            let mut material = Material::standard_two_sides();
            Log::verify(material.set_property(
                &ImmutableString::new("diffuseTexture"),
                PropertyValue::Sampler {
                    value: Some(texture),
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
        } else {
            Handle::NONE
        }
    }

    fn icon(
        &self,
        resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<SharedTexture> {
        resource.try_cast::<Texture>().map(into_gui_texture)
    }
}

pub struct SoundPreview;

impl AssetPreview for SoundPreview {
    fn generate(
        &mut self,
        resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        scene: &mut Scene,
    ) -> Handle<Node> {
        if let Some(buffer) = resource.try_cast::<SoundBuffer>() {
            SoundBuilder::new(BaseBuilder::new())
                .with_buffer(Some(buffer))
                .with_status(Status::Playing)
                .build(&mut scene.graph)
        } else {
            Handle::NONE
        }
    }

    fn icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<SharedTexture> {
        load_image(include_bytes!("../../resources/embed/model.png"))
    }
}

pub struct ModelPreview;

impl AssetPreview for ModelPreview {
    fn generate(
        &mut self,
        resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        scene: &mut Scene,
    ) -> Handle<Node> {
        if let Some(model) = resource.try_cast::<Model>() {
            model.instantiate(scene)
        } else {
            Handle::NONE
        }
    }

    fn icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<SharedTexture> {
        load_image(include_bytes!("../../resources/embed/sound.png"))
    }
}

pub struct ShaderPreview;

impl AssetPreview for ShaderPreview {
    fn generate(
        &mut self,
        resource: &UntypedResource,
        resource_manager: &ResourceManager,
        scene: &mut Scene,
    ) -> Handle<Node> {
        if let Some(shader) = resource.try_cast::<Shader>() {
            let material = MaterialResource::new_ok(Material::from_shader(
                shader,
                Some(resource_manager.clone()),
            ));

            MeshBuilder::new(BaseBuilder::new())
                .with_surfaces(vec![SurfaceBuilder::new(SurfaceSharedData::new(
                    SurfaceData::make_sphere(32, 32, 1.0, &Matrix4::identity()),
                ))
                .with_material(material)
                .build()])
                .build(&mut scene.graph)
        } else {
            Handle::NONE
        }
    }

    fn icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<SharedTexture> {
        load_image(include_bytes!("../../resources/embed/shader.png"))
    }
}
