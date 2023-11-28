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
        curve::CurveResourceState,
        model::{Model, ModelResourceExtension},
        texture::Texture,
    },
    scene::{
        base::BaseBuilder,
        mesh::{
            surface::{SurfaceBuilder, SurfaceData, SurfaceSharedData},
            MeshBuilder, RenderPath,
        },
        node::Node,
        sound::{HrirSphereResourceData, SoundBuffer, SoundBuilder, Status},
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
        this.add(Material::type_uuid(), MaterialPreview);
        this.add(HrirSphereResourceData::type_uuid(), HrirPreview);
        this.add(CurveResourceState::type_uuid(), CurvePreview);
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
            let scale = if let Some(size) = texture.data_ref().kind().rectangle_size() {
                let aspect_ratio = size.x as f32 / size.y as f32;
                Vector3::new(aspect_ratio, 1.0, 1.0)
            } else {
                Vector3::repeat(1.0)
            };

            let mut material = Material::standard_two_sides();
            Log::verify(material.set_property(
                &ImmutableString::new("diffuseTexture"),
                PropertyValue::Sampler {
                    value: Some(texture),
                    fallback: Default::default(),
                },
            ));
            let material = MaterialResource::new_ok(Default::default(), material, true);

            MeshBuilder::new(BaseBuilder::new())
                .with_surfaces(vec![SurfaceBuilder::new(SurfaceSharedData::new(
                    SurfaceData::make_quad(
                        &(UnitQuaternion::from_axis_angle(
                            &Vector3::z_axis(),
                            180.0f32.to_radians(),
                        )
                        .to_homogeneous()
                            * Matrix4::new_nonuniform_scaling(&scale)),
                    ),
                ))
                .with_material(material)
                .build()])
                .with_render_path(RenderPath::Forward)
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
        load_image(include_bytes!("../../resources/sound.png"))
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
        load_image(include_bytes!("../../resources/model.png"))
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
            let material = MaterialResource::new_ok(
                Default::default(),
                Material::from_shader(shader, Some(resource_manager.clone())),
                true,
            );

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
        load_image(include_bytes!("../../resources/shader.png"))
    }
}

pub struct MaterialPreview;

impl AssetPreview for MaterialPreview {
    fn generate(
        &mut self,
        resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        scene: &mut Scene,
    ) -> Handle<Node> {
        if let Some(material) = resource.try_cast::<Material>() {
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
        load_image(include_bytes!("../../resources/material.png"))
    }
}

pub struct HrirPreview;

impl AssetPreview for HrirPreview {
    fn generate(
        &mut self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        _scene: &mut Scene,
    ) -> Handle<Node> {
        Handle::NONE
    }

    fn icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<SharedTexture> {
        load_image(include_bytes!("../../resources/hrir.png"))
    }
}

pub struct CurvePreview;

impl AssetPreview for CurvePreview {
    fn generate(
        &mut self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        _scene: &mut Scene,
    ) -> Handle<Node> {
        Handle::NONE
    }

    fn icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<SharedTexture> {
        load_image(include_bytes!("../../resources/curve.png"))
    }
}
