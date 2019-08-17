pub mod level;
pub mod player;
pub mod weapon;

use crate::{
    engine::{
        Engine,
        duration_to_seconds_f64,
    },
    game::level::Level,
    gui::{
        UINode,
        UINodeKind,
        Column,
        Row,
        GridBuilder,
        ButtonBuilder,
        Thickness,
        TextBuilder,
        ScrollBarBuilder
    },
    utils::pool::Handle,
    math::vec2::Vec2,
};
use std::{
    cell::RefCell,
    fs::File,
    path::Path,
    fmt::Write,
    time::Instant,
    rc::Rc,
};
use crate::gui::ScrollViewerBuilder;

pub struct MenuState {
    save_game: Option<()>,
    load_game: Option<()>,
    start_new_game: Option<()>,
    quit_game: Option<()>
}

pub struct Menu {
    state: Rc<RefCell<MenuState>>,
}

pub struct Game {
    menu: Menu,
    engine: Engine,
    level: Option<Level>,
    debug_text: Handle<UINode>,
}

pub struct GameTime {
    elapsed: f64,
    delta: f64,
}

impl Game {
    pub fn new() -> Game {
        let engine = Engine::new();
        let mut game = Game {
            menu: Menu { state: Rc::new(RefCell::new(MenuState {
                start_new_game: None,
                quit_game: None,
                save_game: None,
                load_game: None,
            })) },
            debug_text: Handle::none(),
            engine,
            level: None,
        };
        game.create_ui();
        game
    }

    pub fn create_ui(&mut self) {
        let ui = self.engine.get_ui_mut();

        self.debug_text = TextBuilder::new()
            .with_width(200.0)
            .with_height(200.0)
            .build(ui);

        ScrollViewerBuilder::new()
            .with_desired_position(Vec2::make(550.0, 300.0))
            .with_width(300.0)
            .with_height(200.0)
            .with_content(ButtonBuilder::new()
                .with_width(300.0)
                .with_height(300.0)
                .with_text("TEST BUTTON")
                .build(ui))
            .build(ui);

        GridBuilder::new()
            .add_column(Column::stretch())
            .add_row(Row::strict(50.0))
            .add_row(Row::strict(50.0))
            .add_row(Row::strict(50.0))
            .add_row(Row::strict(50.0))
            .add_row(Row::strict(50.0))
            .add_row(Row::strict(50.0))
            .with_width(300.0)
            .with_height(400.0)
            .with_desired_position(Vec2::make(200.0, 200.0))
            .with_child( {
                let menu_state = self.menu.state.clone();
                ButtonBuilder::new()
                    .with_text("New Game")
                    .on_column(0)
                    .on_row(0)
                    .with_margin(Thickness::uniform(4.0))
                    .with_click(Box::new(move |_ui, _handle| {
                        if let Ok(mut state) = menu_state.try_borrow_mut() {
                            state.start_new_game = Some(());
                        }
                    }))
                    .build(ui)
            })
            .with_child({
                let menu_state = self.menu.state.clone();
                ButtonBuilder::new()
                    .with_text("Save Game")
                    .on_column(0)
                    .on_row(1)
                    .with_margin(Thickness::uniform(4.0))
                    .with_click(Box::new(move |_ui, _handle| {
                        if let Ok(mut state) = menu_state.try_borrow_mut() {
                            state.save_game = Some(());
                        }
                    }))
                    .build(ui)
            })
            .with_child({
                let menu_state = self.menu.state.clone();
                ButtonBuilder::new()
                    .with_text("Load Game")
                    .on_column(0)
                    .on_row(2)
                    .with_margin(Thickness::uniform(4.0))
                    .with_click(Box::new(move |_ui, _handle| {
                        if let Ok(mut state) = menu_state.try_borrow_mut() {
                            state.load_game = Some(());
                        }
                    }))
                    .build(ui)
            })
            .with_child({
                ButtonBuilder::new()
                    .with_text("Settings")
                    .on_column(0)
                    .on_row(3)
                    .with_margin(Thickness::uniform(4.0))
                    .with_click(Box::new(|_ui, _handle| {
                        println!("Settings Clicked!");
                    }))
                    .build(ui)
            })
            .with_child({
                let menu_state = self.menu.state.clone();
                ButtonBuilder::new()
                    .with_text("Quit")
                    .on_column(0)
                    .on_row(4)
                    .with_margin(Thickness::uniform(4.0))
                    .with_click(Box::new(move |_ui, _handle| {
                        if let Ok(mut state) = menu_state.try_borrow_mut() {
                            state.quit_game = Some(());
                        }
                    }))
                    .build(ui)
            })
            .with_child(ScrollBarBuilder::new()
                .on_row(5)
                .with_margin(Thickness::uniform(4.0))
                .build(ui))
            .build(ui);
    }

    pub fn save_game(&self) {
        match File::create(Path::new("save.json")) {
            Ok(file) => {
                self.engine.save(file)
            }
            Err(_) => println!("unable to create a save"),
        }
    }

    pub fn load_game(&mut self) {
        match File::open(Path::new("save.json")) {
            Ok(file) => {
                self.engine.load(file)
            }
            Err(_) => println!("failed to load a save!")
        }
    }

    fn destroy_level(&mut self) {
        if let Some(ref mut level) = self.level.take() {
            level.destroy(&mut self.engine);
        }
    }

    pub fn start_new_game(&mut self) {
        self.destroy_level();
        self.level = Some(Level::new(&mut self.engine));
    }

    pub fn update_menu(&mut self) {
        if let Ok(mut state) = self.menu.state.clone().try_borrow_mut() {
            if state.start_new_game.take().is_some() {
                self.start_new_game();
            }

            if state.quit_game.take().is_some() {
                self.destroy_level();
                self.engine.stop();
            }

            if state.save_game.take().is_some() {
                self.save_game();
            }

            if state.load_game.take().is_some() {
                self.load_game();
            }
        }
    }

    pub fn update(&mut self, time: &GameTime) {
        if let Some(ref mut level) = self.level {
            level.update(&mut self.engine, time);
        }
        self.engine.update(time.delta);
        self.update_menu();
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
                        if let Some(ref mut level) = self.level {
                            level.get_player_mut().process_event(&event);
                        }
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
        self.destroy_level();
    }
}