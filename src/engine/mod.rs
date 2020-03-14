pub mod resource_manager;
pub mod error;

use crate::{
    core::{
        math::vec2::Vec2,
        visitor::{
            Visitor,
            VisitResult,
            Visit,
        },
    },
    sound::context::Context,
    engine::{
        resource_manager::ResourceManager,
        error::EngineError,
    },
    gui::UserInterface,
    renderer::{
        Renderer,
        error::RendererError,
        gl,
    },
    window::{
        WindowBuilder,
        Window,
    },
    scene::SceneContainer,
    PossiblyCurrent,
    GlRequest,
    GlProfile,
    WindowedContext,
    NotCurrent,
    Api,
    event_loop::EventLoop,
    gui::Control
};
use std::{
    sync::{Arc, Mutex},
    time,
    time::Duration
};

pub struct Engine<M: 'static, C: 'static + Control<M, C>> {
    context: glutin::WindowedContext<PossiblyCurrent>,
    pub renderer: Renderer,
    pub user_interface: UserInterface<M, C>,
    pub sound_context: Arc<Mutex<Context>>,
    pub resource_manager: Arc<Mutex<ResourceManager>>,
    pub scenes: SceneContainer,
    pub ui_time: Duration
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
    ///
    /// let evt = EventLoop::new();
    /// let window_builder = WindowBuilder::new()
    ///     .with_title("Test")
    ///     .with_fullscreen(None);
    /// let mut engine = Engine::new(window_builder, &evt).unwrap();
    /// ```
    #[inline]
    pub fn new(window_builder: WindowBuilder, events_loop: &EventLoop<()>) -> Result<Engine<M, C> , EngineError> {
        let context_wrapper: WindowedContext<NotCurrent> = glutin::ContextBuilder::new()
            .with_vsync(true)
            .with_gl_profile(GlProfile::Core)
            .with_gl(GlRequest::Specific(Api::OpenGl, (3, 3)))
            .build_windowed(window_builder, events_loop)?;

        let context = unsafe {
            let context = match context_wrapper.make_current() {
                Ok(context) => context,
                Err((_, e)) => return Err(EngineError::from(e)),
            };
            gl::load_with(|symbol| context.get_proc_address(symbol) as *const _);
            context
        };

        let client_size = context.window().inner_size();

        Ok(Engine {
            context,
            resource_manager: Arc::new(Mutex::new(ResourceManager::new())),
            sound_context: Context::new()?,
            scenes: SceneContainer::new(),
            renderer: Renderer::new(client_size.into())?,
            user_interface: UserInterface::new(),
            ui_time: Default::default()
        })
    }

    /// Returns reference to main window.  Could be useful to set fullscreen mode, change
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

    pub fn get_ui_mut(&mut self) -> &mut UserInterface<M, C>  {
        &mut self.user_interface
    }

    #[inline]
    pub fn render(&mut self, dt: f32) -> Result<(), RendererError> {
        self.user_interface.draw();
        self.renderer.render(&self.scenes, &self.user_interface.get_drawing_context(), &self.context, dt)
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Visit for Engine<M, C>  {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        if visitor.is_reading() {
            self.resource_manager.lock().unwrap().update(0.0);
            self.scenes.clear();
        }

        self.resource_manager.lock()?.visit("ResourceManager", visitor)?;
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

