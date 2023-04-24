//! Example - 2D
//!
//! Difficulty: Easy.
//!
//! This example shows simple 2D scene with light sources.

use fyrox::resource::model::{Model, ModelResourceExtension};
use fyrox::{
    asset::manager::ResourceManager,
    core::{algebra::Vector3, color::Color, futures::executor::block_on, pool::Handle},
    engine::{executor::Executor, GraphicsContext, GraphicsContextParams},
    event::Event,
    event::{ElementState, VirtualKeyCode, WindowEvent},
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
        camera::{CameraBuilder, OrthographicProjection, Projection},
        node::Node,
        Scene,
    },
    window::WindowAttributes,
};

struct SceneLoader {
    scene: Scene,
    camera: Handle<Node>,
}

impl SceneLoader {
    fn load_with(resource_manager: ResourceManager) -> Self {
        let mut scene = Scene::new();

        scene.ambient_lighting_color = Color::opaque(50, 50, 50);

        // Create camera first.
        let camera = CameraBuilder::new(BaseBuilder::new())
            .with_projection(Projection::Orthographic(OrthographicProjection {
                z_near: -0.1,
                z_far: 16.0,
                vertical_size: 4.0,
            }))
            .build(&mut scene.graph);

        // Load scene.
        block_on(resource_manager.request::<Model, _>("examples/data/2d/scene.rgs"))
            .unwrap()
            .instantiate(&mut scene);

        Self { scene, camera }
    }
}

struct InputController {
    move_forward: bool,
    move_backward: bool,
    move_left: bool,
    move_right: bool,
}

struct Game {
    input_controller: InputController,
    scene: Handle<Scene>,
    camera: Handle<Node>,
    debug_text: Handle<UiNode>,
}

impl Plugin for Game {
    fn update(&mut self, context: &mut PluginContext, _control_flow: &mut ControlFlow) {
        let mut offset = Vector3::default();
        if self.input_controller.move_forward {
            offset.y += 1.0
        }
        if self.input_controller.move_backward {
            offset.y -= 1.0
        }
        if self.input_controller.move_left {
            offset.x += 1.0
        }
        if self.input_controller.move_right {
            offset.x -= 1.0
        }

        let graph = &mut context.scenes[self.scene].graph;

        if let Some(offset) = offset.try_normalize(f32::EPSILON) {
            graph[self.camera]
                .local_transform_mut()
                .offset(offset.scale(0.1));
        }

        if let GraphicsContext::Initialized(ref mut graphics_context) = context.graphics_context {
            context.user_interface.send_message(TextMessage::text(
                self.debug_text,
                MessageDirection::ToWidget,
                format!(
                    "Example - 2D\n{}",
                    graphics_context.renderer.get_statistics()
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
            // Handle key input events via `WindowEvent`, not via `DeviceEvent` (#32)
            if let Some(key_code) = input.virtual_keycode {
                match key_code {
                    VirtualKeyCode::W => {
                        self.input_controller.move_forward = input.state == ElementState::Pressed
                    }
                    VirtualKeyCode::S => {
                        self.input_controller.move_backward = input.state == ElementState::Pressed
                    }
                    VirtualKeyCode::A => {
                        self.input_controller.move_left = input.state == ElementState::Pressed
                    }
                    VirtualKeyCode::D => {
                        self.input_controller.move_right = input.state == ElementState::Pressed
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
        let loader = SceneLoader::load_with(context.resource_manager.clone());

        Box::new(Game {
            // Create input controller - it will hold information about needed actions.
            input_controller: InputController {
                move_forward: false,
                move_backward: false,
                move_left: false,
                move_right: false,
            },
            // Add scene to engine - engine will take ownership over scene and will return
            // you a handle to scene which can be used later on to borrow it and do some
            // actions you need.
            scene: context.scenes.add(loader.scene),
            camera: loader.camera,
            // Create simple user interface that will show some useful info.
            debug_text: TextBuilder::new(WidgetBuilder::new())
                .build(&mut context.user_interface.build_ctx()),
        })
    }
}

fn main() {
    let mut executor = Executor::from_params(
        Default::default(),
        GraphicsContextParams {
            window_attributes: WindowAttributes {
                title: "Example - 2D Scene".to_string(),
                ..Default::default()
            },
            vsync: true,
        },
    );
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
