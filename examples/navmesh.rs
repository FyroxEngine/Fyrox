//! Example 01. Simple scene.
//!
//! Difficulty: Easy.
//!
//! This example shows how to create simple scene with animated model.

pub mod shared;

use crate::shared::create_camera;

use fyrox::{
    core::{
        algebra::{Matrix4, Point3, UnitQuaternion, Vector2, Vector3},
        arrayvec::ArrayVec,
        color::Color,
        math::PositionProvider,
        pool::Handle,
        sstorage::ImmutableString,
    },
    dpi::LogicalPosition,
    engine::{executor::Executor, resource_manager::ResourceManager},
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
    gui::{
        message::MessageDirection,
        text::{TextBuilder, TextMessage},
        widget::WidgetBuilder,
        BuildContext, UiNode,
    },
    material::SharedMaterial,
    material::{Material, PropertyValue},
    plugin::{Plugin, PluginConstructor, PluginContext},
    scene::mesh::surface::SurfaceSharedData,
    scene::{
        base::BaseBuilder,
        debug::Line,
        graph::physics::{Intersection, RayCastOptions},
        mesh::{
            surface::{SurfaceBuilder, SurfaceData},
            MeshBuilder,
        },
        node::Node,
        transform::TransformBuilder,
        Scene,
    },
    utils::navmesh::NavmeshAgent,
};

fn create_ui(ctx: &mut BuildContext) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new()).build(ctx)
}

struct GameScene {
    scene: Scene,
    agent: Handle<Node>,
    cursor: Handle<Node>,
    camera: Handle<Node>,
}

async fn create_scene(resource_manager: ResourceManager) -> GameScene {
    let mut scene = Scene::new();

    // Set ambient light.
    scene.ambient_lighting_color = Color::opaque(200, 200, 200);

    // Camera is our eyes in the world - you won't see anything without it.
    let camera = create_camera(
        resource_manager.clone(),
        Vector3::new(0.0, 5.0, 0.0),
        &mut scene.graph,
    )
    .await;

    scene.graph[camera]
        .local_transform_mut()
        .set_rotation(UnitQuaternion::from_axis_angle(
            &Vector3::x_axis(),
            90.0f32.to_radians(),
        ));

    resource_manager
        .request_model("examples/data/navmesh_scene.rgs")
        .await
        .unwrap()
        .instantiate(&mut scene);

    let mut cursor_material = Material::standard();
    cursor_material
        .set_property(
            &ImmutableString::new("diffuseColor"),
            PropertyValue::Color(Color::opaque(255, 0, 0)),
        )
        .unwrap();

    let cursor = MeshBuilder::new(BaseBuilder::new())
        .with_surfaces(vec![SurfaceBuilder::new(SurfaceSharedData::new(
            SurfaceData::make_sphere(10, 10, 0.1, &Matrix4::identity()),
        ))
        .with_material(SharedMaterial::new(cursor_material))
        .build()])
        .build(&mut scene.graph);

    let mut agent_material = Material::standard();
    agent_material
        .set_property(
            &ImmutableString::new("diffuseColor"),
            PropertyValue::Color(Color::opaque(0, 200, 0)),
        )
        .unwrap();

    let agent = MeshBuilder::new(
        BaseBuilder::new().with_local_transform(
            TransformBuilder::new()
                .with_local_scale(Vector3::new(1.0, 2.0, 1.0))
                .build(),
        ),
    )
    .with_surfaces(vec![SurfaceBuilder::new(SurfaceSharedData::new(
        SurfaceData::make_sphere(10, 10, 0.2, &Matrix4::identity()),
    ))
    .with_material(SharedMaterial::new(agent_material))
    .build()])
    .build(&mut scene.graph);

    GameScene {
        scene,
        cursor,
        agent,
        camera,
    }
}

#[derive(Default)]
struct InputController {
    rotate_left: bool,
    rotate_right: bool,
}

struct Game {
    input_controller: InputController,
    scene_handle: Handle<Scene>,
    agent: Handle<Node>,
    cursor: Handle<Node>,
    camera: Handle<Node>,
    target_position: Vector3<f32>,
    mouse_position: Vector2<f32>,
    navmesh_agent: NavmeshAgent,
    debug_text: Handle<UiNode>,
}

