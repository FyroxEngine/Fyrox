use rg3d::{
    core::color::{Color, Hsv},
    engine::simple::prelude::*,
};

struct State {
    hue: f32,
}

fn main() {
    // Framework is a simple wrapper that initializes engine and hides game loop details, allowing
    // you to focus only on important things.
    Framework::new()
        .unwrap()
        .title("Example - Framework")
        // Define a function that initializes game state.
        .init(|_| State { hue: 0.0 })
        // Define a function that will update game logic and will be called at fixed rate of 60 Hz.
        .tick(|engine, state, dt| {
            let state = state.unwrap();

            // Increase hue at fixed rate of 24 degrees per second.
            state.hue += 24.0 * dt;

            // Slowly change color of the window.
            engine
                .renderer
                .set_backbuffer_clear_color(Color::from(Hsv::new(state.hue % 360.0, 100.0, 100.0)))
        })
        .run();
}
