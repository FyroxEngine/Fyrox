use fyrox::{
    core::pool::Handle,
    engine::{framework::prelude::*, Engine},
    event::WindowEvent,
    gui::{
        button::ButtonBuilder,
        grid::{Column, GridBuilder, Row},
        message::MessageDirection,
        widget::WidgetBuilder,
        widget::WidgetMessage,
        HorizontalAlignment, UiNode, VerticalAlignment,
    },
};
use fyrox_ui::Thickness;

struct Game {
    grid: Handle<UiNode>,
}

impl GameState for Game {
    fn init(engine: &mut Engine) -> Self
    where
        Self: Sized,
    {
        let window_inner_size = engine.get_window().inner_size();
        let ctx = &mut engine.user_interface.build_ctx();

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

        Self { grid }
    }

    fn on_window_event(&mut self, engine: &mut Engine, event: WindowEvent) {
        if let WindowEvent::Resized(size) = event {
            engine.user_interface.send_message(WidgetMessage::width(
                self.grid,
                MessageDirection::ToWidget,
                size.width as f32,
            ));

            engine.user_interface.send_message(WidgetMessage::height(
                self.grid,
                MessageDirection::ToWidget,
                size.height as f32,
            ));
        }
    }
}

fn main() {
    Framework::<Game>::new()
        .unwrap()
        .title("Example - Grid")
        .run();
}
