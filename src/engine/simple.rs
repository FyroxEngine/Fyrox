//! A helper framework that reduces amount of low-level code and allows newcomers to quickly start
//! writing games without getting bogged down in details.
//!
//! Once you get familiar with the engine, you should **not** use the framework because it is too
//! limiting and may slow you down.

use crate::{
    core::instant::Instant,
    engine::{error::EngineError, Engine},
    event::{DeviceEvent, DeviceId, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    gui::{
        node::{StubNode, UINode},
        BuildContext,
    },
    utils::translate_event,
    window::WindowBuilder,
};

/// Simplified engine type alias.
pub type GameEngine = Engine<(), StubNode>;

/// Simplified UI node type alias.
pub type UiNode = UINode<(), StubNode>;

/// Simplified UI build context type alias.
pub type UiBuildContext<'a> = BuildContext<'a, (), StubNode>;

/// Simplified UI message type alias.
pub type UiMessage = crate::gui::message::UiMessage<(), StubNode>;

#[doc(hidden)]
pub mod prelude {
    pub use super::{Framework, GameEngine, UiBuildContext, UiMessage, UiNode};
}

/// See module docs.
pub struct Framework<State> {
    engine: GameEngine,
    title: String,
    event_loop: EventLoop<()>,
    on_init: Option<Box<dyn FnOnce(&mut GameEngine) -> State>>,
    on_tick: Option<Box<dyn FnMut(&mut GameEngine, Option<&mut State>, f32) + 'static>>,
    on_ui_message: Option<Box<dyn FnMut(&mut GameEngine, Option<&mut State>, UiMessage) + 'static>>,
    on_device_event: Option<
        Box<dyn FnMut(&mut GameEngine, Option<&mut State>, DeviceId, DeviceEvent) + 'static>,
    >,
    on_window_event:
        Option<Box<dyn FnMut(&mut GameEngine, Option<&mut State>, WindowEvent) + 'static>>,
}

impl<State: 'static> Framework<State> {
    /// Creates new framework instance. Framework is a simple wrapper that initializes game
    /// engine and hides game loop details, allowing you to focus only on important things.
    pub fn new() -> Result<Self, EngineError> {
        let event_loop = EventLoop::new();

        let window_builder = WindowBuilder::new().with_title("Game").with_resizable(true);

        let engine = GameEngine::new(window_builder, &event_loop, false)?;

        Ok(Self {
            title: "Game".to_owned(),
            engine,
            event_loop,
            on_init: None,
            on_tick: None,
            on_ui_message: None,
            on_device_event: None,
            on_window_event: None,
        })
    }

    /// Sets desired title of game's window.
    #[must_use]
    pub fn title<S: AsRef<str>>(mut self, title: S) -> Self {
        self.title = title.as_ref().to_owned();
        self
    }

    /// Defines initializer function that will be called once after engine's initialization
    /// allowing you to initialize the state your game.
    #[must_use]
    pub fn init<I>(mut self, on_init: I) -> Self
    where
        I: FnOnce(&mut GameEngine) -> State + 'static,
    {
        self.on_init = Some(Box::new(on_init));
        self
    }

    /// Defines a function that will contain game logic. It has stabilized update rate of
    /// 60 Hz.
    #[must_use]
    pub fn tick<T>(mut self, on_tick: T) -> Self
    where
        T: FnMut(&mut GameEngine, Option<&mut State>, f32) + 'static,
    {
        self.on_tick = Some(Box::new(on_tick));
        self
    }

    /// Defines a function that will be called when there is any message from user interface.
    #[must_use]
    pub fn ui_message<U>(mut self, on_ui_message: U) -> Self
    where
        U: FnMut(&mut GameEngine, Option<&mut State>, UiMessage) + 'static,
    {
        self.on_ui_message = Some(Box::new(on_ui_message));
        self
    }

    /// Defines a function that will be called when a device event has occurred.
    #[must_use]
    pub fn device_event<D>(mut self, on_device_event: D) -> Self
    where
        D: FnMut(&mut GameEngine, Option<&mut State>, DeviceId, DeviceEvent) + 'static,
    {
        self.on_device_event = Some(Box::new(on_device_event));
        self
    }

    /// Defines a function that will be called when a window event has occurred.
    #[must_use]
    pub fn window_event<W: FnMut(&mut GameEngine, Option<&mut State>, WindowEvent) + 'static>(
        mut self,
        on_window_event: W,
    ) -> Self {
        self.on_window_event = Some(Box::new(on_window_event));
        self
    }

    /// Runs a framework and your game. This function is never returns.
    pub fn run(self) -> ! {
        let mut engine = self.engine;
        engine.get_window().set_title(&self.title);
        let mut state = self.on_init.map(|init| init(&mut engine));
        let mut on_tick = self.on_tick;
        let mut on_ui_message = self.on_ui_message;
        let mut on_device_event = self.on_device_event;
        let mut on_window_event = self.on_window_event;
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

                        if let Some(tick) = on_tick.as_mut() {
                            tick(&mut engine, state.as_mut(), fixed_timestep);
                        }

                        engine.update(fixed_timestep);
                    }

                    while let Some(ui_msg) = engine.user_interface.poll_message() {
                        if let Some(handler) = on_ui_message.as_mut() {
                            handler(&mut engine, state.as_mut(), ui_msg);
                        }
                    }

                    engine.get_window().request_redraw();
                }
                Event::RedrawRequested(_) => {
                    engine.render(fixed_timestep).unwrap();
                }
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(size) => {
                            engine.renderer.set_frame_size(size.into());
                        }
                        _ => (),
                    }

                    if let Some(os_event) = translate_event(&event) {
                        engine.user_interface.process_os_event(&os_event);
                    }

                    if let Some(handler) = on_window_event.as_mut() {
                        handler(&mut engine, state.as_mut(), event);
                    }
                }
                Event::DeviceEvent { device_id, event } => {
                    if let Some(handler) = on_device_event.as_mut() {
                        handler(&mut engine, state.as_mut(), device_id, event);
                    }
                }
                _ => *control_flow = ControlFlow::Poll,
            })
    }
}
