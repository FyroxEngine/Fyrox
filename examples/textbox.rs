use fyrox::event_loop::EventLoop;
use fyrox::{
    core::{algebra::Vector2, color::Color, pool::Handle},
    engine::{executor::Executor, GraphicsContextParams},
    gui::{brush::Brush, text_box::TextBoxBuilder, widget::WidgetBuilder},
    gui::{formatted_text::WrapMode, text_box::TextCommitMode},
    plugin::{Plugin, PluginConstructor, PluginContext},
    scene::Scene,
    window::WindowAttributes,
};

struct Game {}

impl Plugin for Game {}

struct GameConstructor;

impl PluginConstructor for GameConstructor {
    fn create_instance(
        &self,
        _override_scene: Handle<Scene>,
        context: PluginContext,
    ) -> Box<dyn Plugin> {
        let ctx = &mut context.user_interface.build_ctx();

        TextBoxBuilder::new(
            WidgetBuilder::new()
                .with_desired_position(Vector2::new(100.0, 100.0))
                .with_width(300.0)
                .with_height(200.0)
                .with_background(Brush::Solid(Color::opaque(90, 90, 90))),
        )
        .with_multiline(true)
        .with_wrap(WrapMode::Word)
        .with_text_commit_mode(TextCommitMode::LostFocus)
        .with_text("This is some text")
        .build(ctx);

        Box::new(Game {})
    }
}

fn main() {
    let mut executor = Executor::from_params(
        EventLoop::new().unwrap(),
        GraphicsContextParams {
            window_attributes: WindowAttributes {
                title: "Example - Text Box".to_string(),
                ..Default::default()
            },
            vsync: true,
        },
    );
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
