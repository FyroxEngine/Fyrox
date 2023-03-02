//! Shows how to create a grid widget with some content that is buttom-right anchored.
//! It also shows how to automatically adjust UI to new window size.

use fyrox::{
    core::{algebra::Vector2, pool::Handle},
    engine::executor::Executor,
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
    gui::{
        button::ButtonBuilder,
        grid::{Column, GridBuilder, Row},
        message::MessageDirection,
        widget::{WidgetBuilder, WidgetMessage},
        HorizontalAlignment, Thickness, UiNode, UserInterface, VerticalAlignment,
    },
    plugin::{Plugin, PluginConstructor, PluginContext},
    scene::Scene,
};

struct Game {
    grid: Handle<UiNode>,
}

impl Game {
    fn handle_resize(&self, ui: &UserInterface, size: Vector2<f32>) {
        // Adjust size of the root grid to make sure it equals to the size of screen.
        ui.send_message(WidgetMessage::width(
            self.grid,
            MessageDirection::ToWidget,
            size.x,
        ));
        ui.send_message(WidgetMessage::height(
            self.grid,
            MessageDirection::ToWidget,
            size.y,
        ));
    }
}

impl Plugin for Game {
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
            self.handle_resize(
                context.user_interface,
                Vector2::new(size.width as f32, size.height as f32),
            );
        }
    }

    fn on_graphics_context_created(
        &mut self,
        context: PluginContext,
        _control_flow: &mut ControlFlow,
    ) {
        if let Some(graphics_context) = context.graphics_context.as_ref() {
            let size = graphics_context.window.inner_size();
            self.handle_resize(
                context.user_interface,
                Vector2::new(size.width as f32, size.height as f32),
            );
        }
    }
}

struct GameConstructor;

impl PluginConstructor for GameConstructor {
    fn create_instance(
        &self,
        _override_scene: Handle<Scene>,
        context: PluginContext,
    ) -> Box<dyn Plugin> {
        let ctx = &mut context.user_interface.build_ctx();

        let grid = GridBuilder::new(
            WidgetBuilder::new().with_child(
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
    executor.graphics_context_params.window_attributes.title =
        "Example - Right Anchored Button".to_string();
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
