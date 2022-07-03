//! Example - 2D
//!
//! Difficulty: Easy.
//!
//! This example shows simple 2D scene with light sources.

use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        color::Color,
        pool::Handle,
        uuid::{uuid, Uuid},
    },
    engine::{executor::Executor, resource_manager::ResourceManager},
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
        camera::{CameraBuilder, OrthographicProjection, Projection},
        dim2::rectangle::RectangleBuilder,
        light::{point::PointLightBuilder, spot::SpotLightBuilder, BaseLightBuilder},
        node::{Node, TypeUuidProvider},
        transform::TransformBuilder,
        Scene,
    },
};

struct SceneLoader {
    scene: Scene,
    camera: Handle<Node>,
    spot_light: Handle<Node>,
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
                vertical_size: 2.0,
            }))
            .build(&mut scene.graph);

        // Add some sprites.
        for y in 0..10 {
            for x in 0..10 {
                let sprite_size = 0.35;
                let spacing = 0.1;
                RectangleBuilder::new(
                    BaseBuilder::new().with_local_transform(
                        TransformBuilder::new()
                            .with_local_position(Vector3::new(
                                0.1 + x as f32 * (sprite_size + spacing),
                                0.1 + y as f32 * (sprite_size + spacing),
                                0.0, // Keep Z at zero.
                            ))
                            .with_local_scale(Vector3::new(sprite_size, sprite_size, f32::EPSILON))
                            .build(),
                    ),
                )
                .with_texture(resource_manager.request_texture("examples/data/starship.png"))
                .build(&mut scene.graph);
            }
        }

        // Add some lights.
        PointLightBuilder::new(
            BaseLightBuilder::new(
                BaseBuilder::new().with_local_transform(
                    TransformBuilder::new()
                        .with_local_position(Vector3::new(2.5, 2.5, 0.0))
                        .build(),
                ),
            )
            .with_scatter_enabled(false),
        )
        .with_radius(1.0)
        .build(&mut scene.graph);

        let spot_light = SpotLightBuilder::new(
            BaseLightBuilder::new(
                BaseBuilder::new().with_local_transform(
                    TransformBuilder::new()
                        .with_local_position(Vector3::new(3.0, 1.0, 0.0))
                        .build(),
                ),
            )
            .with_scatter_enabled(false),
        )
        .with_distance(2.0)
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
    scene: Handle<Scene>,
    camera: Handle<Node>,
    spot_light: Handle<Node>,
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

        let local_transform = graph[self.spot_light].local_transform_mut();
        let new_rotation = **local_transform.rotation()
            * UnitQuaternion::from_euler_angles(0.0, 0.0, 1.0f32.to_radians());
        local_transform.set_rotation(new_rotation);

        context.user_interface.send_message(TextMessage::text(
            self.debug_text,
            MessageDirection::ToWidget,
            format!("Example - 2D\n{}", context.renderer.get_statistics()),
        ));
    }

    fn id(&self) -> Uuid {
        GameConstructor::type_uuid()
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
            spot_light: loader.spot_light,
            // Create simple user interface that will show some useful info.
            debug_text: TextBuilder::new(WidgetBuilder::new())
                .build(&mut context.user_interface.build_ctx()),
        })
    }
}

fn main() {
    let mut executor = Executor::new();
    executor.get_window().set_title("Example - 2D");
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
