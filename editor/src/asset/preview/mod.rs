// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

pub mod cache;

use crate::{
    asset,
    fyrox::{
        asset::{manager::ResourceManager, untyped::ResourceKind, untyped::UntypedResource},
        core::{
            algebra::{Matrix4, UnitQuaternion, Vector2, Vector3},
            color::Color,
            log::Log,
            pool::Handle,
            uuid::Uuid,
            TypeUuidProvider,
        },
        engine::{Engine, GraphicsContext},
        fxhash::FxHashMap,
        graph::{BaseSceneGraph, SceneGraphNode},
        graphics::{
            framebuffer::ReadTarget,
            gpu_texture::{GpuTextureKind, PixelKind},
        },
        gui::{
            font::Font, formatted_text::WrapMode, screen::ScreenBuilder, text::TextBuilder,
            widget::WidgetBuilder, HorizontalAlignment, UserInterface, VerticalAlignment,
        },
        material::{shader::Shader, Material, MaterialResource},
        resource::{
            curve::CurveResourceState,
            model::{Model, ModelResourceExtension},
            texture::{
                Texture, TextureKind, TexturePixelKind, TextureResource, TextureResourceExtension,
            },
        },
        scene::{
            base::BaseBuilder,
            camera::{Camera, CameraBuilder, FitParameters, Projection},
            light::{directional::DirectionalLightBuilder, BaseLightBuilder},
            mesh::{
                surface::{SurfaceBuilder, SurfaceData, SurfaceResource},
                MeshBuilder, RenderPath,
            },
            node::Node,
            skybox::SkyBox,
            sound::{HrirSphereResourceData, SoundBuffer, SoundBuilder, Status},
            EnvironmentLightingSource, Scene,
        },
    },
    load_image,
};
use fyrox::renderer::ui_renderer::UiRenderInfo;
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

pub fn make_preview_scene(lighting: bool) -> Scene {
    let mut scene = Scene::new();
    scene.set_skybox(Some(SkyBox::from_single_color(Color::repeat_opaque(40))));
    let color = if lighting {
        Color::repeat_opaque(80)
    } else {
        Color::repeat_opaque(180)
    };
    scene.rendering_options.ambient_lighting_color = color;
    scene.rendering_options.clear_color = Some(color);
    scene.rendering_options.environment_lighting_source = EnvironmentLightingSource::AmbientColor;
    if lighting {
        DirectionalLightBuilder::new(BaseLightBuilder::new(BaseBuilder::new()))
            .build(&mut scene.graph);
    }
    scene
}

#[derive(Clone)]
pub struct AssetPreviewTexture {
    pub texture: TextureResource,
    pub flip_y: bool,
    pub color: Color,
}

impl AssetPreviewTexture {
    pub fn from_texture_with_gray_tint(texture: TextureResource) -> Self {
        Self {
            texture,
            flip_y: false,
            color: Color::opaque(190, 190, 190),
        }
    }
}

pub trait AssetPreviewGenerator: Send + Sync + 'static {
    /// Generates a scene, that will be used in the asset browser. Not all assets could provide
    /// sensible scene for themselves, in this case this method should return [`Handle::NONE`].
    fn generate_scene(
        &mut self,
        resource: &UntypedResource,
        resource_manager: &ResourceManager,
        scene: &mut Scene,
        preview_camera: Handle<Node>,
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
    ) -> Option<TextureResource>;
}

pub struct TexturePreview;

impl AssetPreviewGenerator for TexturePreview {
    fn generate_scene(
        &mut self,
        resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        scene: &mut Scene,
        _preview_camera: Handle<Node>,
    ) -> Handle<Node> {
        if let Some(texture) = resource.try_cast::<Texture>() {
            let scale = if let Some(size) = texture.data_ref().kind().rectangle_size() {
                let aspect_ratio = size.x as f32 / size.y as f32;
                Vector3::new(aspect_ratio, 1.0, 1.0)
            } else {
                Vector3::repeat(1.0)
            };

            let mut material = Material::standard_two_sides();
            material.bind("diffuseTexture", texture);
            let material = MaterialResource::new_embedded(material);

            MeshBuilder::new(BaseBuilder::new())
                .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_embedded(
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
                color: Color::WHITE,
            })
    }

    fn simple_icon(
        &self,
        resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<TextureResource> {
        resource.try_cast::<Texture>()
    }
}

pub struct SoundPreview;

impl AssetPreviewGenerator for SoundPreview {
    fn generate_scene(
        &mut self,
        resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        scene: &mut Scene,
        _preview_camera: Handle<Node>,
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
                let height = asset::item::DEFAULT_SIZE;
                let half_height = height / 2.0;
                let width = asset::item::DEFAULT_SIZE;
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
                    Uuid::new_v4(),
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
                    color: Color::WHITE,
                });
            }
        }
        None
    }

    fn simple_icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<TextureResource> {
        load_image!("../../../resources/sound.png")
    }
}

