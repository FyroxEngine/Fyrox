use fyrox::{
    core::{
        color::{Color, Hsv},
        pool::Handle,
    },
    engine::executor::Executor,
    event_loop::ControlFlow,
    plugin::{Plugin, PluginConstructor, PluginContext},
    scene::Scene,
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
        if let Some(graphics_context) = context.graphics_context.as_mut() {
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
    let mut executor = Executor::new();
    executor.graphics_context_params.window_attributes.title = "Example - Plugins".to_string();
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
