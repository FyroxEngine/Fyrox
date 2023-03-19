pub mod shared;

use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        color::Color,
        pool::Handle,
    },
    engine::{
        executor::Executor, resource_manager::ResourceManager, GraphicsContext,
        GraphicsContextParams,
    },
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
    gui::{
        message::MessageDirection,
        text::{TextBuilder, TextMessage},
        widget::WidgetBuilder,
        UiNode,
    },
    plugin::{Plugin, PluginConstructor, PluginContext},
    scene::{
        base::BaseBuilder,
        camera::CameraBuilder,
        light::{point::PointLightBuilder, BaseLightBuilder},
        node::Node,
        transform::TransformBuilder,
        Scene,
    },
    window::WindowAttributes,
};

struct GameSceneLoader {
    scene: Scene,
    model_handle: Handle<Node>,
}

impl GameSceneLoader {
    async fn load_with(resource_manager: ResourceManager) -> Self {
        let mut scene = Scene::new();

        scene.ambient_lighting_color = Color::opaque(200, 200, 200);

        CameraBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(Vector3::new(0.0, 2.0, -5.0))
                    .build(),
            ),
        )
        .build(&mut scene.graph);

        PointLightBuilder::new(BaseLightBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(Vector3::new(0.0, 12.0, 0.0))
                    .build(),
            ),
        ))
        .with_radius(20.0)
        .build(&mut scene.graph);

        let model_resource = resource_manager
            .request_model("examples/data/morph2.fbx")
            .await
            .unwrap();

        let model_handle = model_resource.instantiate(&mut scene);

        scene.graph[model_handle]
            .local_transform_mut()
            .set_scale(Vector3::new(0.05, 0.05, 0.05));

        let sphere = scene.graph.find_by_name_from_root("Sphere001").unwrap().0;
        let blend_shape = scene.graph[sphere].as_mesh_mut();

        for surface in blend_shape.surfaces_mut() {
            let data = surface.data();
            let mut data = data.lock();
            data.update_blend_shape_weights(&[100.0, 100.0]).unwrap();
        }

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
    scene: Handle<Scene>,
    model_handle: Handle<Node>,
    input_controller: InputController,
    debug_text: Handle<UiNode>,
    model_angle: f32,
}

impl Plugin for Game {
    fn update(&mut self, context: &mut PluginContext, _control_flow: &mut ControlFlow) {
        let scene = &mut context.scenes[self.scene];

        // Rotate model according to input controller state
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
                    "Example 01 - Simple Scene\nUse [A][D] keys to rotate model.\nFPS: {}",
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
        let scene = fyrox::core::futures::executor::block_on(GameSceneLoader::load_with(
            context.resource_manager.clone(),
        ));

        Box::new(Game {
            debug_text: TextBuilder::new(WidgetBuilder::new())
                .build(&mut context.user_interface.build_ctx()),
            scene: context.scenes.add(scene.scene),
            model_handle: scene.model_handle,
            // Create input controller - it will hold information about needed actions.
            input_controller: InputController {
                rotate_left: false,
                rotate_right: false,
            },
            // We will rotate model using keyboard input.
            model_angle: 180.0f32.to_radians(),
        })
    }
}

fn main() {
    let mut executor = Executor::from_params(
        Default::default(),
        GraphicsContextParams {
            window_attributes: WindowAttributes {
                title: "Example - Simple".to_string(),
                ..Default::default()
            },
            vsync: true,
        },
    );
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
