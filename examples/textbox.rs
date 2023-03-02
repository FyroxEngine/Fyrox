use fyrox::{
    core::{algebra::Vector2, color::Color, pool::Handle},
    engine::executor::Executor,
    gui::{brush::Brush, text_box::TextBoxBuilder, widget::WidgetBuilder},
    plugin::{Plugin, PluginConstructor, PluginContext},
    scene::Scene,
};
use fyrox_ui::formatted_text::WrapMode;
use fyrox_ui::text_box::TextCommitMode;

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
    let mut executor = Executor::new();
    executor.graphics_context_params.window_attributes.title = "Example - Text Box".to_string();
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
