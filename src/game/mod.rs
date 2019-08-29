pub mod level;
pub mod player;
pub mod weapon;

use crate::{
    engine::{Engine, duration_to_seconds_f64},
    game::level::Level,
    utils::{
        pool::Handle,
        visitor::{
            Visitor,
            VisitResult,
            Visit,
        },
    },
    math::vec2::Vec2,
    gui::{
        node::{UINode, UINodeKind},
        button::ButtonBuilder,
        Thickness,
        grid::{GridBuilder, Column, Row},
        text::TextBuilder,
        scroll_bar::ScrollBarBuilder,
        window::WindowBuilder,
    },
};
use std::{
    cell::RefCell,
    fs::File,
    path::Path,
    time::Instant,
    rc::Rc,
    io::Write,
};
use crate::gui::Visibility;

pub struct MenuState {
    save_game: Option<()>,
    load_game: Option<()>,
    start_new_game: Option<()>,
    quit_game: Option<()>,
}

pub struct Menu {
    state: Rc<RefCell<MenuState>>,
    root: Handle<UINode>,
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
            menu: Menu {
                state: Rc::new(RefCell::new(MenuState {
                    start_new_game: None,
                    quit_game: None,
                    save_game: None,
                    load_game: None,
                })),
                root: Handle::none(),
            },
            debug_text: Handle::none(),
            engine,
            level: None,
        };
        game.create_ui();
        game
    }

    pub fn create_ui(&mut self) {
        let frame_size = self.engine.get_frame_size();
        let ui = self.engine.get_ui_mut();

        self.debug_text = TextBuilder::new()
            .with_width(200.0)
            .with_height(200.0)
            .build(ui);

        self.menu.root = GridBuilder::new()
            .add_row(Row::stretch())
            .add_row(Row::strict(600.0))
            .add_row(Row::stretch())
            .add_column(Column::stretch())
            .add_column(Column::strict(450.0))
            .add_column(Column::stretch())
            .with_width(frame_size.x)
            .with_height(frame_size.y)
            .with_child(WindowBuilder::new()
                .on_row(1)
                .on_column(1)
                .with_content(GridBuilder::new()
                    .with_margin(Thickness::uniform(20.0))
                    .add_column(Column::stretch())
                    .add_row(Row::strict(50.0))
                    .add_row(Row::strict(50.0))
                    .add_row(Row::strict(50.0))
                    .add_row(Row::strict(50.0))
                    .add_row(Row::strict(50.0))
                    .add_row(Row::strict(50.0))
                    .with_child({
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
                    .build(ui))
                .build(ui))
            .build(ui);
    }

    pub fn save_game(&mut self) -> VisitResult {
        let mut visitor = Visitor::new();

        // Visit engine state first.
        self.engine.visit("Engine", &mut visitor)?;

        self.level.visit("Level", &mut visitor)?;

        // Debug output
        if let Ok(mut file) = File::create(Path::new("save.txt")) {
            file.write_all(visitor.save_text().as_bytes()).unwrap();
        }

        visitor.save_binary(Path::new("save.bin"))
    }

    pub fn load_game(&mut self) {
        match Visitor::load_binary(Path::new("save.bin")) {
            Ok(mut visitor) => {
                // Clean up.
                self.destroy_level();

                // Load engine state first
                match self.engine.visit("Engine", &mut visitor) {
                    Ok(_) => println!("Engine state successfully loaded!"),
                    Err(e) => println!("Failed to load engine state! Reason: {}", e)
                }

                // Then load game state.
                match self.level.visit("Level", &mut visitor) {
                    Ok(_) => println!("Game state successfully loaded!"),
                    Err(e) => println!("Failed to load game state! Reason: {}", e)
                }
            }
            Err(e) => {
                println!("failed to load a save, reason: {}", e);
            }
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
        self.set_menu_visible(false);
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
                match self.save_game() {
                    Ok(_) => println!("successfully saved"),
                    Err(e) => println!("failed to make a save, reason: {}", e),
                }
            }

            if state.load_game.take().is_some() {
                self.load_game();
            }
        }
    }

    pub fn set_menu_visible(&mut self, visible: bool) {
        if let Some(root) = self.engine.get_ui_mut().get_node_mut(self.menu.root) {
            root.set_visibility(if visible { Visibility::Visible } else { Visibility::Collapsed })
        }
    }

    pub fn is_menu_visible(&self) -> bool {
        if let Some(root) = self.engine.get_ui().get_node(self.menu.root) {
            root.get_visibility() == Visibility::Visible
        } else {
            false
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
                // Get events from OS.
                self.engine.poll_events();

                // Feed engine with events.
                while let Some(event) = self.engine.pop_event() {
                    if let glutin::Event::WindowEvent { event, .. } = event {
                        // Some events can be consumed so they won't be dispatched further,
                        // this allows to catch events by UI for example and don't send them
                        // to player controller so when you click on some button in UI you
                        // won't shoot from your current weapon in game.
                        let mut event_processed = self.engine.get_ui_mut().process_event(&event);

                        if !event_processed {
                            if let Some(ref mut level) = self.level {
                                if let Some(player) = level.get_player_mut() {
                                    event_processed = player.process_event(&event);
                                }
                            }
                        }

                        // Some events processed in any case.
                        match event {
                            glutin::WindowEvent::CloseRequested => self.engine.stop(),
                            glutin::WindowEvent::KeyboardInput { input, .. } => {
                                if let glutin::ElementState::Pressed = input.state {
                                    if let Some(key) = input.virtual_keycode {
                                        if key == glutin::VirtualKeyCode::Escape {
                                            self.set_menu_visible(!self.is_menu_visible());
                                        }
                                    }
                                }
                            }
                            glutin::WindowEvent::Resized(new_size) => {
                                let frame_size = Vec2::make(new_size.width as f32, new_size.height as f32);
                                self.engine.set_frame_size(frame_size);
                                if let Some(root) = self.engine.get_ui_mut().get_node_mut(self.menu.root) {
                                    root.set_width(frame_size.x);
                                    root.set_height(frame_size.y);
                                }
                            }
                            _ => ()
                        }
                    }
                }
                self.update(&game_time);
            }

            debug_string.clear();
            use std::fmt::Write;
            write!(debug_string, "Frame time: {:.2} ms\nFPS: {}\nUp time: {:.2} s",
                   self.engine.get_rendering_statisting().frame_time * 1000.0,
                   self.engine.get_rendering_statisting().current_fps,
                   game_time.elapsed).unwrap();

            if let Some(ui_node) = self.engine.get_ui_mut().get_node_mut(self.debug_text) {
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