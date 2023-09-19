use fyrox::event_loop::EventLoop;
use fyrox::{
    core::pool::Handle,
    engine::{executor::Executor, GraphicsContextParams},
    gui::{
        button::ButtonBuilder,
        grid::{Column, GridBuilder, Row},
        text::TextBuilder,
        widget::WidgetBuilder,
    },
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

        GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    TextBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
                        .with_text("Some")
                        .build(ctx),
                )
                .with_child(
                    TextBuilder::new(WidgetBuilder::new().on_row(0).on_column(1))
                        .with_text("Text")
                        .build(ctx),
                )
                .with_child(
                    ButtonBuilder::new(WidgetBuilder::new().on_row(0).on_column(2))
                        .with_text("TEST BUTTON")
                        .build(ctx),
                ),
        )
        .add_row(Row::stretch())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .draw_border(true)
        .build(ctx);

        Box::new(Game {})
    }
}

fn main() {
    let mut executor = Executor::from_params(
        EventLoop::new().unwrap(),
        GraphicsContextParams {
            window_attributes: WindowAttributes {
                title: "Example - Grid".to_string(),
                ..Default::default()
            },
            vsync: true,
        },
    );
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
