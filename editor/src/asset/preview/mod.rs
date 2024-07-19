pub mod cache;

use crate::{
    fyrox::{
        asset::{manager::ResourceManager, untyped::ResourceKind, untyped::UntypedResource},
        core::{
            algebra::{Matrix4, UnitQuaternion, Vector2, Vector3},
            color::Color,
            log::Log,
            pool::Handle,
            sstorage::ImmutableString,
            uuid::Uuid,
            TypeUuidProvider,
        },
        engine::{Engine, GraphicsContext},
        fxhash::FxHashMap,
        graph::BaseSceneGraph,
        gui::{
            font::Font, formatted_text::WrapMode, screen::ScreenBuilder, text::TextBuilder,
            widget::WidgetBuilder, HorizontalAlignment, UserInterface, VerticalAlignment,
        },
        material::{shader::Shader, Material, MaterialResource, PropertyValue},
        renderer::framework::gpu_texture::{GpuTextureKind, PixelKind},
        resource::{
            curve::CurveResourceState,
            model::{Model, ModelResourceExtension},
            texture::{
                Texture, TextureKind, TexturePixelKind, TextureResource, TextureResourceExtension,
            },
        },
        scene::{
            base::BaseBuilder,
            camera::{CameraBuilder, FitParameters, Projection},
            light::{directional::DirectionalLightBuilder, BaseLightBuilder},
            mesh::{
                surface::{SurfaceBuilder, SurfaceData, SurfaceResource},
                MeshBuilder, RenderPath,
            },
            node::Node,
            sound::{HrirSphereResourceData, SoundBuffer, SoundBuilder, Status},
            Scene,
        },
    },
    load_image,
};
use image::{ColorType, GenericImage, Rgba};

#[derive(Default)]
pub struct AssetPreviewGeneratorsCollection {
    pub map: FxHashMap<Uuid, Box<dyn AssetPreviewGenerator>>,
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
        this.add(SurfaceData::type_uuid(), SurfaceDataPreview);
        this
    }

    pub fn add<T: AssetPreviewGenerator>(
        &mut self,
        resource_type_uuid: Uuid,
        generator: T,
    ) -> Option<Box<dyn AssetPreviewGenerator>> {
        self.map.insert(resource_type_uuid, Box::new(generator))
    }
}

#[derive(Clone)]
pub struct AssetPreviewTexture {
    pub texture: TextureResource,
    pub flip_y: bool,
}

pub trait AssetPreviewGenerator: Send + Sync + 'static {
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
    ) -> Option<AssetPreviewTexture>;

    /// Returns simplified icon for assets, usually it is just a picture.
    fn simple_icon(
        &self,
        resource: &UntypedResource,
        resource_manager: &ResourceManager,
    ) -> Option<UntypedResource>;
}

pub struct TexturePreview;

