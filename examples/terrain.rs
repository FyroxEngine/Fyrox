//! Example - Terrain.

pub mod shared;

use crate::shared::create_camera;
use fyrox::resource::texture::Texture;
use fyrox::{
    asset::manager::ResourceManager,
    core::{
        algebra::{UnitQuaternion, Vector2, Vector3},
        color::Color,
        pool::Handle,
        rand::Rng,
        sstorage::ImmutableString,
        uuid::{uuid, Uuid},
        TypeUuidProvider,
    },
    engine::{executor::Executor, GraphicsContext, GraphicsContextParams},
    event::{ElementState, Event, WindowEvent},
    event_loop::ControlFlow,
    gui::{
        message::MessageDirection,
        text::{TextBuilder, TextMessage},
        widget::WidgetBuilder,
        UiNode,
    },
    material::{shader::SamplerFallback, Material, PropertyValue, SharedMaterial},
    plugin::{Plugin, PluginConstructor, PluginContext},
    rand::thread_rng,
    scene::{
        base::BaseBuilder,
        light::{point::PointLightBuilder, BaseLightBuilder},
        node::Node,
        terrain::{Brush, BrushMode, BrushShape, Layer, TerrainBuilder},
        transform::TransformBuilder,
        Scene,
    },
    window::WindowAttributes,
};
use winit::keyboard::KeyCode;

struct SceneLoader {
    scene: Scene,
    model_handle: Handle<Node>,
}

fn setup_layer_material(
    material: &mut Material,
    resource_manager: ResourceManager,
    diffuse_texture: &str,
    normal_texture: &str,
) {
    material
        .set_property(
            &ImmutableString::new("diffuseTexture"),
            PropertyValue::Sampler {
                value: Some(resource_manager.request::<Texture, _>(diffuse_texture)),
                fallback: SamplerFallback::White,
            },
        )
        .unwrap();
    material
        .set_property(
            &ImmutableString::new("normalTexture"),
            PropertyValue::Sampler {
                value: Some(resource_manager.request::<Texture, _>(normal_texture)),
                fallback: SamplerFallback::Normal,
            },
        )
        .unwrap();
    material
        .set_property(
            &ImmutableString::new("texCoordScale"),
            PropertyValue::Vector2(Vector2::new(10.0, 10.0)),
        )
        .unwrap();
}

impl SceneLoader {
    async fn load_with(resource_manager: ResourceManager) -> Self {
        let mut scene = Scene::new();

        // Set ambient light.
        scene.ambient_lighting_color = Color::opaque(200, 200, 200);

        // Camera is our eyes in the world - you won't see anything without it.
        let model_handle = create_camera(
            resource_manager.clone(),
            Vector3::new(32.0, 6.0, 32.0),
            &mut scene.graph,
        )
        .await;

        // Add terrain.
        let terrain = TerrainBuilder::new(BaseBuilder::new())
            .with_chunk_size(Vector2::new(32.0, 32.0))
            .with_width_chunks(0..2)
            .with_length_chunks(0..2)
            .with_layers(vec![
                Layer {
                    material: {
                        let mut material = Material::standard_terrain();
                        setup_layer_material(
                            &mut material,
                            resource_manager.clone(),
                            "examples/data/Grass_DiffuseColor.jpg",
                            "examples/data/Grass_NormalColor.jpg",
                        );
                        SharedMaterial::new(material)
                    },
                    ..Default::default()
                },
                Layer {
                    material: {
                        let mut material = Material::standard_terrain();
                        setup_layer_material(
                            &mut material,
                            resource_manager.clone(),
                            "examples/data/Rock_DiffuseColor.jpg",
                            "examples/data/Rock_Normal.jpg",
                        );
                        SharedMaterial::new(material)
                    },
                    ..Default::default()
                },
            ])
            .build(&mut scene.graph);

        let terrain = scene.graph[terrain].as_terrain_mut();

        for _ in 0..60 {
            let x = thread_rng().gen_range(4.0..60.00);
            let z = thread_rng().gen_range(4.0..60.00);
            let radius = thread_rng().gen_range(2.0..4.0);
            let height = thread_rng().gen_range(1.0..3.0);

            // Draw something on the terrain.

            // Pull terrain.
            terrain.draw(&Brush {
                center: Vector3::new(x, 0.0, z),
                shape: BrushShape::Circle { radius },
                mode: BrushMode::ModifyHeightMap { amount: height },
            });

            // Draw rock texture on top.
            terrain.draw(&Brush {
                center: Vector3::new(x, 0.0, z),
                shape: BrushShape::Circle { radius },
                mode: BrushMode::DrawOnMask {
                    layer: 1,
                    alpha: 1.0,
                },
            });
        }

        // Add some light.
        PointLightBuilder::new(BaseLightBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(Vector3::new(0.0, 12.0, 0.0))
                    .build(),
            ),
        ))
        .with_radius(20.0)
        .build(&mut scene.graph);

        Self {
            scene,
            model_handle,
        }
    }
}

