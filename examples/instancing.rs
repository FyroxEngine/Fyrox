//! Example 10. Instancing.
//!
//! Difficulty: Easy.
//!
//! This example shows how to create simple scene with lots of animated models with low performance
//! impact.

pub mod shared;

use crate::shared::create_camera;
use fyrox::resource::model::{Model, ModelResourceExtension};
use fyrox::resource::texture::Texture;
use fyrox::{
    asset::manager::ResourceManager,
    core::{
        algebra::{Matrix4, UnitQuaternion, Vector3},
        color::Color,
        pool::Handle,
        sstorage::ImmutableString,
    },
    engine::{executor::Executor, GraphicsContext, GraphicsContextParams},
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
    gui::{
        message::MessageDirection,
        text::{TextBuilder, TextMessage},
        widget::WidgetBuilder,
        UiNode,
    },
    material::{shader::SamplerFallback, Material, PropertyValue, SharedMaterial},
    plugin::{Plugin, PluginConstructor, PluginContext},
    rand::Rng,
    renderer::QualitySettings,
    scene::{
        animation::AnimationPlayer,
        base::BaseBuilder,
        light::{point::PointLightBuilder, BaseLightBuilder},
        mesh::{
            surface::{SurfaceBuilder, SurfaceData, SurfaceSharedData},
            MeshBuilder,
        },
        node::Node,
        transform::TransformBuilder,
        Scene,
    },
    window::WindowAttributes,
};

struct SceneLoader {
    scene: Scene,
    camera: Handle<Node>,
}

impl SceneLoader {
    async fn load_with(resource_manager: ResourceManager) -> Self {
        let mut scene = Scene::new();

        // Set ambient light.
        scene.ambient_lighting_color = Color::opaque(100, 100, 100);

        // Camera is our eyes in the world - you won't see anything without it.
        let camera = create_camera(
            resource_manager.clone(),
            Vector3::new(0.0, 32.0, -140.0),
            &mut scene.graph,
        )
        .await;

        // Load model and animation resource in parallel. Is does *not* adds anything to
        // our scene - it just loads a resource then can be used later on to instantiate
        // models from it on scene. Why loading of resource is separated from instantiation?
        // Because it is too inefficient to load a resource every time you trying to
        // create instance of it - much more efficient is to load it once and then make copies
        // of it. In case of models it is very efficient because single vertex and index buffer
        // can be used for all models instances, so memory footprint on GPU will be lower.
        let (model_resource, walk_animation_resource) = fyrox::core::futures::join!(
            resource_manager.request::<Model, _>("examples/data/mutant/mutant.FBX"),
            resource_manager.request::<Model, _>("examples/data/mutant/walk.fbx")
        );

        for z in -10..10 {
            for x in -10..10 {
                // Instantiate model on scene - but only geometry, without any animations.
                // Instantiation is a process of embedding model resource data in desired scene.
                let model_handle = model_resource.clone().unwrap().instantiate(&mut scene);

                // Now we have whole sub-graph instantiated, we can start modifying model instance.
                scene.graph[model_handle]
                    .local_transform_mut()
                    // Our model is too big, fix it by scale.
                    .set_scale(Vector3::new(0.05, 0.05, 0.05))
                    .set_rotation(UnitQuaternion::from_axis_angle(
                        &Vector3::y_axis(),
                        180.0f32.to_radians(),
                    ))
                    .set_position(Vector3::new((x as f32) * 7.0, 0.0, (z as f32) * 7.0));

                // Add simple animation for our model. Animations are loaded from model resources -
                // this is because animation is a set of skeleton bones with their own transforms.
                // Once animation resource is loaded it must be re-targeted to our model instance.
                // Why? Because animation in *resource* uses information about *resource* bones,
                // not model instance bones, retarget_animations maps animations of each bone on
                // model instance so animation will know about nodes it should operate on.
                let walk_animation = *walk_animation_resource
                    .clone()
                    .unwrap()
                    .retarget_animations(model_handle, &mut scene.graph)
                    .get(0)
                    .unwrap();

                let animation_player_handle = scene
                    .graph
                    .find(model_handle, &mut |n| {
                        n.query_component_ref::<AnimationPlayer>().is_some()
                    })
                    .unwrap()
                    .0;

                (**scene.graph[animation_player_handle]
                    .query_component_mut::<AnimationPlayer>()
                    .unwrap()
                    .animations_mut())
                .get_mut(walk_animation)
                .set_speed(fyrox::rand::thread_rng().gen_range(0.8..1.2));
            }
        }

        // Add point light with shadows.
        PointLightBuilder::new(BaseLightBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(Vector3::new(0.0, 30.0, -50.0))
                    .build(),
            ),
        ))
        .with_radius(100.0)
        .build(&mut scene.graph);

        let mut material = Material::standard();

        material
            .set_property(
                &ImmutableString::new("diffuseTexture"),
                PropertyValue::Sampler {
                    value: Some(
                        resource_manager.request::<Texture, _>("examples/data/concrete2.dds"),
                    ),
                    fallback: SamplerFallback::White,
                },
            )
            .unwrap();

        // Add floor.
        MeshBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(Vector3::new(0.0, -0.25, 0.0))
                    .build(),
            ),
        )
        .with_surfaces(vec![SurfaceBuilder::new(SurfaceSharedData::new(
            SurfaceData::make_cube(Matrix4::new_nonuniform_scaling(&Vector3::new(
                300.0, 0.25, 300.0,
            ))),
        ))
        .with_material(SharedMaterial::new(material))
        .build()])
        .build(&mut scene.graph);

        Self { scene, camera }
    }
}

