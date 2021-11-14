use rg3d::{
    core::{algebra::Vector2, pool::Handle, rand::Rng},
    engine::{framework::prelude::*, Engine},
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        message::{MessageDirection, UiMessage},
        widget::{WidgetBuilder, WidgetMessage},
        UiNode,
    },
    rand::thread_rng,
};

struct Game {
    button: Handle<UiNode>,
}

impl GameState for Game {
    fn init(engine: &mut Engine) -> Self
    where
        Self: Sized,
    {
        let ctx = &mut engine.user_interface.build_ctx();

        // The simplest button can be created in a few lines of code.
        let button = ButtonBuilder::new(WidgetBuilder::new())
            .with_text("Click me!")
            .build(ctx);

        Self { button }
    }

    fn on_ui_message(&mut self, engine: &mut Engine, message: UiMessage) {
        // Simple example of message system. We'll catch "Click" messages from the button
        // and send new message to the button that will contain new position for it.
        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.button {
                // Generate random position in the window.
                let client_size = engine.get_window().inner_size();

                let mut rng = thread_rng();

                let new_position = Vector2::new(
                    rng.gen_range(0.0..(client_size.width as f32 - 100.0)),
                    rng.gen_range(0.0..(client_size.height as f32 - 100.0)),
                );

                // "Tell" the button to "teleport" in the new location.
                engine
                    .user_interface
                    .send_message(WidgetMessage::desired_position(
                        self.button,
                        MessageDirection::ToWidget,
                        new_position,
                    ));
            }
        }
    }
}

fn main() {
    Framework::<Game>::new()
        .unwrap()
        .title("Example - Button")
        .run();
}
