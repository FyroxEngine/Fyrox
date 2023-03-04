use fyrox::{
    core::{
        algebra::{Matrix4, UnitQuaternion, Vector3},
        color::Color,
        pool::Handle,
    },
    engine::{executor::Executor, GraphicsContextParams},
    event_loop::{ControlFlow, EventLoop},
    plugin::{Plugin, PluginConstructor, PluginContext},
    scene::{
        base::BaseBuilder,
        camera::CameraBuilder,
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

struct Game {
    scene: Handle<Scene>,
    cube: Handle<Node>,
    angle: f32,
}

impl Plugin for Game {
    fn update(&mut self, context: &mut PluginContext, _control_flow: &mut ControlFlow) {
        context.scenes[self.scene].graph[self.cube]
            .local_transform_mut()
            .set_rotation(UnitQuaternion::from_axis_angle(
                &Vector3::x_axis(),
                self.angle,
            ));

        self.angle += context.dt;
    }

    fn on_graphics_context_initialized(
        &mut self,
        context: PluginContext,
        _control_flow: &mut ControlFlow,
    ) {
        context
            .graphics_context
            .as_initialized_mut()
            .renderer
            .set_backbuffer_clear_color(Color::TRANSPARENT);
    }
}

struct GameConstructor;

impl PluginConstructor for GameConstructor {
    fn create_instance(
        &self,
        _override_scene: Handle<Scene>,
        context: PluginContext,
    ) -> Box<dyn Plugin> {
        let mut scene = Scene::new();

        CameraBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(Vector3::new(0.0, 0.0, -2.0))
                    .build(),
            ),
        )
        .build(&mut scene.graph);

        let cube = MeshBuilder::new(BaseBuilder::new())
            .with_surfaces(vec![SurfaceBuilder::new(SurfaceSharedData::new(
                SurfaceData::make_cube(Matrix4::identity()),
            ))
            .build()])
            .build(&mut scene.graph);

        let scene = context.scenes.add(scene);

        Box::new(Game {
            scene,
            cube,
            angle: 0.0,
        })
    }
}

fn main() {
    let mut executor = Executor::from_params(
        EventLoop::new(),
        GraphicsContextParams {
            window_attributes: WindowAttributes {
                transparent: true,
                title: "Example - Transparent Window".to_string(),
                ..Default::default()
            },
            vsync: true,
        },
    );
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
