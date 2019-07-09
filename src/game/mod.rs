pub mod level;
pub mod player;
pub mod weapon;

use std::fs::File;
use std::path::Path;
use std::fmt::Write;

use crate::game::level::Level;

use crate::engine::{Engine, duration_to_seconds_f64};
use std::time::{Duration, Instant};
use crate::gui::draw::{FormattedText, FormattedTextBuilder};
use crate::math::Rect;

pub struct Game {
    engine: Engine,
    level: Level,
    debug_text: Option<FormattedText>,
}

pub struct GameTime {
    elapsed: f64,
    delta: f64,
}

impl Game {
    pub fn new() -> Game {
        let mut engine = Engine::new();
        let level = Level::new(&mut engine);

        let debug_text = FormattedTextBuilder::new().build();

        Game {
            engine,
            level,
            debug_text: Some(debug_text),
        }
    }

    pub fn make_save(&self) {
        match File::create(Path::new("test.json")) {
            Ok(file) => {
                serde_json::to_writer_pretty(file, self.engine.get_state()).unwrap();
            }
            Err(_) => println!("unable to create a save"),
        }
    }

    pub fn update(&mut self, time: &GameTime) {
        self.level.update(&mut self.engine, time);
    }

    pub fn run(&mut self) {
        let fixed_fps = 60.0;
        let fixed_timestep = 1.0 / fixed_fps;
        let clock = Instant::now();
        let mut game_time = GameTime { elapsed: 0.0, delta: fixed_timestep };

        let mut debug_string = String::new();
        while self.engine.is_running() {
            let mut dt = duration_to_seconds_f64(clock.elapsed()) - game_time.elapsed;
            while dt >= fixed_timestep {
                dt -= fixed_timestep;
                game_time.elapsed += fixed_timestep;
                self.engine.poll_events();
                while let Some(event) = self.engine.pop_event() {
                    if let glutin::Event::WindowEvent { event, .. } = event {
                        self.level.get_player_mut().process_event(&event);
                        match event {
                            glutin::WindowEvent::CloseRequested => self.engine.stop(),
                            glutin::WindowEvent::KeyboardInput {
                                input: glutin::KeyboardInput {
                                    virtual_keycode: Some(glutin::VirtualKeyCode::Escape),
                                    ..
                                },
                                ..
                            } => self.engine.stop(),
                            _ => ()
                        }
                    }
                }
                self.update(&game_time);
                self.engine.update(fixed_timestep);
            }

            debug_string.clear();
            write!(debug_string, "Frame time: {:.2} ms\nFPS: {}\nUp time: {:.2} s",
                   self.engine.get_rendering_statisting().frame_time * 1000.0,
                   self.engine.get_rendering_statisting().current_fps,
                   game_time.elapsed);
            self.debug_text = Some(FormattedTextBuilder::reuse(self.debug_text.take().unwrap())
                .with_font(self.engine.get_default_font())
                .with_text(debug_string.as_str())
                .with_bounds(Rect::new(0.0, 0.0, 300.0, 300.0))
                .build());
            let drawing_context = self.engine.get_ui_mut().get_drawing_context_mut();
            if let Some(ref debug_text) = self.debug_text {
                drawing_context.draw_text(debug_text);
            }

            // Render at max speed
            self.engine.render();
        }
    }
}