pub mod resource_manager;
pub mod error;

use rg3d_core::{
    math::vec2::Vec2,
    visitor::{Visitor, VisitResult, Visit},
};
use rg3d_sound::context::Context;
use std::sync::{Arc, Mutex};
use crate::{
    engine::{resource_manager::ResourceManager, error::EngineError},
    gui::UserInterface,
    renderer::{Renderer, error::RendererError, gl},
    window::{WindowBuilder, Window},
    scene::SceneContainer,
    PossiblyCurrent,
    GlRequest,
    GlProfile,
    WindowedContext,
    NotCurrent,
    Api,
    event_loop::EventLoop
};

pub struct Engine {
    context: glutin::WindowedContext<PossiblyCurrent>,
    renderer: Renderer,
    user_interface: UserInterface,
    sound_context: Arc<Mutex<Context>>,
    resource_manager: ResourceManager,
    scenes: SceneContainer,
}

pub struct EngineInterfaceMut<'a> {
    pub ui: &'a mut UserInterface,
    pub renderer: &'a mut Renderer,
    pub sound_context: Arc<Mutex<Context>>,
    pub resource_manager: &'a mut ResourceManager,
    pub scenes: &'a mut SceneContainer,
}

pub struct EngineInterface<'a> {
    pub ui: &'a UserInterface,
    pub renderer: &'a Renderer,
    pub sound_context: Arc<Mutex<Context>>,
    pub resource_manager: &'a ResourceManager,
    pub scenes: &'a SceneContainer,
}

impl Engine {
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
    pub fn new(window_builder: WindowBuilder, events_loop: &EventLoop<()>) -> Result<Engine, EngineError> {
        let context_wrapper: WindowedContext<NotCurrent> = glutin::ContextBuilder::new()
            .with_vsync(true)
            .with_gl_profile(GlProfile::Core)
            .with_gl_debug_flag(true)
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
            resource_manager: ResourceManager::new(),
            sound_context: Context::new()?,
            scenes: SceneContainer::new(),
            renderer: Renderer::new(client_size.into())?,
            user_interface: UserInterface::new(),
        })
    }

    /// Returns reference to main window.  Could be useful to set fullscreen mode, change
    /// size of window, its title, etc.
    #[inline]
    pub fn get_window(&self) -> &Window {
        self.context.window()
    }

    /// Borrows as mutable all available components at once. Should be used in pair with
    /// destructuring, like this:
    /// ```no_run
    /// use rg3d::engine::{Engine, EngineInterfaceMut};
    /// use rg3d::window::WindowBuilder;
    /// use rg3d::event_loop::EventLoop;
    ///
    /// let evt = EventLoop::new();
    /// let mut engine = Engine::new(WindowBuilder::new(), &evt).unwrap();
    /// let EngineInterfaceMut{scenes, ui, renderer, sound_context, resource_manager} = engine.interface_mut();
    /// ```
    /// This is much more easier than if there would be separate `get_resource_manager_mut()`,
    /// `get_ui_mut()`, etc. Also this allows you to have mutable references to all
    /// components at once, which would be impossible if there would be separate methods.
    #[inline]
    pub fn interface_mut(&mut self) -> EngineInterfaceMut {
        EngineInterfaceMut {
            scenes: &mut self.scenes,
            renderer: &mut self.renderer,
            ui: &mut self.user_interface,
            sound_context: self.sound_context.clone(),
            resource_manager: &mut self.resource_manager,
        }
    }

    /// Borrows all available components at once. Should be used in pair with destructuring,
    /// like this:
    /// ```no_run
    /// use rg3d::engine::{EngineInterface, Engine};
    /// use rg3d::window::WindowBuilder;
    /// use rg3d::event_loop::EventLoop;
    ///
    /// let evt = EventLoop::new();
    /// let engine = Engine::new(WindowBuilder::new(), &evt).unwrap();
    /// let EngineInterface{scenes, ui, renderer, sound_context, resource_manager} = engine.interface();
    /// ```
    /// This is much more easier than if there would be separate `get_resource_manager()`,
    /// `get_ui()`, etc.
    #[inline]
    pub fn interface(&self) -> EngineInterface {
        EngineInterface {
            scenes: &self.scenes,
            ui: &self.user_interface,
            renderer: &self.renderer,
            resource_manager: &self.resource_manager,
            sound_context: self.sound_context.clone(),
        }
    }

    /// Performs single update tick with given time delta. Engine internally will perform update
    /// of all scenes, sub-systems, user interface, etc. Must be called in order to get engine
    /// functioning.
    pub fn update(&mut self, dt: f32) {
        let client_size = self.context.window().inner_size();
        let aspect_ratio = (client_size.width / client_size.height) as f32;

        self.resource_manager.update();

        for scene in self.scenes.iter_mut() {
            scene.update(aspect_ratio, dt);
        }

        self.sound_context.lock().unwrap().update().unwrap();

        self.user_interface.update(Vec2::new(client_size.width as f32, client_size.height as f32), dt);
    }

    pub fn get_ui_mut(&mut self) -> &mut UserInterface {
        &mut self.user_interface
    }

    #[inline]
    pub fn render(&mut self) -> Result<(), RendererError> {
        self.renderer.upload_resources(&mut self.resource_manager);
        self.user_interface.draw();
        self.renderer.render(&self.scenes, &self.user_interface.get_drawing_context(), &self.context)
    }
}

impl Visit for Engine {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        if visitor.is_reading() {
            self.resource_manager.update();
            self.scenes.clear();
        }

        self.resource_manager.visit("ResourceManager", visitor)?;
        self.scenes.visit("Scenes", visitor)?;
        self.sound_context.lock()?.visit("SoundContext", visitor)?;

        if visitor.is_reading() {
            self.resource_manager.reload_resources();
            for scene in self.scenes.iter_mut() {
                scene.resolve();
            }
        }

        visitor.leave_region()
    }
}