impl AssetPreviewGenerator for TexturePreview {
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
                .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_ok(
                    ResourceKind::Embedded,
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
    ) -> Option<AssetPreviewTexture> {
        resource
            .try_cast::<Texture>()
            .map(|texture| AssetPreviewTexture {
                texture,
                flip_y: false,
            })
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

impl AssetPreviewGenerator for SoundPreview {
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
        resource: &UntypedResource,
        _engine: &mut Engine,
    ) -> Option<AssetPreviewTexture> {
        if let Some(buffer) = resource.try_cast::<SoundBuffer>() {
            if let Some(data) = buffer.state().data() {
                let height = 60.0;
                let half_height = height / 2.0;
                let width = 60.0;
                let mut image =
                    image::DynamicImage::new(width as u32, height as u32, ColorType::Rgba8);

                for i in 0..(width as u32) {
                    for j in 0..(height as u32) {
                        image.put_pixel(j, i, Rgba::from([100, 100, 100, 255]));
                    }
                }

                let samples = data.samples();
                let mut min = f32::MAX;
                let mut max = -f32::MAX;
                for sample in samples.iter() {
                    if *sample < min {
                        min = *sample;
                    }
                    if *sample > max {
                        max = *sample;
                    }
                }
                let amplitude_range = (max.abs() + min.abs()) / 4.0;

                let sample_count = samples.len();
                let step = sample_count / width as usize;
                for (step_num, sample_num) in (0..sample_count).step_by(step).enumerate() {
                    let current_amplitude = samples[sample_num] / amplitude_range;
                    let next_amplitude = samples
                        [(sample_num + step).min(sample_count.saturating_sub(1))]
                        / amplitude_range;
                    let current_x = step_num as f32;
                    let next_x = (step_num + 1) as f32;
                    let current_y = half_height + current_amplitude * half_height;
                    let next_y = half_height + next_amplitude * half_height;
                    imageproc::drawing::draw_antialiased_line_segment_mut(
                        &mut image,
                        (current_x as i32, current_y as i32),
                        (next_x as i32, next_y as i32),
                        Rgba::from([255, 127, 39, 255]),
                        imageproc::pixelops::interpolate,
                    );
                }

                return TextureResource::from_bytes(
                    TextureKind::Rectangle {
                        width: width as u32,
                        height: height as u32,
                    },
                    TexturePixelKind::RGBA8,
                    image.as_bytes().to_vec(),
                    ResourceKind::Embedded,
                )
                .map(|texture| AssetPreviewTexture {
                    texture,
                    flip_y: false,
                });
            }
        }
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

fn render_scene_to_texture(
    engine: &mut Engine,
    scene: &mut Scene,
    rt_size: Vector2<f32>,
) -> Option<AssetPreviewTexture> {
    let GraphicsContext::Initialized(ref mut graphics_context) = engine.graphics_context else {
        Log::warn("Cannot render an asset preview when the renderer is not initialized!");
        return None;
    };

    scene.rendering_options.render_target = Some(TextureResource::new_render_target(
        rt_size.x as u32,
        rt_size.y as u32,
    ));

    let camera = CameraBuilder::new(BaseBuilder::new()).build(&mut scene.graph);

    scene.update(rt_size, 0.016, Default::default());

    let scene_aabb = scene
        .graph
        .aabb_of_descendants(scene.graph.root(), |_, _| true)
        .unwrap_or_default();
    let camera = scene.graph[camera].as_camera_mut();
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

    scene.update(rt_size, 0.016, Default::default());

    let temp_handle = Handle::new(u32::MAX, u32::MAX);
    if let Some(ldr_texture) = graphics_context
        .renderer
        .render_scene(temp_handle, scene, 0.0)
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
        .map(|texture| AssetPreviewTexture {
            texture,
            // OpenGL was designed by mathematicians.
            flip_y: true,
        })
    } else {
        None
    }
}

pub struct ModelPreview;

impl AssetPreviewGenerator for ModelPreview {
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
    ) -> Option<AssetPreviewTexture> {
        let model = resource.try_cast::<Model>()?;
        let mut scene = Scene::new();
        scene.rendering_options.ambient_lighting_color = Color::opaque(180, 180, 180);
        model.instantiate(&mut scene);
        render_scene_to_texture(engine, &mut scene, Vector2::new(128.0, 128.0))
    }

    fn simple_icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<UntypedResource> {
        load_image(include_bytes!("../../../resources/model.png"))
    }
}

pub struct SurfaceDataPreview;

impl AssetPreviewGenerator for SurfaceDataPreview {
    fn generate_scene(
        &mut self,
        resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        scene: &mut Scene,
    ) -> Handle<Node> {
        if let Some(surface) = resource.try_cast::<SurfaceData>() {
            MeshBuilder::new(BaseBuilder::new())
                .with_surfaces(vec![SurfaceBuilder::new(surface.clone()).build()])
                .build(&mut scene.graph)
        } else {
            Handle::NONE
        }
    }

