//! A helper framework that reduces amount of low-level code and allows newcomers to quickly start
//! writing games without getting bogged down in details.
//!
//! Once you get familiar with the engine, you should **not** use the framework because it is too
//! limiting and may slow you down.

use crate::gui::message::UiMessage;
use crate::utils::log::{Log, MessageKind};
use crate::{
    core::instant::Instant,
    engine::{error::EngineError, resource_manager::ResourceManagerBuilder, Engine},
    event::{DeviceEvent, DeviceId, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    utils::translate_event,
    window::WindowBuilder,
};

#[doc(hidden)]
pub mod prelude {
    pub use super::{Framework, GameState};
}

/// A trait for your game state, it contains all possible methods which will be called in
/// various situations. Every method, except `init` is optional.
pub trait GameState: 'static {
    /// An initializer function that will be called once after engine's initialization
    /// allowing you to initialize the state your game.
    fn init(engine: &mut Engine) -> Self
    where
        Self: Sized;

    /// Defines a function that will contain game logic. It has stabilized update rate of
    /// 60 Hz. Callee can alter control flow of the game by modifying _control_flow parameter.
    fn on_tick(&mut self, _engine: &mut Engine, _dt: f32, _control_flow: &mut ControlFlow) {}

    /// Defines a function that will be called when there is any message from user interface.
    fn on_ui_message(&mut self, _engine: &mut Engine, _message: UiMessage) {}

    /// Defines a function that will be called when a device event has occurred.
    fn on_device_event(&mut self, _engine: &mut Engine, _device_id: DeviceId, _event: DeviceEvent) {
    }

    /// Defines a function that will be called when a window event has occurred.
    fn on_window_event(&mut self, _engine: &mut Engine, _event: WindowEvent) {}

    /// Defines a function that will be called when game is about to close.
    fn on_exit(&mut self, _engine: &mut Engine) {}
}

/// See module docs.
pub struct Framework<State: GameState> {
    engine: Engine,
    title: String,
    event_loop: EventLoop<()>,
    state: State,
}

impl<State: GameState> Framework<State> {
    /// Creates new framework instance. Framework is a simple wrapper that initializes game
    /// engine and hides game loop details, allowing you to focus only on important things.
    pub fn new() -> Result<Self, EngineError> {
        let event_loop = EventLoop::new();

        let window_builder = WindowBuilder::new().with_title("Game").with_resizable(true);
        let resource_manager_builder = ResourceManagerBuilder::new();

        let mut engine = Engine::new(window_builder, resource_manager_builder, &event_loop, false)?;

        Ok(Self {
            title: "Game".to_owned(),
            state: State::init(&mut engine),
            engine,
            event_loop,
        })
    }

    /// Sets desired title of game's window.
    #[must_use]
    pub fn title<S: AsRef<str>>(mut self, title: S) -> Self {
        self.title = title.as_ref().to_owned();
        self
    }

    /// Runs a framework and your game. This function is never returns.
    pub fn run(self) -> ! {
        let mut engine = self.engine;
        engine.get_window().set_title(&self.title);
        let mut state = self.state;
        let clock = Instant::now();
        let fixed_timestep = 1.0 / 60.0;
        let mut elapsed_time = 0.0;

        self.event_loop
            .run(move |event, _, control_flow| match event {
                Event::MainEventsCleared => {
                    let mut dt = clock.elapsed().as_secs_f32() - elapsed_time;
                    while dt >= fixed_timestep {
                        dt -= fixed_timestep;
                        elapsed_time += fixed_timestep;

                        state.on_tick(&mut engine, fixed_timestep, control_flow);

                        engine.update(fixed_timestep);
                    }

                    while let Some(ui_msg) = engine.user_interface.poll_message() {
                        state.on_ui_message(&mut engine, ui_msg);
                    }

                    engine.get_window().request_redraw();
                }
                Event::RedrawRequested(_) => {
                    engine.render().unwrap();
                }
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(size) => {
                            if let Err(e) = engine.set_frame_size(size.into()) {
                                Log::writeln(
                                    MessageKind::Error,
                                    format!("Unable to set frame size: {:?}", e),
                                );
                            }
                        }
                        _ => (),
                    }

                    if let Some(os_event) = translate_event(&event) {
                        engine.user_interface.process_os_event(&os_event);
                    }

                    state.on_window_event(&mut engine, event);
                }
                Event::DeviceEvent { device_id, event } => {
                    state.on_device_event(&mut engine, device_id, event);
                }
                Event::LoopDestroyed => state.on_exit(&mut engine),
                _ => *control_flow = ControlFlow::Poll,
            })
    }
}
