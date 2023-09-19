use fyrox::engine::GraphicsContextParams;
use fyrox::window::WindowAttributes;
use fyrox::{
    core::{
        algebra::{Point2, Vector2, Vector3},
        color::Color,
        pool::Handle,
    },
    engine::executor::Executor,
    event_loop::ControlFlow,
    plugin::{Plugin, PluginConstructor, PluginContext},
    scene::{
        base::BaseBuilder,
        camera::{CameraBuilder, OrthographicProjection, Projection},
        dim2::{
            collider::{ColliderBuilder, ColliderShape, CuboidShape},
            physics::RayCastOptions,
            rectangle::RectangleBuilder,
            rigidbody::RigidBodyBuilder,
        },
        graph::Graph,
        node::Node,
        rigidbody::RigidBodyType,
        transform::TransformBuilder,
        Scene,
    },
};
use winit::event_loop::EventLoop;

struct Game {
    from: Vector2<f32>,
    camera: Handle<Node>,
    scene: Handle<Scene>,
    cursor: Handle<Node>,
    intersection: Handle<Node>,
}

impl Plugin for Game {
    fn update(&mut self, context: &mut PluginContext, _control_flow: &mut ControlFlow) {
        let scene = &mut context.scenes[self.scene];

        let cursor_pos = scene.graph[self.camera]
            .as_camera()
            .make_ray(
                context.user_interface.cursor_position(),
                context.user_interface.screen_size(),
            )
            .origin;

        scene.graph[self.cursor]
            .local_transform_mut()
            .set_position(cursor_pos);

        let ray_direction = cursor_pos.xy() - self.from;

        let mut buffer = Vec::new();
        scene.graph.physics2d.cast_ray(
            RayCastOptions {
                ray_origin: Point2::from(self.from),
                ray_direction,
                max_len: ray_direction.norm(),
                groups: Default::default(),
                sort_results: true,
            },
            &mut buffer,
        );

        if let Some(first) = buffer.first() {
            scene.graph[self.intersection]
                .local_transform_mut()
                .set_position(first.position.coords.to_homogeneous());
        }
    }
}

struct GameConstructor;

fn create_rect(
    graph: &mut Graph,
    position: Vector3<f32>,
    size: Vector2<f32>,
    color: Color,
) -> Handle<Node> {
    RectangleBuilder::new(
        BaseBuilder::new().with_local_transform(
            TransformBuilder::new()
                .with_local_position(position)
                .with_local_scale(Vector3::new(size.x, size.y, 1.0))
                .build(),
        ),
    )
    .with_color(color)
    .build(graph)
}

impl PluginConstructor for GameConstructor {
    fn create_instance(
        &self,
        _override_scene: Handle<Scene>,
        context: PluginContext,
    ) -> Box<dyn Plugin> {
        let mut scene = Scene::new();

        // Camera
        let camera = CameraBuilder::new(BaseBuilder::new())
            .with_projection(Projection::Orthographic(OrthographicProjection::default()))
            .build(&mut scene.graph);

        // Obstacles
        for pos in [
            Vector2::new(0.0, 3.0),
            Vector2::new(0.0, -3.0),
            Vector2::new(3.0, 0.0),
            Vector2::new(-3.0, 0.0),
        ] {
            RigidBodyBuilder::new(
                BaseBuilder::new()
                    .with_local_transform(
                        TransformBuilder::new()
                            .with_local_position(pos.to_homogeneous())
                            .build(),
                    )
                    .with_children(&[
                        ColliderBuilder::new(BaseBuilder::new())
                            .with_shape(ColliderShape::Cuboid(CuboidShape {
                                half_extents: Vector2::new(1.0, 1.0),
                            }))
                            .build(&mut scene.graph),
                        create_rect(
                            &mut scene.graph,
                            Default::default(),
                            Vector2::new(2.0, 2.0),
                            Default::default(),
                        ),
                    ]),
            )
            .with_body_type(RigidBodyType::Static)
            .build(&mut scene.graph);
        }

        let from = Vector2::new(0.0, 0.0);
        create_rect(
            &mut scene.graph,
            from.to_homogeneous(),
            Vector2::new(0.1, 0.1),
            Color::GREEN,
        );

        // Cursor
        let cursor = create_rect(
            &mut scene.graph,
            Default::default(),
            Vector2::new(0.1, 0.1),
            Color::RED,
        );

        // Intersection
        let intersection = create_rect(
            &mut scene.graph,
            Default::default(),
            Vector2::new(0.1, 0.1),
            Color::opaque(255, 0, 255),
        );

        let scene = context.scenes.add(scene);

        Box::new(Game {
            cursor,
            from,
            intersection,
            camera,
            scene,
        })
    }
}

fn main() {
    let mut executor = Executor::from_params(
        EventLoop::new().unwrap(),
        GraphicsContextParams {
            window_attributes: WindowAttributes {
                title: "Example - 2D Raycasting".to_string(),
                ..Default::default()
            },
            vsync: true,
        },
    );
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