fn render_scene_to_texture(
    engine: &mut Engine,
    scene: &mut Scene,
    rt_size: Vector2<f32>,
) -> Option<AssetPreviewTexture> {
    let elapsed_time = engine.elapsed_time();
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
    match camera.fit(&scene_aabb, aspect_ratio, 1.05) {
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
    if let Ok(scene_data) = graphics_context.renderer.render_scene(
        temp_handle,
        scene,
        elapsed_time,
        0.0,
        &engine.resource_manager,
    ) {
        let ldr_texture = scene_data.scene_data.ldr_scene_frame_texture();

        let (width, height) = match ldr_texture.kind() {
            GpuTextureKind::Rectangle { width, height } => (width, height),
            _ => unreachable!(),
        };

        let pixels = scene_data
            .scene_data
            .ldr_scene_framebuffer
            .read_pixels(ReadTarget::Color(0))?;

        // TODO: This is a hack, refactor `render_scene` method to accept render data from
        // outside, instead of messing around with these temporary handles.
        graphics_context
            .renderer
            .scene_data_map
            .remove(&temp_handle);

        TextureResource::from_bytes(
            Uuid::new_v4(),
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
            color: Color::WHITE,
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
        preview_camera: Handle<Node>,
    ) -> Handle<Node> {
        if let Some(model) = resource.try_cast::<Model>() {
            let instance = model.instantiate(scene);

            for camera in scene
                .graph
                .pair_iter_mut()
                .filter(|(h, _)| *h != preview_camera)
                .filter_map(|(_, n)| n.component_mut::<Camera>())
            {
                camera.set_enabled(false);
            }

            instance
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
        let mut scene = make_preview_scene(true);
        model.instantiate(&mut scene);
        render_scene_to_texture(engine, &mut scene, asset::item::DEFAULT_VEC_SIZE)
    }

    fn simple_icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<TextureResource> {
        load_image!("../../../resources/model.png")
    }
}

pub struct SurfaceDataPreview;

impl AssetPreviewGenerator for SurfaceDataPreview {
    fn generate_scene(
        &mut self,
        resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        scene: &mut Scene,
        _preview_camera: Handle<Node>,
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
        let mut scene = make_preview_scene(true);
        MeshBuilder::new(BaseBuilder::new())
            .with_surfaces(vec![SurfaceBuilder::new(surface.clone()).build()])
            .build(&mut scene.graph);
        render_scene_to_texture(engine, &mut scene, asset::item::DEFAULT_VEC_SIZE)
    }

    fn simple_icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<TextureResource> {
        load_image!("../../../resources/model.png")
    }
}

pub struct ShaderPreview;

impl AssetPreviewGenerator for ShaderPreview {
    fn generate_scene(
        &mut self,
        resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        scene: &mut Scene,
        _preview_camera: Handle<Node>,
    ) -> Handle<Node> {
        if let Some(shader) = resource.try_cast::<Shader>() {
            let material = MaterialResource::new_embedded(Material::from_shader(shader));

            MeshBuilder::new(BaseBuilder::new())
                .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_embedded(
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
    ) -> Option<TextureResource> {
        load_image!("../../../resources/shader.png")
    }
}

pub struct MaterialPreview;

impl AssetPreviewGenerator for MaterialPreview {
    fn generate_scene(
        &mut self,
        resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        scene: &mut Scene,
        _preview_camera: Handle<Node>,
    ) -> Handle<Node> {
        if let Some(material) = resource.try_cast::<Material>() {
            MeshBuilder::new(BaseBuilder::new())
                .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_embedded(
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
        let mut scene = make_preview_scene(true);
        self.generate_scene(resource, &engine.resource_manager, &mut scene, Handle::NONE);

        render_scene_to_texture(engine, &mut scene, asset::item::DEFAULT_VEC_SIZE)
    }

    fn simple_icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<TextureResource> {
        load_image!("../../../resources/material.png")
    }
}

pub struct HrirPreview;

impl AssetPreviewGenerator for HrirPreview {
    fn generate_scene(
        &mut self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        _scene: &mut Scene,
        _preview_camera: Handle<Node>,
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
    ) -> Option<TextureResource> {
        load_image!("../../../resources/hrir.png")
    }
}

pub struct CurvePreview;

impl AssetPreviewGenerator for CurvePreview {
    fn generate_scene(
        &mut self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        _scene: &mut Scene,
        _preview_camera: Handle<Node>,
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
    ) -> Option<TextureResource> {
        load_image!("../../../resources/curve.png")
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
    let render_target = TextureResource::new_render_target(
        asset::item::DEFAULT_SIZE as u32,
        asset::item::DEFAULT_SIZE as u32,
    );
    graphics_context
        .renderer
        .render_ui(UiRenderInfo {
            render_target: Some(render_target.clone()),
            screen_size,
            drawing_context: ui.draw(),
            clear_color: Color::opaque(100, 100, 100),
            resource_manager: &engine.resource_manager,
        })
        .ok()?;

    assert!(graphics_context
        .renderer
        .ui_frame_buffers
        .remove(&render_target.key())
        .is_some());

    Some(AssetPreviewTexture {
        texture: render_target,
        flip_y: true,
        color: Color::WHITE,
    })
}

impl AssetPreviewGenerator for FontPreview {
    fn generate_scene(
        &mut self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        _scene: &mut Scene,
        _preview_camera: Handle<Node>,
    ) -> Handle<Node> {
        Handle::NONE
    }

    fn generate_preview(
        &mut self,
        resource: &UntypedResource,
        engine: &mut Engine,
    ) -> Option<AssetPreviewTexture> {
        if let Some(font) = resource.try_cast::<Font>() {
            let mut ui = UserInterface::new(asset::item::DEFAULT_VEC_SIZE);
            ScreenBuilder::new(
                WidgetBuilder::new().with_child(
                    TextBuilder::new(WidgetBuilder::new())
                        .with_font(font)
                        .with_font_size(16.0.into())
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
    ) -> Option<TextureResource> {
        load_image!("../../../resources/font.png")
    }
}

pub struct UserInterfacePreview;

impl AssetPreviewGenerator for UserInterfacePreview {
    fn generate_scene(
        &mut self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        _scene: &mut Scene,
        _preview_camera: Handle<Node>,
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
            ui.set_screen_size(asset::item::DEFAULT_VEC_SIZE);
            render_ui_to_texture(&mut ui, engine)
        } else {
            None
        }
    }

    fn simple_icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<TextureResource> {
        load_image!("../../../resources/ui.png")
    }
}
