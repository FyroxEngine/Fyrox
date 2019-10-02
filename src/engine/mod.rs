pub mod resource_manager;
pub mod error;

use rg3d_core::{
    math::vec2::Vec2,
    visitor::{Visitor, VisitResult, Visit},
    pool::Pool,
};
use rg3d_sound::context::Context;
use std::sync::{Arc, Mutex};
use glutin::{
    PossiblyCurrent, GlProfile,
    GlRequest, Api, EventsLoop,
};
use crate::{
    engine::resource_manager::ResourceManager,
    gui::UserInterface,
    renderer::{Renderer, error::RendererError, gl},
    engine::error::EngineError,
    WindowBuilder, Window, scene::Scene,
};

pub struct Engine {
    context: glutin::WindowedContext<PossiblyCurrent>,
    renderer: Renderer,
    user_interface: UserInterface,
    sound_context: Arc<Mutex<Context>>,
    resource_manager: ResourceManager,
    scenes: Pool<Scene>,
}

pub struct EngineInterfaceMut<'a> {
    pub ui: &'a mut UserInterface,
    pub renderer: &'a mut Renderer,
    pub sound_context: Arc<Mutex<Context>>,
    pub resource_manager: &'a mut ResourceManager,
    pub scenes: &'a mut Pool<Scene>,
}

pub struct EngineInterface<'a> {
    pub ui: &'a UserInterface,
    pub renderer: &'a Renderer,
    pub sound_context: Arc<Mutex<Context>>,
    pub resource_manager: &'a ResourceManager,
    pub scenes: &'a Pool<Scene>,
}

impl Engine {
    #[inline]
    pub fn new(window_builder: WindowBuilder, events_loop: &EventsLoop) -> Result<Engine, EngineError> {
        let context_wrapper = glutin::ContextBuilder::new()
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

        let client_size = context.window().get_inner_size().unwrap();

        Ok(Engine {
            context,
            resource_manager: ResourceManager::new(),
            sound_context: Context::new()?,
            scenes: Pool::new(),
            renderer: Renderer::new(client_size.into())?,
            user_interface: UserInterface::new(),
        })
    }

    /// Returns reference to main window.
    /// Could be useful to set fullscreen mode, change size of window, etc.
    #[inline]
    pub fn get_window(&self) -> &Window {
        self.context.window()
    }

    /// Borrows as mutable all available components at once. Should be used in pair with
    /// destructuring, like this:
    /// ```
    /// use rg3d::engine::EngineInterfaceMut;
    ///
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
    /// ```
    /// use rg3d::engine::EngineInterface;
    ///
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

    pub fn update(&mut self, dt: f32) {
        let client_size = self.context.window().get_inner_size().unwrap();
        let aspect_ratio = (client_size.width / client_size.height) as f32;

        self.resource_manager.update();

        for scene in self.scenes.iter_mut() {
            scene.update(aspect_ratio, dt);
        }

        self.sound_context.lock().unwrap().update().unwrap();

        self.user_interface.update(Vec2::make(client_size.width as f32, client_size.height as f32));
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

        // Make sure to delete unused resources.
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
