//! Engine is container for all subsystems (renderer, ui, sound, resource manager). It also
//! creates a window and an OpenGL context.

#![warn(missing_docs)]

pub mod error;
pub mod resource_manager;

use crate::scene2d::Scene2dContainer;
use crate::{
    core::{
        algebra::Vector2,
        instant,
        visitor::{Visit, VisitResult, Visitor},
    },
    engine::{error::EngineError, resource_manager::ResourceManager},
    event_loop::EventLoop,
    gui::{message::MessageData, Control, UserInterface},
    renderer::framework::error::FrameworkError,
    renderer::Renderer,
    resource::texture::TextureKind,
    scene::SceneContainer,
    sound::engine::SoundEngine,
    utils::log::{Log, MessageKind},
    window::{Window, WindowBuilder},
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

/// See module docs.
pub struct Engine<M: MessageData, C: Control<M, C>> {
    #[cfg(not(target_arch = "wasm32"))]
    context: glutin::WindowedContext<glutin::PossiblyCurrent>,
    #[cfg(target_arch = "wasm32")]
    window: winit::window::Window,
    /// Current renderer. You should call at least [render](Self::render) method to see your scene on
    /// screen.
    pub renderer: Renderer,
    /// User interface allows you to build interface of any kind. UI itself is *not* thread-safe,
    /// but it uses messages to "talk" with outside world and message queue (MPSC) *is* thread-safe
    /// so its sender part can be shared across threads.   
    pub user_interface: UserInterface<M, C>,
    /// Sound context control all sound sources in the engine. It is wrapped into Arc<Mutex<>>
    /// because internally sound engine spawns separate thread to mix and send data to sound
    /// device. For more info see docs for Context.
    pub sound_engine: Arc<Mutex<SoundEngine>>,
    /// Current resource manager. Resource manager wrapped into Arc<Mutex<>> to be able to
    /// use resource manager from any thread, this is useful to load resources from multiple
    /// threads to decrease loading times of your game by utilizing all available power of
    /// your CPU.
    pub resource_manager: ResourceManager,
    /// All available scenes in the engine.
    pub scenes: SceneContainer,
    /// The time user interface took for internal needs. TODO: This is not the right place
    /// for such statistics, probably it is best to make separate structure to hold all
    /// such data.
    pub ui_time: Duration,
    /// All available 2d scenes.
    pub scenes2d: Scene2dContainer,
}

impl<M: MessageData, C: Control<M, C>> Engine<M, C> {
    /// Creates new instance of engine from given window builder and events loop.
    ///
    /// Automatically creates all sub-systems (renderer, sound, ui, etc.).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rg3d::engine::Engine;
    /// use rg3d::window::WindowBuilder;
    /// use rg3d::event_loop::EventLoop;
    /// use rg3d::gui::node::StubNode;
    ///
    /// let evt = EventLoop::new();
    /// let window_builder = WindowBuilder::new()
    ///     .with_title("Test")
    ///     .with_fullscreen(None);
    /// let mut engine: Engine<(), StubNode> = Engine::new(window_builder, &evt, true).unwrap();
    /// ```
    #[inline]
    pub fn new(
        window_builder: WindowBuilder,
        events_loop: &EventLoop<()>,
        #[allow(unused_variables)] vsync: bool,
    ) -> Result<Self, EngineError> {
        #[cfg(not(target_arch = "wasm32"))]
        let (context, client_size) = {
            let context_wrapper: glutin::WindowedContext<glutin::NotCurrent> =
                glutin::ContextBuilder::new()
                    .with_vsync(vsync)
                    .with_gl_profile(glutin::GlProfile::Core)
                    .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 3)))
                    .build_windowed(window_builder, events_loop)?;

            let ctx = match unsafe { context_wrapper.make_current() } {
                Ok(context) => context,
                Err((_, e)) => return Err(EngineError::from(e)),
            };
            let inner_size = ctx.window().inner_size();
            (
                ctx,
                Vector2::new(inner_size.width as f32, inner_size.height as f32),
            )
        };

        #[cfg(target_arch = "wasm32")]
        let (window, client_size, glow_context) = {
            let winit_window = window_builder.build(events_loop).unwrap();

            use crate::core::wasm_bindgen::JsCast;
            use crate::platform::web::WindowExtWebSys;

            let canvas = winit_window.canvas();

            let window = crate::core::web_sys::window().unwrap();
            let document = window.document().unwrap();
            let body = document.body().unwrap();

            body.append_child(&canvas)
                .expect("Append canvas to HTML body");

            let webgl2_context = canvas
                .get_context("webgl2")
                .unwrap()
                .unwrap()
                .dyn_into::<crate::core::web_sys::WebGl2RenderingContext>()
                .unwrap();
            let glow_context = glow::Context::from_webgl2_context(webgl2_context);

            let inner_size = winit_window.inner_size();
            (
                winit_window,
                Vector2::new(inner_size.width as f32, inner_size.height as f32),
                glow_context,
            )
        };

        #[cfg(not(target_arch = "wasm32"))]
        let glow_context =
            { unsafe { glow::Context::from_loader_function(|s| context.get_proc_address(s)) } };

        let sound_engine = SoundEngine::new();

        Ok(Self {
            renderer: Renderer::new(glow_context, (client_size.x as u32, client_size.y as u32))?,
            resource_manager: ResourceManager::new(),
            scenes: SceneContainer::new(sound_engine.clone()),
            sound_engine,
            user_interface: UserInterface::new(client_size),
            ui_time: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            context,
            #[cfg(target_arch = "wasm32")]
            window,
            scenes2d: Default::default(),
        })
    }

    /// Returns reference to main window. Could be useful to set fullscreen mode, change
    /// size of window, its title, etc.
    #[inline]
    pub fn get_window(&self) -> &Window {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.context.window()
        }
        #[cfg(target_arch = "wasm32")]
        {
            &self.window
        }
    }

    /// Performs single update tick with given time delta. Engine internally will perform update
    /// of all scenes, sub-systems, user interface, etc. Must be called in order to get engine
    /// functioning.
    pub fn update(&mut self, dt: f32) {
        let inner_size = self.get_window().inner_size();
        let window_size = Vector2::new(inner_size.width as f32, inner_size.height as f32);

        self.resource_manager.state().update(dt);

        for scene in self.scenes.iter_mut().filter(|s| s.enabled) {
            let frame_size = scene.render_target.as_ref().map_or(window_size, |rt| {
                if let TextureKind::Rectangle { width, height } = rt.data_ref().kind {
                    Vector2::new(width as f32, height as f32)
                } else {
                    panic!("only rectangle textures can be used as render target!");
                }
            });

            scene.update(frame_size, dt);
        }

        let time = instant::Instant::now();
        self.user_interface.update(window_size, dt);
        self.ui_time = instant::Instant::now() - time;
    }

    /// Performs rendering of single frame, must be called from your game loop, otherwise you won't
    /// see anything.
    #[inline]
    pub fn render(&mut self, dt: f32) -> Result<(), FrameworkError> {
        self.user_interface.draw();

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.renderer.render_and_swap_buffers(
                &self.scenes,
                &self.user_interface.get_drawing_context(),
                &self.scenes2d,
                &self.context,
                dt,
            )
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.renderer.render_and_swap_buffers(
                &self.scenes,
                &self.user_interface.get_drawing_context(),
                &self.scenes2d,
                dt,
            )
        }
    }
}

impl<M: MessageData, C: Control<M, C>> Visit for Engine<M, C> {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        if visitor.is_reading() {
            self.renderer.flush();
            self.resource_manager.state().update(0.0);
            self.scenes.clear();
        }

        self.resource_manager.visit("ResourceManager", visitor)?;
        self.sound_engine.visit("SoundEngine", visitor)?;
        self.scenes.visit("Scenes", visitor)?;

        if visitor.is_reading() {
            crate::core::futures::executor::block_on(self.resource_manager.reload_resources());

            let mut sound_engine = self.sound_engine.lock().unwrap();
            for scene in self.scenes.iter_mut() {
                // Fix scenes with previous format.
                if !sound_engine.has_context(&scene.sound_context) {
                    Log::writeln(
                        MessageKind::Warning,
                        "Restoring sound context of the scene!".to_owned(),
                    );
                    sound_engine.add_context(scene.sound_context.clone());
                }
                scene.resolve();
            }
        }

        visitor.leave_region()
    }
}
