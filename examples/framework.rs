use rg3d::{
    core::color::{Color, Hsv},
    engine::simple::prelude::*,
};

struct Game {
    hue: f32,
}

impl GameState for Game {
    fn init(_engine: &mut GameEngine) -> Self
    where
        Self: Sized,
    {
        Self { hue: 0.0 }
    }

    // Implement a function that will update game logic and will be called at fixed rate of 60 Hz.
    fn on_tick(&mut self, engine: &mut GameEngine, dt: f32) {
        // Increase hue at fixed rate of 24 degrees per second.
        self.hue += 24.0 * dt;

        // Slowly change color of the window.
        engine
            .renderer
            .set_backbuffer_clear_color(Color::from(Hsv::new(self.hue % 360.0, 100.0, 100.0)))
    }
}

fn main() {
    // Framework is a simple wrapper that initializes engine and hides game loop details, allowing
    // you to focus only on important things.
    Framework::<Game>::new()
        .unwrap()
        .title("Example - Framework")
        .run();
}
