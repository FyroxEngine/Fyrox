pub mod state;
pub mod resource_manager;

use crate::{
    gui::UserInterface,
    resource::{ttf::Font},
    renderer::{
        Renderer, Statistics,
        error::RendererError,
    },
    engine::state::State,
};

use std::{
    collections::VecDeque,
    path::Path,
    sync::{Arc, Mutex},
};

use rg3d_core::{
    math::vec2::Vec2,
    visitor::{Visitor, VisitResult, Visit},
};

use rg3d_sound::context::Context;
use std::rc::Rc;
use std::cell::RefCell;

pub struct Engine {
    renderer: Renderer,
    pub state: State,
    events: VecDeque<glutin::Event>,
    running: bool,
    default_font: Rc<RefCell<Font>>,
    user_interface: UserInterface,
    sound_context: Arc<Mutex<Context>>,
}

impl Engine {
    #[inline]
    pub fn new() -> Engine {
        let default_font = Rc::new(RefCell::new(Font::load(
            Path::new("data/fonts/font.ttf"),20.0,(0..255).collect()).unwrap()));

        Engine {
            sound_context: Context::new().unwrap(),
            state: State::new(),
            renderer: Renderer::new().unwrap(),
            events: VecDeque::new(),
            running: true,
            user_interface: UserInterface::new(default_font.clone()),
            default_font,
        }
    }

    #[inline]
    pub fn get_state(&self) -> &State {
        &self.state
    }

    #[inline]
    pub fn get_state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    #[inline]
    pub fn is_running(&self) -> bool {
        self.running
    }

    #[inline]
    pub fn get_sound_context(&self) -> Arc<Mutex<Context>> {
        self.sound_context.clone()
    }

    pub fn update(&mut self, dt: f32) {
        let client_size = self.renderer.context.window().get_inner_size().unwrap();
        let aspect_ratio = (client_size.width / client_size.height) as f32;

        self.state.get_resource_manager_mut().update();

        for scene in self.state.get_scenes_mut().iter_mut() {
            scene.update(aspect_ratio, dt);
        }

        self.sound_context.lock().unwrap().update().unwrap();

        self.user_interface.update(Vec2::make(client_size.width as f32, client_size.height as f32));
    }

    pub fn poll_events(&mut self) {
        // Gather events
        let events = &mut self.events;
        events.clear();
        self.renderer.events_loop.poll_events(|event| {
            events.push_back(event);
        });
    }

    #[inline]
    pub fn get_default_font(&self) -> Rc<RefCell<Font>> {
        self.default_font.clone()
    }

    #[inline]
    pub fn get_ui(&self) -> &UserInterface {
        &self.user_interface
    }

    #[inline]
    pub fn get_ui_mut(&mut self) -> &mut UserInterface {
        &mut self.user_interface
    }

    #[inline]
    pub fn get_rendering_statisting(&self) -> Statistics {
        self.renderer.get_statistics()
    }

    #[inline]
    pub fn set_frame_size(&mut self, new_size: Vec2) -> Result<(), RendererError> {
        self.renderer.set_frame_size(new_size)
    }

    #[inline]
    pub fn get_frame_size(&self) -> Vec2 {
        self.renderer.get_frame_size()
    }

    pub fn render(&mut self) -> Result<(), RendererError> {
        self.renderer.upload_resources(&mut self.state);
        self.user_interface.draw();
        self.renderer.render(&self.state, &self.user_interface.get_drawing_context())
    }

    #[inline]
    pub fn stop(&mut self) {
        self.running = false;
    }

    #[inline]
    pub fn pop_event(&mut self) -> Option<glutin::Event> {
        self.events.pop_front()
    }
}

impl Visit for Engine {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        // Make sure to delete unused resources.
        if visitor.is_reading() {
            self.state.get_resource_manager_mut().update();
            self.state.get_scenes_mut().clear();
        }

        self.state.visit("State", visitor)?;
        self.sound_context.lock()?.visit("SoundContext", visitor)?;

        if visitor.is_reading() {
            self.state.resolve();
        }

        visitor.leave_region()
    }
}
