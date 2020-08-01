//! Engine is container for all subsystems (renderer, ui, sound, resource manager). It also
//! creates a window and an OpenGL context.

#![warn(missing_docs)]

pub mod error;
pub mod resource_manager;

use crate::{
    core::{
        math::vec2::Vec2,
        visitor::{Visit, VisitResult, Visitor},
    },
    engine::{error::EngineError, resource_manager::ResourceManager},
    event_loop::EventLoop,
    gui::{Control, UserInterface},
    renderer::{error::RendererError, Renderer},
    scene::SceneContainer,
    sound::context::Context,
    window::{Window, WindowBuilder},
    Api, GlProfile, GlRequest, NotCurrent, PossiblyCurrent, WindowedContext,
};
use std::{
    sync::{Arc, Mutex},
    time::{self, Duration},
};

/// See module docs.
pub struct Engine<M: 'static, C: 'static + Control<M, C>> {
    context: glutin::WindowedContext<PossiblyCurrent>,
    /// Current renderer. You should call at least [render] method to see your scene on screen.
    pub renderer: Renderer,
    /// User interface allows you to build interface of any kind. UI itself is *not* thread-safe,
    /// but it uses messages to "talk" with outside world and message queue (MPSC) *is* thread-safe
    /// so its sender part can be shared across threads.   
    pub user_interface: UserInterface<M, C>,
    /// Sound context control all sound sources in the engine. It is wrapped into Arc<Mutex<>>
    /// because internally sound engine spawns separate thread to mix and send data to sound
    /// device. For more info see docs for Context.
    pub sound_context: Arc<Mutex<Context>>,
    /// Current resource manager. Resource manager wrapped into Arc<Mutex<>> to be able to
    /// use resource manager from any thread, this is useful to load resources from multiple
    /// threads to decrease loading times of your game by utilizing all available power of
    /// your CPU.
    pub resource_manager: Arc<Mutex<ResourceManager>>,
    /// All available scenes in the engine.
    pub scenes: SceneContainer,
    /// The time user interface took for internal needs. TODO: This is not the right place
    /// for such statistics, probably it is best to make separate structure to hold all
    /// such data.
    pub ui_time: Duration,
}

impl<M, C: 'static + Control<M, C>> Engine<M, C> {
    /// Creates new instance of engine from given window builder and events loop.
    ///
    /// Automatically creates all sub-systems (renderer, sound, ui, etc.).
    ///
    /// # Examples
    ///
    /// ```
    /// use rg3d::engine::Engine;
    /// use rg3d::window::WindowBuilder;
    /// use rg3d::event_loop::EventLoop;
    /// use rg3d::gui::node::StubNode;
    ///
    /// let evt = EventLoop::new();
    /// let window_builder = WindowBuilder::new()
    ///     .with_title("Test")
    ///     .with_fullscreen(None);
    /// let mut engine: Engine<(), StubNode> = Engine::new(window_builder, &evt).unwrap();
    /// ```
    #[inline]
    pub fn new(
        window_builder: WindowBuilder,
        events_loop: &EventLoop<()>,
    ) -> Result<Engine<M, C>, EngineError> {
        let context_wrapper: WindowedContext<NotCurrent> = glutin::ContextBuilder::new()
            .with_vsync(true)
            .with_gl_profile(GlProfile::Core)
            .with_gl(GlRequest::Specific(Api::OpenGl, (3, 3)))
            .build_windowed(window_builder, events_loop)?;

        let mut context = match unsafe { context_wrapper.make_current() } {
            Ok(context) => context,
            Err((_, e)) => return Err(EngineError::from(e)),
        };

        let client_size = context.window().inner_size();

        Ok(Engine {
            renderer: Renderer::new(&mut context, client_size.into())?,
            resource_manager: Arc::new(Mutex::new(ResourceManager::new())),
            sound_context: Context::new()?,
            scenes: SceneContainer::new(),
            user_interface: UserInterface::new(Vec2::new(
                client_size.width as f32,
                client_size.height as f32,
            )),
            ui_time: Default::default(),
            context,
        })
    }

    /// Returns reference to main window. Could be useful to set fullscreen mode, change
    /// size of window, its title, etc.
    #[inline]
    pub fn get_window(&self) -> &Window {
        self.context.window()
    }

    /// Performs single update tick with given time delta. Engine internally will perform update
    /// of all scenes, sub-systems, user interface, etc. Must be called in order to get engine
    /// functioning.
    pub fn update(&mut self, dt: f32) {
        let inner_size = self.context.window().inner_size();
        let frame_size = Vec2::new(inner_size.width as f32, inner_size.height as f32);

        // Resource manager might be locked by some other worker thread and it cannot be updated,
        // engine will try to update it in next frame. Resource update is just controls TTLs of
        // resource so it is not problem to defer update call.
        if let Ok(mut resource_manager) = self.resource_manager.try_lock() {
            resource_manager.update(dt);
        }

        for scene in self.scenes.iter_mut() {
            scene.update(frame_size, dt);
        }

        let time = time::Instant::now();
        self.user_interface.update(frame_size, dt);
        self.ui_time = time::Instant::now() - time;
    }

    /// Performs rendering of single frame, must be called from your game loop, otherwise you won't
    /// see anything.
    #[inline]
    pub fn render(&mut self, dt: f32) -> Result<(), RendererError> {
        self.user_interface.draw();
        self.renderer.render_and_swap_buffers(
            &self.scenes,
            &self.user_interface.get_drawing_context(),
            &self.context,
            dt,
        )
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Visit for Engine<M, C> {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        if visitor.is_reading() {
            self.renderer.flush();
            self.resource_manager.lock().unwrap().update(0.0);
            self.scenes.clear();
        }

        self.resource_manager
            .lock()?
            .visit("ResourceManager", visitor)?;
        self.scenes.visit("Scenes", visitor)?;
        self.sound_context.lock()?.visit("SoundContext", visitor)?;

        if visitor.is_reading() {
            self.resource_manager.lock()?.reload_resources();
            for scene in self.scenes.iter_mut() {
                scene.resolve();
            }
        }

        visitor.leave_region()
    }
}