impl Plugin for Game {
    fn update(&mut self, context: &mut PluginContext, _control_flow: &mut ControlFlow) {
        // Use stored scene handle to borrow a mutable reference of scene in
        // engine.
        let scene = &mut context.scenes[self.scene_handle];

        scene.drawing_context.clear_lines();

        let ray = scene.graph[self.camera]
            .as_camera()
            .make_ray(self.mouse_position, context.renderer.get_frame_bounds());

        let mut buffer = ArrayVec::<Intersection, 64>::new();
        scene.graph.physics.cast_ray(
            RayCastOptions {
                ray_origin: Point3::from(ray.origin),
                ray_direction: ray.dir,
                max_len: 9999.0,
                groups: Default::default(),
                sort_results: true,
            },
            &mut buffer,
        );

        if let Some(first) = buffer.first() {
            self.target_position = first.position.coords;
            scene.graph[self.cursor]
                .local_transform_mut()
                .set_position(self.target_position);
        }

        let navmesh = scene.navmeshes.iter_mut().next().unwrap();

        let last = std::time::Instant::now();
        self.navmesh_agent.set_target(self.target_position);
        let _ = self.navmesh_agent.update(context.dt, navmesh);
        let agent_time = std::time::Instant::now() - last;

        scene.graph[self.agent]
            .local_transform_mut()
            .set_position(self.navmesh_agent.position());

        // Debug drawing.
        for pt in navmesh.vertices() {
            for neighbour in pt.neighbours() {
                scene.drawing_context.add_line(Line {
                    begin: pt.position(),
                    end: navmesh.vertices()[*neighbour as usize].position(),
                    color: Color::opaque(0, 0, 200),
                });
            }
        }

        for pts in self.navmesh_agent.path().windows(2) {
            scene.drawing_context.add_line(Line {
                begin: pts[0],
                end: pts[1],
                color: Color::opaque(255, 0, 0),
            });
        }

        let fps = context.renderer.get_statistics().frames_per_second;
        let text = format!(
            "Example 12 - Navigation Mesh\nFPS: {}\nAgent time: {:?}",
            fps, agent_time
        );
        context.user_interface.send_message(TextMessage::text(
            self.debug_text,
            MessageDirection::ToWidget,
            text,
        ));
    }

    fn on_os_event(
        &mut self,
        event: &Event<()>,
        context: PluginContext,
        _control_flow: &mut ControlFlow,
    ) {
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::KeyboardInput { input, .. } => {
                    // Handle key input events via `WindowEvent`, not via `DeviceEvent` (#32)
                    if let Some(key_code) = input.virtual_keycode {
                        match key_code {
                            VirtualKeyCode::A => {
                                self.input_controller.rotate_left =
                                    input.state == ElementState::Pressed
                            }
                            VirtualKeyCode::D => {
                                self.input_controller.rotate_right =
                                    input.state == ElementState::Pressed
                            }
                            _ => (),
                        }
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let p: LogicalPosition<f32> =
                        position.to_logical(context.window.scale_factor());
                    self.mouse_position = Vector2::new(p.x as f32, p.y as f32);
                }
                _ => (),
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
        let debug_text = create_ui(&mut context.user_interface.build_ctx());

        // Create test scene.
        let GameScene {
            scene,
            agent,
            cursor,
            camera,
        } = fyrox::core::futures::executor::block_on(create_scene(
            context.resource_manager.clone(),
        ));

        // Add scene to engine - engine will take ownership over scene and will return
        // you a handle to scene which can be used later on to borrow it and do some
        // actions you need.
        let scene_handle = context.scenes.add(scene);

        let mut navmesh_agent = NavmeshAgent::new();
        navmesh_agent.set_speed(0.75);

        Box::new(Game {
            input_controller: Default::default(),
            scene_handle,
            agent,
            cursor,
            camera,
            target_position: Default::default(),
            mouse_position: Default::default(),
            navmesh_agent,
            debug_text,
        })
    }
}

fn main() {
    let mut executor = Executor::new();
    executor
        .get_window()
        .set_title("Example 12 - Navigation Mesh");
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
