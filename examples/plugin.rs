use fyrox::engine::GraphicsContext;
use fyrox::event_loop::EventLoop;
use fyrox::{
    core::{
        color::{Color, Hsv},
        pool::Handle,
    },
    engine::{executor::Executor, GraphicsContextParams},
    event_loop::ControlFlow,
    plugin::{Plugin, PluginConstructor, PluginContext},
    scene::Scene,
    window::WindowAttributes,
};

struct Game {
    hue: f32,
}

impl Plugin for Game {
    // Implement a function that will update game logic and will be called at fixed rate of 60 Hz.
    fn update(&mut self, context: &mut PluginContext, _control_flow: &mut ControlFlow) {
        // Increase hue at fixed rate of 24 degrees per second.
        self.hue += 24.0 * context.dt;

        // Slowly change color of the window.
        if let GraphicsContext::Initialized(ref mut graphics_context) = context.graphics_context {
            graphics_context
                .renderer
                .set_backbuffer_clear_color(Color::from(Hsv::new(self.hue % 360.0, 100.0, 100.0)))
        }
    }
}

struct GameConstructor;

impl PluginConstructor for GameConstructor {
    fn create_instance(
        &self,
        _override_scene: Handle<Scene>,
        _context: PluginContext,
    ) -> Box<dyn Plugin> {
        Box::new(Game { hue: 0.0 })
    }
}

fn main() {
    let mut executor = Executor::from_params(
        EventLoop::new().unwrap(),
        GraphicsContextParams {
            window_attributes: WindowAttributes {
                title: "Example - Plugins".to_string(),
                ..Default::default()
            },
            vsync: true,
        },
    );
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
