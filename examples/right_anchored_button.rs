//! Shows how to create a grid widget with some content that is buttom-right anchored.
//! It also shows how to automatically adjust UI to new window size.

use fyrox::{
    core::{
        pool::Handle,
        uuid::{uuid, Uuid},
    },
    engine::executor::Executor,
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
    gui::{
        button::ButtonBuilder,
        grid::{Column, GridBuilder, Row},
        message::MessageDirection,
        widget::{WidgetBuilder, WidgetMessage},
        HorizontalAlignment, Thickness, UiNode, VerticalAlignment,
    },
    plugin::{Plugin, PluginConstructor, PluginContext},
    scene::{node::TypeUuidProvider, Scene},
};

struct Game {
    grid: Handle<UiNode>,
}

impl Plugin for Game {
    fn id(&self) -> Uuid {
        GameConstructor::type_uuid()
    }

    fn on_os_event(
        &mut self,
        event: &Event<()>,
        context: PluginContext,
        _control_flow: &mut ControlFlow,
    ) {
        if let Event::WindowEvent {
            event: WindowEvent::Resized(size),
            ..
        } = event
        {
            // Adjust size of the root grid to make sure it equals to the size of screen.
            context.user_interface.send_message(WidgetMessage::width(
                self.grid,
                MessageDirection::ToWidget,
                size.width as f32,
            ));
            context.user_interface.send_message(WidgetMessage::height(
                self.grid,
                MessageDirection::ToWidget,
                size.height as f32,
            ));
        }
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
        let window_inner_size = context.window.inner_size();
        let ctx = &mut context.user_interface.build_ctx();

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                // Create root grid with size of screen.
                .with_width(window_inner_size.width as f32)
                .with_height(window_inner_size.height as f32)
                .with_child(
                    ButtonBuilder::new(
                        WidgetBuilder::new()
                            // Set size of the button, otherwise it will fill the entire screen.
                            .with_width(250.0)
                            .with_height(100.0)
                            // Set offset from bottom right corner.
                            .with_margin(Thickness {
                                left: 0.0,
                                top: 0.0,
                                right: 10.0,
                                bottom: 10.0,
                            })
                            .on_row(0)
                            .on_column(0)
                            // Make sure the button will be "anchored" to bottom right.
                            .with_vertical_alignment(VerticalAlignment::Bottom)
                            .with_horizontal_alignment(HorizontalAlignment::Right),
                    )
                    .with_text("Bottom-Right Anchored Button")
                    .build(ctx),
                ),
        )
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        Box::new(Game { grid })
    }
}

fn main() {
    let mut executor = Executor::new();
    executor
        .get_window()
        .set_title("Example - Right Anchored Button");
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