struct InputController {
    rotate_left: bool,
    rotate_right: bool,
}

struct Game {
    input_controller: InputController,
    debug_text: Handle<UiNode>,
    camera_angle: f32,
    scene: Handle<Scene>,
    camera: Handle<Node>,
}

impl Plugin for Game {
    fn update(&mut self, context: &mut PluginContext, _control_flow: &mut ControlFlow) {
        // Use stored scene handle to borrow a mutable reference of scene in
        // engine.
        let scene = &mut context.scenes[self.scene];

        // Rotate model according to input controller state.
        if self.input_controller.rotate_left {
            self.camera_angle -= 5.0f32.to_radians();
        } else if self.input_controller.rotate_right {
            self.camera_angle += 5.0f32.to_radians();
        }

        scene.graph[self.camera].local_transform_mut().set_rotation(
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), self.camera_angle),
        );

        if let GraphicsContext::Initialized(ref ctx) = context.graphics_context {
            context.user_interface.send_message(TextMessage::text(
                self.debug_text,
                MessageDirection::ToWidget,
                format!(
                    "Example 10 - Instancing\n\
                    Use [A][D] keys to rotate camera.\n\
                    {}",
                    ctx.renderer.get_statistics()
                ),
            ));
        }
    }

    fn on_graphics_context_initialized(
        &mut self,
        context: PluginContext,
        _control_flow: &mut ControlFlow,
    ) {
        let mut settings = QualitySettings::ultra();
        settings.point_shadows_distance = 1000.0;
        context
            .graphics_context
            .as_initialized_mut()
            .renderer
            .set_quality_settings(&settings)
            .unwrap();
    }

    fn on_os_event(
        &mut self,
        event: &Event<()>,
        _context: PluginContext,
        _control_flow: &mut ControlFlow,
    ) {
        if let Event::WindowEvent {
            event: WindowEvent::KeyboardInput { input, .. },
            ..
        } = event
        {
            if let Some(key_code) = input.virtual_keycode {
                match key_code {
                    VirtualKeyCode::A => {
                        self.input_controller.rotate_left = input.state == ElementState::Pressed
                    }
                    VirtualKeyCode::D => {
                        self.input_controller.rotate_right = input.state == ElementState::Pressed
                    }
                    _ => (),
                }
            }
        }
    }
}

struct GameConstructor;

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
            input_controller: InputController {
                rotate_left: false,
                rotate_right: false,
            },
            debug_text: TextBuilder::new(WidgetBuilder::new())
                .build(&mut context.user_interface.build_ctx()),
            camera_angle: 0.0,
            scene: context.scenes.add(loader.scene),
            camera: loader.camera,
        })
    }
}

fn main() {
    let mut executor = Executor::from_params(
        Default::default(),
        GraphicsContextParams {
            window_attributes: WindowAttributes {
                title: "Example - Instancing".to_string(),
                ..Default::default()
            },
            vsync: true,
        },
    );
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
