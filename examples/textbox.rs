use fyrox::{
    core::{
        color::Color,
        pool::Handle,
        uuid::{uuid, Uuid},
    },
    engine::executor::Executor,
    gui::{brush::Brush, text_box::TextBoxBuilder, widget::WidgetBuilder},
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

        TextBoxBuilder::new(
            WidgetBuilder::new()
                .with_width(300.0)
                .with_height(200.0)
                .with_background(Brush::Solid(Color::opaque(90, 90, 90))),
        )
        .with_text("This is some text")
        .build(ctx);

        Box::new(Game {})
    }
}

fn main() {
    let mut executor = Executor::new();
    executor.get_window().set_title("Example - Text Box");
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
