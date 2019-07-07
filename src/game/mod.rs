pub mod level;
pub mod player;
pub mod weapon;

use std::fs::File;
use std::path::Path;

use crate::game::level::Level;

use crate::engine::Engine;
use std::time::{Duration, Instant};
use crate::math::vec2::Vec2;
use crate::gui::draw::{Color, CommandKind, FormattedText};

pub struct Game {
    engine: Engine,
    level: Level,
}

pub struct GameTime {
    elapsed: f64,
    delta: f64,
}

fn duration_to_seconds_f64(duration: Duration) -> f64 {
    duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1_000_000_000.0
}

impl Game {
    pub fn new() -> Game {
        let mut engine = Engine::new();
        let level = Level::new(&mut engine);

        /*
        match File::create(Path::new("test.json")) {
            Err(_) => println!("unable to create a save"),
            Ok(file) => {
                serde_json::to_writer_pretty(file, engine.get_state()).unwrap();
            }
        }*/

        Game {
            engine,
            level,
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
        let mut test_text = FormattedText::new();
        while self.engine.is_running() {
            {
                test_text.set_text("The quick brown fox jumps over a lazy dog. 1234567890!@#$%^&*()_+",
                                   self.engine.get_default_font(),
                                   &Vec2::new(),
                                   Color::white());

                let dc = self.engine.get_ui_mut().get_drawing_context_mut();
                dc.draw_text(&test_text);
            }
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
            // Render at max speed
            self.engine.render();
        }
    }
}