use fyrox::scene::mesh::surface::SurfaceSharedData;
use fyrox::{
    core::{
        algebra::{Matrix4, UnitQuaternion, Vector3},
        color::Color,
        pool::Handle,
    },
    engine::executor::Executor,
    event_loop::ControlFlow,
    plugin::{Plugin, PluginConstructor, PluginContext},
    scene::{
        base::BaseBuilder,
        camera::CameraBuilder,
        mesh::{
            surface::{SurfaceBuilder, SurfaceData},
            MeshBuilder,
        },
        node::Node,
        transform::TransformBuilder,
        Scene,
    },
    window::WindowBuilder,
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
}

struct GameConstructor;

impl PluginConstructor for GameConstructor {
    fn create_instance(
        &self,
        _override_scene: Handle<Scene>,
        context: PluginContext,
    ) -> Box<dyn Plugin> {
        context
            .renderer
            .set_backbuffer_clear_color(Color::TRANSPARENT);

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
        WindowBuilder::new()
            .with_transparent(true)
            .with_title("Example - Transparent Window"),
        true,
    );
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
