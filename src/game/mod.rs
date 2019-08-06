pub mod level;
pub mod player;
pub mod weapon;

use std::{
    fs::File,
    path::Path,
    fmt::Write,
    time::{Instant}
};

use crate::game::level::Level;

use crate::engine::{Engine, duration_to_seconds_f64};
use crate::gui::{UINode, UINodeKind, Text};
use crate::utils::pool::Handle;
use crate::math::vec2::Vec2;

pub struct Game {
    engine: Engine,
    level: Level,
    debug_text: Handle<UINode>,
}

pub struct GameTime {
    elapsed: f64,
    delta: f64,
}

impl Game {
    pub fn new() -> Game {
        let mut engine = Engine::new();
        let level = Level::new(&mut engine);

        let mut text = Text::new("");
        text.set_font(engine.get_default_font());
        let mut ui_node = UINode::new(UINodeKind::Text(text));
        ui_node.set_width(200.0);
        ui_node.set_height(200.0);

        let button_handle = engine.get_ui_mut().create_button("Test button");
        if let Some(button) = engine.get_ui_mut().get_node_mut(&button_handle) {
            button.set_desired_local_position(Vec2::make(100.0, 100.0));
            if let UINodeKind::Button(btn) = button.get_kind_mut() {
                btn.set_on_click(Box::new(|_ui, _handle| {
                    println!("Clicked!");
                }));
            }
        }

        Game {
            debug_text: engine.get_ui_mut().add_node(ui_node),
            engine,
            level
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
        self.engine.update(time.delta);
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

                        self.engine.get_ui_mut().process_event(&event);
                    }
                }
                self.update(&game_time);
            }

            debug_string.clear();
            write!(debug_string, "Frame time: {:.2} ms\nFPS: {}\nUp time: {:.2} s",
                   self.engine.get_rendering_statisting().frame_time * 1000.0,
                   self.engine.get_rendering_statisting().current_fps,
                   game_time.elapsed).unwrap();

            if let Some(ui_node) = self.engine.get_ui_mut().get_node_mut(&self.debug_text) {
                if let UINodeKind::Text(text) = ui_node.get_kind_mut() {
                    text.set_text(debug_string.as_str());
                }
            }

            // Render at max speed
            self.engine.render();
        }
    }
}