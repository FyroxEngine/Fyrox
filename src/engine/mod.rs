use crate::scene::*;
use crate::utils::pool::*;
use crate::renderer::renderer::*;

pub struct Engine {
    renderer: Renderer,
    scenes: Pool<Scene>,
    events: Vec<glutin::Event>,
    running: bool,
}

impl Engine {
    pub fn new() -> Engine {
        Engine {
            scenes: Pool::new(),
            renderer: Renderer::new(),
            events: Vec::new(),
            running: true,
        }
    }

    pub fn add_scene(&mut self, scene: Scene) -> Handle<Scene> {
        self.scenes.spawn(scene)
    }

    pub fn borrow_scene(&self, handle: &Handle<Scene>) -> Option<&Scene> {
        if let Some(scene) = self.scenes.borrow(handle) {
            return Some(scene);
        }
        None
    }

    pub fn borrow_scene_mut(&mut self, handle: &Handle<Scene>) -> Option<&mut Scene> {
        if let Some(scene) = self.scenes.borrow_mut(handle) {
            return Some(scene);
        }
        None
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn update(&mut self) {
        let client_size = self.renderer.context.get_inner_size().unwrap();
        let aspect_ratio = (client_size.width / client_size.height) as f32;

        for i in 0..self.scenes.capacity() {
            if let Some(scene) = self.scenes.at_mut(i) {
                scene.update(aspect_ratio);
            }
        }
    }

    pub fn poll_events(&mut self) {
        // Gather events
        let events = &mut self.events;
        events.clear();
        self.renderer.events_loop.poll_events(|event| {
            events.push(event);
        });
    }

    pub fn render(&mut self) {
        let mut alive_scenes: Vec<&Scene> = Vec::new();
        for i in 0..self.scenes.capacity() {
            if let Some(scene) = self.scenes.at(i) {
                alive_scenes.push(scene);
            }
        }
        self.renderer.render(alive_scenes.as_slice());
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn pop_event(&mut self) -> Option<glutin::Event> {
        self.events.pop()
    }
}