    fn generate_preview(
        &mut self,
        resource: &UntypedResource,
        engine: &mut Engine,
    ) -> Option<AssetPreviewTexture> {
        let surface = resource.try_cast::<SurfaceData>()?;
        let mut scene = Scene::new();
        scene.rendering_options.ambient_lighting_color = Color::opaque(180, 180, 180);
        MeshBuilder::new(BaseBuilder::new())
            .with_surfaces(vec![SurfaceBuilder::new(surface.clone()).build()])
            .build(&mut scene.graph);
        render_scene_to_texture(engine, &mut scene, Vector2::new(128.0, 128.0))
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

impl AssetPreviewGenerator for ShaderPreview {
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
                .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_ok(
                    ResourceKind::Embedded,
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
    ) -> Option<AssetPreviewTexture> {
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

impl AssetPreviewGenerator for MaterialPreview {
    fn generate_scene(
        &mut self,
        resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        scene: &mut Scene,
    ) -> Handle<Node> {
        if let Some(material) = resource.try_cast::<Material>() {
            MeshBuilder::new(BaseBuilder::new())
                .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_ok(
                    ResourceKind::Embedded,
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
        resource: &UntypedResource,
        engine: &mut Engine,
    ) -> Option<AssetPreviewTexture> {
        let mut scene = Scene::new();
        self.generate_scene(resource, &engine.resource_manager, &mut scene);
        DirectionalLightBuilder::new(BaseLightBuilder::new(BaseBuilder::new()))
            .build(&mut scene.graph);
        render_scene_to_texture(engine, &mut scene, Vector2::new(128.0, 128.0))
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

impl AssetPreviewGenerator for HrirPreview {
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
    ) -> Option<AssetPreviewTexture> {
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

impl AssetPreviewGenerator for CurvePreview {
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
    ) -> Option<AssetPreviewTexture> {
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

pub fn render_ui_to_texture(
    ui: &mut UserInterface,
    engine: &mut Engine,
) -> Option<AssetPreviewTexture> {
    let GraphicsContext::Initialized(ref mut graphics_context) = engine.graphics_context else {
        Log::warn("Cannot render an asset preview when the renderer is not initialized!");
        return None;
    };

    let screen_size = ui.screen_size();
    ui.update(screen_size, 0.016, &Default::default());
    while ui.poll_message().is_some() {}
    ui.update(screen_size, 0.016, &Default::default());
    let render_target = TextureResource::new_render_target(256, 256);
    graphics_context
        .renderer
        .render_ui_to_texture(
            render_target.clone(),
            screen_size,
            ui.draw(),
            Color::opaque(100, 100, 100),
            PixelKind::RGBA8,
        )
        .ok()?;

    Some(AssetPreviewTexture {
        texture: render_target,
        flip_y: true,
    })
}

impl AssetPreviewGenerator for FontPreview {
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
        resource: &UntypedResource,
        engine: &mut Engine,
    ) -> Option<AssetPreviewTexture> {
        if let Some(font) = resource.try_cast::<Font>() {
            let mut ui = UserInterface::new(Vector2::new(60.0, 60.0));
            ScreenBuilder::new(
                WidgetBuilder::new().with_child(
                    TextBuilder::new(WidgetBuilder::new())
                        .with_font(font)
                        .with_font_size(16.0)
                        .with_vertical_text_alignment(VerticalAlignment::Center)
                        .with_horizontal_text_alignment(HorizontalAlignment::Center)
                        .with_wrap(WrapMode::Letter)
                        .with_text("AaBbCcDd1234567890")
                        .build(&mut ui.build_ctx()),
                ),
            )
            .build(&mut ui.build_ctx());
            render_ui_to_texture(&mut ui, engine)
        } else {
            None
        }
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

impl AssetPreviewGenerator for UserInterfacePreview {
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
        resource: &UntypedResource,
        engine: &mut Engine,
    ) -> Option<AssetPreviewTexture> {
        if let Some(ui_resource) = resource.try_cast::<UserInterface>() {
            let mut ui = ui_resource.data_ref().clone();
            ui.set_screen_size(Vector2::new(256.0, 256.0));
            render_ui_to_texture(&mut ui, engine)
        } else {
            None
        }
    }

    fn simple_icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<UntypedResource> {
        load_image(include_bytes!("../../../resources/ui.png"))
    }
}
