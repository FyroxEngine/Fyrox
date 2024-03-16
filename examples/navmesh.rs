use fyrox_graph::SceneGraph;
use fyrox_impl::{
    asset::manager::ResourceManager,
    core::{
        algebra::{Matrix4, Point3, UnitQuaternion, Vector2, Vector3},
        arrayvec::ArrayVec,
        color::Color,
        pool::Handle,
        sstorage::ImmutableString,
    },
    dpi::LogicalPosition,
    engine::{executor::Executor, GraphicsContext, GraphicsContextParams},
    event::{ElementState, Event, WindowEvent},
    event_loop::EventLoop,
    keyboard::KeyCode,
    material::{Material, MaterialResource, PropertyValue},
    plugin::{Plugin, PluginConstructor, PluginContext},
    resource::model::{Model, ModelResourceExtension},
    scene::{
        base::BaseBuilder,
        camera::CameraBuilder,
        debug::Line,
        graph::physics::{Intersection, RayCastOptions},
        mesh::{
            surface::{SurfaceBuilder, SurfaceData, SurfaceSharedData},
            MeshBuilder,
        },
        node::{Node, NodeTrait},
        transform::TransformBuilder,
        Scene,
    },
    utils::navmesh::NavmeshAgent,
};
use winit::keyboard::PhysicalKey;

struct GameScene {
    scene: Scene,
    agent: Handle<Node>,
    cursor: Handle<Node>,
    camera: Handle<Node>,
}

async fn create_scene(resource_manager: ResourceManager) -> GameScene {
    let mut scene = Scene::new();

    // Camera is our eyes in the world - you won't see anything without it.
    let camera = CameraBuilder::new(
        BaseBuilder::new().with_local_transform(
            TransformBuilder::new()
                .with_local_position(Vector3::new(0.0, 5.0, 0.0))
                .build(),
        ),
    )
    .build(&mut scene.graph);

    scene.graph[camera]
        .local_transform_mut()
        .set_rotation(UnitQuaternion::from_axis_angle(
            &Vector3::x_axis(),
            90.0f32.to_radians(),
        ));

    resource_manager
        .request::<Model>("examples/data/navmesh_scene.rgs")
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
        .with_material(MaterialResource::new_ok(
            Default::default(),
            cursor_material,
        ))
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
    .with_material(MaterialResource::new_ok(Default::default(), agent_material))
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
}

impl Plugin for Game {
    fn update(&mut self, context: &mut PluginContext) {
        // Use stored scene handle to borrow a mutable reference of scene in
        // engine.
        let scene = &mut context.scenes[self.scene_handle];

        scene.drawing_context.clear_lines();

        if let GraphicsContext::Initialized(ref graphics_context) = context.graphics_context {
            let ray = scene.graph[self.camera].as_camera().make_ray(
                self.mouse_position,
                graphics_context.renderer.get_frame_bounds(),
            );

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
        }

        let navmesh_handle = scene.graph.find_by_name_from_root("Navmesh").unwrap().0;
        let navmesh_node = scene.graph[navmesh_handle].as_navigational_mesh_mut();
        navmesh_node.debug_draw(&mut scene.drawing_context);
        let navmesh = navmesh_node.navmesh_ref();

        self.navmesh_agent.set_target(self.target_position);
        let _ = self.navmesh_agent.update(context.dt, &navmesh);
        drop(navmesh);

        scene.graph[self.agent]
            .local_transform_mut()
            .set_position(self.navmesh_agent.position());

        for pts in self.navmesh_agent.path().windows(2) {
            scene.drawing_context.add_line(Line {
                begin: pts[0],
                end: pts[1],
                color: Color::opaque(255, 0, 0),
            });
        }
    }

    fn on_os_event(&mut self, event: &Event<()>, context: PluginContext) {
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::KeyboardInput { event: input, .. } => match input.physical_key {
                    PhysicalKey::Code(KeyCode::KeyA) => {
                        self.input_controller.rotate_left = input.state == ElementState::Pressed
                    }
                    PhysicalKey::Code(KeyCode::KeyD) => {
                        self.input_controller.rotate_right = input.state == ElementState::Pressed
                    }
                    _ => (),
                },
                WindowEvent::CursorMoved { position, .. } => {
                    if let GraphicsContext::Initialized(ref graphics_context) =
                        context.graphics_context
                    {
                        let p: LogicalPosition<f32> =
                            position.to_logical(graphics_context.window.scale_factor());
                        self.mouse_position = Vector2::new(p.x, p.y);
                    }
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
        _override_scene: Option<&str>,
        context: PluginContext,
    ) -> Box<dyn Plugin> {
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
        })
    }
}

fn main() {
    let mut executor = Executor::from_params(
        EventLoop::new().unwrap(),
        GraphicsContextParams {
            window_attributes: Default::default(),
            vsync: true,
            msaa_sample_count: None,
        },
    );
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
