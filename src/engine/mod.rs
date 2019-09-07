pub mod state;
pub mod resource_manager;

use crate::{
    gui::UserInterface,
    math::vec2::Vec2,
    resource::{ttf::Font},
    utils::{
        pool::{Pool, Handle},
        visitor::{Visitor, VisitResult, Visit},
    },
    renderer::render::{Renderer, Statistics},
    engine::state::State,
};

use std::{
    collections::VecDeque,
    time::Duration,
    path::Path,
};
use crate::renderer::error::RendererError;

pub struct Engine {
    renderer: Renderer,
    pub state: State,
    events: VecDeque<glutin::Event>,
    running: bool,
    font_cache: Pool<Font>,
    default_font: Handle<Font>,
    user_interface: UserInterface,
}

impl Engine {
    #[inline]
    pub fn new() -> Engine {
        let mut font_cache = Pool::new();
        let default_font = font_cache.spawn(Font::load(
            Path::new("data/fonts/font.ttf"),
            20.0,
            (0..255).collect()).unwrap());
        let mut renderer = Renderer::new().unwrap();
        renderer.upload_font_cache(&mut font_cache);
        Engine {
            state: State::new(),
            renderer,
            events: VecDeque::new(),
            running: true,
            user_interface: UserInterface::new(default_font),
            default_font,
            font_cache,
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

    pub fn update(&mut self, dt: f32) {
        let client_size = self.renderer.context.window().get_inner_size().unwrap();
        let aspect_ratio = (client_size.width / client_size.height) as f32;

        self.state.get_resource_manager_mut().update();

        for scene in self.state.get_scenes_mut().iter_mut() {
            scene.update(aspect_ratio, dt);
        }

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
    pub fn get_font(&self, font_handle: Handle<Font>) -> Option<&Font> {
        self.font_cache.borrow(font_handle)
    }

    #[inline]
    pub fn get_default_font(&self) -> Handle<Font> {
        self.default_font
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
    pub fn get_rendering_statisting(&self) -> &Statistics {
        self.renderer.get_statistics()
    }

    #[inline]
    pub fn set_frame_size(&mut self, new_size: Vec2) -> Result<(), RendererError>{
        self.renderer.set_frame_size(new_size)
    }

    #[inline]
    pub fn get_frame_size(&self) -> Vec2 {
        self.renderer.get_frame_size()
    }

    pub fn render(&mut self) -> Result<(), RendererError> {
        self.renderer.upload_font_cache(&mut self.font_cache);
        self.renderer.upload_resources(&mut self.state);
        self.user_interface.draw(&self.font_cache);
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

        if visitor.is_reading() {
            self.state.resolve();
        }

        visitor.leave_region()
    }
}

pub fn duration_to_seconds_f64(duration: Duration) -> f64 {
    duration.as_secs() as f64 + f64::from(duration.subsec_nanos()) / 1_000_000_000.0
}

pub fn duration_to_seconds_f32(duration: Duration) -> f32 {
    duration_to_seconds_f64(duration) as f32
}
