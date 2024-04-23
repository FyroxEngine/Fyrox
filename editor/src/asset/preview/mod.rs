pub mod cache;

use crate::{
    fyrox::{
        asset::{manager::ResourceManager, untyped::ResourceKind, untyped::UntypedResource},
        core::{
            algebra::{Matrix4, UnitQuaternion, Vector3},
            log::Log,
            pool::Handle,
            sstorage::ImmutableString,
            uuid::Uuid,
            TypeUuidProvider,
        },
        engine::{Engine, GraphicsContext},
        fxhash::FxHashMap,
        gui::{font::Font, UserInterface},
        material::{shader::Shader, Material, MaterialResource, PropertyValue},
        renderer::framework::gpu_texture::GpuTextureKind,
        resource::{
            curve::CurveResourceState,
            model::{Model, ModelResourceExtension},
            texture::{
                Texture, TextureKind, TexturePixelKind, TextureResource, TextureResourceExtension,
            },
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
    },
    load_image,
};
use fyrox::graph::BaseSceneGraph;
use fyrox::scene::camera::{CameraBuilder, FitParameters, Projection};

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
        this.add(Font::type_uuid(), FontPreview);
        this.add(UserInterface::type_uuid(), UserInterfacePreview);
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

pub trait AssetPreview: Send + Sync + 'static {
    /// Generates a scene, that will be used in the asset browser. Not all assets could provide
    /// sensible scene for themselves, in this case this method should return [`Handle::NONE`].
    fn generate_scene(
        &mut self,
        resource: &UntypedResource,
        resource_manager: &ResourceManager,
        scene: &mut Scene,
    ) -> Handle<Node>;

    /// Generates a preview image for an asset. For example, in case of prefabs, it will be the
    /// entire prefab content rendered to an image. In case of sounds it will be its waveform, and
    /// so on.  
    fn generate_preview(
        &mut self,
        resource: &UntypedResource,
        engine: &mut Engine,
    ) -> Option<TextureResource>;

    /// Returns simplified icon for assets, usually it is just a picture.
    fn simple_icon(
        &self,
        resource: &UntypedResource,
        resource_manager: &ResourceManager,
    ) -> Option<UntypedResource>;
}

pub struct TexturePreview;

