use fyrox::engine::executor::Executor;
use fyrox::{
    core::{
        pool::Handle,
        uuid::{uuid, Uuid},
    },
    gui::{
        button::ButtonBuilder,
        grid::{Column, GridBuilder, Row},
        text::TextBuilder,
        widget::WidgetBuilder,
    },
    plugin::{Plugin, PluginConstructor, PluginContext},
    scene::{node::TypeUuidProvider, Scene},
};

struct Game {}

impl Plugin for Game {
    fn id(&self) -> Uuid {
        GameConstructor::type_uuid()
    }
}

struct GameConstructor;

impl TypeUuidProvider for GameConstructor {
    fn type_uuid() -> Uuid {
        uuid!("f615ac42-b259-4a23-bb44-407d753ac178")
    }
}

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
    let mut executor = Executor::new();
    executor.get_window().set_title("Example - Grid");
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
