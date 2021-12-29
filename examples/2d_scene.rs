//! Example - 2D
//!
//! Difficulty: Easy.
//!
//! This example shows simple 2D scene with light sources.

use rg3d::{
    core::{algebra::Vector3, color::Color, futures::executor::block_on, pool::Handle},
    engine::{
        framework::prelude::*, resource_manager::MaterialSearchOptions,
        resource_manager::ResourceManager, Engine,
    },
    event::{ElementState, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
    gui::{
        message::MessageDirection,
        text::{TextBuilder, TextMessage},
        widget::WidgetBuilder,
        UiNode,
    },
    scene::{
        base::BaseBuilder,
        camera::{CameraBuilder, OrthographicProjection, Projection},
        dim2::rectangle::RectangleBuilder,
        node::Node,
        transform::TransformBuilder,
        Scene,
    },
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
        block_on(resource_manager.request_model(
            "examples/data/2d/scene.rgs",
            MaterialSearchOptions::UsePathDirectly,
        ))
        .unwrap()
        .instantiate_geometry(&mut scene);

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

impl GameState for Game {
    fn init(engine: &mut Engine) -> Self
    where
        Self: Sized,
    {
        // Create test scene.
        let loader = SceneLoader::load_with(engine.resource_manager.clone());

        Self {
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
            scene: engine.scenes.add(loader.scene),
            camera: loader.camera,
            // Create simple user interface that will show some useful info.
            debug_text: TextBuilder::new(WidgetBuilder::new())
                .build(&mut engine.user_interface.build_ctx()),
        }
    }

    fn on_tick(&mut self, engine: &mut Engine, _dt: f32, _: &mut ControlFlow) {
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

        let graph = &mut engine.scenes[self.scene].graph;

        if let Some(offset) = offset.try_normalize(f32::EPSILON) {
            graph[self.camera]
                .local_transform_mut()
                .offset(offset.scale(0.1));
        }

        engine.user_interface.send_message(TextMessage::text(
            self.debug_text,
            MessageDirection::ToWidget,
            format!("Example - 2D\n{}", engine.renderer.get_statistics()),
        ));
    }

    fn on_window_event(&mut self, _engine: &mut Engine, event: WindowEvent) {
        if let WindowEvent::KeyboardInput { input, .. } = event {
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

fn main() {
    Framework::<Game>::new()
        .unwrap()
        .title("Example - 2D")
        .run();
}
