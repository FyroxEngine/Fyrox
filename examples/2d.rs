//! Example - 2D
//!
//! Difficulty: Easy.
//!
//! This example shows simple 2D scene with light sources.

extern crate rg3d;

use rg3d::{
    core::{algebra::Vector2, pool::Handle},
    engine::{framework::prelude::*, resource_manager::ResourceManager},
    event::{ElementState, VirtualKeyCode, WindowEvent},
    gui::{
        message::{MessageDirection, TextMessage},
        text::TextBuilder,
        widget::WidgetBuilder,
    },
    scene2d::{
        base::BaseBuilder, camera::CameraBuilder, light::point::PointLightBuilder,
        light::spot::SpotLightBuilder, light::BaseLightBuilder, node::Node, sprite::SpriteBuilder,
        transform::TransformBuilder, Scene2d,
    },
};

struct SceneLoader {
    scene: Scene2d,
    camera: Handle<Node>,
    spot_light: Handle<Node>,
}

impl SceneLoader {
    fn load_with(resource_manager: ResourceManager) -> Self {
        let mut scene = Scene2d::new();

        // Create camera first.
        let camera = CameraBuilder::new(BaseBuilder::new()).build(&mut scene.graph);

        // Add some sprites.
        for y in 0..10 {
            for x in 0..10 {
                let sprite_size = 64.0;
                let spacing = 5.0;
                SpriteBuilder::new(
                    BaseBuilder::new().with_local_transform(
                        TransformBuilder::new()
                            .with_position(Vector2::new(
                                100.0 + x as f32 * (sprite_size + spacing),
                                100.0 + y as f32 * (sprite_size + spacing),
                            ))
                            .build(),
                    ),
                )
                .with_texture(resource_manager.request_texture("examples/data/starship.png"))
                .with_size(sprite_size)
                .build(&mut scene.graph);
            }
        }

        // Add some lights.
        PointLightBuilder::new(BaseLightBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_position(Vector2::new(300.0, 200.0))
                    .build(),
            ),
        ))
        .with_radius(200.0)
        .build(&mut scene.graph);

        let spot_light = SpotLightBuilder::new(BaseLightBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_position(Vector2::new(500.0, 400.0))
                    .build(),
            ),
        ))
        .with_radius(200.0)
        .build(&mut scene.graph);

        Self {
            scene,
            camera,
            spot_light,
        }
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
    scene: Handle<Scene2d>,
    camera: Handle<Node>,
    spot_light: Handle<Node>,
    debug_text: Handle<UiNode>,
}

impl GameState for Game {
    fn init(engine: &mut GameEngine) -> Self
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
            scene: engine.scenes2d.add(loader.scene),
            camera: loader.camera,
            spot_light: loader.spot_light,
            // Create simple user interface that will show some useful info.
            debug_text: TextBuilder::new(WidgetBuilder::new())
                .build(&mut engine.user_interface.build_ctx()),
        }
    }

    fn on_tick(&mut self, engine: &mut GameEngine, _dt: f32) {
        let mut offset = Vector2::default();
        if self.input_controller.move_forward {
            offset.y -= 10.0
        }
        if self.input_controller.move_backward {
            offset.y += 10.0
        }
        if self.input_controller.move_left {
            offset.x -= 10.0
        }
        if self.input_controller.move_right {
            offset.x += 10.0
        }

        let graph = &mut engine.scenes2d[self.scene].graph;

        if let Some(offset) = offset.try_normalize(f32::EPSILON) {
            graph[self.camera].local_transform_mut().offset(offset);
        }

        graph[self.spot_light]
            .local_transform_mut()
            .turn(10.0f32.to_radians());

        engine.user_interface.send_message(TextMessage::text(
            self.debug_text,
            MessageDirection::ToWidget,
            format!("Example - 2D\n{}", engine.renderer.get_statistics()),
        ));
    }

    fn on_window_event(&mut self, _engine: &mut GameEngine, event: WindowEvent) {
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