impl AssetPreview for TexturePreview {
    fn generate_scene(
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
            let material = MaterialResource::new_ok(Default::default(), material);

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

    fn generate_preview(
        &mut self,
        resource: &UntypedResource,
        _engine: &mut Engine,
    ) -> Option<TextureResource> {
        resource.try_cast::<Texture>()
    }

    fn simple_icon(
        &self,
        resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<UntypedResource> {
        resource.try_cast::<Texture>().map(Into::into)
    }
}

pub struct SoundPreview;

impl AssetPreview for SoundPreview {
    fn generate_scene(
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

    fn generate_preview(
        &mut self,
        _resource: &UntypedResource,
        _engine: &mut Engine,
    ) -> Option<TextureResource> {
        // TODO: Generate waveform image.
        None
    }

    fn simple_icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<UntypedResource> {
        load_image(include_bytes!("../../../resources/sound.png"))
    }
}

pub struct ModelPreview;

impl AssetPreview for ModelPreview {
    fn generate_scene(
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

    fn generate_preview(
        &mut self,
        resource: &UntypedResource,
        engine: &mut Engine,
    ) -> Option<TextureResource> {
        let GraphicsContext::Initialized(ref mut graphics_context) = engine.graphics_context else {
            Log::warn("Cannot render an asset preview when the renderer is not initialized!");
            return None;
        };

        let model = resource.try_cast::<Model>()?;

        let mut scene = Scene::new();

        model.instantiate(&mut scene);

        let camera = CameraBuilder::new(BaseBuilder::new()).build(&mut scene.graph);

        let scene_aabb = scene
            .graph
            .aabb_of_descendants(scene.graph.root(), |_, _| true)
            .unwrap_or_default();
        let camera = scene.graph[camera].as_camera_mut();
        // TODO: Calculate aspect ratio.
        let aspect_ratio = 1.0;
        match camera.fit(&scene_aabb, aspect_ratio) {
            FitParameters::Perspective { position, .. } => {
                camera.local_transform_mut().set_position(position);
            }
            FitParameters::Orthographic {
                position,
                vertical_size,
            } => {
                if let Projection::Orthographic(ortho) = camera.projection_mut() {
                    ortho.vertical_size = vertical_size;
                }
                camera.local_transform_mut().set_position(position);
            }
        }

        let temp_handle = Handle::new(u32::MAX, u32::MAX);
        if let Some(ldr_texture) = graphics_context
            .renderer
            .render_scene(temp_handle, &scene, 0.0)
            .ok()
            .and_then(|data| {
                data.ldr_scene_framebuffer
                    .color_attachments()
                    .first()
                    .map(|a| a.texture.clone())
            })
        {
            let mut ldr_texture = ldr_texture.borrow_mut();
            let (width, height) = match ldr_texture.kind() {
                GpuTextureKind::Rectangle { width, height } => (width, height),
                _ => unreachable!(),
            };

            let pipeline_state = graphics_context.renderer.pipeline_state();
            let pixels = ldr_texture
                .bind_mut(pipeline_state, 0)
                .read_pixels(pipeline_state);

            // TODO: This is a hack, refactor `render_scene` method to accept render data from
            // outside, instead of messing around with these temporary handles.
            graphics_context
                .renderer
                .scene_data_map
                .remove(&temp_handle);

            TextureResource::from_bytes(
                TextureKind::Rectangle {
                    width: width as u32,
                    height: height as u32,
                },
                TexturePixelKind::RGBA8,
                pixels,
                ResourceKind::Embedded,
            )
        } else {
            None
        }
    }

    fn simple_icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<UntypedResource> {
        load_image(include_bytes!("../../../resources/model.png"))
    }
}

pub struct ShaderPreview;

impl AssetPreview for ShaderPreview {
    fn generate_scene(
        &mut self,
        resource: &UntypedResource,
        resource_manager: &ResourceManager,
        scene: &mut Scene,
    ) -> Handle<Node> {
        if let Some(shader) = resource.try_cast::<Shader>() {
            let material = MaterialResource::new_ok(
                Default::default(),
                Material::from_shader(shader, Some(resource_manager.clone())),
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

    fn generate_preview(
        &mut self,
        _resource: &UntypedResource,
        _engine: &mut Engine,
    ) -> Option<TextureResource> {
        // Shaders do not have any sensible preview, the simple icon will be used instead.
        None
    }

    fn simple_icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<UntypedResource> {
        load_image(include_bytes!("../../../resources/shader.png"))
    }
}

pub struct MaterialPreview;

impl AssetPreview for MaterialPreview {
    fn generate_scene(
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

    fn generate_preview(
        &mut self,
        _resource: &UntypedResource,
        _engine: &mut Engine,
    ) -> Option<TextureResource> {
        // TODO
        None
    }

    fn simple_icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<UntypedResource> {
        load_image(include_bytes!("../../../resources/material.png"))
    }
}

pub struct HrirPreview;

impl AssetPreview for HrirPreview {
    fn generate_scene(
        &mut self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        _scene: &mut Scene,
    ) -> Handle<Node> {
        Handle::NONE
    }

    fn generate_preview(
        &mut self,
        _resource: &UntypedResource,
        _engine: &mut Engine,
    ) -> Option<TextureResource> {
        // Head-related impulse response do not have any sensible preview, the simple icon will be
        // used instead.
        None
    }

    fn simple_icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<UntypedResource> {
        load_image(include_bytes!("../../../resources/hrir.png"))
    }
}

pub struct CurvePreview;

impl AssetPreview for CurvePreview {
    fn generate_scene(
        &mut self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        _scene: &mut Scene,
    ) -> Handle<Node> {
        Handle::NONE
    }

    fn generate_preview(
        &mut self,
        _resource: &UntypedResource,
        _engine: &mut Engine,
    ) -> Option<TextureResource> {
        // TODO: Implement.
        None
    }

    fn simple_icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<UntypedResource> {
        load_image(include_bytes!("../../../resources/curve.png"))
    }
}

pub struct FontPreview;

impl AssetPreview for FontPreview {
    fn generate_scene(
        &mut self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        _scene: &mut Scene,
    ) -> Handle<Node> {
        Handle::NONE
    }

    fn generate_preview(
        &mut self,
        _resource: &UntypedResource,
        _engine: &mut Engine,
    ) -> Option<TextureResource> {
        // TODO: Implement.
        None
    }

    fn simple_icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<UntypedResource> {
        load_image(include_bytes!("../../../resources/font.png"))
    }
}

pub struct UserInterfacePreview;

impl AssetPreview for UserInterfacePreview {
    fn generate_scene(
        &mut self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        _scene: &mut Scene,
    ) -> Handle<Node> {
        Handle::NONE
    }

    fn generate_preview(
        &mut self,
        _resource: &UntypedResource,
        _engine: &mut Engine,
    ) -> Option<TextureResource> {
        // TODO: Implement.
        None
    }

    fn simple_icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<UntypedResource> {
        load_image(include_bytes!("../../../resources/ui.png"))
    }
}