struct InputController {
    rotate_left: bool,
    rotate_right: bool,
}

struct Game {
    debug_text: Handle<UiNode>,
    input_controller: InputController,
    model_angle: f32,
    scene: Handle<Scene>,
    model_handle: Handle<Node>,
}

impl Plugin for Game {
    fn update(&mut self, context: &mut PluginContext, _control_flow: &mut ControlFlow) {
        // Use stored scene handle to borrow a mutable reference of scene in
        // engine.
        let scene = &mut context.scenes[self.scene];

        // Rotate model according to input controller state.
        if self.input_controller.rotate_left {
            self.model_angle -= 5.0f32.to_radians();
        } else if self.input_controller.rotate_right {
            self.model_angle += 5.0f32.to_radians();
        }

        scene.graph[self.model_handle]
            .local_transform_mut()
            .set_rotation(UnitQuaternion::from_axis_angle(
                &Vector3::y_axis(),
                self.model_angle,
            ));

        if let GraphicsContext::Initialized(ref graphics_context) = context.graphics_context {
            context.user_interface.send_message(TextMessage::text(
                self.debug_text,
                MessageDirection::ToWidget,
                format!(
                    "Example - Terrain\nUse [A][D] keys to rotate camera.\nFPS: {}",
                    graphics_context.renderer.get_statistics().frames_per_second
                ),
            ));
        }
    }

    fn on_os_event(
        &mut self,
        event: &Event<()>,
        _context: PluginContext,
        _control_flow: &mut ControlFlow,
    ) {
        if let Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event: input, .. },
            ..
        } = event
        {
            // Handle key input events via `WindowEvent`, not via `DeviceEvent` (#32)
            match input.physical_key {
                KeyCode::KeyA => {
                    self.input_controller.rotate_left = input.state == ElementState::Pressed
                }
                KeyCode::KeyD => {
                    self.input_controller.rotate_right = input.state == ElementState::Pressed
                }
                _ => (),
            }
        }
    }
}

struct GameConstructor;

impl TypeUuidProvider for GameConstructor {
    fn type_uuid() -> Uuid {
        uuid!("f615ac42-b259-4a23-bb44-407d753ac178")
    }
}

impl PluginConstructor for GameConstructor {
    fn create_instance(
        &self,
        _override_scene: Handle<Scene>,
        context: PluginContext,
    ) -> Box<dyn Plugin> {
        // Create test scene.
        let loader = fyrox::core::futures::executor::block_on(SceneLoader::load_with(
            context.resource_manager.clone(),
        ));

        Box::new(Game {
            debug_text: TextBuilder::new(WidgetBuilder::new())
                .build(&mut context.user_interface.build_ctx()),
            input_controller: InputController {
                rotate_left: false,
                rotate_right: false,
            },
            scene: context.scenes.add(loader.scene),
            model_angle: 180.0f32.to_radians(),
            model_handle: loader.model_handle,
        })
    }
}

fn main() {
    let mut executor = Executor::from_params(
        Default::default(),
        GraphicsContextParams {
            window_attributes: WindowAttributes {
                title: "Example - Terrain".to_string(),
                ..Default::default()
            },
            vsync: true,
        },
    );
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
